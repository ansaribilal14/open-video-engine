# 07 — MLT FRAMEWORK DEEP SUMMARY

> Status: PARTIAL (v0.1).
> Owner: agent 3-b. Tracks: B (media), A (editor architecture).
> Sources: MLT framework docs (mltframework.org/docs/framework/), MLT XML doc
> (docs/mltxml/), source headers (mlt_types.h, mlt_profile.h), README,
> Kdenlive's dev-docs/mlt-intro.md as a consumer's view. Ledger IDs S-2b2,
> S-2b3, S-2b4.

## 1. What MLT is

- A C99/POSIX multimedia framework "designed for video editing"; the README
  states "MLT is a LGPL multimedia framework" and the headers carry LGPL-2.1+
  boilerplate ("version 2.1 of the License, or ... any later version") —
  license verified: **LGPL-2.1-or-later** (S-2b3).
- No dependencies beyond C99/POSIX for the framework itself; follows an OO
  design in C; "much of the design is loosely based on the Producer/Consumer
  design pattern"; uses Reverse Polish Notation for applying audio/video FX
  (framework doc, S-2b2).
- Consumers: Kdenlive, Shotcut, Flowblade (their READMEs list MLT as the core
  engine dependency, S-2b4/S-2b5/S-2b7); also the `melt` CLI player/renderer.

## 2. The service model

Four service kinds (framework doc S-2b2; Kdenlive mlt-intro.md S-2b4):

- **Producer** — source of frames (files via avformat/FFmpeg, color, images
  via pixbuf, titles, `timewarp` for speed changes).
- **Consumer** — sink of frames (sdl2 preview, encoders). Naming asymmetry:
  a consumer may "produce" an output file; the names refer to MLT Frame flow.
- **Filter** — per-frame modifier, connected in a chain; every service can
  have *attached* filters.
- **Transition** — combines frames from two tracks (multitrack compositing).

A **Frame** carries an uncompressed image plus audio samples. Communication
between connected services has three phases: get frame, get image, get audio.
"MLT employs lazy evaluation — the image and audio need not be extracted from
the source until the get image and audio methods are invoked." The consumer
*pulls*: "threading is typically in the domain of the consumer implementation."

## 3. The editing graph: playlist, multitrack, field, tractor

- **Playlist** — ordered producer entries with in/out points (the clip list of
  one track); gaps become blanks. The MLT XML doc shows entries like
  `<entry producer="producer0" in="0" out="2999"/>` (S-2b2).
- **Multitrack** — N parallel tracks; *not* a producer: a consumer pulling one
  frame would only get track 0; "something, somewhere, must ensure that all
  frames are pulled from the multitrack and elect the correct frame to pass
  on."
- **Tractor** — the wrapper that "pulls the multitrack evenly, that the
  correct frame is output, and that we have producer-like behavior."
- **Field** — where filters and transitions are "planted" for the
  tractor/multitrack pair (the doc's combine-harvester metaphor). Track order
  determines compositing precedence (higher track number wins by default).
- Track contents can be producers, playlists, or other tractors (nesting).

## 4. Properties and profiles

- **Properties** is MLT's universal, string-keyed value bag attached to every
  service (and used for events, metadata, normalization hints). Kdenlive's
  document format is literally MLT XML + `kdenlive:`-prefixed properties
  (S-2b4); Shotcut does the same with `shotcut:` properties (S-2b9). The
  property bag is MLT's extension mechanism and its backwards-compatibility
  shield.
- **Profile** — the rendering contract: `frame_rate_num/den`,
  `width/height`, `progressive`, `sample_aspect_num/den`,
  `display_aspect_num/den`, `colorspace` (mlt_profile.h, S-2b3). Time in MLT
  is a `mlt_position` — `int32_t` by default (64-bit double only if compiled
  with `MLT_DOUBLE_POSITION`) — in profile frame units. Frame rate is
  rational: 30000/1001 is exact (S-2b3).

## 5. XML serialization

An MLT XML document is "essentially a list of producers": `<producer>` (with
in/out/resource properties), `<playlist>` (entries with in/out, blanks),
`<tractor>` (tracks + field with filters/transitions). Renderers can consume
it directly (`melt project.xml`), and it is a well-known automation/interop
path. Kdenlive's file format is "XML, based on MLT's format" with extra
namespaced data MLT ignores (S-2b4 fileformat.md).

## 6. Verified strengths

1. Pull-based lazy pipeline: composable, debuggable (melt plays any project
   XML), naturally supports headless rendering.
2. Normalizing filters at load (scalers, deinterlacers, resamplers, field
   normalizers) give consumers what they ask for — a clean conformance layer.
3. Properties-as-document: applications can live inside MLT XML instead of
   inventing a parallel format (Shotcut's whole project format).
4. Rational frame-rate profile + integer positions: frame-accurate editing
   math without floating-point drift (supports claim C-001).
5. Reference-counted producer instances allow multiple playlist references to
   one source (framework doc).

## 7. Verified limitations and risks

1. **Pixel format age**: "the framework is designed to be color space neutral
   — the currently implemented modules, however, are very much 8bit YUV422
   oriented" (framework doc). HDR/10-bit paths exist in newer modules
   (mlt_image_yuv420p10/yuv444p10/yuv422p16 and rgba64 in mlt_types.h,
   S-2b3) but the core orientation is SD-era.
2. **GPU story limited/optional**: movit and opengl texture image formats
   exist (mlt_image_movit, mlt_image_opengl_texture, S-2b3) but the GPU
   path has been historically fragile in consumers; Kdenlive community
   reports of libmovit render instability (S-2b9, quality 9). MLT's own docs
   describe image formats as a negotiation, not a GPU compute model.
3. **Threading is consumer-centric**: pull model means parallelism lives in
   consumer implementations and worker tasks (e.g., audio level prefetch in
   Shotcut, UNVERIFIED detail); the framework does not give a dataflow
   parallelism model like GStreamer's streams/threads.
4. **C API age / property-string culture**: powerful but stringly-typed;
   typos in property names fail silently. Kdenlive/Shotcut wrap it in C++
   (mlt++) and their own models.
5. **32-bit positions by default**: `mlt_position` int32 caps a 30 fps
   timeline at ~2.26 years of frames (2^31 / 30 / 86400 ≈ 821 days) — fine,
   but callers must opt into `MLT_DOUBLE_POSITION` for safety, and that
   switch makes positions doubles (a fp caution for us; we would use int64).
6. **Consumer pulls one frame at a time** — a structural match for preview,
   but random-access scrubbing across many sources requires producer-level
   seeking competence per module (quality varies).

## 8. Lessons for the Open Video Engine (decision-relevant)

1. The service graph (producer/filter/transition/consumer) + pull model is a
   proven minimal editing-engine core; but a modern engine should put the
   parallelism and GPU story in the framework, not in each consumer.
2. The universal property bag is what let three independent NLEs build very
   different UX on one engine — our engine needs an equivalent escape hatch
   (typed metadata on every node).
3. Document-as-engine-XML (Kdenlive/Shotcut) gives headless rendering for
   free; our project format should degrade to a renderable form (or the
   engine should accept the document directly).
4. Do not inherit MLT's 8-bit-first assumptions; design color/HDR from the
   start (see 22_COLOR, track I).
5. Integer positions + rational rates are the correct time core (C-001); use
   int64, not int32.
