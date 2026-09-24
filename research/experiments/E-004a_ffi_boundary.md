# E-004a — Rust FFI boundary pattern (JNI/UniFFI proxy layer semantics)

> Status: RUN-COMPLETE (5/5 checks). Full Android device path (UniFFI Kotlin codegen,
> MediaCodec surface wiring) remains PLANNED — requires NDK/device.

- **QUESTION**: Do exact-rational times, opaque handles, error-code returns, and panic
  containment survive the Rust FFI boundary cleanly?
- **HYPOTHESIS**: A `#[repr(C)]` ABI with i64 (num,den) pairs, Box::into_raw handles,
  sentinel errors, and `catch_unwind` at every boundary is viable — the pattern UniFFI/JNI
  will generate.
- **IMPLEMENTATION**: `scripts/experiments/E-004a_ffi_boundary.rs` (cdylib) +
  `E-004a_driver.py` (ctypes driver standing in for the JNI proxy: same ABI questions).
- **HARDWARE**: container (x86_64 Linux, Rust 1.98.1).
- **RESULT** (5/5 PASS):
  1. Exact rational end-time across boundary: start 1001/24000 + dur 48000·1001/30000 →
     192197005/120000 == Fraction-exact expectation (no float on the wire).
  2. Opaque handle free completes (Rust-side drop).
  3. Null handle → sentinel (-1/1), no crash.
  4. Zero denominator rejected at construction (null returned).
  5. **Panic contained at boundary**: internal panic caught by catch_unwind, error code 3
     returned, process alive. (Contained-panic stderr trace is expected output.)
- **LIMITATIONS**: ctypes driver ≠ real JNI (threading attach, exception translation,
  callback marshalling untested — documented ADR-001 residual risks); UniFFI codegen
  still to run; Android mediaProcessing FGS budget constraints (doc 14) untested.
- **DECISION**: ADR-001 (Rust core) FFI-risk reduced: the exact-time ABI pattern works.
  Residual R-01 (adapter trait leakage) unchanged → E-004b (UniFFI Kotlin codegen) and
  E-004c (real JNI thread/exception tests) planned.
