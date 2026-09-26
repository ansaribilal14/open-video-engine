# STATUS — Honest Subsystem Classification

> Directive rule: **NO FAKE COMPLETION.** Classifications allowed:
> IMPLEMENTED · TESTED · BENCHMARKED · EXPERIMENTAL · PARTIAL · PLANNED · BLOCKED · UNKNOWN
> Nothing may be upgraded without evidence committed to this repository.

Last updated: 2026-09-27 (v0.8 — WAVE 2 LANDED: ove-media + ove-decode per DECODER_SPEC /
FRAME_CONTRACT; conformance suite D-1..D-12 GREEN on committed self-generated corpus with
committed ffprobe golden tables; libav* linkage confined to ove-decode, CI-enforced;
E-017 libav binding probe record added. v0.7 had reconciled the parallel audit+Wave0/1
line with remote v0.5/v0.6 (E-012 augmented-AVL ove-timeline; local gap-buffer crate
superseded, preserved at ee31fb9).)

## Mission phases

| Phase | Scope | Status |
|---|---|---|
| FORENSIC AUDIT | Whole-repo claim verification + 17 deliverables | **COMPLETE** 2026-09-26 — research/audit/ (11 audits + OPEN_GAPS.md), docs/specs/ (6 contracts), whitepaper (PROVISIONAL FINAL), ENGINE_BUILD_PLAN (Waves 0–9), CROSS_PLATFORM_CONFORMANCE_PLAN; every STATUS claim verified against code (A1–A20); defects D-1..D-6 recorded, D-1/D-4/D-6 closed same day |
| PHASE 0 | Research infrastructure, source ledger, claim ledger | **PARTIAL** (scaffolding done; 137 sources ledgered; 7 claims tracked with evidence) |
| PHASE 0 | Track research (editors, media, timeline, GPU, web, Android, desktop, AI, MCP + audio, captions, CV, plugins, scripting, headless, storage, performance, security, UX, licenses, YouTube, transcripts) | **PARTIAL** (42 research files, ~8,135 lines incl. doc 45; depth requirement met for 2/15 projects, partially for 6, honestly-absent for 4 — see RESEARCH_COMPLETENESS_AUDIT) |
| PHASE 0 | Cross-track synthesis + knowledge base | **PARTIAL** (v0.7: ADR-001/002/007/008/009/010/011 **ACCEPTED** (002 with named runtime-validation revisit triggers; 011 REVISED at its integration gate 2026-09-26); ADR-003/004/005/006 PROPOSED with named evidence legs) |
| PHASE 0 | Architecture gates 1–14 | **PARTIAL** — 13 UNDERSTOOD (several v0.1 with named residuals) / 1 PARTIAL (gate 7: E-006b needs webkit2gtk+display, no root) / 0 GAP (research/gates/GATE_STATUS.md; UPDATED_GATE_STATUS.md = audit-day snapshot) |
| PHASE 0 | Experiments & benchmarks | **PARTIAL** — 10 experiments RUN + 2 partial legs (E-001 partial, E-002 13/13, E-002c, E-003 9/9, E-004a 5/5, E-004b 8/8, E-006 browser-leg, E-007+E-007b, E-009 36/36, **E-012 10/10 + cross-validated bench**; E-005 correctness leg 4096/4096 software-Vulkan, E-004c build leg NDK aarch64); ove-time TESTED 15/15; ove-timeline TESTED 10/10; hardware/runtime residuals: E-004c runtime (device), E-005 real-GPU perf + zero-copy, E-006b (webkit2gtk) |
| PHASE 0 | CI | **TESTED-infra** — GitHub Actions ci.yml (fmt + clippy -D warnings + tests release across all 4 crates incl. libav-linked ove-decode via bundled FFmpeg 7.1, cargo-audit, **libav-confinement job**) |
| PHASE 1 | Core media abstraction → **Media Foundation vertical slice** (per 2026-09-26 directive) | **IN PROGRESS** — Wave 0 COMPLETE; Wave 1 gap-buffer crate superseded by E-012 AVL crate; **Wave 2 COMPLETE 2026-09-27: ove-media (TESTED 13/13) + ove-decode (TESTED 20/20: D-1..D-12 conformance + probe integration)** |
| PHASE 2 | Timeline engine | **LANDED 2026-09-26** — engine/ove-timeline (E-012): verbs with exact inverses, batch atomicity, undo/redo, typed errors; 10/10 properties (verb identity triples, oracle equivalence, replay determinism, threaded shard leg); augmented AVL primary per ADR-011, gap buffer noted alternative |
| PHASE 3 | Project system | **PLANNED** (Wave 5 — PROJECT_FORMAT_SPEC acceptance suite P-1..P-8) |
| PHASE 4–22 | Decode/playback → production hardening | **PLANNED** (Waves 2–9: ove-media/ove-decode decoder-first, ove-render golden frames, ove-encode ffprobe gate, GPU promotion, audio, keyframes, platform legs) |

