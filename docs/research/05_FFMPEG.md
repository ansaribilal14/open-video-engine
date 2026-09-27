# 05 — FFMPEG AS EMBEDDED ENGINE CORE

> Status: PARTIAL (v0.1).

Owner: agent 3-c. Ledger IDs: S-2c0, S-2c1, S-2c2, S-2c6.
Scope: library surface, filter graph model, hwaccel architecture, licensing,
thread model, and embedding pitfalls. All version/license claims verified against
ffmpeg.org and the FFmpeg source tree (tag n9.0.2) on the research date.

---

## 1. Verified version state (research date: 2026-09)

- Latest stable: **FFmpeg 9.0.2 "Lei"**, released 2026-09-18; 9.0 branch cut from
  master on 2026-06-26. Verified: https://ffmpeg.org/download.html and
  https://ffmpeg.org/releases/ (S-2c0).
- Active supported branches (verified via https://ffmpeg.org/releases/ +
  https://endoflife.date/api/ffmpeg.json, S-2c6): 9.0 (current), 8.1 "Hoare"
  (2026-03-16), 8.0 "Huffman" (2025-08-22), 7.1 "Péter" (2024-09-30, LTS).
- Release cadence: a new major roughly every ~9-12 months; point releases
  between majors. Plan for yearly major bumps, not five-year ones.

## 2. Library surface (URLs: doxygen headers at https://ffmpeg.org/doxygen/trunk/)

- **libavformat**: container I/O. AVFormatContext (open_input/input callbacks/
  find_stream_info), AVStream (time_base, codecpar, r_frame_rate, avg_frame_rate,
  side data incl. rotation/color), av_read_frame / av_interleaved_write_frame,
  custom AVIO (memory/network), avformat_seek_file + AVSEEK flags, avformat_flush.
  Protocol layer (file/rtmp/hls/srt...) is here too.
- **libavcodec**: codec API. AVCodecContext (avcodec_alloc_context3,
  avcodec_open2, send_packet/receive_frame, send_frame/receive_packet for
  encoders), AVPacket/AVFrame (refcounted since 4.0), extradata via
  AVCodecParameters.extradata, side data on packets and frames.
  Decoders: software (h264, hevc, vp9, av1, prores...) + hwaccel-enabled
  wrappers (see section 4).
- **libavfilter**: "Graph-based frame editing library" (verified header comment,
  libavfilter/avfilter.h n9.0.2). Scale/crop/pad/overlay/blend/tone-map/color
  filters; also audio. This is the natural place to run per-clip transform
  chains server-side or in preview workers.
- **libswscale**: scaling + pixel-format conversion (yuv<->rgb, bit depth,
  range). Fast, SIMD; approximation caveats for colorimetry (see 22_COLOR.md).
- **libswresample**: audio resampling/rematrix/sample-fmt conversion.
- Support libraries: libavutil (frames, hwcontext, rational, dict, log, opt),
  libavdevice (capture), postproc (GPL).

## 3. Filter graph model (URL: https://ffmpeg.org/ffmpeg-filters.html)

- An AVFilterGraph contains linked filters; frames enter via buffersrc, exit via
  buffersink. The classic CLI string ("scale=1280:720,fps=30") maps 1:1 to the
  API (avfilter_graph_parse_ptr / create_filter + links).
- Graphs are configuration-bound: resolution/format/rate changes require graph
  reconfiguration (libavfilter re-inits a filter when inputs change, but the
  application must handle buffersink format negotiation and re-create graphs
  for structural changes).
- Frame ownership: buffersrc consumes your reference (or you push a copy);
  buffersink gives you a frame you must unref. Everything is refcounted;
  av_frame_ref/av_frame_unref discipline is mandatory.
- For an editor: use separate graphs per clip or per transform chain; don't try
  to build one mega-graph for the whole timeline (seek/flush semantics and
  latency get unwieldy). Cross-check with 03_NLE_ARCHITECTURE.md.

## 4. Hardware acceleration architecture (URLs:
https://ffmpeg.org/doxygen/trunk/libavutil_2hwcontext_8h.html ;
doc/examples/hw_decode.c)

