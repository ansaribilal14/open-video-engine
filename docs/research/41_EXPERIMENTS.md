# 41 — EXPERIMENTS

> Status: PARTIAL (11 experiments run 2026-09-23/26; E-004c runtime / E-005 perf / E-006b remain defined;
> E-008/E-010/E-011/E-013 proposed by wave-3 agents).
> 2026-09-24 update: E-005 correctness leg RUN+PASS (software Vulkan), E-004c build
> leg PASS (NDK) — runtime/hardware residuals stay named in GATE_STATUS.
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
| E-004c | Rust→Android device leg | [record](../../research/experiments/E-004c_android_build_leg.md) | scripts/experiments/E-004b_uniffi/ (same crate, NDK build) | PARTIAL-RUN: BUILD leg PASS — libove_time_ffi.so for aarch64/Android 24 via NDK r27c, full uniffi fn+checksum surface exported (readelf), DT_NEEDED=libdl/libc only; zigbuild disqualified (no bionic). RUNTIME leg (System.loadLibrary + JNI dispatch) device-bound |
| E-005 | GPU compositor leg: wgpu YUV→RGB composite | [record](../../research/experiments/E-005_wgpu_compositor.md) | scripts/experiments/E-005_wgpu_swvk/ | PARTIAL-RUN: CORRECTNESS PASS 4096/4096 px tolerance=0 vs host reference on rootless Mesa lavapipe (Vulkan 1.3 sw, wgpu 25.0.2); copy-path upload exercised; interpolated-UV offset bug found → builtin-position rule for frame-exact passes; PERF leg INVALID (llvmpipe); zero-copy import still real-GPU-bound |
| E-006 | IPC transport costs (browser leg) | [record](../../research/experiments/E-006_ipc_costs.md) | E-006_ipc_browser_leg.cjs | PARTIAL-RUN: JSON invoke fine ≤~10KB (1–24µs); 1.4MB JSON = 42% of frame budget vs binary 0.5% (~89×) → doc-17 rule numerically grounded; structuredClone SLOWER than JSON on object-heavy payloads; real-Tauri leg BLOCKED (webkit2gtk) → E-006b |
| E-006a | IPC serialization costs, NATIVE side (fills E-006's unmeasured native leg) | [record](../../research/experiments/E-006a_ipc_serialization.md) | E-006a_ipc_serialization.rs (crate E-006a_ipc_crate/) | RUN: serde_json fine for commands (0.33ms/1000) + state (5.4ms/20k clips); frames disqualified (1 MiB RGBA → 3.57× blowup, 1080p60 ≈ 1.6 CPU-s/s); bincode/postcard 10–20× cheaper → binary path mandatory for pixels from BOTH sides |
| E-007 | Smart-render / stream-copy semantics (real media) | [record](../../research/experiments/E-007_smart_render.md) | E-007_smart_render.py | RUN: keyframe-exact copy cuts 72/72 ✓; mid-GOP copy cut selects WRONG content (1.5s request → 0.0s output) — keyframe index mandatory; timing leg invalid at micro scale → superseded by E-007b |
| E-007b | Smart-render timing at real resolutions | [record](../../research/experiments/E-007b_real_res_timing.md) | E-007b_real_res_timing.py | RUN: 1080p keyframe-aligned copy 67–69ms duration-exact vs re-encode 458–815ms → 6.8–11.9× speedup; full re-encode 1897ms/10s → segment-copy renderer confirmed |
| E-009 | Shared command surface: agent + UI edits in ONE log (Q-09) | [record](../../research/experiments/E-009_shared_command_surface.md) | E-009_shared_command_surface.py | RUN: 36/36 PASS — 10 command surfaces (8 payload + batch + undo) wrapped in MCP 2026-07-28 stdio; UI/wire hash-identical at every step; owner-agnostic replay; cross-ownership undo; atomic batches; float payloads rejected at both edges → ADR-010 **ACCEPTED** |
| E-012 | Timeline structure INTEGRATION: ADR-011 revisit gate (gap+lazy-index vs augmented AVL; scrub budget; multi-track) | [record](../../research/experiments/E-012_timeline_integration.md) | engine/ove-timeline (crate; tests/properties.rs; src/bin/e012_bench.rs) | RUN: properties 10/10; bench hash-cross-validated — W4 scrub-burst DECISIVE (gap+lazy p99 3.62ms **FAIL** vs 1ms budget; AVL p99 0.52ms **PASS**); AVL wins random 6×/far-jump 10×; gap wins cursor-local 8×/by-id scan 3.4×/small-N 3.2×; **schema rule #4: engine-allocated ids ride the log explicitly** (implicit allocation diverges replay — reproduced then fixed) → ADR-011 **ACCEPTED (REVISED: AVL primary)** |
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
| E-013 | Real Claude Code end-to-end over the E-009 MCP server (approval tiers, receipts) | agent runtime in CI |

## Experiment ID registry (avoid collisions)
Run: E-001, E-002, E-002b, E-002c (+E-002c2 addendum), E-003, E-004a, E-004b, E-004c (build leg), E-005 (correctness leg), E-006, E-006a, E-007, E-007b, E-009, E-012,
ove-time suite. Defined: E-006b, E-008, E-010, E-011, E-013. Residuals:
E-004c runtime (device), E-005 perf+zero-copy (real GPU), E-006b (webkit2gtk + display).
Wave-3 agents additionally proposed (mapped to free IDs): E-014 frame-budget breakdown
measured (doc 32), E-015 proxy throughput + relink fuzzing (doc 32/33), E-016
range-parallel scale-out crossover (doc 30/32).
Next free ID: **E-017**.
