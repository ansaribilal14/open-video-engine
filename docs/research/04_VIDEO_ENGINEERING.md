# 04 — VIDEO ENGINEERING FUNDAMENTALS + HARDWARE ACCELERATION MATRIX

> Status: PARTIAL (v0.1).

Owner: agent 3-c (media pipeline engineer). Ledger IDs: S-2c0..S-2c9.
Scope: (1) media pipeline fundamentals written for engineers who will build on this
engine; (2) hardware acceleration capability matrix and fallback strategy.
Written from standard engineering knowledge of the codecs/containers plus verified
primary sources (FFmpeg headers and docs, GStreamer design docs, ITU-R, vendor SDKs).

---

## 1. Media pipeline fundamentals

### 1.1 Container vs codec (URL: https://en.wikipedia.org/wiki/Digital_container_format)

A **container** (MP4/ISO BMFF, Matroska, QuickTime MOV, MPEG-TS, Ogg, WebM) is a
structured envelope that stores multiplexed **elementary streams** plus timing,
index, and metadata. A **codec** (H.264/AVC, H.265/HEVC, VP9, AV1, ProRes, DNxHR,
FFV1) is the bitstream syntax and decoding algorithm for one elementary stream.

Consequences for an engine:
- Demuxing and decoding are orthogonal. A remux (MP4 -> MKV) requires zero codec work.
- A container can store codecs the demuxer does not "know" well; capability tables
  must be per (demuxer, codec) pair, not per container.
- Codec parameters (**extradata**: SPS/PPS for H.264, VPS/SPS/PPS for HEVC, sequence
  headers) frequently live in container headers, NOT in the stream. Losing extradata
  makes a bitstream undecodable. It must be copied on every stream copy/remux.
- Per-container muxing constraints are real: e.g. AVI+HEVC is a poor match, MP4
  historically dislikes VP9/AV1 without special boxes. Validate (container, codec)
  at export time.

### 1.2 Demuxing (URL: https://ffmpeg.org/doxygen/trunk/group__lavf.html)

Demuxing = parsing the container to emit packets per stream. Key mechanics:
- The demuxer exposes **streams** (indexed), each with a time_base (rational),
  codec parameters, disposition (default/forced), rotation/metadata side data.
- **Interleaving**: packets arrive globally ordered by decode time across streams.
  A good muxer interleaves within a buffer window; a naive engine that buffers
  "all video then all audio" will violate muxer interleaving rules and stall.
- Duration is often only an estimate (from index or bitrate); authoritative duration
  can be missing for streams/fragments (live recordings, MPEG-TS).

### 1.3 Packet vs frame (URL: https://ffmpeg.org/doxygen/trunk/structAVPacket.html)

- A **packet** is a unit of *compressed* data (for video: typically exactly one
  access unit / coded picture, possibly split across the byte stream; for audio:
  usually one decode unit). Packets carry pts, dts, duration, flags (KEY, CORRUPT,
  DISCARD), and side data.
- A **frame** is a unit of *decompressed* media (one video picture or one audio
  buffer of N samples). The decoder consumes packets and produces frames; the
  count is not 1:1 for video with B-frames (delay) or packet-merging codecs.
- An editing engine should treat packets as an opaque transport layer and frames
  as the working currency. Never assume packet count == frame count.

### 1.4 PTS/DTS (URL: https://ffmpeg.org/doxygen/trunk/structAVPacket.html#ab0b0e358e14ba3b7116ba32cfa9b7bb0)

- **PTS** (presentation timestamp): when the sample is displayed.
- **DTS** (decode timestamp): when the decoder must consume the packet.
- With B-frames, DTS <= PTS and packet order (DTS order) differs from display
  order (PTS order). Any pipeline that sorts by DTS will display B-frames
  reordered; any pipeline that reorders by PTS before decoding will feed the
  decoder garbage. Decode in DTS order, present in PTS order.
- DTS must be **monotonically non-decreasing** within a stream for most muxers;
  timestamps are rational (time_base), convert to microseconds/int64 only once,
  at the boundary, and keep rational math internally where possible.
- All-zero-start assumptions are wrong: streams may start at arbitrary PTS
  (offsets, edit lists in MP4 that shift PTS). Normalize offsets once at ingest.

### 1.5 GOP / keyframes (URL: https://en.wikipedia.org/wiki/Group_of_pictures)

- A GOP (group of pictures) starts with a **keyframe** (I-frame); P-frames and
  B-frames reference it (P: previous; B: previous AND future references).
- H.264/HEVC distinguish **IDR** (closes the GOP: no reference may cross it) from
  non-IDR I-frames (cross-reference possible). Only IDR positions are safe cut
  points for bit-exact stream copy.
