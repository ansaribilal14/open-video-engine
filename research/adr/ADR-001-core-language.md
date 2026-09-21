# ADR-001: Core language

- **Status**: PROPOSED (blocked on GATE-1..7 + E-004)
- **Date**: 2026-09-21 · **Confidence**: MEDIUM

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
(Open) Provisional: Rust core + Kotlin/TS shells. Falsified if E-004 breaks.

## REJECTED ALTERNATIVES
D (cannot reach desktop/Android natively); B (safety + hiring + FFI churn).

## CONFIDENCE & RISK
MEDIUM. Revisit trigger: E-004 failure, UniFFI panic-semantics gaps (Q: doc 14).
