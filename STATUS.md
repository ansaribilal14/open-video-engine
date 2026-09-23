# STATUS — Honest Subsystem Classification

> Directive rule: **NO FAKE COMPLETION.** Classifications allowed:
> IMPLEMENTED · TESTED · BENCHMARKED · EXPERIMENTAL · PARTIAL · PLANNED · BLOCKED · UNKNOWN
> Nothing may be upgraded without evidence committed to this repository.

Last updated: 2026-09-21 (Phase 0 kickoff)

## Mission phases

| Phase | Scope | Status |
|---|---|---|
| PHASE 0 | Research infrastructure, source ledger, claim ledger | **PARTIAL** (scaffolding done; 77 sources ledgered; 7 claims tracked with evidence) |
| PHASE 0 | Track research (editors, media, timeline, GPU, web, Android, desktop, AI, MCP) | **PARTIAL** (v0.1 first-pass scan complete: 26 of 41 research docs written, 4,600+ lines, via 6 parallel agents; depth requirement NOT yet met) |
| PHASE 0 | Cross-track synthesis + knowledge base | **PARTIAL** (v0.1 synthesis done: 40_ARCHITECTURE_COMPARISON, KNOWLEDGE_BASE, RISK_REGISTER, OPEN_QUESTIONS, ADR-001/002/007/010 PROPOSED) |
| PHASE 0 | Architecture gates 1–14 | **PLANNED** — none passed yet |
| PHASE 0 | Experiments & benchmarks | **PARTIAL** — 4 experiments RUN (E-001 partial, E-002 13/13, E-003 9/9, E-004a 5/5, E-007 4/5): C-001 experimentally confirmed HIGH; command-log format viable; FFI pattern proven; real-H.264 decode in browser 48/48; smart-render semantics confirmed on real media |
| PHASE 1 | Core media abstraction | **PLANNED** |
| PHASE 2 | Timeline engine | **PLANNED** |
| PHASE 3 | Project system | **PLANNED** |
| PHASE 4–22 | Decode/playback → production hardening | **PLANNED** |

## Research documents (docs/research/)

| Doc | Topic | Status |
|---|---|---|
| 41_EXPERIMENTS | Experiment index + records | PARTIAL (E-001/002/003/004a/007 run; E-002c/E-004b/E-006/E-007b defined) |
| 01_RESEARCH_INDEX | Index of all research | PARTIAL |
| 02_EXISTING_EDITORS | New-gen open editors (OpenCut, Clypra, Cutlass, Kerf, Velocut, OpenReelio, OpenTake, Frontstage…) | PARTIAL (v0.1 scan; repo-level source study NOT done) |
| 03_NLE_ARCHITECTURE | Kdenlive/Shotcut/Olive/Flowblade/MLT architecture | PARTIAL (v0.1) |
| 04–07 media engines | FFmpeg/GStreamer/MLT engineering | PARTIAL (v0.1) |
| 08–12 GPU & web | wgpu/WebGPU/WebCodecs/WASM | PARTIAL (v0.1) |
| 13–17 platforms | Rust/Android/Media3/MediaCodec/Tauri | PARTIAL (v0.1) |
| 19–20 | Project formats, timeline math | PARTIAL (v0.1) |
| 25–27 | AI video editing, agents, MCP | PARTIAL (v0.1) |
| 38_PAPERS | Academic paper index | PARTIAL (4 target papers + survey list) |
| 40_ARCHITECTURE_COMPARISON | 3 candidate architectures | PARTIAL (options framed, not decided) |
| 42_RISKS / 43_OPEN_QUESTIONS | Risk register, open questions | PARTIAL |
| 44_ARCHITECTURE_WHITEPAPER | Evidence-backed architecture | **PLANNED** (blocked on gates) |

## Engine implementation

| Subsystem | Status |
|---|---|
| Everything in `engine/` | **EMPTY by design** — directive forbids building before gates pass |

## Experiments

| ID | Question | Status |
|---|---|---|
| E-001 | WebCodecs→WebGPU pipeline | PARTIAL-RUN (decode 48/48 ✓ real media; GPU import blocked by SwiftShader — harness committed for real-GPU rerun) |
| E-002 | fp vs rational time + timeline bench | RUN 13/13 (C-001 → HIGH) |
| E-003 | Command log + snapshot determinism | RUN 9/9 (C-007 viable) |
| E-004a | Rust FFI boundary pattern | RUN 5/5 |
| E-007 | Smart-render semantics (real media) | RUN 4/5 (timing leg honestly invalidated) |
| E-002c/E-004b/E-006/E-007b | structure / UniFFI / Tauri IPC / real-res timing | PLANNED |
