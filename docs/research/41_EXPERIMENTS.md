# 41 — EXPERIMENTS

> Status: PARTIAL (4 experiments run 2026-09-23; E-002c/E-004b/E-006/E-007b defined).
> Records follow the directive format: QUESTION / HYPOTHESIS / IMPLEMENTATION / HARDWARE /
> RESULT / LIMITATIONS / DECISION. Raw outputs in `experiments/`, code in
> `scripts/experiments/`.

| ID | Question | Record | Code | Result |
|---|---|---|---|---|
| E-001 | WebCodecs→WebGPU browser pipeline | [record](../../research/experiments/E-001_webcodecs_webgpu.md) | E-001_cdp_v2.cjs (+E-001a sweep) | PARTIAL-RUN: H.264 decode 48/48 ✓ (real media, Chrome 154); GPU-import legs blocked by software rendering; secure-context discovery; support matrix captured |
| E-002 | fp-vs-rational time + timeline structure | [record](../../research/experiments/E-002_time_representation.md) | E-002_time_representation.py + E-002b_timeline_bench.rs | RUN: 13/13 PASS — fp unsafe confirmed experimentally; C-001→HIGH; Rust: 20k-clip build 3.4ms; Vec-split cost 45.6ms → E-002c needed |
| E-003 | Command log + snapshot replay determinism | [record](../../research/experiments/E-003_command_replay_determinism.md) | E-003_command_replay_determinism.py | RUN: 9/9 PASS — replay/undo/snapshot all hash-exact; float payloads diverge 300/300; explicit-id schema lesson |
| E-004a | Rust FFI boundary pattern | [record](../../research/experiments/E-004a_ffi_boundary.md) | E-004a_ffi_boundary.rs + E-004a_driver.py | RUN: 5/5 PASS — exact i64 time + handles + panic containment viable |
| E-007 | Smart-render / stream-copy semantics (real media) | [record](../../research/experiments/E-007_smart_render.md) | E-007_smart_render.py | RUN: keyframe-exact copy cuts 72/72 ✓; mid-GOP copy cut selects WRONG content (1.5s request → 0.0s output) — keyframe index mandatory; timing leg invalid at micro scale |

## Planned / defined

| ID | Question | Blocked by |
|---|---|---|
| E-002c | Timeline primary structure: gap-buffer vs BTreeMap vs piece-table (Vec rejected) | — (next session) |
| E-004b | UniFFI Kotlin bindings codegen for ove-time API | — |
| E-004c | Real JNI: thread attach, exception translation, callback marshalling | Android NDK/device |
| E-005 | wgpu YUV compositor pass-graph (WGSL, multi-layer) | real-GPU hardware (or deeper SwiftShader debugging) |
| E-006 | Tauri IPC: JSON invoke vs channel vs custom-protocol frame costs | — |
| E-007b | Smart-render timing at real resolutions (1080p multi-Mbps) | — |
