# ARCHITECTURE GATE STATUS — 14 gates × evidence

> Directive requires all 14 gates "understood" before final architecture.
> Statuses: **UNDERSTOOD** (evidence-complete for v0.x decision-making) ·
> **PARTIAL** (understood with named residuals) · **GAP** (not yet studied).
> Every claim links to docs/, experiments, or ADRs committed in this repo.

| Gate | Question | Status | Evidence |
|---|---|---|---|
| 1 Media pipeline | demux/decode/encode/seek/color fundamentals | **UNDERSTOOD** | docs 04/05/06/22; E-007+E-007b real-media semantics + timing; E-001 decode leg 48/48; ADR-003/004/005 PROPOSED |
| 2 Timeline model | data structure, time math, edit ops | **UNDERSTOOD** | docs 03/20; E-002 13/13 (fp falsified), E-002c 4-structure bench; ADR-006/011; ove-time TESTED 15/15 |
| 3 Rendering model | preview + export render pipeline | **UNDERSTOOD** (v0.1) | doc 08 pass-graph compiler + WGSL library; libplacebo/OBS references (S-2d*); E-007b segment scheduler |
| 4 GPU strategy | wgpu/WebGPU compositor + video import | **PARTIAL** | docs 08/09/10; E-001: support matrix + importExternalTexture/copyExternalImageToTexture paths mapped; **residual E-005**: real-GPU YUV compositor run (SwiftShader blocked) |
| 5 Android strategy | bridge, codec, lifecycle | **PARTIAL** | docs 13/14/15/16; E-004a 5/5 FFI pattern; E-004b 8/8 UniFFI→Kotlin live roundtrip; FGS 6h checkpointing rule; **residual E-004c**: on-device JNI |
| 6 Browser strategy | codecs, GPU, storage, workers | **UNDERSTOOD** (v0.1) | docs 11/12/18; E-001 real decode; E-006 browser IPC leg quantified (JSON ≤10KB fine; 1.4MB = 42% frame budget); OPFS/SQLite-WASM storage map; iOS ~2GB jetsam budget |
| 7 Desktop strategy | Tauri shell, IPC, packaging | **PARTIAL** | doc 17 (Tauri 2.11 verified); E-006 browser-leg IPC rule grounded; **residual E-006b**: real-Tauri invoke/channel/custom-protocol bench (webkit2gtk + display needed); WebKitGTK codec fragility tracked |
| 8 Project format | storage model, portability | **UNDERSTOOD** | doc 19; E-003 9/9 (snapshot+log equivalence); ADR-008 PROPOSED (manifest + command log + content-addressed assets) |
| 9 AI command architecture | agent path, safety, MCP | **UNDERSTOOD** | docs 25/26/27; E-009 36/36 shared command surface; ADR-010 **ACCEPTED**; Q-09 answered |
| 10 Plugin boundary | native/WASM/process isolation | **UNDERSTOOD** (survey) | doc 28: three-tier proposal (Rust trait / WASM component-model via wasmtime 49 / out-of-process AI helpers); CLAP/VST3-MIT/OBS/Figma precedents verified |
| 11 Performance risks | budgets, proxies, memory, parallelism | **UNDERSTOOD** | doc 32; quantified anchors: E-002c 140ns/rational-add, E-006 89× JSON-vs-binary, E-007b 6.8–11.9× copy; FFmpeg threading verified; perf-CI gate plan |
| 12 Security model | hostile media, project files, supply chain | **UNDERSTOOD** (survey) | doc 33: 513-CVE FFmpeg history → sidecar parsing rule; 7 command-payload validation rules (extends E-003); sandboxing ladder; cargo-audit/Tauri updater verified |
| 13 Licensing | engine license, codec patents, deps | **UNDERSTOOD** | doc 35: MIT OR Apache-2.0 verdict; AV1 defensive-termination verified; 32-row RED/GREEN dependency audit; GPL-contamination rules |
| 14 Testing strategy | property, golden, conformance, CI | **PARTIAL** | ove-time property suite (15/15); E-003/E-009 replay+undo property checks; E-002c cross-structure hash gates; doc 32 perf-CI ratio gates; **gap**: dedicated testing-strategy doc not yet written (next wave) |

## Gate → ADR decision mapping

| ADR | Subject | Status | Blocking residuals |
|---|---|---|---|
| 001 Core language | Rust + Kotlin/TS shells | **ACCEPTED** 2026-09-24 | E-004c tracked under Android ADR |
| 002 Core architecture | layered hybrid | PROPOSED | GATE-4/5/7 residuals (E-005, E-004c, E-006b) |
| 003 Media backend | per-platform adapters | PROPOSED | integration bench + license sign-off |
| 004 Decode abstraction | Decoder trait | PROPOSED | — |
| 005 Encode abstraction | Encoder trait + StreamCopy | PROPOSED | cross-encoder stitch sync |
| 006 Timeline representation | hybrid clip-lists + render graph | PROPOSED | keyframe model detail (Phase 2) |
| 007 Time representation | exact rationals | **ACCEPTED** 2026-09-24 | — (ove-time 15/15) |
| 008 Project format | manifest + command log | PROPOSED | — |
| 009 Undo/redo | inverse commands + markers | PROPOSED | per-verb property tests in CI |
| 010 Command system | one API for human+AI | **ACCEPTED** 2026-09-24 | verb-set growth at integration |
| 011 Timeline structure | gap buffer primary | PROPOSED | E-012 integration benchmark |

## Verdict
10 of 14 gates UNDERSTOOD; 4 PARTIAL with named, hardware-bound residuals
(E-005 real-GPU, E-004c real-device, E-006b real-Tauri, testing doc). No gate is GAP.
Final architecture selection (ADR-002 ACCEPT) is blocked only on the three
hardware-bound experiments — all other decision inputs are in place.
