#!/bin/sh
# LOCAL-ONLY build environment for engine/ove-decode (sandbox without root).
# CI does NOT need this: GitHub runners install libclang-dev + nasm via apt and
# ffmpeg-sys-next's `build` feature compiles a pinned FFmpeg 7.1 from source.
# Locally we use distro FFmpeg 7.1.5 dev files obtained root-free via
# `apt-get download libav*-dev nasm libclang1-19 libclang-common-19-dev
# libllvm19 libz3-4` + dpkg -x into $FFDEV. ABI: avformat 61.7.103,
# avcodec 61.19.101, avutil 59.39.100. Proven by scripts/experiments/W2_sysprobe.
FFDEV="$HOME/my-project/ffmpeg-dev"
INCDIR="$FFDEV/usr/include/x86_64-linux-gnu"     # Debian multi-arch headers
export PATH="$FFDEV/usr/bin:$PATH"                       # nasm 2.16 (x86asm)
export LIBCLANG_PATH="$FFDEV/usr/lib/llvm-19/lib"        # libclang-19 (bindgen)
export BINDGEN_EXTRA_CLANG_ARGS="-resource-dir $FFDEV/usr/lib/llvm-19/lib/clang/19 -I$INCDIR"
export PKG_CONFIG_PATH="$FFDEV/usr/lib/x86_64-linux-gnu/pkgconfig"
export PKG_CONFIG_SYSROOT_DIR="$FFDEV"
export CFLAGS="${CFLAGS:-} -I$INCDIR"
export RUSTFLAGS="${RUSTFLAGS:-} -L $FFDEV/usr/lib/x86_64-linux-gnu"
