# 06 — GSTREAMER (PIPELINE MODEL) + LIBPLACEBO/MPV/OBS LESSONS

> Status: PARTIAL (v0.1).

Owner: agent 3-c. Ledger IDs: S-2c3, S-2c4, S-2c5, S-2c7, S-2c9.
Scope: (3) GStreamer architecture and license; (4) what libplacebo/mpv/OBS
demonstrate about GPU rendering and color management that generic FFmpeg
filtering lacks.

---

## PART A — GStreamer

## 1. Verified version + license state (research date: 2026-09)

- Current stable series: **GStreamer 1.28**. 1.28.0 released 2026-01-27; latest
  bugfix 1.28.7 released 2026-09-07. Verified:
  https://gstreamer.freedesktop.org/releases/1.28/ (S-2c3).
  Previous series 1.26 (2025-03-11) still maintained at 1.26.x; 1.24 is EOL
  (S-2c6 endoflife data).
- License: core (subprojects/gstreamer) COPYING = **LGPL-2.1**; gst-plugins-bad
  and gst-plugins-ugly module COPYING also LGPL-2.1 (verified raw files,
  S-2c5). Caveat: individual plugins that link GPL libraries (e.g. x264enc
  linking GPL libx264) effectively drag GPL into the distributed binary set;
  the module-level LGPL does not launder that. Treat "GPL-linked plugins" as a
  distribution landmine and exclude them from LGPL builds. (libx264 itself is
  GPL-2.0+; plugin docs mark such plugins GPL — plugin doc page check was
  inconclusive in this pass: UNVERIFIED tag for the exact per-plugin label.)

## 2. Object model (URLs: https://gstreamer.freedesktop.org/documentation/additional/design/)

- **Element**: processing unit (source/decoder/filter/muxer/sink). **Bin**:
  container of elements; **Pipeline**: bin with a global clock and bus.
  (design/gstbin.html, design/gstpipeline.html)
- **Pad**: input/output port with a template of supported **caps** (typed,
  structured media descriptors: media type, format, width/height, framerate,
  features like memory:GLMemory). Linking requires compatible caps.
  (design/caps.html)
- **Mini-object/buffer model**: GstBuffer wraps GstMemory blocks (sysmem,
  GLMemory, dmabuf, D3D11, IOSurface...), reference-counted
  (design/miniobject.html, design/memory.html, design/dmabuf.html). Zero-copy
  between plugins on Linux is typically dmabuf/GLMemory based.
- **Events** flow downstream (SEGMENT, CAPS, EOS, FLUSH_START/STOP) and
  upstream (SEEK, QOS, RECONFIGURE); **queries** (POSITION, DURATION, LATENCY,
  CAPS, ACCEPT_CAPS) traverse as needed (design/events.html, design/query.html).
- **States**: NULL -> READY -> PAUSED -> PLAYING; preroll happens in PAUSED;
  clocks drive sync (design/clocks.html, design/preroll.html). Latency
  compensation is a first-class concept (design/latency.html).
- **Bus**: application receives ERROR/WARNING/EOS/STATE_CHANGED/MESSAGE
  (design/gstbus.html, design/messages.html). An editor embedding GStreamer
  lives mostly on the bus + a small set of pad probes (design/probes.html).

## 3. Caps negotiation (verified: design/negotiation.html)

- Documented basic rules (verbatim): "downstream suggests formats / upstream
  decides on format".
- Four mechanisms: GST_QUERY_CAPS (list possible formats), GST_QUERY_ACCEPT_CAPS
  (check a fixed caps), GST_EVENT_CAPS (configure downstream), GST_EVENT_RECONFIGURE
  (tell upstream to renegotiate). CAPS queries recurse and accept filter caps
  whose order expresses preference.
- Practical effect: encoders/decoders/sources constrain what a pipeline can do;
  a flexible element (videoscale, videoconvert) sits where conversion is
  needed. Debugging failed pipelines is mostly reading negotiation state
  (GST_DEBUG=3,*caps*).

## 4. Application integration: appsrc/appsink (verified:
https://gstreamer.freedesktop.org/documentation/app/appsink.html)

- **appsink**: "a sink plugin that supports many different methods for making
  the application get a handle on the GStreamer data in the pipeline" — pull
  (pull_sample) or callback (new-sample) modes; caps selectable; max-buffers and
  drop settings control backpressure.
- **appsrc**: symmetric; push buffers/signals need-data/enough-data for flow
  control. Use for feeding generated frames (compositor output) into a
  GStreamer encode path, or receiving decoded frames into a wgpu/D3D renderer.
- Editor pattern: decode pipeline -> appsink (frames as GL/dmabuf memory) ->
  our compositor -> appsrc -> encode/mux pipeline. This keeps GStreamer inside
  a transport/codec role, not owning the timeline.

## 5. Hardware codecs (URL: https://gstreamer.freedesktop.org/documentation/plugins.html)

- VAAPI: elements renamed in 1.18+ from vaapih264dec to vah264dec style
  (va* prefix); NVDEC/NVENC: nvh264dec/nvh264enc (+ legacy nvdec/nvenc);
  Intel QSV: qsvh264dec/qsvh264enc; Apple: vtdec/vtenc; AMD AMF: amfh264dec/
  amfh264enc; Android: amcviddec (androidmedia); Windows: d3d11h264dec and
  d3d12 variants. Exact availability per platform/version: UNVERIFIED in this
  pass (element tables are large); verify per target platform in v0.2.
- Hardware elements negotiate caps with memory features (GLMemory, dmabuf) so
  zero-copy decode->GL compositing is achievable, which is GStreamer's main
  architectural advantage over naive FFmpeg apps.

