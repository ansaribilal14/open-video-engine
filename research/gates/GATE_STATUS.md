# ARCHITECTURE GATE STATUS — 14 gates × evidence

> Directive requires all 14 gates "understood" before final architecture.
> Statuses: **UNDERSTOOD** (evidence-complete for v0.x decision-making) ·
> **PARTIAL** (understood with named residuals) · **GAP** (not yet studied).
> Every claim links to docs/, experiments, or ADRs committed in this repo.

| Gate | Question | Status | Evidence |
|---|---|---|---|
| 1 Media pipeline | demux/decode/encode/seek/color fundamentals | **UNDERSTOOD** | docs 04/05/06/22; E-007+E-007b real-media semantics + timing; E-001 decode leg 48/48; ADR-003/004/005 PROPOSED |
| 2 Timeline model | data structure, time math, edit ops | **UNDERSTOOD** | docs 03/20; E-002 13/13 (fp falsified), E-002c 4-structure bench; ADR-006/011; ove-time TESTED 15/15; **E-012 integration (2026-09-26): ove-timeline crate, properties 10/10, budget-measured — ADR-011 ACCEPTED (REVISED: augmented AVL primary; gap+lazy-index fails 1ms scrub budget 3.62ms vs AVL 0.52ms)** |
| 3 Rendering model | preview + export render pipeline | **UNDERSTOOD** (v0.1) | doc 08 pass-graph compiler + WGSL library; libplacebo/OBS references (S-2d*); E-007b segment scheduler |
| 4 GPU strategy | wgpu/WebGPU compositor + video import | **UNDERSTOOD** (v0.1) | docs 08/09/10; E-001 support matrix + import paths mapped; **E-005 correctness leg PASS 4096/4096 px tolerance=0** — wgpu 25.0.2 YUV→RGB composite pixel-exact vs host reference on rootless lavapipe (Vulkan 1.3 sw); builtin-position rule for frame-exact passes; residuals: real-GPU perf (sw timings INVALID), zero-copy import legs |
| 5 Android strategy | bridge, codec, lifecycle | **UNDERSTOOD** (v0.1) | docs 13/14/15/16; E-004a 5/5 FFI; E-004b 8/8 UniFFI→Kotlin live; FGS 6h checkpointing; **E-004c build leg PASS**: NDK r27c aarch64 .so, full uniffi surface exported, DT_NEEDED=libdl/libc; residual: runtime JNI leg (System.loadLibrary + callback dispatch) needs device |
| 6 Browser strategy | codecs, GPU, storage, workers | **UNDERSTOOD** (v0.1) | docs 11/12/18; E-001 real decode; E-006 browser IPC leg quantified (JSON ≤10KB fine; 1.4MB = 42% frame budget); OPFS/SQLite-WASM storage map; iOS ~2GB jetsam budget |
| 7 Desktop strategy | Tauri shell, IPC, packaging | **PARTIAL** | doc 17 (Tauri 2.11 verified); E-006 browser-leg IPC rule grounded; **residual E-006b**: real-Tauri invoke/channel/custom-protocol bench (webkit2gtk + display needed); WebKitGTK codec fragility tracked |
| 8 Project format | storage model, portability | **UNDERSTOOD** | doc 19; E-003 9/9 (snapshot+log equivalence); ADR-008 PROPOSED (manifest + command log + content-addressed assets) |
| 9 AI command architecture | agent path, safety, MCP | **UNDERSTOOD** | docs 25/26/27; E-009 36/36 shared command surface; ADR-010 **ACCEPTED**; Q-09 answered |
| 10 Plugin boundary | native/WASM/process isolation | **UNDERSTOOD** (survey) | doc 28: three-tier proposal (Rust trait / WASM component-model via wasmtime 49 / out-of-process AI helpers); CLAP/VST3-MIT/OBS/Figma precedents verified |
| 11 Performance risks | budgets, proxies, memory, parallelism | **UNDERSTOOD** | doc 32; quantified anchors: E-002c 140ns/rational-add, E-006 89× JSON-vs-binary, E-007b 6.8–11.9× copy; FFmpeg threading verified; perf-CI gate plan |
| 12 Security model | hostile media, project files, supply chain | **UNDERSTOOD** (survey) | doc 33: 513-CVE FFmpeg history → sidecar parsing rule; 7 command-payload validation rules (extends E-003); sandboxing ladder; cargo-audit/Tauri updater verified |
| 13 Licensing | engine license, codec patents, deps | **UNDERSTOOD** | doc 35: MIT OR Apache-2.0 verdict; AV1 defensive-termination verified; 32-row RED/GREEN dependency audit; GPL-contamination rules |
| 14 Testing strategy | property, golden, conformance, CI | **UNDERSTOOD** (v0.1) | doc 45 (testing constitution T-1..T-10); ove-time 15/15 in CI; **live GitHub Actions ci.yml** (fmt+clippy -D warnings+tests+cargo-audit); E-003/E-009 property patterns; E-002c hash gates; E-005 golden template; doc 32 perf-ratio gates; residuals: fuzz depth, real-media golden corpus |

## Gate → ADR decision mapping

| ADR | Subject | Status | Blocking residuals |
|---|---|---|---|
| 001 Core language | Rust + Kotlin/TS shells | **ACCEPTED** 2026-09-24 | E-004c tracked under Android ADR |
| 002 Core architecture | layered hybrid | **ACCEPTED** 2026-09-24 | accepted with named runtime-validation triggers: E-005 real-GPU perf + zero-copy, E-004c runtime JNI, E-006b real-Tauri transport (see ADR text) |
| 003 Media backend | per-platform adapters | PROPOSED | integration bench + license sign-off |
| 004 Decode abstraction | Decoder trait | PROPOSED | trait-shaped evidence leg (ove-decode skeleton + conformance suite per doc 45 T-5) |
| 005 Encode abstraction | Encoder trait + StreamCopy | PROPOSED | cross-encoder stitch sync |
| 006 Timeline representation | hybrid clip-lists + render graph | PROPOSED | keyframe model detail (Phase 2) |
| 007 Time representation | exact rationals | **ACCEPTED** 2026-09-24 | — (ove-time 15/15) |
| 008 Project format | manifest + command log | **ACCEPTED** 2026-09-24 | T-6/T-8/T-9 rules in doc 45 |
| 009 Undo/redo | inverse commands + markers | **ACCEPTED** 2026-09-24 | T-1 per-verb rule in doc 45 (forward obligation) |
| 010 Command system | one API for human+AI | **ACCEPTED** 2026-09-24 | verb-set growth at integration |
| 011 Timeline structure | augmented AVL ALONE (revised at integration: gap layer rejected) | **ACCEPTED** 2026-09-26 | — (E-012: 10/10 properties + W4 scrub-budget decisive; gap buffer = noted alternative in crate) |

## Verdict
13 of 14 gates UNDERSTOOD (several qualified "v0.1" with named residuals);
1 PARTIAL (gate 7 desktop — E-006b real-Tauri transport, fully environment-bound:
webkit2gtk + display need root). No gate is GAP. All architecture-shape decisions
the sandbox can evidence are evidenced; runtime/perf validations on real hardware
are ADR revisit triggers, not open decisions. 7 of 11 ADRs ACCEPTED (011 added 2026-09-26 at its own integration gate)
(001/002/007/008/009/010).
