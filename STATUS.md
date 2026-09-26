# STATUS — Honest Subsystem Classification

> Directive rule: **NO FAKE COMPLETION.** Classifications allowed:
> IMPLEMENTED · TESTED · BENCHMARKED · EXPERIMENTAL · PARTIAL · PLANNED · BLOCKED · UNKNOWN
> Nothing may be upgraded without evidence committed to this repository.

Last updated: 2026-09-26 (forensic audit COMPLETE — all 17 directive deliverables; Wave 0 repo integrity: workspace + CI + LICENSE + E-006a evidence chain closed)

## Mission phases

| Phase | Scope | Status |
|---|---|---|
| FORENSIC AUDIT | Whole-repo claim verification + 17 deliverables | **COMPLETE** 2026-09-26 — research/audit/ (11 audits + OPEN_GAPS.md), docs/specs/ (6 contracts), whitepaper (PROVISIONAL FINAL), ENGINE_BUILD_PLAN (Waves 0–9), CROSS_PLATFORM_CONFORMANCE_PLAN; every STATUS claim verified against code (A1–A20); defects D-1..D-6 recorded, D-1/D-4/D-6 closed same day |
| PHASE 0 | Research infrastructure, source ledger, claim ledger | **PARTIAL** (scaffolding done; 137 sources ledgered; 7 claims tracked with evidence) |
| PHASE 0 | Track research (editors, media, timeline, GPU, web, Android, desktop, AI, MCP + audio, captions, CV, plugins, scripting, headless, storage, performance, security, UX, licenses, YouTube, transcripts) | **PARTIAL** (v0.2: 42 research files, ~8,135 lines — count corrected by audit X-7; 13 wave-3 docs added 2026-09-24; depth requirement met for 2/15 projects, partially for 6, honestly-absent for 4 — see RESEARCH_COMPLETENESS_AUDIT) |
| PHASE 0 | Cross-track synthesis + knowledge base | **PARTIAL** (v0.2: ADR-001/007/010 **ACCEPTED**; ADR-002/003/004/005/006/008/009/011 PROPOSED; GATE_STATUS.md: 10/14 gates UNDERSTOOD, 4 PARTIAL with named residuals) |
| PHASE 0 | Architecture gates 1–14 | **PARTIAL** — 10 UNDERSTOOD / 4 PARTIAL / 0 GAP (research/gates/GATE_STATUS.md) |
| PHASE 0 | Experiments & benchmarks | **PARTIAL** — 9 experiments RUN (E-001 partial, E-002 13/13, E-002c, E-003 9/9, E-004a 5/5, E-004b 8/8, E-006 browser-leg, E-007+E-007b, **E-009 36/36**); ove-time TESTED 15/15; blocked-only-on-hardware: E-004c/E-005/E-006b |
| PHASE 1 | Core media abstraction → **Media Foundation vertical slice** (per 2026-09-26 directive) | **EXPERIMENTAL → wave-1 starting** (ove-time TESTED 15/15 re-verified live by audit; workspace created; next: ove-timeline per ENGINE_BUILD_PLAN Wave 1) |
| PHASE 2 | Timeline engine | **PLANNED** (Wave 1 — ove-timeline crate: gap buffer + derived index, verbs + exact inverses, property suite porting E-003/E-009 invariants) |
| PHASE 3 | Project system | **PLANNED** (Wave 5 — PROJECT_FORMAT_SPEC acceptance suite P-1..P-8) |
| PHASE 4–22 | Decode/playback → production hardening | **PLANNED** (Waves 2–9: ove-media/ove-decode decoder-first, ove-render golden frames, ove-encode ffprobe gate, GPU promotion, audio, keyframes, platform legs) |

## Research documents (docs/research/)

| Doc | Topic | Status |
|---|---|---|
| 41_EXPERIMENTS | Experiment index + records | PARTIAL (E-001/002/002c/003/004a/004b/006/007/007b/**009** run + ove-time suite; E-004c/E-005/E-006b/E-008/E-010/E-011/E-012/E-013 defined) |
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
| 42_RISKS / 43_OPEN_QUESTIONS | Risk register, open questions | PARTIAL (Q-09 ANSWERED) |
| GATE_STATUS (research/gates/) | 14 architecture gates × evidence | PARTIAL (10 UNDERSTOOD / 4 PARTIAL) |
| 44_ARCHITECTURE_WHITEPAPER | Evidence-backed architecture | **PROVISIONAL FINAL** 2026-09-26 (docs/FINAL_ARCHITECTURE_WHITEPAPER.md — architecture C with 3 amendments; three hardware-bound residuals named as reopen conditions) |

## Engine implementation

| Subsystem | Status |
|---|---|
| engine/ove-time | **TESTED** — exact rational time primitives; 4 unit + 11 property tests PASS (re-run live by audit AND after fmt/clippy alignment 2026-09-26); zero deps; ADR-007 encoded incl. tick-axis overflow regression guard; license MIT OR Apache-2.0 |
| engine workspace | **IMPLEMENTED** — engine/Cargo.toml (members: ove-time; fmt/clippy clean at -D warnings); CI workflow .github/workflows/engine.yml (fmt+clippy+test --release) |
| repo hygiene | LICENSE-MIT + LICENSE-APACHE added 2026-09-26 (LICENSE_AUDIT closed); E-006a code+raw output committed (D-1/X-5 closed); mode churn normalized (D-4 closed); orphaned engine/crates/ cache removed |
| Everything else in `engine/` | **EMPTY by design** — no crate starts without a failing acceptance test asking for it (anti-overbuild rule) |

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
| E-006a | IPC serialization costs, native side | RE-RUN 2026-09-26 (D-1 closed: crate + raw output committed; method median-of-30; extrapolation basis corrected honestly — JSON frames ≈ 2.65+4.97 CPU-s/s at 1080p60, rule unchanged and strengthened) |
| E-007 | Smart-render semantics (real media) | RUN (semantics 72/72 ✓; timing superseded by E-007b) |
| E-007b | Smart-render timing at 1080p | RUN (copy 6.8–11.9× faster, duration-exact) |
| E-009 | Shared command surface (MCP vs direct) | RUN 36/36 (Q-09 answered YES; ADR-010 → ACCEPTED) |
| ove-time suite | Exact-time property tests | TESTED 15/15 (ADR-007 acceptance evidence) |
| E-004c/E-005/E-006b/E-008 | device JNI / wgpu compositor / real Tauri / audio latency | PLANNED / BLOCKED (see 41_EXPERIMENTS) |
| E-010/E-011/E-012/E-013 | scrub-burst / upload drill / gap-buffer integration / Claude Code E2E | PLANNED |

## Architecture decision records

| ADR | Subject | Status |
|---|---|---|
| 001 | Core language (Rust + Kotlin/TS shells) | **ACCEPTED** 2026-09-24 (E-004a/b) |
| 007 | Time representation (exact rationals) | **ACCEPTED** 2026-09-24 (E-002 + ove-time 15/15) |
| 010 | Command system (human + AI one API) | **ACCEPTED** 2026-09-24 (E-009 36/36) |
| 002 | Core architecture (layered hybrid) | PROPOSED (blocked on E-005/E-004c/E-006b) |
| 003/004/005 | Media backend / decode / encode | PROPOSED (wave-2) |
| 006/008/009 | Timeline representation / project format / undo | PROPOSED (wave-2) |
| 011 | Timeline structure (gap buffer) | PROPOSED (integration gate E-012) |