## 6. playbin vs custom pipelines

- **playbin3** (and decodebin3): one-liner playback with automatic decoder
  selection, subtitles, gapsless; good for previews and thumbnails. Limited
  control over exact branch topology and output color handling.
- Custom pipelines: explicit decodebin3 -> converter -> (your sink) etc. Gives
  control (e.g. forcing hw decode, caps filters, buffer pools) at the cost of
  writing negotiation and error handling yourself. An editor that needs
  precise frame stepping, custom filters, or accurate seeks ends up needing
  custom pipelines almost immediately (design/seeking.html documents the
  segment/flush mechanics you must handle).

## 7. Why some editors avoid GStreamer (analysis; qualitative, community-sourced)

- Complexity/steepness: negotiation, latency, state machine, and probe APIs
  have real learning curves; failure modes are opaque without GST_DEBUG logs.
- Deployment/fragmentation: plugin sets and versions differ per distro/OS
  bundle; cerbero exists for cross-builds but is its own maintenance burden.
- Latency and frame accuracy: pipeline timing (preroll, latency compensation)
  is playback-oriented; frame-accurate NLE scrubbing demands care.
- Existing editors instead chose: MLT framework (Kdenlive, Shotcut, Flowblade —
  see 07_MLT.md and 3-b docs), or direct libav* embedding (Olive), keeping
  simpler mental models. This is an architectural choice, not a capability
  judgment: GStreamer is the most complete open multimedia framework for
  playback/encode plumbing; editors traded control for that completeness.
- Where GStreamer is a good fit for us: platform preview players, camera/
  capture ingest, transcode worker fleet (encode-only), and as a hw-decode
  bridge where its zero-copy memory feature negotiation is decisive.

---

## PART B — libplacebo / mpv / OBS: GPU rendering + color lessons

## 8. libplacebo (URL: https://code.videolan.org/videolan/libplacebo ;
GitHub mirror verified: https://github.com/haasn/libplacebo)

- GPU video-processing library by Niklas Haas; powers mpv --vo=gpu-next and
  VLC 4's renderer. LGPL-2.1-or-later (license UNVERIFIED in this pass; repo
  badge/README verified, S-2c7).
- Capabilities relevant to us: proper color management (icc, HDR metadata),
  tone mapping (multiple algorithms incl. BT.2390-style), gamut mapping,
  debanding, dithering, film grain synthesis (AV1 grain), shader caching,
  Vulkan/GL/D3D11 backends. This is essentially "the rendering half of an
  engine" that generic FFmpeg filtering (swscale + basic filters) does not
  provide with display-grade quality.

## 9. mpv (URL: https://github.com/mpv-player/mpv ; options verified in
DOCS/man/options.rst, S-2c7)

- Documents the production vocabulary our engine will need:
  `--tone-mapping=<value>` (algorithm selection), `--target-colorspace-hint`
  (tell the display to switch HDR modes; recommended modes auto/yes), inverse
  tone mapping, black point compensation during HDR tone mapping (verified
  strings in options.rst).
- Lesson: HDR handling is a *pipeline contract* between decoder tags, renderer
  processing, and display capability — not a per-filter concern.

## 10. OBS Studio (URL: https://github.com/obsproject/obs-studio)

- OBS 28.0 (2022-08-31) introduced color management: color format/color space/
  range settings, 10-bit and HDR capture/encode, sRGB/linear working space
  (secondary confirmation via release coverage; S-2c9). Before 28, OBS
  effectively assumed NV12/Rec.709/limited and mis-displayed other content —
  the same class of bug our engine must avoid from day one (see 22_COLOR.md).
- Lesson: even a mature app needed a *global* color configuration concept
  (project working space + per-source tags), reinforcing that color conversion
  belongs at defined boundaries in the pipeline.

## 11. What these demonstrate vs generic FFmpeg filtering

| Concern | FFmpeg filters (typical) | libplacebo/mpv/OBS |
|---|---|---|
| Tone mapping | basic filters (tonemap), fewer algorithms | multiple curated algorithms + tuning |
| Gamut mapping | limited | explicit gamut mapping stage |
| Dither/deband | filters exist but separate | integrated, GPU, quality-focused |
| Display adaptation | none | ICC/HDR display hints |
| Zero-copy GPU chain | possible but manual | designed around GPU memory |

- Conclusion: for preview/display, either invest in a libplacebo-like
  rendering layer or accept visibly worse quality; ffmpeg filtering alone is
  not display-grade. Cross-check with 08_GPU_ARCHITECTURE.md (agent 3-d).

## 12. Sources

- https://gstreamer.freedesktop.org/releases/1.28/ (S-2c3)
- https://gstreamer.freedesktop.org/documentation/additional/design/ (S-2c4)
  (negotiation.html, buffer.html, events.html, caps.html, seeking.html,
  playbin.html, miniobject.html, dmabuf.html)
- GStreamer monorepo COPYING files (S-2c5):
  https://raw.githubusercontent.com/GStreamer/gstreamer/main/subprojects/gstreamer/COPYING
  (and gst-plugins-bad/-ugly equivalents)
- https://github.com/mpv-player/mpv DOCS/man/options.rst (S-2c7)
- https://github.com/haasn/libplacebo README (S-2c7)
- OBS 28 release notes/coverage (S-2c9)

## 13. Depth tracking (v0.1 -> v0.2)

- Verify per-plugin license tags (x264enc GPL label) and hardware element
  availability matrix per platform from docs snapshots.
- Prototype: GStreamer decode -> appsink (dmabuf) -> wgpu import on Linux.
- Read libplacebo doxygen for the exact tone/gamut mapping API surface.
