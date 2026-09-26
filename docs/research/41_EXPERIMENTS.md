# 41 — EXPERIMENTS

> Status: PARTIAL (9 experiments run 2026-09-23/24; E-004c/E-005/E-006b remain defined;
> E-008/E-010/E-011/E-013 proposed by wave-3 agents).
> Records follow the directive format: QUESTION / HYPOTHESIS / IMPLEMENTATION / HARDWARE /
> RESULT / LIMITATIONS / DECISION. Raw outputs in `experiments/`, code in
> `scripts/experiments/`.

| ID | Question | Record | Code | Result |
|---|---|---|---|---|
| E-001 | WebCodecs→WebGPU browser pipeline | [record](../../research/experiments/E-001_webcodecs_webgpu.md) | E-001_cdp_v2.cjs (+E-001a sweep) | PARTIAL-RUN: H.264 decode 48/48 ✓ (real media, Chrome 154); GPU-import legs blocked by software rendering; secure-context discovery; support matrix captured |
| E-002 | fp-vs-rational time + timeline structure | [record](../../research/experiments/E-002_time_representation.md) | E-002_time_representation.py + E-002b_timeline_bench.rs | RUN: 13/13 PASS — fp unsafe confirmed experimentally; C-001→HIGH; Rust: 20k-clip build 3.4ms; Vec-split cost 45.6ms → E-002c needed |
| E-002c | Timeline primary structure: gap vs BTreeMap vs piece-table | [record](../../research/experiments/E-002c_timeline_structure.md) | E-002c_timeline_structure.rs | RUN: 4 structures × 4 workloads, hash-equality PASS — gap buffer 34× faster than Vec on cursor-local edits (1.1ms vs 37.4ms), O(1) cursor delete; BTreeMap wins scattered (1.8ms); walk bounded by rational add (~140ns) not traversal → ADR-011 PROPOSED; P3b finding: free-rational accumulators overflow i64 → fixed tick axis mandated (ADR-007 refinement) |
| E-003 | Command log + snapshot replay determinism | [record](../../research/experiments/E-003_command_replay_determinism.md) | E-003_command_replay_determinism.py | RUN: 9/9 PASS — replay/undo/snapshot all hash-exact; float payloads diverge 300/300; explicit-id schema lesson |
| E-004a | Rust FFI boundary pattern | [record](../../research/experiments/E-004a_ffi_boundary.md) | E-004a_ffi_boundary.rs + E-004a_driver.py | RUN: 5/5 PASS — exact i64 time + handles + panic containment viable |
| E-004b | UniFFI Kotlin codegen + runtime for ove-time | [record](../../research/experiments/E-004b_uniffi_kotlin.md) | scripts/experiments/E-004b_uniffi/ | RUN: codegen ✓ (1283-line kt, JNA-only dep); unmodified Kotlin compiles (kotlinc 2.1.20); LIVE JNA roundtrip 8/8 PASS incl. boundary-floor exactness + fp-free-API reflection check → ADR-001 codegen trigger cleared |
| E-006 | IPC transport costs (browser leg) | [record](../../research/experiments/E-006_ipc_costs.md) | E-006_ipc_browser_leg.cjs | PARTIAL-RUN: JSON invoke fine ≤~10KB (1–24µs); 1.4MB JSON = 42% of frame budget vs binary 0.5% (~89×) → doc-17 rule numerically grounded; structuredClone SLOWER than JSON on object-heavy payloads; real-Tauri leg BLOCKED (webkit2gtk) → E-006b |
| E-006a | IPC serialization costs, NATIVE side (fills E-006's unmeasured native leg) | [record](../../research/experiments/E-006a_ipc_serialization.md) | **crate `scripts/experiments/E-006a_ipc_crate/`** (code + raw output re-committed 2026-09-26, closing audit D-1/X-5) | RE-RUN (median of 30, corrected extrapolation basis): serde_json fine for commands (0.15ms/1000) + state (4.1ms/20k clips); frames disqualified (1 MiB RGBA → **4.00× blowup**, JSON 1080p60 ≈ **2.65 CPU-s/s ser + 5.0 de**; prior record's 1.6 CPU-s/s used an inconsistent basis — fixed honestly in the re-run); binary 18.6ms/frame and the true native path is zero serialization → binary path mandatory for pixels from BOTH sides |
| E-007 | Smart-render / stream-copy semantics (real media) | [record](../../research/experiments/E-007_smart_render.md) | E-007_smart_render.py | RUN: keyframe-exact copy cuts 72/72 ✓; mid-GOP copy cut selects WRONG content (1.5s request → 0.0s output) — keyframe index mandatory; timing leg invalid at micro scale → superseded by E-007b |
| E-007b | Smart-render timing at real resolutions | [record](../../research/experiments/E-007b_real_res_timing.md) | E-007b_real_res_timing.py | RUN: 1080p keyframe-aligned copy 67–69ms duration-exact vs re-encode 458–815ms → 6.8–11.9× speedup; full re-encode 1897ms/10s → segment-copy renderer confirmed |
| E-009 | Shared command surface: agent + UI edits in ONE log (Q-09) | [record](../../research/experiments/E-009_shared_command_surface.md) | E-009_shared_command_surface.py | RUN: 36/36 PASS — 10 command surfaces (8 payload + batch + undo) wrapped in MCP 2026-07-28 stdio; UI/wire hash-identical at every step; owner-agnostic replay; cross-ownership undo; atomic batches; float payloads rejected at both edges → ADR-010 **ACCEPTED** |
| ove-time | Permanent property suite for exact time | [crate](../../engine/ove-time/) | engine/ove-time (cargo test) | TESTED: 15/15 PASS (4 unit + 11 properties, incl. P3b overflow regression guard) — ADR-007 acceptance condition; evidence experiments/ove-time_property_suite.txt |

## Planned / defined

| ID | Question | Blocked by |
|---|---|---|
| E-004c | Real JNI on-device: thread attach, exception translation, callback marshalling | Android NDK/device |
| E-005 | wgpu YUV compositor pass-graph (WGSL, multi-layer) | real-GPU hardware (or deeper SwiftShader debugging) |
| E-006b | Real Tauri v2 IPC costs (invoke vs channel vs custom-protocol, incl. native side) | webkit2gtk + display in container |
| E-008 | Audio device latency probe matrix (proposed by doc 21: WASAPI/CoreAudio/ALSA round-trip) | audio hardware |
| E-010 | Scrub-burst harness (proposed by doc 34: hit-test + rehydrate latency under load) | timeline integration (Phase 2) |
| E-011 | Resumable-upload drill (proposed by doc 36: 308/Range checkpoint resume against real API) | YouTube API credentials |
| E-012 | Gap-buffer integration benchmark: far-jump cursor worst case + concurrent multi-track edits (ADR-011 revisit gate) | Phase 2 timeline integration |
| E-013 | Real Claude Code end-to-end over the E-009 MCP server (approval tiers, receipts) | agent runtime in CI |

## Experiment ID registry (avoid collisions)
Run: E-001, E-002, E-002b, E-002c (+E-002c2 addendum), E-003, E-004a, E-004b, E-006, E-006a, E-007, E-007b, E-009,
ove-time suite. Defined: E-004c, E-005, E-006b, E-008, E-010, E-011, E-012, E-013.
Wave-3 agents additionally proposed (mapped to free IDs): E-014 frame-budget breakdown
measured (doc 32), E-015 proxy throughput + relink fuzzing (doc 32/33), E-016
range-parallel scale-out crossover (doc 30/32).
Next free ID: **E-017**.
