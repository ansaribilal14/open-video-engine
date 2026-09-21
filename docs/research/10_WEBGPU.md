> Status: PARTIAL (v0.1).
# 10 — WEBGPU

> Owner: agent 3-d · Tracks: D, E · Date: 2026-09-21
> Scope: browser support matrix (verified today), feature set relevant to video (external
> textures, limits, float targets, compute), mobile state, risks. Sources: S-2d7 spec,
> S-2d8 caniuse, S-2d0 MDN BCD, vendor blogs (see ledger).

## 1. Support matrix (verified 2026-09-21: caniuse webgpu.json + MDN BCD GPUAdapter.json)

| Browser | Default-enabled | Notes |
|---|---|---|
| Chrome/Edge desktop | **113** (May 2023) on Windows/macOS/ChromeOS | Linux default only since **144** and gated to Intel Gen12+ GPUs per BCD note; earlier Linux behind flag (`enable-unsafe-webgpu`) |
| Chrome Android | **121** | and_chr current: yes |
| Firefox desktop | **141** (Jul 2025) — **Windows first** | macOS 26 Tahoe on Apple Silicon from **145**; Linux still behind `dom.webgpu.enabled`; all contexts except service workers |
| Firefox Android | **not supported** | BCD: `false` |
| Safari (macOS) | **26** (Sep 2025) | default-enabled on macOS 26 Tahoe+; earlier macOS versions of Safari 26 = partial (caniuse note #7) |
| Safari iOS/iPadOS/visionOS | **26** | caniuse ios_saf 26.0 = yes |
| Deno | 1.39+ (flagged) | BCD |

Milestone: by Nov 2025 all major engines ship WebGPU by default somewhere (webgpu.com report
"WebGPU Hits Critical Mass", Dec 2025) — but the *matrix is not uniform*: Firefox on
macOS<26 / Linux and all Firefox Android remain gaps, and Safari pre-26 has nothing.
For OVE this means WebGPU is the primary but not sole renderer; WebGL2 fallback (wgpu web
backend) is still required for a browser editor in 2026+.

## 2. Video-relevant feature set

- **GPUExternalTexture / importExternalTexture(VideoFrame | HTMLVideoElement)**: imports a
  video frame (often still YUV, decoder-owned) as a samplable external texture. Verified in
  BCD: Chrome (shipped with WebGPU; desktop consolidated note = 144 incl. Linux gate),
  Chrome Android 121, **Firefox 144 (Windows only)**, **Safari 26**. The Chromium
  "Intent to Ship: WebGPU WebCodecs integration" (blink-dev, Jun 2023) is the design record.
  Constraints: usable in exactly one begin/pass pair per `GPUCommandEncoder` lifetime
  (spec: external textures expire after use), no mips, no storage binding.
- **copyExternalImageToTexture (GPUQueue)**: the universal path — copies canvas/ImageBitmap/
  OffscreenCanvas/VideoFrame into an RGBA(-ish) GPU texture with color-space conversion.
  Works everywhere WebGPU does; costs a conversion copy per frame.
- **Color management**: WebGPU defines color conversions at `copyExternalImageToTexture()`
  and `importExternalTexture()` interface points (spec section verified today), supporting
  all `PredefinedColorSpace` values — relevant to doc 22 (color) for consistent preview.
- **Compute**: full compute passes with storage buffers/textures — usable for color
  conversion, histograms, LUT application.
- **Float16 / float32 targets**: `float32-filterable`, `rg11b10-ufloat` renderable,
  `shader-f16` etc. are *optional* features — availability varies by GPU/driver; OVE must
  feature-detect and fall back for HDR intermediate targets (open question Q3 in doc 08).
- **Timestamp query, subgroups**: optional features (perf analysis; do not depend on them).

## 3. Guaranteed limits that constrain a compositor (spec, verified today)

- `maxTextureDimension2D`: maximum 8192 (default 4096) — 8K preview needs adapter-limits
  negotiation, not assumption.
- `maxBufferSize`: **256 MiB** guaranteed maximum — big LUTs / atlas buffers must be split
  (native wgpu allows much larger via `downlevel`/native limits).
- `maxComputeWorkgroupStorageSize`: 16384 bytes; `maxComputeInvocationsPerWorkgroup`: 256
  (default 128) — constrains fancy parallel blur implementations on the guaranteed path.
- Higher limits are commonly available via `requestAdapter`/`requestDevice` + limit
  validation, but code must degrade gracefully (OVE rule: guaranteed limits are the contract,
  adapter limits are an optimization).

## 4. Mobile / Android state

- Chrome Android: yes since 121 (Vulkan-backed on most devices; caniuse and_chr 151 = yes).
- iOS Safari 26: yes (caniuse ios_saf 26.0). Before iOS 26: nothing (only the Safari TP).
- Firefox Android: no.
- Android reality check (for 3-e): GPU drivers on low-end Android remain the weak point;
  WebGPU-on-Vulkan errors/fallbacks must be expected. OVE preview must ship a WebGL2 or
  canvas2D degraded mode.

## 5. Interaction with WebCodecs (the core OVE preview path)

1. `VideoDecoder.decode()` -> `VideoFrame` (GPU-resident if HW decode, or CPU otherwise).
2. `device.importExternalTexture({source: videoFrame})` -> bind as `texture_external`
   in WGSL -> shader does YUV->RGB + transforms in one pass (best: no CPU roundtrip,
   no intermediate RGBA copy).
3. Or `queue.copyExternalImageToTexture({source: videoFrame}, texture, size)` when the pass
   graph needs a real texture (mips, multiple reads, storage writes).
4. Compose layers -> render pass to canvas context (`getContext('webgpu')` configure) for
   preview, or to offscreen texture + `copyExternalImageFromTexture`/canvas source for
   export-encode.

## 6. Risks

- R1: **Firefox matrix holes** (macOS pre-26 on non-AS or macOS<26, Linux flag, Android none)
  → WebGL2 fallback mandatory.
- R2: external-texture single-use semantics complicate multi-sampling of the same frame in
  one render graph (e.g., blur reads it twice) — must copy in that case; verify exact
  lifecycle in E-001.
- R3: Linux driver gates (Chrome 144 Intel Gen12+ note) mean "WebGPU supported" is not
  "WebGPU works" — runtime capability probing, not UA sniffing, is the OVE policy.
- R4: Safari 26 needs macOS 26 for default-enable; users on older macOS get WebGL2 path.

## Sources (fetched 2026-09-21)
- S-2d7 W3C WebGPU TR (limits, importExternalTexture, color sections).
- S-2d8 caniuse webgpu.json; S-2d0 MDN BCD GPUAdapter.json / GPUDevice#importExternalTexture.
- webkit.org "WebKit Features in Safari 26.0" (Sep 15, 2025).
- mozillagfx.wordpress.com "Shipping WebGPU on Windows in Firefox 141" (Jul 15, 2025).
- developer.chrome.com "What's New in WebGPU — Chrome 113"; blink-dev Intent to Ship
  WebGPU WebCodecs integration (Jun 16, 2023).

## Depth tracking (v0.1)
- TODO: full optional-feature matrix across engines (f16, timestamp-query, subgroups,
  bgra8unorm-storage); WGSL external-texture lifecycle experiment (E-001); canvas-config
  formats/HDR canvas status; WebGPU+OffscreenCanvas worker support matrix.