- **Open vs closed GOP**: open GOPs allow B-frames before the I-frame to reference
  the previous GOP; closed GOPs do not. Closed GOPs are safer for split/splice.
- Keyframe interval (GOP length) trades compression efficiency (longer = better
  ratio) against seek granularity and error resilience. Web/HLS defaults are
  commonly ~1-2 s; broadcast uses ~0.5-1 s.
- Scene-cut keyframes are inserted adaptively by encoders; a recorded file's
  keyframe map is irregular. Always read the actual keyframe index
  (MP4 stss box; Matroska cues; FFmpeg AV_PKT_FLAG_KEY) rather than assuming
  a fixed interval. (Evidence: AV_PKT_FLAG_KEY in AVPacket flags, FFmpeg avcodec.h.)

### 1.6 Seeking mechanics (URLs: https://ffmpeg.org/ffmpeg-formats.html ;
https://gstreamer.freedesktop.org/documentation/additional/design/seeking.html)

Two fundamentally different seek targets:
- **Byte seek**: move the file cursor. Only meaningful in demuxer space; needs
  the demuxer to resync (fine for MPEG-TS, unreliable for many formats).
- **Time seek**: resolve timestamp -> byte position. Needs an index
  (MP4: stco/stsc/stss; MKV: cues) or a scan (MPEG-TS "blind" seek over the
  byte stream, reading timestamps as you go).

Seek modes:
- **Keyframe (fast) seek**: jump to the nearest keyframe at-or-before the target.
  Cheap but lands early. `AVSEEK_FLAG_BACKWARD` in FFmpeg; `KEY_UNIT` flag in
  GStreamer seeks.
- **Accurate seek**: seek to the keyframe at-or-before target, then decode forward
  and DISCARD frames until the target (FFmpeg's classic `-ss` accurate behavior
  with `-accurate_seek`, the default for transcoding). This is the only way to
  get frame-accurate playback/trim into a codec without encoding the whole GOP.
  Alternative used by NLEs: decode-from-keyframe and trim at the frame queue
  (same mechanism, implemented in-process).
- Seeking invalidates decoder state: FFmpeg requires avcodec_flush_buffers() on
  the codec context and dropping all buffered frames after avformat_seek_file();
  GStreamer flushes the pipeline with FLUSH_START/FLUSH_STOP events. Skipping
  this produces reference-corrupted frames ("smearing").
- avformat_seek_file() takes (min_ts, ts, max_ts) bounds plus flags — the demuxer
  may land anywhere in the window; treat the returned position as approximate and
  verify first decoded PTS. (Verified: libavformat/avformat.h, n9.0.2, which also
  still notes "part of the new seek API which is still under construction".)
- For an editor, the robust pattern is: keep a keyframe index per clip (from
  demuxer index if present; else scan once on ingest), seek by index, decode
  GOP, trim to target frame. GStreamer design doc "seeking.html" documents the
  same segment/flush model.

### 1.7 VFR vs CFR (URL: https://en.wikipedia.org/wiki/Variable_frame_rate)

- CFR = constant frame rate: uniform PTS grid. VFR = timestamps vary (phone
  recordings, screen capture, animated GIF sources, game capture).
- Why VFR breaks naive pipelines:
  1. Frame-count-based timing (frame N is at N/fps) drifts seconds off on long
     VFR clips; audio desyncs.
  2. "fps" is ambiguous: FFmpeg exposes r_frame_rate (lowest-grid real base) and
     avg_frame_rate (frames/duration) — they differ on VFR. Picking one silently
     distorts duration math. (Verified: FFmpeg AVStream fields r_frame_rate /
     avg_frame_rate, libavformat/avformat.h.)
  3. Decoders and some filters emit frames with variable durations; anything
     that assumes fixed duration per frame must be VFR-safe or the pipeline must
     normalize.
- Engine choices: (a) support VFR natively in the timeline model (every clip has
  a PTS->timeline mapping), or (b) normalize to CFR at ingest (duplicate/drop).
  Browsers and many simple pipelines do (b) and lose real timing; a proper
  engine should do (a) for playback and offer (b) for export targets that
  require CFR (some broadcast containers).
- Monotonic DTS is still mandatory even in VFR; "fix timestamp" passes must
  preserve monotonicity, not regrid blindly.

### 1.8 Stream copy vs transcode vs remux (URL: https://trac.ffmpeg.org/wiki/Concatenate)

- **Remux** (container change, no codec work): lossless, fast; constrained by
  container codec support and subtitle/attachment handling.
- **Stream copy** (`-c copy`): splice/cut only at keyframes; requires compatible
  codec parameters; cannot change resolution/rate/filters. Timestamp rewrite is
  allowed and expected.
