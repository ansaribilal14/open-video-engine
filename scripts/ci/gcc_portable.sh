#!/bin/sh
# Portable gcc wrapper for the CI bundled-FFmpeg build.
#
# WHY THIS EXISTS (WAVE 0.5 SIGILL fix, see ADR-012):
#   ffmpeg-sys-next 7.1.3 build.rs hardcodes
#     configure.arg("--extra-cflags=-march=native -mtune=native")
#   for every host build. GCC then bakes the BUILD machine's ISA into the
#   libav C objects (e.g. `vmovdqu8 %zmm0` inlined into h264dec.o /
#   mpegvideo.o — verified by objdump on a live build). Those objects are
#   NOT runtime-dispatched (FFmpeg's runtime dispatch covers its hand-written
#   .asm, not compiler-chosen instructions). Swatinem/rust-cache restores
#   target/ across GitHub runners whose CPUs differ, and the conformance
#   binary then dies with SIGILL on any runner missing an instruction the
#   build runner had.
#
# FIX: rewrite the native tuning to the x86-64 baseline (SSE2), which every
# x86-64 runner executes. FFmpeg's hand-written SIMD remains fully
# runtime-dispatched, so decode coverage/perf features are unchanged; only
# compiler-chosen baseline changes. Tradeoff recorded in ADR-012.
#
# INSTALL: symlink this file as `gcc` in a private dir placed FIRST on PATH
# for the cargo build (see .github/workflows/ci.yml). The basename MUST be
# `gcc`: ffmpeg-sys-next derives FFmpeg's --cross-prefix from the compiler
# basename, and any other name would break the host build.

REAL_GCC=/usr/bin/gcc

if [ ! -x "$REAL_GCC" ]; then
    # resolve the real gcc once, in case the distro puts it elsewhere
    REAL_GCC="$(command -v gcc || command -v gcc-12 || command -v gcc-13 || command -v gcc-14)"
    if [ -z "$REAL_GCC" ] || [ "$REAL_GCC" = "$0" ]; then
        echo "gcc_portable: cannot locate a real gcc behind the wrapper" >&2
        exit 127
    fi
fi

args=""
found_native=0
for a in "$@"; do
    case "$a" in
        -march=native)  args="$args -march=x86-64";  found_native=1 ;;
        -mtune=native)  args="$args -mtune=generic"; found_native=1 ;;
        *)              args="$args $a" ;;
esac
done

if [ "$found_native" = "1" ]; then
    # loud, single-line, rate-limited marker (CI greps the build log in the guard step)
    mkdir -p /tmp/ove-portable-gcc
    if [ ! -f /tmp/ove-portable-gcc/marked ]; then
        touch /tmp/ove-portable-gcc/marked
        echo "gcc_portable: rewrote -march=native/-mtune=native -> -march=x86-64/-mtune=generic (portable bundled build)" >&2
    fi
fi

exec "$REAL_GCC" $args
