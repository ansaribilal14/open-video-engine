# ADR-002: Core architecture — layered hybrid engine

- **Status**: ACCEPTED (2026-09-24) — architecture-shape evidence complete for this
  sandbox; acceptance exercised what the environment can answer: E-005 correctness
  leg PASS (shared wgpu compositor math, 4096/4096 px exact, software Vulkan),
  E-004c build leg PASS (core ships as Android aarch64 .so via NDK), E-004a/b FFI +
  UniFFI legs, E-006/E-006a IPC costs quantified on BOTH sides, E-003/E-009 shared
  command surface 36/36. **Named runtime-validation revisit triggers** (not open
  decisions): E-005 real-GPU perf + zero-copy import, E-004c runtime JNI leg,
  E-006b real-Tauri transport leg (environment-bound: webkit2gtk + display).
  A re-opened trigger that contradicts the layering re-opens this ADR.
- **Date**: 2026-09-21 · **Confidence**: MEDIUM

## CONTEXT
Directive end-state: one engine, clients = Android app, web app, desktop app, AI, MCP,
scripts, headless. Must define what is shared vs platform-bound.

## OPTIONS
A. Rust-first unified engine (own all media) · B. Native-per-platform media + shared spec
· C. Layered hybrid: shared model/graph core + platform media adapters

## EVIDENCE
Full comparison: [40_ARCHITECTURE_COMPARISON]. Key lines:
- Browser makes A's "own all media" false at ~40% of the surface (WebCodecs/Mediabunny
  are browser-native regardless) — [11, 18]
- B's four-stack parity is the historical NLE failure mode — [03, 07]
- Independent convergence: browser layered-sharing analysis (SHARED=model/graph/shaders/
  color; ADAPTER=media I/O + GPU import; PLATFORM-SPECIFIC=probing/storage) — [18]
- Capability matrix from browser+Android research supports C — [14, 15, 16, 18]

## DECISION
(Open) Provisional: C. Pass-graph compiler + command system are the engine's product;
decoders/encoders are swappable adapters.

## REJECTED ALTERNATIVES
A: strongest consistency story, falsified by browser reality. B: best raw per-platform
performance, worst determinism/maintenance.

## CONFIDENCE & RISK
MEDIUM. Falsification: R-01 trait leakage, E-001 importExternalTexture limits, E-003
replay determinism, E-006 IPC costs — any failure re-opens toward B per-platform splits.
