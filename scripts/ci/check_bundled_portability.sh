#!/bin/sh
# Regression guard for the WAVE 0.5 SIGILL class (ADR-012):
# non-portable (beyond x86-64 baseline) compiler-emitted code inside the
# bundled FFmpeg build output.
#
# What it checks — EVIDENCE, not recorded strings:
#   (1) Every C-compiled object in the bundled build is scanned for SIMD
#       beyond the SSE2 baseline (ymm/zmm/k-mask registers). "C-compiled"
#       = has a sibling .d dependency file (FFmpeg's -MMD is C-only; nasm
#       objects have none), so runtime-dispatched hand-written .asm with
#       legitimate ymm/zmm is excluded by construction.
#   (2) If the wrapper's rewrite marker exists, report the rewrite happened.
#
# Why not grep ffbuild/config.mak for -march=native: config.mak RECORDS the
# --extra-cflags string passed by ffmpeg-sys-next verbatim, while the
# EFFECTIVE flags are rewritten per-gcc-invocation by gcc_portable.sh —
# the string grep produces false positives by design. Object bytes are the
# ground truth of what will execute on a restored-cache runner.
#
# Passes silently when the bundled feature was not built in this workspace.
set -e

cd "$(dirname "$0")/../../engine"

fail=0
checked=0
OBJROOTS=$(ls -d target/release/build/ffmpeg-sys-next-*/out/ffmpeg-* 2>/dev/null || true)

if [ -z "$OBJROOTS" ]; then
    echo "no bundled FFmpeg build output found in target/ -> nothing to check"
    exit 0
fi

if [ -f /tmp/ove-portable-gcc/marked ]; then
    echo "wrapper rewrite marker present (gcc_portable.sh intercepted this build)"
fi

if ! command -v objdump >/dev/null 2>&1; then
    echo "objdump not available -> cannot verify object portability"
    exit 1
fi

for root in $OBJROOTS; do
    echo "== scanning C-compiled objects under $root"
    for d in libavutil libavcodec libavformat libswresample libswscale libavfilter; do
        [ -d "$root/$d" ] || continue
        for o in "$root/$d"/*.o; do
            [ -f "$o" ] || continue
            [ -f "${o%.o}.d" ] || continue   # C-only discriminator
            checked=$((checked + 1))
            if objdump -d "$o" 2>/dev/null | grep -Eq '%zmm|%ymm[0-9]|%k[0-7]'; then
                echo "FAIL: $o contains SIMD beyond the x86-64 baseline (SIGILL class)"
                fail=1
            fi
        done
    done
done

echo "scanned $checked C-compiled objects"
if [ "$fail" != "0" ]; then
    echo
    echo "This is the WAVE 0.5 SIGILL failure class (ADR-012). Do not weaken this"
    echo "guard; fix the build flags so bundled artifacts are x86-64-baseline portable."
    exit 1
fi
echo "portability guard: PASS"