- **Transcode** (decode -> filter -> encode): full flexibility, quality loss
  (generation loss), cost dominated by decode+encode. Every re-encode is a
  quality decision the user should control (CRF/CQ/QP; bitrate modes).
- **Smart rendering / segment re-encode**: copy all GOPs except at cut points;
  re-encode only boundary segments with matched parameters (resolution, rate,
  profile, GOP alignment). This is what consumer editors use for fast exports
  and is worth building once the basic pipeline works.
- Engine rule: any operation that does not change codec-level data must be
  expressible as remux/copy. Decode is the enemy of speed and fidelity.

### 1.9 Scaling and pixel-format conversion costs
(URLs: https://ffmpeg.org/doxygen/trunk/group__lavfi.html,
https://ffmpeg.org/ffmpeg-filters.html#scale)

- Decode output format is determined by the codec (8-bit H.264 => yuv420p; 10-bit
  HEVC => yuv420p10le; 4:2:2 prores => yuv422p10le). The engine must convert to
  the working format exactly ONCE, at the decode/preview boundary, not per effect.
- Scaling cost model: bytes read+written per pixel, cache locality, SIMD.
  A 4K->1080p downscale of 60 fps yuv420p is ~0.5 GB/s of memory traffic for the
  chroma+luma planes alone; do it once, not per filter stage.
- Scale quality knobs: filter choice (bilinear/bicubic/lanczos/spline), chroma
  location handling, range preservation. Wrong chroma upsampling during
  4:2:0->RGB conversion causes chroma bleed (classic cause of "red/green
  fringing" at edges after resize).
- Integer rounding: repeated round-trips between formats accumulate rounding
  error and range shifts; keep a single working precision (>= 16-bit integer or
  half/float) through the effect chain.

---

## 2. Hardware acceleration matrix

FFmpeg exposes hwaccel through a device/frame abstraction
(AVHWDeviceType enum, verified from libavutil/hwcontext.h at tag n9.0.2):
NONE, VDPAU, CUDA, VAAPI, DXVA2, QSV, VIDEOTOOLBOX, D3D11VA, DRM, OPENCL,
MEDIACODEC, VULKAN, D3D12VA, AMF, OHCODEC.
Official usage example: doc/examples/hw_decode.c (av_hwdevice_ctx_create +
hw_device_ctx + get_format callback).

| Vendor / platform | Decode API | Encode API | FFmpeg exposure | Notes on quality/behavior |
|---|---|---|---|---|
| NVIDIA | NVDEC (Video Codec SDK) | NVENC | AV_HWDEVICE_TYPE_CUDA; hwaccels h264_cuvid/cuviddec & nvdec-style decoders; encoders *_nvenc; hw frames cuda/drm_prime | NVDEC/NVENC documented in NVIDIA Video Codec SDK 13.1 (2026-07-31 announcement, developer.nvidia.com). Encode quality differs from x264/x265 SW; requires matching driver on target machine; session-count limits on consumer GPUs. |
| Intel | QSV / VAAPI (Linux) | QSV / VAAPI | AV_HWDEVICE_TYPE_QSV, AV_HWDEVICE_TYPE_VAAPI; decoders h264_qsv...; encoders h264_qsv, hevc_qsv...; oneVPL is the current SDK (MIT, github.com/oneapi-src/oneVPL) | QSV on Windows uses D3D11/VA); on Linux layered over VAAPI driver support; older-Gen GPU support varies. |
| AMD | VAAPI (Linux), AMF (Windows/Linux) | AMF, VAAPI | AV_HWDEVICE_TYPE_VAAPI / AV_HWDEVICE_TYPE_AMF; amf encoders h264_amf, hevc_amf, av1_amf | AMF SDK is MIT-licensed (verified LICENSE.txt, GPUOpen-LibrariesAndSDKs/AMF). VAAPI driver quality varies by Mesa generation; UVD/VCN decode capability lists are generation-specific. |
| Apple | VideoToolbox | VideoToolbox | AV_HWDEVICE_TYPE_VIDEOTOOLBOX; decoders h264_videotoolbox etc.; encoders h264_videotoolbox, hevc_videotoolbox, prores_videotoolbox | Decode is transparent-ish via VideoToolbox; encode gives limited rate-control control compared to x264; 10-bit HEVC encode available on Apple Silicon. |
| Android | MediaCodec | MediaCodec | AV_HWDEVICE_TYPE_MEDIACODEC; decoders/encoders h264_mediacodec... | Capability sets are per-device; misbehaving vendor decoders are common; codec2 is the modern path. See also 16_MEDIACODEC.md. |
| Windows (generic) | DXVA2 / D3D11VA / D3D12VA | Media Foundation | AV_HWDEVICE_TYPE_DXVA2 / D3D11VA / D3D12VA; MF transcode via external API, not avcodec | D3D11VA is the modern default for decode on Windows; hardware MF encoders accessed outside FFmpeg's codec API or via d3d11va-filter pipelines. |
| Linux (generic) | VAAPI / VDPAU | VAAPI | AV_HWDEVICE_TYPE_VAAPI / VDPAU; decode via the vaapi hwaccel path (hwdownload needed unless renderer imports drm_prime); encoders h264_vaapi, hevc_vaapi, av1_vaapi | VDPAU is legacy. VAAPI frame pools + drm_prime export interoperate with wgpu/Vulkan import paths (see 08/09 docs). |
| Vulkan | Vulkan Video | Vulkan Video encode | AV_HWDEVICE_TYPE_VULKAN | Still the least uniform path across drivers; hwcontext_vulkan.h received active API changes through 2026-06 (verified in doc/APIchanges). |
| OpenHarmony | OHCodec | OHCodec | AV_HWDEVICE_TYPE_OHCODEC (new) | Present in n9.0.2 enum; support breadth UNVERIFIED. |

Failure/fallback design:
- HW device creation can fail (no driver, wrong permissions, VM without GPU
  passthrough, headless server). Every hwaccel path needs an SW fallback chain:
  try hw decode -> on failure, reopen demuxer/decoder as SW -> log and continue.
- HW decoders do not support every profile/level (e.g. some 4:2:2 10-bit, old
  DivX-style profiles, lossless modes). Capability detection must be per-stream,
  not per-machine: probe with the actual stream parameters.
- HW decoders can emit corrupted output on driver bugs; always keep
  checksum/spot-check tooling in QA (compare SW decode vs HW decode on a sample).
- Zero-copy: decode -> render without download is only possible when the
  renderer can import the hw frame type (CUDA, VA-API/drm_prime, D3D11,
  CVPixelBuffer/IOSurface). Otherwise every frame pays a GPU->CPU->GPU round
  trip, which is usually the single largest cost in a preview pipeline.
  Cross-vendor import in one process is still not universally possible
  (e.g. CUDA <-> VAAPI); plan per-platform zero-copy paths. (Support matrix
  for our wgpu renderer: see 08_GPU_ARCHITECTURE.md / 09_WGPU.md.)

---

## 3. Implications for our engine architecture

1. Timeline model must be VFR-safe and PTS-driven; never frame-count arithmetic.
2. Ingest must capture: keyframe index, extradata, color tags, rotation,
   time_base, r_frame_rate vs avg_frame_rate.
3. Decode: one SW path (reference correctness), one HW fast path per platform
   with SW fallback; zero-copy where the renderer can import.
4. Seek service: keyframe-index seek + accurate trim; flush codec state on every
   seek; expose both fast (keyframe) and accurate modes.
5. Copy/remux must be first-class operations; export should choose copy when
   the operation is lossless-able.
6. Pixel format conversion exactly once at boundaries; single working format.

---

## 4. Sources

- FFmpeg releases/download page (versions): https://ffmpeg.org/download.html (S-2c0)
- FFmpeg legal/licensing: https://ffmpeg.org/legal.html (S-2c1)
- FFmpeg source n9.0.2: hwcontext.h, avcodec.h, avformat.h, doc/examples/hw_decode.c,
  doc/APIchanges — https://github.com/FFmpeg/FFmpeg (S-2c2)
- endoflife.date FFmpeg/GStreamer support data: https://endoflife.date/api/ffmpeg.json (S-2c6)
- GStreamer design docs (seeking, negotiation, buffer, caps):
  https://gstreamer.freedesktop.org/documentation/additional/design/ (S-2c4)
- NVIDIA Video Codec SDK 13.1 announcement: https://developer.nvidia.com (S-2c8)
- oneVPL (MIT): https://github.com/oneapi-src/oneVPL ; AMF (MIT):
  https://github.com/GPUOpen-LibrariesAndSDKs/AMF (S-2c8)
- ITU-R recommendations (color sections of 22_COLOR.md): BT.601/709/2020/2100 (S-2c9)

## 5. Depth tracking (v0.1 -> v0.2)

- Read actual MP4 (stss/stco/stts) and MKV (cues) index formats and document
  per-container seek cost tables.
- Measure: SW vs NVDEC/VAAPI decode throughput on target hardware (E-00x).
- Verify per-GPU NVDEC session limits and 4:2:2/10-bit decode coverage matrix.
- Confirm OHCODEC support breadth; confirm MF-encode story on Windows.
