# W3b REPORT — wave-3 research agent (docs 23 CAPTIONS, 24 COMPUTER_VISION, 37 TRANSCRIPTS)

Date: 2026-09-24 · Agent: W3-b · Network: unauthenticated only (GITHUB_TOKEN absent).

## Docs written (replacing PLANNED stubs)

| Doc | Lines | Status line |
|---|---|---|
| docs/research/23_CAPTIONS.md | 177 | PARTIAL (v0.1 — wave-3 depth pass 2026-09-24) |
| docs/research/24_COMPUTER_VISION.md | 167 | PARTIAL (v0.1 — wave-3 depth pass 2026-09-24) |
| docs/research/37_TRANSCRIPTS.md | 153 | PARTIAL (v0.1 — wave-3 depth pass 2026-09-24) |

No other repo files modified (01_RESEARCH_INDEX.md rows 23/24/37 left as-is for the
principal to flip to 🔶; SOURCE_LEDGER.md untouched).

## Ledger

research/sources/LEDGER_W3b.md — rows S-3b0..S-3b9 (exact pipe-table format), all fetched
2026-09-24. Existing rows cited inline: S-005/S-2c2 (FFmpeg), S-2d0 (BCD), S-2e0 (Media3),
S-4f0 (survey), S-4f8 (kinocut), S-4f9 (untrusted data).

## Key findings

Captions (23):
- Task-brief correction with primary evidence: **SRT is millisecond precision
  (`HH:MM:SS,mmm`, comma decimal) → rational (n,1000)**; the *centisecond* trap
  (n/100) belongs to ASS/SSA. Both exact under ADR-007; fp seconds never stored (E-003 P5).
- WebVTT is **not yet a W3C Recommendation** — current TR snapshot is a Candidate
  Recommendation Draft (2026-05-20). Timestamp grammar, cue settings
  (vertical/line/position/size/align/region) and `::cue`/`::cue-region` verified verbatim.
- **libass license verified ISC** (COPYING, "Copyright (C) 2006-2016 libass contributors");
  latest 0.17.5 (2026-06-24). Permissive → optional ASS render backend via FFI.
- Every modern browser ships a WebVTT renderer (BCD TextTrack: Chrome 23 / FF 31 /
  Safari 6) → "bundled renderer" approach wins for sidecar playback; own compositor pass
  for burn-in/export; one cue model feeds both (render-path decision space documented).
- Shaping: **rustybuzz 0.20.1 MIT** ("complete harfbuzz shaping algorithm port") chosen as
  the deterministic shaping core for burn-in; harfbuzz C + bidi/CJK crates UNVERIFIED.
- IMSC Text Profile 1.3 = W3C REC 2026-05-21 (broadcast/streaming interchange leg);
  CTA-708/CEA-608 standards are CTA-sold (bit-level UNVERIFIED); ccextractor (GPL-2.0) =
  external pre-pass only.

Computer vision (24):
- Scene detection: PySceneDetect **BSD-3 v0.7.1** (ContentDetector/AdaptiveDetector/
  ThresholdDetector semantics from README); TransNetV2 **MIT** + paper arXiv 2008.04838,
  PyTorch inference released. Rust equivalent: **av-scenechange 0.24.1 MIT** — but
  encoder-keyframe objective by design (README warns it is not human-perception tuned);
  no other permissive Rust crate found. Browser equivalent: only **GPL-2.0**
  @doedja/scenecut-core 0.2.0 → license blocker; gap proposal = WGSL content-detector pass.
- Licensing blockers: **Ultralytics YOLO = AGPL-3.0** (no linkage; plugin/process isolation
  or RT-DETR), **RobustVideoMatting = GPL-3.0** code (weights license UNVERIFIED),
  @doedja/scenecut-core GPL-2.0. Permissive verified pair: **RT-DETR Apache-2.0** (arXiv
  2304.08069) + **ByteTrack MIT** (arXiv 2110.06864); MediaPipe Apache-2.0; ONNX Runtime MIT.
- Matting: MODNet Apache-2.0 (arXiv 2011.11961) is the permissive pick; Real-ESRGAN BSD-3.
- Smart-reframe industry evidence is marketing-level only (OpusClip AI Reframe/ClipAnything
  verified as *features* on opus.pro; CapCut Auto Reframe page located, unfetched) —
  no engineering blogs found; pipeline shape recorded as our synthesis (detect → track →
  smooth → crop keyframe commands).
- All CV outputs land as **command-generating analysis services** (scene_cuts → split_at;
  subject_track → set_crop_keyframes; matte/sr → set_effect), cached by
  (media_hash, service, model, params); command replay never re-runs inference (E-003).

