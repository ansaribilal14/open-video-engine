# E-005 — GPU compositor leg: wgpu YUV→RGB composite, software-Vulkan correctness run (2026-09-24)

- **Question**: does the engine's compositor core math (decode-frame consumption →
  YUV→RGB conversion → scale → composite → readback) run correctly on wgpu, end to
  end, with pixel-exact verification?
- **Result**: **CORRECTNESS LEG PASS** — 4096/4096 pixels tolerance=0 vs host
  reference, on Mesa lavapipe (Vulkan 1.3 software). Perf leg INVALID (software);
  zero-copy import leg still real-GPU-bound. Raw record:
  `experiments/E-005_result.txt`.

## Why software Vulkan counts (and what it does not prove)

GATE-4's residual demanded a run of *our compositor code* — E-001 proved the browser
stack blocks at SwiftShader's `importExternalTexture`, but a **native wgpu** pipeline
only needs a Vulkan loader + ICD, which can be installed **rootless**: `apt download
libvulkan1 mesa-vulkan-drivers` + `dpkg -x` into a home directory, `VK_ICD_FILENAMES`
pointing at the extracted lavapipe JSON. The run therefore exercises *our* pipeline
(Naga shader compilation, bind groups, render pass, copy-to-buffer readback,
unified-memory semantics) — everything except GPU hardware itself.

It does NOT prove: GPU performance (llvmpipe timings are recorded but INVALID for any
claim), zero-copy import paths (`importExternalTexture` browser, AHardwareBuffer,
`VK_EXT_external_memory`) — those remain hardware/driver residuals.

## Method

- Harness: `scripts/experiments/E-005_wgpu_swvk/` (wgpu 25.0.2, pollster; ~400 lines).
- Two procedural "decoded frames" (deterministic stand-ins for decoder output):
  frame A 64×64 YUV420p (gradient + chroma blocks), frame B 16×16 (radial luma ramp +
  checkerboard chroma). Planes uploaded as `R8Uint` textures via `queue.write_texture`
  — the **copy path** ADR-004 designates as V1 universal.
- One fragment shader pass: BT.601 limited-range integer YUV→RGB
  (`(298·(Y−16) + 409·(V−128) + 128) >> 8`, arithmetic shifts on i32), nearest-
  neighbor 2× upscale of frame B into a 32×32 rect (integer `local·src/dst` mapping,
  `textureLoad` — no sampler state), over-composite onto frame A. Output `Rgba8Unorm`
  → `copy_texture_to_buffer` → `map_async` readback.
- Host reference in Rust runs the **identical integer math**; comparison is exact
  (tolerance 0), 64×64×4 channels.

## Findings

1. **PASS**: 4096/4096 pixels match the host reference exactly. The wgpu pipeline —
   shader compile (Naga), texture upload, render, readback — is numerically correct
   on lavapipe. This retires "compositor core math never ran" and leaves GATE-4 with
   only hardware-bound residuals.
2. **Own bug found & fixed — interpolated UVs are NOT frame-exact**: the first run
   used `@location(0) uv` from a fullscreen triangle and produced a **constant
   (+8,+7) pixel translation** on llvmpipe (verified by hand: composite@(8,7) ==
   readback@(0,0)). Switching to `@builtin(position)` (framebuffer coords − 0.5)
   is exact. **Engine rule**: frame-exact compositor passes must derive coordinates
   from `@builtin(position)`, never interpolated varyings. Diagnostic passthrough
   mode preserved in the harness (`E005_DIAG=1` → raw YUV readback).
3. **WGSL lesson**: `if` is a *statement* in WGSL, not an expression
   (`let x = if … ` is invalid); use `var` + `if/else` assignment.
4. **Rootless software-Vulkan stack is reusable**: `/home/z/vkroot/env.sh`
   (`VK_ICD_FILENAMES` + `LD_LIBRARY_PATH`) turns any CI/GitHub-Actions runner into a
   correctness-leg runner for future compositor passes — no GPU, no root, no container
   changes. Perf-ratio gates (doc 45 T-7) still require real hardware and stay manual.
5. Reuse: this shader + harness is the seed of `engine/ove-compositor` (pass #1:
   YUV→RGB, nearest-scale, over). Next compositor legs: fractional scaling (declared
   tolerance per doc 45 T-4), color-tag propagation (doc 22), multi-layer pass graph.

## Classification (honest)

| Leg | Status |
|---|---|
| Compositor math on wgpu (correctness) | **RUN, PASS** (software Vulkan, tolerance 0) |
| Performance | **INVALID** (llvmpipe; 10 renders 1.4 ms recorded, never citable) |
| Zero-copy frame import | **NOT RUN** (real GPU/driver + browser legs; E-001 harness ready for real-GPU rerun) |
