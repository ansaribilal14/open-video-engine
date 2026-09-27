> Status: PARTIAL (v0.1).
# 11 — WEBCODECS

> Owner: agent 3-d · Tracks: E, B · Date: 2026-09-21
> Scope: the full WebCodecs surface (VideoDecoder/VideoEncoder/VideoFrame/EncodedVideoChunk/
> Audio*), codec strings, HW/SW paths, browser matrix (verified today), the mux/demux gap,
> worker/transfer semantics, VideoFrame->GPU path, and who actually ships it.
> Verdict target: claim C-005.

## 1. What WebCodecs is

Low-level per-frame codec access: "work with components of a video stream, such as frames
and unmixed chunks of encoded video" (developer.chrome.com, Jan 2025). The W3C TR
(S-2d0/S-2d2, fetched 2026-09-21) defines, per media type, a symmetric pair:

- **VideoDecoder / VideoEncoder**, **AudioDecoder / AudioEncoder**
- **VideoFrame / AudioData** — uncompressed media as [Serializable, Transferable] objects,
  `Exposed=(Window, DedicatedWorker)` (verified in TR IDL today: both VideoFrame and
  AudioData carry these extended attributes).
- **EncodedVideoChunk / EncodedAudioChunk** — compressed bytes + timestamp + type
  (key/delta).
- **Config model**: `configure(AudioDecoderConfig|VideoDecoderConfig)`; static
  `isConfigSupported(config)` returns `Audio/VideoDecoderSupport{supported, config}`.
  VideoDecoderConfig fields (verified in TR): `codec` (string), `description` (ArrayBuffer —
  avcC/hvcC/sequence headers), `codedWidth/codedHeight`, `displayAspectWidth/Height`,
  `colorSpace`, `hardwareAcceleration` ("no-preference" default | "prefer-hardware" |
  "prefer-software"), `optimizeForLatency`, `rotation`, `flip` (encoder side), plus
  bitrate/framerate/latencyMode/keyFrame/alpha/scaleDownBy on the encoder side.
- **Processing model**: async control-message queues — `decode()`/`encode()` enqueue;
  `decodeQueueSize`/`encodeQueueSize` reflect backlog (TR §2.1); outputs arrive via
  callbacks (`output(frame)`, `error(err)`); `flush()` drains; `reset()`/`close()` control
  lifecycle. No pull API — the app must manage backpressure itself (count in-flight frames).

## 2. Codec strings (registry verified today: w3.org/TR/webcodecs-codec-registry/)

- Audio: `flac`, `mp3`, `mp4a.*` (AAC), `opus`, `vorbis`, `ulaw`, `alaw`, `pcm-*` (linear
  PCM family).
- Video: `av01.*` (AV1), `avc1.*`/`avc3.*` (H.264/AVC), `hev1.*`/`hvc1.*` (HEVC), `vp8`,
  `vp09.*` (VP9). (VVC/H.266 has a draft registration on the WG GitHub — not in the TR
  fetched today; mark UNVERIFIED for browser support.)
- Format rules: AVC strings are full ISO level/profile descriptors, e.g.
  `avc1.640028` (High profile, level 4.0), `avc1.42E01E` (Baseline 3.0); VP9 strings carry
  profile/level/bit-depth, e.g. `vp09.00.10.08`; AV1 carries level/tier/monochrome/subsampling
  e.g. `av01.0.04M.08`. `description` bytes carry the decoder-specific record (avcC for AVC,
  hvcC for HEVC) — when absent, in-band (Annex-B style) is assumed for AVC.
- **Rule for OVE**: always probe with `isConfigSupported()` per codec string, never assume —
  support is a function of browser x OS codecs x hardware, and differs between decode and
  encode.

## 3. Hardware vs software decode/encode

- `hardwareAcceleration: "prefer-hardware" | "prefer-software" | "no-preference"` is a
  *hint*, not a guarantee (TR). `"prefer-hardware"` failing does not reject the config;
  implementations may fall back to software.
- Chromium: HW-accelerated decode/encode through its platform codec stack (the same one
  behind `<video>`); the "Video processing with WebCodecs" guide (Jan 2025) documents
  hardware-tuned usage. Historically HW encode selection was Windows/Android-first, macOS
  later — exact per-OS encode matrix UNVERIFIED at v0.1, to be probed in E-001 on real
  machines.
- Safari: WebKit uses VideoToolbox (HW) for VideoDecoder/Encoder; HEVC decode is available
  where the OS supports it.
- Firefox: platform codecs (mfmedia/FFmpeg/DXVA etc.); HW paths exist but are the least
  documented — UNVERIFIED at v0.1.