- Two abstractions: **AVHWDeviceContext** (device-level state, refcounted
  AVBuffer) and **AVHWFramesContext** (a pool of surfaces with format/size).
  Verified struct docs in libavutil/hwcontext.h (n9.0.2).
- Device types present in n9.0.2 (verified enum AVHWDeviceType): NONE, VDPAU,
  CUDA, VAAPI, DXVA2, QSV, VIDEOTOOLBOX, D3D11VA, DRM, OPENCL, MEDIACODEC,
  VULKAN, D3D12VA, AMF, OHCODEC.
- Integration pattern (verified from doc/examples/hw_decode.c):
  1. av_hwdevice_ctx_create(&ref, type, ...) ;
  2. attach av_buffer_ref(hw_device_ctx) to AVCodecContext->hw_device_ctx;
  3. implement AVCodecContext.get_format to select the hw pix fmt
     (e.g. AV_PIX_FMT_CUDA) among the offered list;
  4. decoded AVFrames come back with hw format; data[0] is an
     AVHWFramesContext surface; map via av_hwframe_map / av_hwframe_transfer_data
     to download (copy) or map (zero-copy) to another API.
- Two hw-accel styles exist: (a) integrated hwaccel (AVCodecContext with
  hw_device_ctx; e.g. h264 with VAAPI/CUDA/D3D11VA hwaccel) and (b) dedicated
  wrapper decoders/encoders (h264_qsv, h264_nvenc, h264_videotoolbox,
  h264_amf, h264_vaapi encoders, h264_mediacodec). API-wise (b) behaves like a
  normal codec; (a) keeps one code path for SW/hw selection via get_format.
- drm_prime/CUDA/VAAPI/D3D11/CVPixelBuffer surfaces can be exported for
  renderer import (zero-copy preview) — see the matrix in 04_VIDEO_ENGINEERING.md.
- New work in 2026 (verified doc/APIchanges n9.0.2): Vulkan hwcontext changes
  (VkAccessFlagBits2, queue_flags), AV_HWDEVICE_TYPE_OHCODEC added; so the
  hwaccel surface is still moving — pin versions.

## 5. Licensing split and redistribution (verified: https://ffmpeg.org/legal.html)

- Base license: **LGPL-2.1-or-later** (COPYING.LGPLv2.1, verified in source).
- Optional parts under **GPL-2.0-or-later** (e.g. libx264/libx265 integration,
  postproc). Building with `--enable-gpl` makes the whole FFmpeg GPL; the
  official page states explicitly that if those parts are used, GPL applies to
  all of FFmpeg. `--enable-nonfree` (e.g. fdk-aac) makes the build
  non-redistributable.
- "FFmpeg is not available under any other licensing terms, especially not
  proprietary/commercial ones, not even in exchange for payment." (legal.html)
- LGPL compliance checklist items from legal.html (summarized; full list S-2c1):
  build without --enable-gpl and --enable-nonfree; dynamic linking; distribute
  FFmpeg source matching the binaries; state the configure line; add FFmpeg
  attribution to about box/EULA; no reverse-engineering bans in EULA; don't
  rename/mis spell FFmpeg DLLs; ensure your app does not itself use GPL
  libraries in the same work.
- Engine implication: default engine builds must be LGPL-only (no x264/x265
  encoders). Provide an optional, separately-distributed GPL build for H.264/HEVC
  software encoding, or push hw encoders (nvenc/videotoolbox/qsv/amf/vaapi)
  which live outside the GPL parts. fdk-aac must be avoided in distributed
  builds (nonfree); use the native aac encoder or platform encoders.

## 6. Thread model (verified: libavcodec/avcodec.h n9.0.2)

