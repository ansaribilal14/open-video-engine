# STATUS — Honest Subsystem Classification

> Directive rule: **NO FAKE COMPLETION.** Classifications allowed:
> IMPLEMENTED · TESTED · BENCHMARKED · EXPERIMENTAL · PARTIAL · PLANNED · BLOCKED · UNKNOWN
> Nothing may be upgraded without evidence committed to this repository.

Last updated: 2026-09-26 (v0.6 wave-4b: E-012 timeline integration — engine/ove-timeline TESTED 10/10 + budget-measured bench → ADR-011 ACCEPTED REVISED (augmented AVL primary); schema rule #4 explicit allocation; 7 of 11 ADRs accepted; E-012 supersedes the Phase-2 blocker for ove-timeline)

## Mission phases

| Phase | Scope | Status |
|---|---|---|
| PHASE 0 | Research infrastructure, source ledger, claim ledger | **PARTIAL** (scaffolding done; 137 sources ledgered; 7 claims tracked with evidence) |
| PHASE 0 | Track research (editors, media, timeline, GPU, web, Android, desktop, AI, MCP + audio, captions, CV, plugins, scripting, headless, storage, performance, security, UX, licenses, YouTube, transcripts) | **PARTIAL** (v0.2: 39 of 41 research docs written, ~7,600+ lines; 13 wave-3 docs added 2026-09-24; depth requirement NOT yet met) |
| PHASE 0 | Cross-track synthesis + knowledge base | **PARTIAL** (v0.6: ADR-001/002/007/008/009/010/011 **ACCEPTED** (011 revised at integration 2026-09-26); ADR-003/004/005/006 PROPOSED with named evidence legs; GATE_STATUS.md: 13/14 gates UNDERSTOOD, 1 PARTIAL environment-bound) |
| PHASE 0 | Architecture gates 1–14 | **PARTIAL** — 13 UNDERSTOOD (several v0.1 with named residuals) / 1 PARTIAL (gate 7: E-006b needs webkit2gtk+display, no root) / 0 GAP (research/gates/GATE_STATUS.md) |
| PHASE 0 | Experiments & benchmarks | **PARTIAL** — 10 experiments RUN + 2 partial legs (E-001 partial, E-002 13/13, E-002c, E-003 9/9, E-004a 5/5, E-004b 8/8, E-006 browser-leg, E-007+E-007b, E-009 36/36, **E-012: 10/10 properties + cross-validated bench**; E-005 correctness leg 4096/4096 PASS software Vulkan, E-004c build leg PASS NDK aarch64); ove-time TESTED 15/15; **ove-timeline TESTED 10/10**; hardware/runtime residuals: E-004c runtime (device), E-005 real-GPU perf + zero-copy, E-006b (webkit2gtk) |
| PHASE 0 | CI | **TESTED-infra** — GitHub Actions ci.yml live (fmt + clippy -D warnings + tests + cargo-audit) on engine/ove-time; results verified after push |
| PHASE 1 | Core media abstraction | **PLANNED** (first crate landed: engine/ove-time TESTED + fmt/clippy-clean; CI-green) |
| PHASE 2 | Timeline engine | **PLANNED** |
| PHASE 3 | Project system | **PLANNED** |
| PHASE 4–22 | Decode/playback → production hardening | **PLANNED** |

## Research documents (docs/research/)

| Doc | Topic | Status |
|---|---|---|
| 41_EXPERIMENTS | Experiment index + records | PARTIAL (E-001/002/002c/003/004a/004b/004c/005/006/007/007b/009/**012** run + ove-time suite; E-006b/E-008/E-010/E-011/E-013 defined) |
| 01_RESEARCH_INDEX | Index of all research | PARTIAL (v0.2) |
| 02_EXISTING_EDITORS | New-gen open editors (OpenCut, Clypra, Cutlass, Kerf, Velocut, OpenReelio, OpenTake, Frontstage…) | PARTIAL (v0.1 scan; repo-level source study NOT done) |
| 03_NLE_ARCHITECTURE | Kdenlive/Shotcut/Olive/Flowblade/MLT architecture | PARTIAL (v0.1) |
| 04–07 media engines | FFmpeg/GStreamer/MLT engineering | PARTIAL (v0.1) |
| 08–12 GPU & web | wgpu/WebGPU/WebCodecs/WASM | PARTIAL (v0.1) |
| 13–17 platforms | Rust/Android/Media3/MediaCodec/Tauri | PARTIAL (v0.1) |
| 19–20 | Project formats, timeline math | PARTIAL (v0.1) |
| 21_AUDIO / 23_CAPTIONS / 24_COMPUTER_VISION / 37_TRANSCRIPTS | audio stack + exact sample clock, caption formats + libass/rustybuzz, CV services (scene/CV licenses), whisper/diarization pipelines | PARTIAL (v0.1, wave-3 2026-09-24) |
| 25–27 | AI video editing, agents, MCP | PARTIAL (v0.1) |
| 28_PLUGINS / 29_SCRIPTING | plugin tiers (Rust/WASM/process), scripting = command-bus client | PARTIAL (v0.1, wave-3) |
| 30_HEADLESS / 31_STORAGE | render-as-service, checkpoints; content-addressed storage + caches | PARTIAL (v0.1, wave-3) |
| 32_PERFORMANCE / 33_SECURITY | budgets/proxies/memory; hostile-media + supply-chain rules | PARTIAL (v0.1, wave-3) |
| 34_UX / 35_LICENSES / 36_YOUTUBE | engine↔UI contract; MIT OR Apache-2.0 verdict + RED/GREEN audit; Data API v3 pipeline | PARTIAL (v0.1, wave-3) |
| 38_PAPERS | Academic paper index | PARTIAL (4 target papers + survey list) |
| 40_ARCHITECTURE_COMPARISON | 3 candidate architectures | PARTIAL (options framed, not decided) |
| 45_TESTING_STRATEGY | Testing constitution (T-1..T-10), pyramid, CI wiring | PARTIAL (v0.1 written 2026-09-24; live CI on ove-time; fuzz depth + real-media golden corpus pending) |
| 42_RISKS / 43_OPEN_QUESTIONS | Risk register, open questions | PARTIAL (Q-09 ANSWERED) |
| GATE_STATUS (research/gates/) | 14 architecture gates × evidence | PARTIAL (13 UNDERSTOOD / 1 PARTIAL environment-bound) |
| 44_ARCHITECTURE_WHITEPAPER | Evidence-backed architecture | **PLANNED** (all shape decisions accepted with named runtime-validation triggers — whitepaper can now be written from ADRs 001/002/007/008/009/010 + gate table) |

## Engine implementation

| Subsystem | Status |
|---|---|
| engine/ove-time | **TESTED** — exact rational time primitives; 4 unit + 11 property tests PASS (cargo test --release); fmt-normalized, clippy-clean (-D warnings); zero deps; ADR-007 encoded incl. tick-axis overflow regression guard; runs in CI |
| scripts/experiments/E-005_wgpu_swvk | **EXPERIMENTAL** — seed of ove-compositor: wgpu YUV→RGB + nearest-scale + over pass, pixel-exact vs host reference (software Vulkan) |
| engine/ove-timeline | **TESTED** (E-012, 2026-09-26) — edit verbs with exact inverses, batch atomicity, undo/redo, typed errors; 10/10 property suite (verb identity triples, oracle equivalence, replay determinism, threaded shard leg); containers: augmented AVL (**primary per ADR-011**) / gap buffer (noted alternative) / oracle; bench: scrub-budget decisive (AVL p99 0.52ms PASS vs gap+lazy 3.62ms FAIL @1ms) — clippy-clean, in CI |
| Everything else in `engine/` | **EMPTY by design** — build-out resumes Phase 2 (render graph, keyframes per ADR-006) |

## Experiments

| ID | Question | Status |
|---|---|---|
| E-001 | WebCodecs→WebGPU pipeline | PARTIAL-RUN (decode 48/48 ✓ real media; GPU import blocked by SwiftShader — harness committed for real-GPU rerun) |
| E-002 | fp vs rational time + timeline bench | RUN 13/13 (C-001 → HIGH) |
| E-002c | Timeline primary structure | RUN (gap buffer provisional winner → ADR-011 PROPOSED; P3b tick-axis finding) |
| E-003 | Command log + snapshot determinism | RUN 9/9 (C-007 viable) |
| E-004a | Rust FFI boundary pattern | RUN 5/5 |
| E-004b | UniFFI Kotlin codegen + JVM runtime | RUN 8/8 (codegen ✓ compile ✓ live roundtrip ✓) |
| E-006 | IPC transport costs | PARTIAL-RUN (browser leg complete; real-Tauri leg blocked → E-006b) |
| E-007 | Smart-render semantics (real media) | RUN (semantics 72/72 ✓; timing superseded by E-007b) |
| E-007b | Smart-render timing at 1080p | RUN (copy 6.8–11.9× faster, duration-exact) |
| E-009 | Shared command surface (MCP vs direct) | RUN 36/36 (Q-09 answered YES; ADR-010 → ACCEPTED) |
| ove-time suite | Exact-time property tests | TESTED 15/15 (ADR-007 acceptance evidence) |
| E-012 | Timeline structure integration (ADR-011 revisit) | **RUN** — properties 10/10; W4 scrub-budget decisive: AVL PASS / gap+lazy FAIL; **schema rule #4: explicit allocation** → ADR-011 ACCEPTED (REVISED) |
| E-004c/E-005/E-006b/E-008 | device-JNI runtime / real-GPU perf / real Tauri / audio latency | PLANNED / BLOCKED (see 41_EXPERIMENTS) |
| E-010/E-011/E-013 | scrub-rehydrate leg / upload drill / Claude Code E2E | PLANNED (E-010 hit-test leg partially covered by E-012 W4) |

## Architecture decision records

| ADR | Subject | Status |
|---|---|---|
| 001 | Core language (Rust + Kotlin/TS shells) | **ACCEPTED** 2026-09-24 (E-004a/b) |
| 007 | Time representation (exact rationals) | **ACCEPTED** 2026-09-24 (E-002 + ove-time 15/15) |
| 010 | Command system (human + AI one API) | **ACCEPTED** 2026-09-24 (E-009 36/36) |
| 011 | Timeline structure (**augmented AVL primary** — revised at integration) | **ACCEPTED** 2026-09-26 (E-012) |
| 002 | Core architecture (layered hybrid) | PROPOSED (blocked on E-005/E-004c/E-006b) |
| 003/004/005 | Media backend / decode / encode | PROPOSED (wave-2) |
| 006 | Timeline representation (clip-lists + render graph) | PROPOSED (keyframe model detail = Phase 2) |