## Research documents (docs/research/)

| Doc | Topic | Status |
|---|---|---|
| 41_EXPERIMENTS | Experiment index + records | PARTIAL (E-001/002/002c/003/004a/004b/004c/005/006/006a/007/007b/009/**012** run + ove-time suite; E-006b/E-008/E-010/E-011/E-013 defined) |
| 01_RESEARCH_INDEX | Index of all research | PARTIAL (v0.2) |
| 02_EXISTING_EDITORS | New-gen open editors (OpenCut, Clypra, Cutlass, Kerf, Velocut, OpenReelio, OpenTake, Frontstage…) | PARTIAL (v0.1 scan; repo-level source study NOT done) |
| 03_NLE_ARCHITECTURE | Kdenlive/Shotcut/Olive/Flowblade/MLT architecture | PARTIAL (v0.1) |
| 04–07 media engines | FFmpeg/GStreamer/MLT engineering | PARTIAL (v0.1) |
| 08–12 GPU & web | wgpu/WebGPU/WebCodecs/WASM | PARTIAL (v0.1) |
| 13–17 platforms | Rust/Android/Media3/MediaCodec/Tauri | PARTIAL (v0.1) |
| 19–20 | Project formats, timeline math | PARTIAL (v0.1) |
| 21_AUDIO / 23_CAPTIONS / 24_COMPUTER_VISION / 37_TRANSCRIPTS | audio stack + exact sample clock, caption formats + libass/rustybuzz, CV services, whisper/diarization pipelines | PARTIAL (v0.1, wave-3 2026-09-24) |
| 25–27 | AI video editing, agents, MCP | PARTIAL (v0.1) |
| 28_PLUGINS / 29_SCRIPTING | plugin tiers (Rust/WASM/process), scripting = command-bus client | PARTIAL (v0.1, wave-3) |
| 30_HEADLESS / 31_STORAGE | render-as-service, checkpoints; content-addressed storage + caches | PARTIAL (v0.1, wave-3) |
| 32_PERFORMANCE / 33_SECURITY | budgets/proxies/memory; hostile-media + supply-chain rules | PARTIAL (v0.1, wave-3) |
| 34_UX / 35_LICENSES / 36_YOUTUBE | engine↔UI contract; MIT OR Apache-2.0 verdict + RED/GREEN audit; Data API v3 pipeline | PARTIAL (v0.1, wave-3) |
| 38_PAPERS | Academic paper index | PARTIAL (4 target papers + survey list) |
| 40_ARCHITECTURE_COMPARISON | 3 candidate architectures | PARTIAL (framed; decision now consolidated in whitepaper) |
| 45_TESTING_STRATEGY | Testing constitution (T-1..T-10), pyramid, CI wiring | PARTIAL (v0.1; live CI on both crates; fuzz depth + real-media golden corpus pending) |
| 42_RISKS / 43_OPEN_QUESTIONS | Risk register, open questions | PARTIAL (Q-09 ANSWERED) |
| GATE_STATUS (research/gates/) | 14 architecture gates × evidence | PARTIAL (13 UNDERSTOOD / 1 PARTIAL environment-bound; UPDATED_GATE_STATUS.md = post-audit snapshot) |
| 44_ARCHITECTURE_WHITEPAPER | Evidence-backed architecture | **PROVISIONAL FINAL** 2026-09-26 (docs/FINAL_ARCHITECTURE_WHITEPAPER.md — architecture C with 3 amendments; three hardware-bound residuals named as reopen conditions) |

## Engine implementation

| Subsystem | Status |
|---|---|
| engine/ove-time | **TESTED** — exact rational time primitives; 4 unit + 11 property tests PASS; fmt/clippy-clean (-D warnings); zero deps; ADR-007 encoded incl. tick-axis overflow regression guard; in CI |
| engine/ove-timeline | **TESTED** (E-012, 2026-09-26) — edit verbs with exact inverses, batch atomicity, undo/redo, typed errors; 10/10 property suite (verb identity triples, oracle equivalence, replay determinism, threaded shard leg); containers: augmented AVL (**primary per ADR-011 REVISED**) / gap buffer (noted alternative; the superseded Wave-1 local crate remains at ee31fb9) / oracle; bench: scrub-budget decisive (AVL p99 0.52ms PASS vs gap+lazy 3.62ms FAIL @1ms); clippy-clean, in CI |
| engine workspace | **IMPLEMENTED** — engine/Cargo.toml (members: ove-time, ove-timeline; fmt/clippy clean at -D warnings); ci.yml live (trigger fixed `[main]`) |
| repo hygiene | LICENSE-MIT + LICENSE-APACHE (dual MIT OR Apache-2.0); E-006a code+raw output committed (D-1/X-5 closed); mode churn normalized (D-4); orphaned engine/crates/ removed |
| engine/ove-media | **TESTED** (Wave 2, 2026-09-27) — libav-free data model: AssetRef (blake3 content-hash identity), ProbeInfo/KeyframeIndex/VfrReport + ProbeBackend trait, MetadataCache (content-hash-keyed, corrupt-file=miss), FrameEnvelope per FRAME_CONTRACT (five mandatory color tags, exact rationals, Cpu/Gpu/Packed memory, unique ownership — not Clone), FramePool with generation staleness detection; 13/13 tests |
| engine/ove-decode | **TESTED** (Wave 2, 2026-09-27) — Decoder trait per DECODER_SPEC (open/capabilities/seek Exact+Snap/next/flush/cancel) + FFmpeg-SW adapter (ffmpeg-sys-next 7.1, libavformat/avcodec/avutil); conformance D-1..D-12 GREEN (pts exactness vs committed ffprobe tables incl. 30000/1001 NTSC, duration sums exact, keyframe truth vs packet flags, seek-Exact byte-identity, Snap = E-007 trap as positive test, flush equivalence, typed cancel, corrupt-corpus typed errors, no-hidden-hw-downgrade, color-tag presence, memory honesty, VFR safety) + threading determinism; 20/20 tests; **the ONLY crate linking libav\*** (CI-enforced) |
| Everything else in `engine/` | **EMPTY by design** — no crate starts without a failing acceptance test asking for it (anti-overbuild rule). Next: Wave 3 ove-render (golden frames) / Wave 4 ove-encode per ENGINE_BUILD_PLAN |

## Experiments

| ID | Question | Status |
|---|---|---|
| E-001 | WebCodecs→WebGPU pipeline | PARTIAL-RUN (decode 48/48 ✓ real media; GPU import blocked by SwiftShader — harness committed for real-GPU rerun) |
| E-002 | fp vs rational time + timeline bench | RUN 13/13 (C-001 → HIGH) |
| E-002c | Timeline primary structure | RUN (4 workloads + c2 addendum; superseded as decision by E-012 integration) |
| E-003 | Command log + snapshot determinism | RUN 9/9 (C-007 viable) |
| E-004a | Rust FFI boundary pattern | RUN 5/5 |
| E-004b | UniFFI Kotlin codegen + JVM runtime | RUN 8/8 (codegen ✓ compile ✓ live roundtrip ✓) |
| E-004c | Android leg | BUILD-LEG PASS (NDK r27c aarch64 .so, full uniffi surface); runtime device leg PLANNED |
| E-005 | wgpu compositor | CORRECTNESS-LEG PASS 4096/4096 (rootless lavapipe, software Vulkan, pixel-exact vs host reference); real-GPU perf + zero-copy PLANNED |
| E-006 | IPC transport costs | PARTIAL-RUN (browser leg complete; real-Tauri leg blocked → E-006b) |
| E-006a | IPC serialization costs, native side | RE-RUN 2026-09-26 (D-1 closed: crate + raw output committed; median-of-30; JSON frames ≈ 2.65+4.97 CPU-s/s at 1080p60 — frames-never-JSON rule, both transport edges) |
| E-007 | Smart-render semantics (real media) | RUN (semantics 72/72 ✓; timing superseded by E-007b) |
| E-007b | Smart-render timing at 1080p | RUN (copy 6.8–11.9× faster, duration-exact) |
| E-009 | Shared command surface (MCP vs direct) | RUN 36/36 (Q-09 answered YES; ADR-010 → ACCEPTED) |
| E-012 | Timeline structure integration (ADR-011 revisit) | **RUN** — properties 10/10; scrub-budget decisive: AVL PASS / gap+lazy FAIL; schema rule #4 explicit allocation → ADR-011 ACCEPTED (REVISED) |
| E-017 | libav binding feasibility (W2 de-risk) | **RUN — PASS**: ffmpeg-sys-next 7.1.3 builds/links FFmpeg 7.1 root-free (5 recorded lessons: nasm, libclang, clang resource-dir, pkg-config sysroot, static-fallback trap); superseded into ove-decode |
| ove-time suite | Exact-time property tests | TESTED 15/15 (ADR-007 acceptance evidence) |
| E-006b/E-008 | real-Tauri transport / audio latency | BLOCKED/PLANNED (see 41_EXPERIMENTS) |
| E-010/E-011/E-013 | scrub-rehydrate / upload drill / Claude Code E2E | PLANNED (E-010 hit-test leg partially covered by E-012 W4) |

## Architecture decision records

| ADR | Subject | Status |
|---|---|---|
| 001 | Core language (Rust + Kotlin/TS shells) | **ACCEPTED** 2026-09-24 (E-004a/b) |
| 002 | Core architecture (layered hybrid) | **ACCEPTED** 2026-09-26 (E-005 correctness leg + E-004c build leg + E-004a/b + E-006/E-006a + E-003/E-009; named runtime-validation revisit triggers: E-005 real-GPU perf + zero-copy, E-004c runtime JNI, E-006b real-Tauri) |
| 007 | Time representation (exact rationals) | **ACCEPTED** 2026-09-24 (E-002 + ove-time 15/15) |
| 008 | Project format | **ACCEPTED** 2026-09-26 (E-003 + schema rule #4 from E-012) |
| 009 | Undo/redo | **ACCEPTED** 2026-09-26 (exact inverses committed as CI-tested code) |
| 010 | Command system (human + AI one API) | **ACCEPTED** 2026-09-24 (E-009 36/36) |
| 011 | Timeline structure (augmented AVL primary — revised at integration) | **ACCEPTED** 2026-09-26 (E-012) |
| 003/004/005 | Media backend / decode / encode | PROPOSED (Wave 2 implementation = next falsification instrument) |
| 006 | Timeline representation (clip-lists + render graph) | PROPOSED (keyframe model detail = Wave 3+) |
