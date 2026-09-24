# E-004c — Rust→Android leg (build sub-leg run 2026-09-24)

- **Question**: can the Rust core ship as an Android aarch64 `.so` with the full
  UniFFI FFI surface, without a device?
- **Result**: **BUILD LEG PASS** (artifact verified); runtime JNI leg remains
  device-bound. Raw record: `experiments/E-004c_result.txt`.

## Method

1. Attempted `cargo-zigbuild` 0.23.4 + zig 0.13: **disqualified for Android** — zig's
   bundled libc set cannot satisfy bionic `-ldl/-llog/-lm/-lc` ("no_fallback" search
   fails). Zig stays useful for glibc/musl cross builds; Android needs the NDK.
2. Real NDK: `android-ndk-r27c-linux.zip` (664 MB, dl.google.com), extracted
   `toolchains/llvm/` only.
3. Built the E-004b UniFFI crate (`ove-time-ffi`, uniffi 0.32) with
   `CARGO_TARGET_AARCH64_LINUX_ANDROID_LINKER=…/aarch64-linux-android24-clang`,
   target `aarch64-linux-android`, release.

## Evidence

- Artifact `libove_time_ffi.so` 654,568 B — `readelf`: *ELF 64-bit LSB shared object,
  ARM aarch64, for Android 24, built by NDK r27c*.
- Exported dyn-syms (readelf --dyn-syms): the complete exact-time surface E-004b
  round-tripped live through Kotlin/JNA — `uniffi_ove_time_ffi_fn_func_ove_time_add`,
  `_sub`, `_half`, `_floor_frame`, `_split_roundtrip`, plus per-function checksum
  symbols. The E-004b generated Kotlin (1,423 lines) resolves these by name via JNA;
  name match verified structurally here.
- `DT_NEEDED`: `libdl.so`, `libc.so` only.

## Findings

1. The only unknown this environment could answer — "does the core cross-compile with
   an intact FFI surface" — is answered **yes**. ADR-001's device-leg risk narrows to
   JNI *runtime* behavior (load, callback dispatch, panic containment across JNI —
   E-004a proved the containment pattern host-side).
2. NDK build leg is CI-automatable (ubuntu-latest + NDK); add to ci.yml when the
   second engine crate lands.
3. minSdk 24 chosen (NDK wrapper; matches Media3's floor and modern coverage).

## Residual (kept honest)

Runtime leg NOT RUN: `System.loadLibrary("ove_time_ffi")` + uniffi Kotlin runtime +
JNI callback dispatch require a device/emulator. This is the only remaining E-004c
residual; tracked in research/gates/GATE_STATUS.md.