- AVCodecContext.thread_count + thread_type; two decode modes (verified doc):
  FF_THREAD_FRAME ("Decode more than one frame at once" — adds one frame of
  delay per thread, so clients that cannot provide future frames should not use
  it) and FF_THREAD_SLICE ("Decode more than one part of a single frame at
  once").
- Consequence for an interactive scrubber: frame threading increases latency
  (frames come out of order relative to submission by one frame per thread);
  for low-latency preview prefer slice threading or thread_count=1, and for
  throughput (export) prefer frame threading.
- Encoders: most video encoders are frame-threaded internally; the
  execute/execute2 callbacks can be overridden with a custom thread pool.
- One AVCodecContext is not thread-safe for concurrent operations; use one
  context per worker (pool of decoders), which also matches per-clip decode
  isolation. Sending packets to one context from multiple threads is unsafe.

## 7. Known embedding pitfalls

1. **API/ABI churn**: majors bump roughly yearly now (avcodec major 62 in 9.0;
   verified via doc/APIchanges header "last version increases ... 2026-06-23"
   and continuous entries). Deprecations can vanish at the next major. Pin a
   version, wrap libav* behind your own C/Rust ABI boundary module, and test
   against two majors (e.g. 7.1 LTS and 9.0) in CI.
2. **Refcounting discipline**: AVPacket/AVFrame are refcounted; misuse leaks or
   dangles. Rule: every receive returns owned data; unref before reusing
   temporaries; av_frame_move_ref when transferring queues.
3. **Flush/seek state**: after avformat_seek_file you must avcodec_flush_buffers
   and drop all in-flight frames; otherwise B-frame references produce garbage.
   avformat_flush exists for stream resync (verified doc: does not reset
   detected parameters; full reset requires reopening). For file "restart",
   reopen the AVFormatContext.
4. **find_stream_info latency**: probing streams can read large chunks of data;
   for formats with headers it is fast, for TS it may decode several seconds'
   worth. Budget for it at ingest, cache results per file.
5. **Timestamp edge cases**: AV_NOPTS_VALUE, negative PTS after accurate seek
   (first frames before the requested timestamp come out with negative dts),
   start_time offsets per stream. Normalize at the boundary; see
   04_VIDEO_ENGINEERING.md section 1.4.
6. **Packet vs frame rate fields**: r_frame_rate vs avg_frame_rate confusion
   creates VFR bugs (see 04 section 1.7).
7. **Encoder option drift**: option names/defaults shift between majors
   (e.g. libx264 options vs native aac); store export presets as structured
   data, map to per-version option tables, verify with FATE-like smoke tests.
8. **Static linking + LGPL**: statically linking LGPL libav* into a
   closed-source app requires object-file availability offers; dynamic linking
   is the official checklist's path. On mobile/iOS, static is customary — get
   a license review before distribution (see 35_LICENSES.md when written).
9. **Windows/macOS distribution**: use maintained builds (BtbN/FFmpeg-Builds,
   gyan.dev) or build from source with a fixed toolchain; each prebuilt variant
   carries its own license set (check --enable-gpl in the variant).

## 8. Sources

- https://ffmpeg.org/download.html (S-2c0) — version 9.0.2 "Lei"
- https://ffmpeg.org/releases/ (S-2c0) — branch history 7.0..9.0
- https://ffmpeg.org/legal.html (S-2c1) — license + compliance checklist
- https://github.com/FFmpeg/FFmpeg tag n9.0.2 (S-2c2): libavutil/hwcontext.h,
  libavcodec/avcodec.h, libavformat/avformat.h, libavfilter/avfilter.h,
  libswscale/swscale.h, doc/APIchanges, doc/examples/hw_decode.c
- https://endoflife.date/api/ffmpeg.json (S-2c6) — LTS/support windows

## 9. Depth tracking (v0.1 -> v0.2)

- Build a minimal embedded demo (open/decode/seek/graph) against 7.1 and 9.0.
- Enumerate current encoder/decoder option tables for our export presets.
- Confirm LGPL binary distribution mechanics per platform (iOS static case).
