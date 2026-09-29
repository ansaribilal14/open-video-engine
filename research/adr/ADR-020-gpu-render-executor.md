# ADR-020 — GPU reference executor (wave 9): wgpu backend with byte parity to software

Date: 2026-09-29
Status: Accepted
Context waves: BUILD_PLAN wave 6 ("GPU upgrade — E-005 promotion"); directive wave 9.

## Context

The software executor (ADR-014) is the correctness reference: deterministic integer
straight-alpha compositing, float-free, byte-identical on every platform. E-005
proved the wgpu compositor math (YUV→RGB, integer blending, readback) can be pixel-
exact on a rootless software-Vulkan stack (Mesa lavapipe), and left two engine rules:
coordinates from `@builtin(position)` never interpolated varyings; uint loads, never
float normalization. Wave 6's gate: **golden parity software↔GPU**; separate perf
bench with committed baselines; feature-detect + fallback.

## Decision

1. **Location & gating.** The GPU executor lives in `ove-render::gpu` behind the
   crate feature `gpu` (`wgpu` 25 + `pollster` optional deps). Default builds do not
   compile it; CI gains a dedicated `gpu-conformance` job so the main pipeline's
   cache/timing is untouched and the GPU leg has its own green/red signal.

2. **Same plan, same arithmetic.** `GpuRenderer::execute_frame` consumes the SAME
   `RenderPlan` as the software executor and replicates `exec.rs` arithmetic exactly:
   all textures `Rgba8Uint` (uint loads — no `Rgba8Unorm` quantization, no f32
   anywhere); per-pixel blend `d = 255·den`, `n_s = min(sa·a_num, d)`,
   `inv = d − n_s`, `out = (cs·n_s + cd·inv + d/2) / d` in u32; output alpha forced
   255; integer translate with bounds clip and transparency outside; ColorConvert
   stays a tag stamp (RGBA v1). Two pipelines (transform, blend) each own one bind
   group layout (0); every target is fully overwritten by the fullscreen triangle.

3. **Surface placement model (the subtle part).** A FETCH surface keeps its native
   frame size; sampling with native bounds-check reproduces software's transparent-
   padded paste-at-origin. A transform/blend TARGET is output-sized with content
   already at final output coordinates — later passes sample it directly. There is
   NO accumulated offset: each pass's own dx/dy does the placement (the first
   implementation double-counted it; the parity suite caught it at pixel 19).

4. **Typed denominator bound.** u32 arithmetic bounds the blend: worst case
   `255·d + d/2 = 65 152.5·den < 2³² ⇒ den ≤ 65 927`; enforced bound
   `MAX_ALPHA_DEN = 65 000` on the reduced denominator. Exceeding it is a typed
   `GpuError::UnsupportedAlphaDen` BEFORE any GPU work — the caller falls back to
   software (which executes any denominator). Never a clamp, never silent divergence.

5. **Feature-detect + fallback.** `GpuRenderer::new()` → `GpuError::NoAdapter`
   (typed, with the loader's reason) when no device is available; `Device` for
   device/IO failures; `Render(e)` re-uses the software executor's typed error set
   (identical TagMismatch / SourceFrameMissing / MissingSurface semantics — pinned
   by G-6). The output `FrameEnvelope` carries `BackendId::Other("ove-render/gpu
   (<adapter>)")` so provenance is always visible.

6. **Parity is the gate.** `tests/gpu_conformance.rs` (G-1..G-8): single layer;
   transforms incl. negative offsets and edge clipping; fractional alphas
   (1/3, 2/3, 1/2, 127/128, 1/1000); three-layer stack with a declared convert;
   retime frame selection (RG-4 via GPU); error parity; denominator-bound typed
   error + software fallback; span-level blake3 hash equality. Tolerance 0
   everywhere.

7. **Perf honesty.** `examples/gpu_perf_probe.rs` renders a 640×360 three-layer plan
   through both executors and stamps every line with the adapter class; software
   rasterizers (llvmpipe/lavapipe/swiftshader/Cpu device type) mark GPU timings
   INVALID for any claim (E-005 rule). The committed record is
   `research/experiments/W9_gpu_perf_record.txt`; real-hardware ratio gates
   (testing-strategy T-7) stay a named hardware-bound residual.

8. **Engine wiring deferred to the platform waves.** Wave 6's gate is parity + bench
   + fallback contract, all delivered here. Which leg (software/GPU) a SESSION uses
   is a platform decision (desktop/Android/browser have different adapter
   realities) — the selection lands with waves 10–12 behind the same typed contract.

## Consequences

- Byte parity GPU↔software is now tested in CI (lavapipe job), not just claimed.
- The GPU path covers the RGBA v1 pass set (Fetch/Convert/Transform/Blend).
  YUV GPU conversion, fractional scaling with declared tolerance, and zero-copy
  import remain named residuals — they extend this pipeline, they do not rework it.
- `cargo test --workspace` (default features) is unaffected; `--features gpu`
  builds add wgpu (~2–4 min cold) only in the dedicated job.

## Reopen conditions

- An adapter whose uint-texture readback or u32 arithmetic diverges from the
  software bytes (parity job red) → the bound/contract conversation reopens.
- A need for den > 65 000 on GPU forces u64 emulation (two u32 lanes) or a
  normalized pre-multiplied representation — a documented design change, not a
  silent clamp.
- A platform leg needs swapchain present/output-to-surface — extends
  `execute_frame` with an optional target view; parity tests stay the gate.
