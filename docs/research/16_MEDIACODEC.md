# 16 — MEDIACODEC REALITY: THE RAW CODEC LAYER UNDER ANY ANDROID VIDEO ENGINE

> Status: PARTIAL (v0.1).

Scope: hardware codec API reality-check for a custom Android pipeline (the alternative
to doc 15's Transformer). Evidence: AOSP MediaCodec.java (S-2e2, fetched this session),
AOSP Codec2 soft components (S-2e3, fetched), official supported-formats page
(S-2e1, fetched). Pitfall claims marked UNVERIFIED where not re-verified against
current docs/source.

## 1. API modes (verified from AOSP source)

- **Async mode (recommended)**: `setCallback(Callback, Handler)` -> onInputBufferAvailable
  / onOutputBufferAvailable / onFormatChanged / onError(CodecException). Callbacks arrive
  on the Handler's Looper — engine must own that thread.
- **Sync mode**: `dequeueInputBuffer` / `dequeueOutputBuffer` polling loops; simpler but
  blocks and misbehaves at 4K rates; AOSP docs recommend async for video.
- **Buffer model**: `MediaCodec.BufferInfo` (offset/size/presentationTimeUs/flags).
  ByteBuffers; or `getInputImage(int)`/`getOutputImage(int)` returning `Image`
  (YUV_420_888 plane access) even in buffer mode (verified javadoc lines).
- **Surface modes**:
  - surface-input encode: `configure(..., inputSurface, CONFIGURE_FLAG_ENCODE)` where
    inputSurface = `createInputSurface()` or a reusable `createPersistentInputSurface()`;
    push frames via EGL; EOS via `signalEndOfInputStream()` (surface-input ONLY;
    buffer-input EOS is `BufferInfo.FLAG_END_OF_STREAM` on queueInputBuffer).
  - surface-output decode: zero-copy to a `Surface`/`SurfaceTexture` consumer;
    `setOutputSurface(Surface)` (API 23+) allows live rerouting without re-instantiating.
  - New flags seen on main: `CONFIGURE_FLAG_USE_BLOCK_MODEL` (large-frame/block model
    for 8K), `CONFIGURE_FLAG_DETACHED_SURFACE`, `CONFIGURE_FLAG_USE_CRYPTO_ASYNC`
    (verified constants; block model = the "large buffer" API, adopt only post-1.0).
- **Monitor**: `setOnFrameRenderedListener` for actual display timing.

## 2. Device capability reality

- Enumerate via `MediaCodecList(ALL_CODECS)` + `MediaCodecInfo`:
  `isHardwareAccelerated` / `isSoftwareOnly` / `isVendor` (API 29+ fields exist on
  MediaCodecInfo; earlier APIs mislead). `Capabilities` -> `VideoCapabilities`
  (supported widths/heights, per-format performance points) / `AudioCapabilities`.
  Encoder "supports size but not at 30fps" traps are common — query performance points,
  not just resolutions. (API shape verified in source; perf-point specifics UNVERIFIED.)
- Software codecs (Codec2): verified component names from AOSP source:
  `c2.android.avc.encoder`, `c2.android.hevc.decoder` (C2SoftAvcEnc.cpp /
  C2SoftHevcDec.cpp define these literally). Family covers avc/hevc/vp8/vp9/av1
  enc+dec, aac/amr/flac/opus/vorbis audio (family claim UNVERIFIED per-file).
  Expect c2.android.* everywhere modern; legacy OMX.* ghosts linger pre-API 27.
- Format mandates (official supported-formats page, S-2e1, verified quotes):
  H.264 AVC BP decoder since 3.0; AVC MP since 6.0 (encoder recommended, not required);
  HEVC dec 5.0+; VP8 4.3+; VP9 4.4+; AV1: decoder mandatory 10+, encoder+decoder
  mandatory 14+; APV (pro video codec) mandatory enc+dec from Android 16.
  CDD encoder recommendation table (H.264): 720p@30fps ~2Mbps "N/A on all devices"
  class — i.e., encoders are best-effort, never assume 1080p60 hardware everywhere.

## 3. Zero-copy path that actually works (decode -> GL -> encode)

1. Decoder configured to surface-output a `SurfaceTexture` (or ImageReader) created
   on the compositor EGL context -> frames land as GL_TEXTURE_EXTERNAL_OES textures
   (doc 14 A1) with no CPU copy.
2. Compositor renders EXTERNAL_OES + layers into an FBO; on the encoder input surface
   use `eglPresentationTimeANDROID` to carry timestamps (UNVERIFIED exact egl ext name
   casing; verify EGL_ANDROID_presentation_time).
3. Encoder takes that input surface; hardware writes to another gralloc queue; muxer
   consumes via MediaCodec output buffers (or surface->MediaMuxer writeSampleData).
- AHardwareBuffer route (API 26+): decode into AHardwareBuffer-backed ImageReader,
  import as EGLImage/VK image, composite, hand to encoder — needed when frames must
  cross processes/APIs; costs more glue; Media3's `HardwareBufferJniWrapper` shows
  Google itself uses HardwareBuffer plumbing here (S-2e0).
- ImageReader for CPU frames: `ImageFormat.YUV_420_888` (decoder) / PRIVATE (GPU);
  plane stride/rowStride vary per device — never assume packed I420.

## 4. Long-standing pitfalls (each must be an experiment or an ADR note)

| Pitfall | Detail | Status |
|---|---|---|
| Codec reuse after EOS | After FLAG_END_OF_STREAM, some devices refuse `flush()`+reconfigure; safest is release() + create new instance per segment | UNVERIFIED current state; was device-dependent for years; test matrix E-00x |
| Encoder input throttling | Decoder-to-surface: when the CONSUMER (encoder) is slow, decoder DROPS frames silently (no backpressure to app) | documented behavior (MediaCodec docs "throttling"); UNVERIFIED quote this session |
| surface-input EOS | signalEndOfInputStream() only valid with createInputSurface()-style input; calling on buffer-mode codec throws | verified semantics in source javadoc region |
| Color format hell | Encoder inputs: COLOR_FormatYUV420Flexible + vendor privates; decoded output may be TILE/SemiPlanar; do NOT hand-convert — use Image API / GPU path | community-canonical; UNVERIFIED per-device |
| Timestamp requirements | Monotonic presentationTimeUs; frame drops if encoder sees non-monotonic input (esp. with surface input) | UNVERIFIED quote; standard practice |
| CodecException severity | `isRecoverable()`/`isTransient()` decide restart vs wait; 4K+ sw fallback can transient-fail | verified API existence in source |
| Keyframe-only trim | ClippingConfiguration.setStartsAtKeyFrame needed for transmux-style trims (ties to doc 15 trim fast path) | verified in Media3 source |
| B-frames / rotation | per-device B-frame support varies; rotation metadata vs rotated pixels (MediaFormat.KEY_ROTATION) differs by vendor | UNVERIFIED specifics |
| Multi-instance limits | Concurrent encoders limited per device (many OEMs allow 1-2 HW instances) | UNVERIFIED; query capabilities |

## 5. Async pipeline shape for the engine (sketch, not application code)

- One `HandlerThread` + `MediaCodec.AsyncCallback` per codec session; inputs pushed
  by a producer task, outputs drained into the muxer task; session state machine:
  INIT -> CONFIGURED -> RUNNING -> DRAINING(EOS) -> RELEASED, with error hooks on
  CodecException(isRecoverable/isTransient).
- Drain discipline: keep <= 1 untracked input buffer per codec (B info flags),
  honor `onOutputFormatChanged` before first sample, and never call codec methods
  from foreign threads (Android MediaCodec is single-threaded-per-instance).
- This shape is identical whether the host is Kotlin or Rust-behind-JNI — which is
  why the doc-14 recommendation places the JNI boundary at the session level.

## 6. Consequences for engine design

- The MediaCodec layer must be wrapped by a codec-session abstraction in the core:
  (create, configure, attach surface/image sinks, run, drain, error-recover,
  release) — so Android is one backend among several (FFmpeg, VideoToolbox, D3D11
  are others; see docs 05/08).
- Never build edit logic around MediaCodec directly; Transformer (doc 15) already
  wraps ~80% of these pitfalls and is maintained by Google device-testing. Custom
  path earns its cost only when Transformer blocks: resumable segmented export,
  custom compositor graphs, or >Transformer codec control.
- Async callbacks + dedicated HandlerThread map 1:1 to a Rust-side codec-session
  thread with a small mailbox — keep JNI boundary at the session level, not
  per-buffer (doc 14 B3).

## 7. Depth remaining (v0.2)

- E-00x matrix: 6-8 devices, decode->OES->composite->encode at 1080p30/4K30, measure
  fps, drop policy, thermal ramp (doc 14 A4).
- Verify current CDD wording on codec reuse/flush semantics (CDD 5.1.x).
- Read Codec2 block-model API (`CONFIGURE_FLAG_USE_BLOCK_MODEL`) docs for 8K intent.
- Confirm eglPresentationTimeANDROID ext and timestamp propagation through SurfaceInput.
