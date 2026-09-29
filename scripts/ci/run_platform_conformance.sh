#!/bin/sh
# Platform conformance runner (waves 10–12; CROSS_PLATFORM_CONFORMANCE_PLAN §3).
#
# Proves the §3 invariants across targets with REAL execution, not claims:
#   1. native run of the canonical scenario      (L-0 reference)
#   2. wasm32-wasip1 build + wasmtime run        (L-3 core evidence)
#   3. identical stdout required (state hashes, undo depth, exact
#      rationals, keyframe evaluation) — any diff is a hard failure
#   4. optional: aarch64-linux-android cargo check (L-2 compile evidence;
#      requires the rustup target; no NDK needed for check)
#
# wasmtime is fetched as a prebuilt binary (no cargo install build cost).
set -e
cd "$(dirname "$0")/../../engine"

WASMTIME_VERSION="${WASMTIME_VERSION:-36.0.1}"
WORK="$(mktemp -d)"
trap 'rm -rf "$WORK"' EXIT

echo "== [1/4] native canonical scenario =="
cargo run -q -p ove-conformance --release --bin platform_core -- "$WORK/native" > "$WORK/native.out"
cat "$WORK/native.out"

echo "== [2/4] wasm32-wasip1 build =="
rustup target add wasm32-wasip1 >/dev/null 2>&1 || true
cargo build -q -p ove-conformance --release --target wasm32-wasip1 --bin platform_core

echo "== [3/4] wasmtime run (L-3 core evidence) =="
if ! command -v wasmtime >/dev/null 2>&1; then
  curl -sL -o "$WORK/wasmtime.tar.xz" \
    "https://github.com/bytecodealliance/wasmtime/releases/download/v${WASMTIME_VERSION}/wasmtime-v${WASMTIME_VERSION}-x86_64-linux.tar.xz"
  tar xf "$WORK/wasmtime.tar.xz" -C "$WORK"
  WASMTIME="$WORK/wasmtime-v${WASMTIME_VERSION}-x86_64-linux/wasmtime"
else
  WASMTIME="wasmtime"
fi
cp target/wasm32-wasip1/release/platform_core.wasm "$WORK/"
# the guest's cwd maps to the preopen root: relative project dirs only
( cd "$WORK" && "$WASMTIME" run --dir . platform_core.wasm wasm > "$WORK/wasi.out" )
cat "$WORK/wasi.out"

echo "== [4/4] native == wasm output diff =="
if diff -u "$WORK/native.out" "$WORK/wasi.out"; then
  echo "PLATFORM-CONFORMANCE: native and wasm32-wasip1 outputs are IDENTICAL"
else
  echo "PLATFORM-CONFORMANCE FAILURE: outputs diverge (§3 invariant violated)" >&2
  exit 1
fi

# L-2 compile evidence (optional — target may not be installed everywhere)
if rustup target list --installed | grep -q aarch64-linux-android; then
  for c in ove-time ove-timeline ove-media ove-render ove-project ove-conformance; do
    cargo check -q -p "$c" --release --target aarch64-linux-android
    echo "ANDROID-CHECK: $c compiles for aarch64-linux-android"
  done
else
  echo "ANDROID-CHECK: skipped (aarch64-linux-android target not installed)"
fi