- Consequence for OVE: encoder throughput/quality is a *runtime capability*; export presets
  must be built from `isConfigSupported` + measured HW availability, with a software
  fallback ladder (H.264 SW -> VP9/VP8 SW -> AV1 SW) or a wasm fallback (doc 12) — this is
  the C-006 argument inside the browser itself.

## 4. Browser support matrix (verified 2026-09-21, caniuse webcodecs.json + MDN BCD)

| API | Chrome | Edge | Safari | Firefox | Firefox Android | Chrome Android |
|---|---|---|---|---|---|---|
| VideoDecoder | 94 (2021) | 94 | 16.4 (2023) | **130** (Sep 2024) | **no** | 94+ |
| VideoEncoder | 94 | 94 | 16.4 | 130 | **no** | 94+ |
| VideoFrame | 94 | 94 | 16.4 | 130 | 130 (mirror) | 94+ |
| EncodedVideoChunk | 94 | 94 | 16.4 | 130 | **no** | 94+ |
| AudioDecoder | 94 | 94 | **26** | 130 | **no** | 94+ |
| AudioEncoder | 94 | 94 | **26** | 130 | **no** | 94+ |

- caniuse marks Safari 16.4-18.7 "partial (video-only)" — Audio* arrived with Safari 26
  (Sep 2025), full "yes" from 26.0 (caniuse + BCD agree).
- **Firefox Android has no WebCodecs at all** (BCD `false`) — a hard hole for a mobile-web
  target; mobile browsers: Chrome Android yes, Samsung Internet 17+ yes (caniuse).
- Summary: video-only WebCodecs is **baseline across all desktop engines in 2026**; the
  full API (audio in/out) is desktop-complete; mobile-web remains Chromium-only.

## 5. The muxing gap (no native container muxing)

WebCodecs produces *elementary streams* only — `EncodedVideoChunk`s with no container.
Shipping an MP4/WebM requires a JS/WASM muxer:

- **Mediabunny** (S-2d3, README fetched 2026-09-21): pure-TypeScript, zero-dependency,
  MPL-2.0. "Read and write MP4, MOV, WebM, MKV, HLS, WAVE, MP3, Ogg, ADTS, FLAC, MPEG-TS";
  "built-in encoding & decoding... hardware-accelerated using the WebCodecs API"; microsecond
  timestamps; streaming I/O for files of any size; tree-shakable (min 5 kB); runs in Node/
  Bun/Deno via @mediabunny/server. This is the current de-facto "FFmpeg for the web" and the
  leading candidate for OVE's web media layer. Sponsors/users include Remotion, Diffusion
  Studio, Tella, Screen Studio, Gling, Mux (README) — strong production signal.
- **mp4box.js**: demux/inspect MP4 (fragmentation, extraction) — battle-tested for demux and
  analysis; muxing exists but is not its strength.
- **mp4-muxer / webm-muxer** (Vanilagy): the predecessors of Mediabunny; now superseded
  (Mediabunny is by the same author). Fine to treat as legacy.
- Gap note: none of these remux exotic containers (e.g. MXF, PRORES MOV variants beyond
  basic); pro-source ingest may still need wasm/native probing (doc 12, C-006).

## 6. The demuxing gap

WebCodecs does **not** demux. You must extract `EncodedVideoChunk`s from containers
yourself: Mediabunny `Input` (read MP4/MOV/WebM/MKV/HLS/TS...), mp4box.js for MP4, or
wasm-FFmpeg for everything else (probe/demux only, keep decode native). Demuxing is
cheap (byte-copy + parse) so wasm is acceptable here; decode is where wasm hurts (doc 12).

## 7. Frame plumbing: workers, transfer, presentation

- `VideoFrame` is Transferable + Serializable (TR IDL verified today): pass by **transfer**
  (zero-copy, source becomes invalid) between window and DedicatedWorker — decode workers ->
  render/compositor -> encode workers without main-thread pixel copies.
- `VideoFrame.close()` releases underlying resources — mandatory discipline; forgetting it
  exhausts GPU/memory pools (Chrome enforces frame limits).
- Timestamps: microseconds; frames carry `duration`, `timestamp`, `visibleRect`, rotation
  metadata — sufficient for a frame-accurate timeline (doc 20 interplay).
- Callbacks run on the owning context's event loop; for steady pacing, OVE should run
  decoders in dedicated workers and schedule composition per frame rather than relying on
  rAF for export (rAF is present-tied; export uses its own pull loop, doc 08 §5.4).

## 8. VideoFrame -> GPU (preview path, ties to doc 10)

1. `importExternalTexture({source: videoFrame})` — bind as `texture_external`; shader does
   YUV->RGB conversion; no CPU copy; single-use per encoder (verify lifecycle, E-001).
2. `queue.copyExternalImageToTexture({source: videoFrame}, ...)` — portable, gives a real
   RGBA texture (mip/filterable/storage-capable) at the cost of a conversion copy.