Transcripts (37):
- Licenses verified from repo files: whisper.cpp MIT (v1.9.4, 2026-09-11), faster-whisper
  MIT, CTranslate2 MIT, **WhisperX BSD-2-Clause** (PyPI 3.8.6; 70x realtime large-v2,
  wav2vec2 forced alignment, pyannote diarization, VAD default-on; arXiv 2303.00747),
  transformers.js Apache-2.0 (ONNX Runtime, WebGPU), mlx-whisper MIT (Apple), openai/whisper
  MIT, Vosk Apache-2.0.
- **pyannote: code MIT (CNRS); HF models gated:"auto" with cardData.license "mit"**
  (segmentation-3.0, speaker-diarization-3.1) — code-vs-weights distinction recorded;
  distribution ships no weights, user accepts gates with HF token.
- Alignment tools: aeneas **AGPL-3.0** (blocker), gentle MIT (repo label only; LICENSE file
  404), stable-ts MIT — all three maintenance statuses UNVERIFIED (recorded honestly).
- whisper.cpp memory envelope verified (tiny 75 MiB/~273 MB … large 2.9 GiB/~3.9 GB);
  whisper.wasm claims tiny/base real-time in browser; faster-whisper README tables show
  ~3.6–6.1 GB GPU for batched large-class runs (row attribution not re-verified).
- OpenAI API pricing page returned HTTP 403 → **no cost/latency numbers recorded**
  (deliberately left UNVERIFIED, not filled from memory).
- Pipeline design: transcript as first-class project asset {media_hash, model+rev,
  quantization, language, origin, segments+words with exact rationals}; cut-by-text →
  remove_interval command batches; speaker labels → per-speaker captions; stale-on-relink
  rule; TRANSCRIPT_UNAVAILABLE explicit marker rule preserved.

## Licensing blockers found

1. Ultralytics YOLO — AGPL-3.0 (detector).
2. RobustVideoMatting — GPL-3.0 code (matting).
3. @doedja/scenecut-core — GPL-2.0 (browser scene detection).
4. aeneas — AGPL-3.0 (forced alignment).
5. ccextractor — GPL-2.0 (external pre-pass boundary, not a linkage blocker if process-isolated).

## NOT-FOUND / NOT-RETRIEVABLE (honest log)

- Permissively-licensed browser scene-detection library (npm searches "scene detection",
  "shot boundary detection"; only GPL-2.0 @doedja/scenecut-core found).
- Other maintained dedicated Rust scene-detection crates (crates.io search; av-scenechange
  is the only serious hit).
- Opus Clip / CapCut **engineering blogs** on clip-selection/reframe pipelines — web search
  returned only marketing/vendor + third-party pages; behavior internals undisclosed.
- CTA-708 / CEA-608 standards documents (sold by CTA; only secondary summaries retrievable).
- OpenAI API pricing page (HTTP 403) — cost/latency left UNVERIFIED.
- gentle LICENSE file (404 at master and main; license identified via GitHub repo label MIT).
- NVIDIA NeMo (deliberately not fetched in-budget — marked UNVERIFIED rather than asserted).

## Source IDs used

S-3b0 (WebVTT spec), S-3b1 (TTML2/IMSC/CTA-708/CEA-608 bundle), S-3b2 (libass+rustybuzz+BCD),
S-3b3 (SRT ecosystem: SubRip wiki, CCExtractor, crates.io, youtube-transcript-api),
S-3b4 (PySceneDetect), S-3b5 (TransNetV2 + av-scenechange + scenecut-core),
S-3b6 (YOLO/RT-DETR/ByteTrack/MediaPipe/ORT), S-3b7 (MODNet/RVM/Real-ESRGAN/Opus),
S-3b8 (whisper.cpp), S-3b9 (ASR ecosystem bundle). Reused: S-005, S-2c2, S-2d0, S-2e0,
S-4f0, S-4f8, S-4f9.

## Next actions (handoffs)

- doc 35 owner: fold license table from 24 §7 into LICENSES doc; verify YOLOX/RF-DETR,
  TensorFlow.js, harfbuzz/freetype/fribidi font-stack licenses, wav2vec2 model cards.
- doc 36 owner: YouTube auto-caption ToS/compliance (youtube-transcript-api evidence S-3b3).
- doc 34 owner: caption length/reading-speed norms, speaker-label UX.
- Engine phase: E-00x experiment — whisper.cpp tiny → transcript asset → cut-by-text
  remove_interval batch → E-003 replay determinism check; av-scenechange vs PySceneDetect
  accuracy spike; WGSL content-detector prototype.
