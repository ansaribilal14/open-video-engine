# ADR-001: Core language

- **Status**: ACCEPTED (2026-09-24 — E-004a 5/5, E-004b 8/8; falsification condition never triggered)
- **Date**: 2026-09-21 · **Confidence**: MEDIUM-HIGH

## CONTEXT
One core must serve desktop, Android (JNI/UniFFI), browser (WASM), and headless CLI.
Owner profile: strong Kotlin/Android + working Rust (source ledger S-001 team evidence).

## OPTIONS
A. Rust core · B. C++ core · C. Kotlin Multiplatform core · D. TypeScript-only (web-first)

## EVIDENCE
- New-gen editor convergence on Rust core: OpenCut rewrite (crates/*), Clypra (Tauri+wgpu),
  Cutlass (20 crates), OpenReelio — [02, 39]
- Rust→Android bridge verified viable: UniFFI 0.32 used extensively by Mozilla; jni crate
  for platform plumbing; cargo-ndk packaging — [14] (residual risk R-01 → E-004)
- Browser: Rust core compiles to WASM for model layer; media I/O is browser-native
  regardless — [12, 18]
- C++ counterpoint: FFmpeg/GStreamer are C — bindings needed from any language; C++ adds
  no safety; [05, 06]
- KMP counterpoint: strongest for owner's Kotlin skill, but wasm target maturity + GPU
  ecosystem (wgpu) favor Rust; [13]

## DECISION
**Rust core + Kotlin/TS shells.** ACCEPTED 2026-09-24: the stated falsification condition
("E-004 breaks") did not occur — E-004a (raw FFI boundary: exact i64 time across FFI,
panic containment, handle safety) 5/5 PASS, E-004b (UniFFI 0.32 codegen → unmodified
Kotlin compiles → live JNA roundtrip incl. boundary-floor exactness) 8/8 PASS. First
crate `engine/ove-time` TESTED 15/15. Residual: E-004c on-device JNI validation is
tracked under the Android architecture ADR, not this language decision.

## REJECTED ALTERNATIVES
D (cannot reach desktop/Android natively); B (safety + hiring + FFI churn).

## CONFIDENCE & RISK
MEDIUM-HIGH. Revisit trigger: E-004c on-device failure, UniFFI panic-semantics gaps (Q: doc 14).