3. Canvas path (fallback/compositing with 2D): `canvas.drawImage(videoFrame)` /
   `OffscreenCanvas` in a worker, then WebGPU `copy_external_image_to_texture` from that
   canvas — two copies; only for degraded mode.
- VideoFrame is also a `CanvasImageSource` in its own right (TR: usable in drawImage),
  which is the canvas-2D editor path.

## 9. Who ships WebCodecs in production (evidence)

- **Clipchamp (Microsoft)** — full in-browser editing pipeline on WebCodecs (W3C talk,
  S-2d4; shipped in Microsoft 365).
- **Mediabunny ecosystem** — Remotion, Diffusion Studio, Tella, Screen Studio, Gling, Mux
  sponsor/use it (README) — i.e., a production-grade WebCodecs media stack exists and is
  financially supported by shipping products.
- **YouTube / Discord / WhatsApp Web** class usage: widely reported, primary sources not yet
  captured in this pass — UNVERIFIED, defer to 3-a catalog.
- **Discord screen-share / Google Meet** use WebCodecs-adjacent paths (WebRTC encoders) —
  not evidence for the WebCodecs API; excluded until verified.
- Conclusion: C-005 (WebCodecs production-viable on Chromium, and now desktop-wide) is
  supported by Clipchamp + the Mediabunny ecosystem; confidence rises LOW -> MEDIUM-HIGH for
  desktop Chromium/WebKit; LOW for Firefox Android (absent).

## 10. Known limitations / risks

- No demux/mux (handled above); no subtitle/graphic tracks; no rate control niceties (CBR
  is approximate); VFR timelines need timestamp discipline; 10-bit/HDR decode support is
  codec/OS dependent and UNVERIFIED; alpha-channel video (`alpha: "discard"|"keep"`) support
  varies; Safari <26 lacks audio encode/decode in WebCodecs (use WebAudio decodeAudioData
  as bridge — verify latency, E-001); Firefox Android none.
- Encoder hardware detection is per-machine; `isConfigSupported` says "yes" even when
  `prefer-hardware` will software-fall back — runtime throughput probing required (E-001).

## Sources (all fetched 2026-09-21)
- S-2d2 W3C WebCodecs TR (IDL: VideoFrame/AudioData Transferable; config dicts; queue model).
- W3C WebCodecs codec registry TR (codec string tables).
- S-2d0 MDN BCD Video*/Audio*.json.
- S-2d8 caniuse webcodecs.json.
- S-2d3 Mediabunny README (GitHub raw).
- S-2d4 W3C: "Improving Clipchamp's in-browser video editing pipeline with WebCodecs" (2021).
- developer.chrome.com "Video processing with WebCodecs" (Jan 22, 2025).
- freeCodeCamp "The WebCodecs Handbook" (Apr 2026) — secondary tutorial.

## 11. Appendix: OVE capability-probe checklist (draft contract)

```
probeVideo():
  for each candidate config:
    { codec: 'avc1.640028'|'avc1.42E01E'|'vp09.00.10.08'|'av01.0.04M.08'|'hev1.1.6.L93.B0',
      width, height, framerate, hardwareAcceleration: 'prefer-hardware' }
    VideoDecoder.isConfigSupported(cfg)          -> decode ladder
    VideoEncoder.isConfigSupported(encCfg)       -> export ladder
  record: { decodeOK[], encodeOK[], measure: decode 1s of frames -> fps }
probeAudio():
  AudioEncoder.isConfigSupported({codec:'opus'|'mp4a.40.2'|'mp3'|'flac', ...})
  AudioDecoder.isConfigSupported({...})
ingest(file):
  Mediabunny Input -> tracks/codecs  -> if unknown codec: wasm probe fallback (doc 12)
policy: never trust strings alone; re-probe on browser upgrade; cache per UA + date
```

Rules derived from the spec + matrix above:

1. `isConfigSupported` gates config acceptance, NOT hardware reality — measure throughput
   before promising an export preset (E-001).
2. Keep a software encode/decode ladder (H.264 -> VP8/VP9 -> AV1) so a "no" on one rung
   downgrades quality instead of failing the feature.
3. Frame accounting: every `VideoFrame` is closed exactly once; pool accounting in the
   decode worker; leaked frames eventually stall the pipeline (engine-enforced).
4. Timestamps are microseconds — the shared core converts to its rational timebase at the
   boundary (ties to C-001 / doc 20).

## Depth tracking (v0.1)
- TODO: per-browser codec-string support tables (probe matrix on real devices, E-001);
  Safari 26 audio codec coverage; HEVC encode availability; latencyMode/latency budgets;
  frame-pool exhaustion behavior; verify YouTube/Discord usage with primary sources;
  WebRTC vs WebCodecs boundary note for live paths.
