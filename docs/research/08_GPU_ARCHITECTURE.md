> Status: PARTIAL (v0.1).
# 08 — GPU / COMPOSITOR ARCHITECTURE

> Owner: agent 3-d · Tracks: D (GPU/compositor) · Date: 2026-09-21
> Scope: how video editors structure the frame-composition stack — layer tree, blend modes,
> transforms, render targets, effects passes, masks, LUTs, text, preview vs export loops,
> and where the GPU/CPU boundary sits. Basis for claims C-002 and C-006.

## 1. What a video-editor compositor must do (requirement set)

A compositor is the subsystem that turns a *timeline state* (layers/clips/parameters at time T)
into one output frame. Derived from the reference systems surveyed below, the minimum
capability set is:

1. **Ordered layer/track compositing** — bottom-up (or top-down) evaluation of a stack of
   layers, each with opacity and blend mode.
2. **Transform stacks** — per-layer 2D/3D affine (position, scale, rotation, anchor,
   perspective), often nested (groups/nested sequences), resolved to a single matrix per layer
   before rasterization.
3. **Blend modes** — at minimum the PDF/porter-duff extended set: normal, multiply, screen,
   overlay, darken, lighten, color-dodge, color-burn, hard-light, soft-light, difference,
   exclusion, hue, saturation, color, luminosity. These map 1:1 to WGSL/WebGPU and Vulkan
   blend equations for the separable subset; the non-separable (hue/sat/color/luminosity) ones
   need a per-pixel shader, not fixed-function blending.
4. **Effects as render passes** — Gaussian blur, glow, sharpen, color correction, etc.
   require intermediate render targets and multi-pass algorithms (separable blur = 2 passes;
   anything with a wide kernel is a pass, not inline math).
5. **Masks** — per-layer alpha masks (video layer, shape, text), including animated mask
   paths; typically implemented as an offscreen mask texture multiplied/stenciled in.
6. **Color management + LUTs** — source color space -> working space -> display space
   conversion; 1D LUTs for tone curves, 3D LUTs (.cube) for creative looks. libplacebo treats
   LUTs as first-class render-graph inputs (`pl_render_params` color-mapping LUT, applied in
   normalized RGB space — see S-2d5).
7. **Text rendering** — layout (shaping, line breaking) is CPU work (HarfBuzz-class
   machinery); rasterization/glyph atlas upload is GPU work. Text must be rendered per-frame
   or cached by content hash.
8. **Frame loop separation** — preview loop (deadline-bound, drops work) vs export loop
   (deadline-free, deterministic, must consume every frame exactly once in order).
9. **Time mapping** — every input is sampled at a *timeline time*, which maps to different
   *media times* per clip (speed ramps, stills, frozen frames).

## 2. Reference architecture: libplacebo (mpv's renderer)

libplacebo (S-2d5, source verified 2026-09-21) is "the core rendering algorithms and ideas of
mpv rewritten as an independent library". Key structural lessons:

- **The renderer is a pass-graph builder with caching.** `pl_render_image()` takes a source
  frame (`pl_frame_from_avframe` — decode-agnostic input) and a target, plus a
  `pl_render_params` struct (S-2d5, `src/include/libplacebo/renderer.h`): upscaler/downscaler
  algorithm pointers, `blend_params`, `hooks` (user shader chain), color-mapping LUT, overlays
  (subtitles/OSD composited alongside video), and an `info_callback` that reports each
  executed `pl_dispatch_info` pass. The renderer internally decomposes the request into GPU
  passes, caches compiled pipelines/pass signatures, and skips disabled passes.
- **Hooks = programmable insertion points.** Custom shaders (RAVU, Anime4K, FSRCNNX) are
  inserted as graph hooks at named stages. This is exactly the "effects as render passes"
  requirement, exposed to users.
- **Color is a core concern, not an afterthought**: dynamic HDR tone mapping with scene
  measurement, Dolby Vision profile 5 reshaping, ICC color management, 3DLUT application,
  gamut mapping, BT.1886 emulation. Backends: Vulkan (incl. MoltenVK), OpenGL, D3D11.
- **Anti-lesson**: libplacebo is single-image (one input -> one output). It is a *video
  processing* engine, not a multi-layer editor compositor; an editor needs the layer/blend/
  mask model on top of this class of machinery.

## 3. Reference architecture: OBS Studio (libobs)

OBS (S-2d6, docs verified 2026-09-21) is the live-production counterpart:

- libobs has a "custom-made programmable graphics subsystem that wraps both Direct3D 11 and
  OpenGL" and "most rendering is dependent upon effects" (docs.obsproject.com, Rendering
  Graphics). I.e., an abstraction layer (gs_* API) over two native backends + an effect
  (.effect shader) system — not a DAG render graph.
- The video pipeline (General Video Pipeline Overview): sources are rendered into textures per
  tick; each source has an ordered filter chain (effects applied to intermediate textures);
  scenes composite their items with transforms, opacity and blend modes; the final mix goes to
  preview displays and to encoders. `obs_enter_graphics()`/`obs_leave_graphics()` guard the
  single graphics context; a dedicated graphics thread owns it.
