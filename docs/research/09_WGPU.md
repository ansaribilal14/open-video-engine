> Status: PARTIAL (v0.1).
# 09 — WGPU (gfx-rs/wgpu)

> Owner: agent 3-d · Tracks: D · Date: 2026-09-21
> Scope: wgpu architecture, backends, surface/texture model, compute, upload patterns,
> maturity, license, ecosystem. Primary source: repo (S-2d1) fetched 2026-09-21.

## 1. What wgpu is (verified from repo README + Cargo.toml, 2026-09-21)

- "Cross-platform, safe, pure-Rust graphics API" — Rust implementation of the WebGPU
  standard as a *native* library.
- Runs natively on **Vulkan, Metal, D3D12, and OpenGL (via ANGLE-angle mapping), and on top
  of WebGL2 and WebGPU on wasm**. One API over all of them.
- Serves as **the core of the WebGPU integration in Firefox, Servo, and Deno** — the
  strongest possible "production credibility" signal for a browser-adjacent project.
- Version: **wgpu 30.0.1** (crates.io, published 2026-08-22; 30.0.0 on 2026-07-01; 28.0.0 on
  2025-12-18). Workspace crates: `wgpu`, `wgpu-core`, `wgpu-hal`, `naga` (shader
  compiler/translator), `wgpu-naga-bridge`, `wgpu-core-remote(-types)`.
- License: `MIT OR Apache-2.0` (Cargo.toml verified 2026-09-21). Safe for proprietary use.

## 2. Layered architecture

```
wgpu (public, WebGPU-shaped API)
  -> wgpu-core   (validation, resource tracking, command graph; platform-agnostic)
       -> wgpu-hal  (per-backend unsafe HAL: Vulkan/Metal/DX12/GL/GLES/noop)
            -> naga (WGSL/HLSL/GLSL/MSL cross-compile via wgpu-naga-bridge)
```

- **naga** is the shader front-end/back-end: WGSL in; SPIR-V, MSL, HLSL, GLSL out. This is
  what lets a *single* shader source serve desktop + web.
- **wgpu-hal** exposes raw backend capabilities (`as_hal` escape hatches exist, e.g.
  `Texture::mark_externally_initialized` for externally-written textures — in 30.x
  changelog), so platform-specific zero-copy imports are reachable without forking.
- Recent changelog items relevant to OVE: `wasm64-unknown-unknown` target support for the web
  backend (requires nightly toolchain), `TEXTURE_COMPONENT_SWIZZLE` feature, queue labeling,
  `theoretical_memory_footprint` on textures, extra storage-buffer/texture limit knobs
  matching new WebGPU limits.

## 3. Object model (WebGPU-shaped)

- `Instance` -> `Adapter` -> `Device` + `Queue` (one queue per device; queue is the only
  submission + upload + present channel).
- Surfaces: `Surface::new(instance, target)` + `configure()` with
  `SurfaceConfiguration{format, present_mode, alpha_mode, usage, view_formats}`.
  Present modes: Fifo (always supported), Mailbox, Immediate, AutoVsync/AutoNoVsync.
- Textures: explicit `TextureDescriptor` with `TextureUsages` bits (RENDER_ATTACHMENT,
  TEXTURE_BINDING, STORAGE_BINDING, COPY_SRC, COPY_DST); views with swizzle support
  (30.x feature `TEXTURE_COMPONENT_SWIZZLE`).
- Buffers: map_async/write_buffer; bind groups + pipeline layouts are explicit (no implicit
  reflection at draw time — good determinism for a render graph).
- **Compute passes**: `CommandEncoder::begin_compute_pass` -> `ComputePassEncoder`;
  storage buffers/textures, workgroup dispatch. Fully supported in the WebGPU shape, so any
  compute (histograms, flash/bloom pre-passes, ML pre/post) written for web also runs native.
- Render passes: explicit color/depth attachments, load/store ops (CLEAR/LOAD/DISCARD),
  render bundles for re-used command streams.

## 4. Upload patterns for video frames (critical for OVE)

| Pattern | API | Notes for OVE |
|---|---|---|
| CPU bytes -> texture | `Queue::write_texture` | Simple; extra copy; fine for thumbnails/stills |
| CPU bytes -> buffer -> texture | `Queue::write_buffer` + `copy_buffer_to_texture` | Amortizable; staging buffer ring |
| Import existing image/canvas | (web) `copy_external_image_to_texture` via GPUQueue | Covers canvas/ImageBitmap/OffscreenCanvas |
| **VideoFrame -> texture (web)** | `GPUDevice::import_external_texture` -> `GPUExternalTexture` binding | Zero-copy-ish path for YUV video; sampled in shader; single-use per pass (see doc 10/11) |
| Native HW frame -> texture | `wgpu-hal` external-memory / `as_hal` | Phase 2; per-platform (Vulkan external memory, IOSurface) |

Key rule: on the web the *fast* path is `importExternalTexture(VideoFrame)` (keeps YUV planes
in GPU/decoder memory, samples with `textureExternalE5`-style binding) and the *portable*
path is `copyExternalImageToTexture` (RGBA conversion at copy). On native there is no
`GPUExternalTexture`; the adapter must convert to a normal texture (upload or external
memory). This asymmetry is a first-class example of the C-002 platform-adapter need.

## 5. Ecosystem (spot-check via crates.io, 2026-09-21)

- **egui-wgpu** (0.36.2): immediate-mode UI renderer on wgpu — the obvious editor-UI
  integration path (timeline/inspector UI drawing into a wgpu surface or offscreen target).
- **encase** (0.13.0): WGSL uniform/storage buffer layout helper (std430-style packing).
- Render graphs: wgpu ships *no* render graph; engines bring their own (Bevy's render graph
  is the largest public example). For OVE the pass-graph compiler (doc 08 §5) is ours to
  build; wgpu is only the executor.
- Learning ecosystem: learn-wgpu, wgpu.rs examples, WebGPU Fundamentals.

## 6. Maturity / risk assessment

- Maturity: high. Firefox ships it (WebGPU since FF141 Windows, doc 10), Deno uses it,
  versioned releases ~3-4/year with disciplined changelogs; large CI + coverage.
- Risks for OVE: (1) release cadence churn — major versions bump frequently (28, 29, 30 in
  ~9 months), so pin and budget upgrade time; (2) GL backend is the weakest (feature gaps,
  ANGLE dependency) — treat GLES as fallback-only; (3) native video-frame import still needs
  custom hal work; (4) wasm32 memory ceiling until wasm64 matures (doc 12).

## 7. Decision-relevant conclusions

1. wgpu is the only serious Rust GPU abstraction that *is* the WebGPU API natively — writing
   the compositor against wgpu makes the WebGPU target nearly free (C-002 supported).
2. The compositor pass graph (doc 08) should be wgpu-executor-agnostic but wgpu-*shaped*
   (explicit bind groups, passes, resource pools map 1:1).
3. License (MIT OR Apache-2.0) imposes no obligation problems for the mission.

## Sources
- S-2d1 repo README/CHANGELOG/Cargo.toml (fetched 2026-09-21).
- crates.io /crates/wgpu, /crates/egui-wgpu, /crates/encase (fetched 2026-09-21).

## Depth tracking (v0.1)
- TODO: read wgpu-hal external memory story in depth; benchmark write_texture vs
  copy_external_image_to_texture vs importExternalTexture in E-001; survey naga WGSL feature
  coverage vs native HLSL workflows; study Bevy render graph for pass-graph patterns.
