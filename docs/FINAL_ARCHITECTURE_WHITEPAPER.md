# FINAL ARCHITECTURE WHITEPAPER (v1 — evidence-backed, 2026-09-26)

> Directive §"ONLY NOW: ARCHITECTURE" deliverable. Status: **PROVISIONAL FINAL** — the
> architecture is stated in full, every element carries its evidence, and the three
> hardware-bound residuals are named as explicit reopen conditions rather than blockers
> for the vertical slice. This is the document ADR-002 will point at when it ACCEPTs.

## 1. Mission restatement (what is being architected)

A universal open-source video engine: media pipeline + timeline + compositor + project
model + AI/agent command API — where editors (Android/browser/desktop), agents, scripts,
and servers are all clients of the same core. Not a UI project; engine-first; human and
AI share one command path.

## 2. Selected architecture: layered hybrid (Architecture C)

One Rust core owning **everything that must be identical across platforms**; platform
adapters owning **everything that cannot be identical**:

| Layer | Owner | Contents | Evidence |
|---|---|---|---|
| Exact time | shared core | ove-time: exact rationals, fixed tick axis for aggregates, fp forbidden on authoritative path | ADR-007 ACCEPTED; E-002 13/13; 15/15 live; E-002c P3b |
| Timeline & commands | shared core | clip model + gap buffer + derived index; one command API (human+AI); inverse-command undo; state = fold(command log) | ADR-006/009/010/011; E-003 9/9; E-009 36/36; E-002c/c2 |
| Project model | shared core | manifest + append-only command log + content-addressed assets + snapshots | ADR-008; E-003; doc 19/31 |
| Render graph | shared core (compiler) | deterministic compile: timeline state → ordered passes; WGSL shader library where GPU exists | doc 08; RENDER_GRAPH_SPEC |
| Frame contract | shared core (types) | FrameEnvelope: timestamps, geometry, pixel format, color metadata, memory location, ownership, backend id | FRAME_CONTRACT (new) |
| Media execution | platform adapters | decode/encode/mux per leg: FFmpeg-SW (desktop/headless), MediaCodec (Android), WebCodecs+mediabunny (browser) | ADR-003/004/005; E-001; E-007b |
| GPU execution | platform adapters | wgpu (desktop/headless), platform surface paths (Android), WebGPU+fallback (browser) — two-level abstraction: core speaks import/copy intents | C-002; E-001; ARCHITECTURE_AUDIT #4 |
| Shells | platform | Kotlin/UniFFI+JNI (Android), WASM+browser APIs (web), Tauri (desktop), CLI (headless) | ADR-001; E-004a/b; E-006 |

**Why not A (Rust owns all media):** browser forbids it (~40% surface), FFmpeg churn hits
core, LGPL entangles core (doc 05/11/18/35).
**Why not B (per-platform everything):** semantic consistency across 4 media stacks is
the historical NLE failure mode (doc 03/07); AI/command API would need per-platform re-binding.
**Why C survives attack:** ARCHITECTURE_AUDIT §2 attacked 10 abstractions; C held with 3
amendments (frame contract, mux/encoder split, capability-gated web).

## 3. The engine's product definition (what makes it "an engine")

The engine's deliverables are: (1) the command API with its verbs + inverses + replay
determinism, (2) the render-graph compiler + golden determinism, (3) the frame contract,
(4) the project format. All four are testable contracts, not code shipments. Decoders,
encoders, GPU backends are swappable behind them. This is why the vertical slice can
start with a software FFmpeg adapter and later gain hardware paths without architectural
change.

## 4. Determinism and correctness spine

1. Exact rationals everywhere authoritative; fixed tick axis for aggregates (P3b rule).
2. State = fold(command log); replay hash-equality is a CI property test, per verb.
3. Render graph compile is a pure function of project state (same state → same passes).
4. Golden frames + golden MP4s (ffprobe-verified) committed as test oracles.
5. Undo = exact inverses; batch = composite inverse; undo markers replay (E-009).

## 5. Platform capability matrix (v1 slice scope)

| Capability | Desktop/headless | Android | Browser |
|---|---|---|---|
| Probe/asset | FFmpeg-SW (shared logic) | FFmpeg-SW or MediaExtractor adapter | mediabunny+WebCodecs probing |
| Decode | FFmpeg-SW (hw later) | MediaCodec (E-004c pending) | WebCodecs (E-001 ✓ decode leg) |
| Render | software raster → wgpu | software → Surface/GPU (later) | software/WebGL2 → WebGPU (feature-gated) |
| Encode/mux | FFmpeg LGPL + StreamCopy route | MediaCodec (later) | WebCodecs+mediabunny (later) |
| Projects | shared format (identical bytes) | same | same (OPFS storage) |
| Validation level today | SOFTWARE | SIMULATED→NONE | SOFTWARE (decode leg only) |

Runtime feature detection is mandatory on web; no "universal browser support" claims.

## 6. Open residuals (explicit reopen conditions — not hand-waving)

| Residual | Reopens | Trigger |
|---|---|---|
| E-005 real-GPU compositor | GATE-4, GPU adapter design | golden-parity fail or importExternalTexture single-use forcing per-layer copies at 1080p60 |
| E-004c device JNI/UniFFI | GATE-5, ADR-001 revisit clause | on-device roundtrip failure or lifecycle kill |
| E-006b real Tauri transport | GATE-7, desktop shell design | channel/custom-protocol cannot carry frames near budget |
| Version-bump replay determinism | ADR-008 final ACCEPT | old-log replay on new engine ≠ hash-equal → snapshot-only fallback |

## 7. What this whitepaper deliberately does NOT claim

- Not production-validated anywhere (no DEVICE/REAL_GPU/CROSS_PLATFORM evidence exists).
- Not performance-complete (budgets set in doc 32; perf benches separate from correctness).
- Not final on GPU internals (E-005), Android execution (E-004c), desktop transport (E-006b).
- The whitepaper is the architecture record; ENGINE_BUILD_PLAN.md is the build order;
  the specs are the contracts. Together they are the "final architecture" deliverable set
  required by the directive, issued under PROVISIONAL FINAL status with named residuals.