- **Relevant lesson**: the "render graph" in a mature production tool is actually a small,
  fixed, immediate-mode composition loop with per-source filter chains — the complexity lives
  in source plugins and effect shaders, not in a general scheduler. This is a deliberate
  robustness trade (predictable latency > optimality) and explains why OBS scales to live use.

## 4. Reference architecture: browser editors

- **Clipchamp** (S-2d4): moved its in-browser pipeline to WebCodecs (W3C Web and TV talk,
  2021) — decode via `VideoDecoder`, composition off-main-thread, export via `VideoEncoder`.
  Now shipping inside Microsoft 365. (Detailed study pending; 3-a owns editor catalog.)
- **Mediabunny-based editors** (S-2d3): the current generation of TS-first web media
  libraries (Remotion, Diffusion Studio, Tella, Screen Studio are sponsors/users per the
  README) standardize on WebCodecs + canvas/WebGPU composition + a TS muxer.
- Browser editors converge on: decode in workers -> `VideoFrame` -> canvas/WebGPU draw ->
  `VideoFrame`/canvas -> `VideoEncoder` -> muxer (JS) -> blob/OPFS. Composition is typically
  a canvas 2D/WebGL immediate draw list per frame, not a retained GPU graph.
- The gap vs native: no cross-vendor compute/geometry; texture upload costs; and (pre-Safari
  26) fragmented WebGPU. This is the core of claim C-002.

## 5. Proposed OVE compositor shape (draft, to be ADR-ized)

```
Timeline state (project model, shared core)
   |
   v
Compositor "scene description" (pure data, serializable)
   - ordered layers: {source, transform, blend, opacity, masks[], effects[], crop}
   - output contract: {w,h,rate,colorspace,tone-map}
   |
   v
Render graph compiler (shared, platform-neutral logic)
   - resolves layer stack into a DAG of passes:
     decode/source -> tone/space convert -> per-layer: transform+effects chain -> mask ->
     blend into accumulator -> master LUT/grade -> present
   - resource planner: pools offscreen render targets, reuses by (size,format,age)
   - caching: pipeline cache keyed by (shader, layout, formats); mask/text atlas caches
   |
   v
Platform adapter (the ONLY thing that differs per platform)
   - browser:  wgpu→WebGPU backend; VideoFrame import via importExternalTexture (zero-copy,
     YUV read in shader) or copyExternalImageToTexture (RGBA copy); canvas present
   - desktop:  wgpu native backends (Vulkan/Metal/D3D12/GL) + same pass graph; hardware
     decode import via platform-specific external memory (Vulkan external memory / IOSurface)
     — phase 2, start with plain upload
   - android:  same, surface from ImageReader/SurfaceTexture (3-e owns)
```

Design rules (draft):

1. **One pass-graph compiler, many executors.** The compiler emits a platform-neutral pass
   list; each adapter executes it. This is the concrete meaning of C-002: not "a different
   engine per platform" but "one graph, thin backends".
2. **Blend accumulation via single target + ping-pong** for non-fixed-function blend modes;
   fixed-function blends can use hardware blending where available (native) but the portable
   baseline is shader blending (WebGPU fixed-function blending is supported, but non-separable
   modes are not).
3. **Offscreen targets**: pool of GPU textures, double-buffered accumulator; never allocate
   per frame in the preview loop.
4. **Preview loop** is vsync/callback-driven, renders at the playhead, may drop effects tiers
   (progressive refinement: full quality when idle). **Export loop** is a pull loop: `for t in
   frames: render(t) -> encode(t)`, no drops, no vsync dependency, deterministic.
5. **LUT application** as an explicit graph stage: 1D curves in one pass; 3D LUT as a
   3D-texture sampled in the final color pass (matches libplacebo's normalized-RGB LUT
   placement).
6. **Text**: layout cache (CPU, shared core) -> glyph atlas (GPU, per-adapter) -> cached
   quads; re-rasterize on parameter change only.
7. **GPU/CPU boundary**: decode stays in platform codec layers (C-006); composition is GPU;
   timeline evaluation is CPU; histogram/analysis passes (for auto-tone) can be GPU readback
   — readback is the expensive boundary crossing and must be rate-limited or avoided.

## 6. Open questions

- Q1: fixed-function vs shader-only blending cost on mobile WebGPU (fidelity vs perf).
- Q2: does the export loop run in the same GPU device as preview (single device contention)
  or a second device/worker? (Browser: OffscreenCanvas/WebGPU in worker supports both.)
- Q3: HDR export on the web — can float16 render targets + tone map be replicated
  cross-browser? (Partially blocked: WebGPU float32-blendable feature availability varies.)
- Q4: nested-sequence (compound clip) representation in the pass graph (sub-graph inlining).

## Depth tracking (v0.1)

- Done: requirement set; libplacebo + OBS + browser-editor structure pass; draft graph design.
- TODO: Olive/Kdenlive compositor internals (3-b), Blender sequencer draw manager study,
  read the libplacebo dispatch/graphics internals in depth, measure pass-graph overhead in
  E-001, study canvas2D/WebGL fallback design, Study DaVinci/Resolve public talks on their
  GPU pipeline for pro-tier reference.
