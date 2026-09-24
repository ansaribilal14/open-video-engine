# 24_COMPUTER_VISION — CV services for the editor (Track K)

> Status: PARTIAL (v0.1 — wave-3 depth pass 2026-09-24).

## 0. Method

Primary fetches 2026-09-24, unauthenticated: raw GitHub LICENSE/README (every license below
is read from the repo's license file unless noted), crates.io + npm registry APIs, arXiv
abstract pages, one vendor homepage, Wikipedia. New rows S-3b4..S-3b7 in
`research/sources/LEDGER_W3b.md`. Prior partial coverage by agent 3-f: PySceneDetect
BSD-3 + TransNetV2 MIT + command-mapping table [S-4f0-row context; doc 25 §6] — this doc
goes deeper (algorithms, licenses from primary files, Rust/browser equivalents, matting/SR).
Governing rule: CV outputs are **analysis services that emit command batches**; they never
mutate the timeline (ADR-010, E-003, doc 25 §6).

## 1. Scene/shot detection

### 1.1 PySceneDetect (Python reference)

- License **BSD-3-Clause** verified from LICENSE ("Copyright (C) 2014, Brandon Castellano");
  README states "released under the BSD 3-Clause license" [S-3b4].
- Version 0.7.1 (PyPI). Deps: click, numpy, opencv-python, platformdirs, tqdm; optional
  PyAV extra [S-3b4].
- Algorithms (README-verified): `ContentDetector` (default content-aware), `AdaptiveDetector`
  ("two-pass version ... handles fast camera movement better"), `ThresholdDetector` ("fade
  out/fade in events") [S-3b4]. API: `detect(video, ContentDetector())` returns scene
  start/end times; CLI + config file exist [S-3b4].
- Cost class: OpenCV per-frame decode+HSV-delta → CPU-H per hour of media (doc 25 table).

### 1.2 TransNetV2 (neural reference)

- License **MIT** verified ("Copyright (c) 2020 Tomáš Souček") [S-3b5]. Paper verified on
  arXiv: "TransNet V2: An effective deep network architecture for fast shot transition
  detection" (arXiv 2008.04838) [S-3b5].
- TensorFlow original + **PyTorch inference released** (inference-pytorch folder)
  [S-3b5]. Throughput/GPU claims not re-verified this pass (UNVERIFIED) — doc 25 carried
  "~fps-realtime on GPU" as community figure.

### 1.3 Rust / browser equivalents (honest scan)

- **av-scenechange (Rust)**: MIT verified; crates.io 0.24.1; description "Estimates frames
  in a video where a scenecut would be ideal"; README: "based on rav1e's scene detection
  code ... focused around detecting scenechange points that will be optimal for an encoder
  to place keyframes. It may not be the best tool if your use case is to generate scene
  changes as a human would interpret them — for that there are other tools such as SCXvid
  and WWXD" [S-3b5]. CLI + library, y4m input, JSON out, flash detection, min-scenecut
  spacing [S-3b5]. → Usable as native-core detector, but encoder-objective ≠ editor
  objective; needs threshold tuning against human-judged cuts.
- crates.io search ("subtitle" cross-check methodology aside) found no other dedicated
  maintained scene-detection crate; SCXvid/WWXD are VapourSynth/AviSynth filters
  (existence UNVERIFIED this pass) [S-3b5].
- **Browser**: npm search found `@doedja/scenecut-core` 0.2.0 — "Platform-agnostic scene
  detection: Xvid motion estimation WASM + detector orchestrator", **GPL-2.0** [S-3b5].
  No permissively-licensed mature browser scene detector found. Options: (a) run detection
  in native core, ship results; (b) isolate GPL/AGPL detector in a separate optional
  process; (c) implement content-detector as a WGSL pass in our compositor (proposal —
  HSV/luma-delta is shader-sized; deterministic on CPU verify path).

## 2. Object detection + tracking (smart crop / reframe substrate)

| Tool | License (verified from repo) | Role | Notes |
|---|---|---|---|
| Ultralytics YOLO (v8/v11) | **AGPL-3.0** [S-3b6] | detector | License blocker for engine distribution (§6) |
| RT-DETR | **Apache-2.0** [S-3b6] | detector | Paper "DETRs Beat YOLOs on Real-time Object Detection" (arXiv 2304.08069) verified; official PyTorch impl (lyuwenyu/RT-DETR) with COCO AP table + ONNX export [S-3b6] |
| ByteTrack | **MIT** [S-3b6] | tracker | "simple, fast and strong multi-object tracker"; paper arXiv 2110.06864 verified [S-3b6] |
| MediaPipe | **Apache-2.0** [S-3b6] | face/landmark detection | README is a forward stub to developers.google.com/mediapipe; solution-level details UNVERIFIED this pass |
| YOLOX / RF-DETR | NOT FETCHED — license UNVERIFIED | alt detectors | candidate Apache-2.0 backfills, verify before adoption |

AGPL note: Ultralytics AGPL-3.0 does not forbid *use*; it forbids building a
non-AGPL distributed product around it. For ove: detector must be an optional,
process-isolated analysis plugin (user-supplied model), or an Apache/BSD alternative
(RT-DETR is the verified permissive pick). ByteTrack (MIT) is linkable in core.

## 3. In-browser CV runtimes

- **onnxruntime-web / ONNX Runtime: MIT verified** ("Copyright (c) Microsoft Corporation")
  [S-3b6]. Models export to ONNX (RT-DETR does; S-3b6) and run via WASM/WebGPU. ExecEP
  parity + determinism across backends is an open risk (GPU nondeterminism) — flag any
  analysis that feeds exact commands to record raw outputs, not re-run inference at
  replay time.
- transformers.js uses ONNX Runtime in-browser (README verified) [S-3b9 — shared row with
  doc 37]; Apache-2.0.
- TensorFlow.js: NOT fetched this pass — license/scope UNVERIFIED.
- No permissive browser scene detector found (§1.3); face/body detection in browser is
  available via ONNX-converted MediaPipe-class models (concrete models UNVERIFIED).

## 4. Smart reframe / vertical-video pipelines (industry behavior)

- **Opus Clip** official site (fetched): "AI Reframe — Resize any video for every platform
  in 1 click", "ClipAnything — turn any video into viral shorts", "Animated Captions",
  "AI B-Roll", "Export to XML — Edit in Adobe Premiere" [S-3b7 — vendor marketing page;
  behavior internals UNVERIFIED]. **No engineering blog describing the clip-selection or
  reframe pipeline was found** (web search 2026-09-24; NOT-FOUND logged).
- **CapCut** "Auto Reframe" feature page located via search ("Resize & Optimize Video
  Content") but page content not fetched (guess-URL 404) — behavior UNVERIFIED [S-3b7].
- Academic anchor for reframing: Reframe Anything / actionness crops (survey S-4f0, area 5
  "spatial editing") — maps to subject-tracked crop keyframes.
- Reconstructed industry pipeline (our synthesis, marked as such): detect subject
  (person/face/saliency) → track across frames (ByteTrack-class) → smooth trajectory with
  shot awareness → emit crop-rect keyframes per aspect. This is exactly the "tracking →
  keyframe commands" mapping in doc 25 §6.

## 5. Background removal / matting + super-resolution

- **MODNet**: Apache-2.0 verified; paper "MODNet: Real-Time Trimap-Free Portrait Matting
  via Objective Decomposition" (arXiv 2011.11961, AAAI 2022 per README) verified [S-3b7].
  Permissive pick for portrait cutouts.
- **RobustVideoMatting (RVM)**: code **GPL-3.0** verified (LICENSE + README changelog
  "Code is re-released under GPL-3.0 license", 2021-09-16); paper "Robust High-Resolution
  Video Matting with Temporal Guidance"; "4K 76FPS and HD 104FPS on an Nvidia GTX 1080 Ti";
  ByteDance [S-3b7]. Recurrent temporal memory = the technically attractive option, but
  GPL-3.0 → optional external plugin only. Weights license UNVERIFIED.
- **Real-ESRGAN**: BSD-3-Clause verified ("(c) 2021, Xintao Wang"); anime-video model
  variants + model zoo documented [S-3b7]. Frame-level SR is GPU-H per hour; position as
  export-time optional service keyed by media hash.
- Matting/SR outputs are per-frame *effects inputs* (alpha matte / upscaled frames), not
  timeline edits: they attach as effect runs on clips, never as ad-hoc pixel mutations.

## 6. Where CV results land in OUR engine (E-003 / ADR-010 conformance)

Contract: `analysis.run{service, media_hash, params} -> ProposalSet` — a typed, cached
object; nothing touches the timeline. Proposal kinds and their command translations:

| Proposal | Payload | User approval → commands (E-003 schema) |
|---|---|---|
| scene_cuts | ordered cut times (exact rationals, confidence) | `split_at{times[]}` batch (one transaction, receipt) |
| subject_track | per-frame/interval crop rects on clip id | `set_crop_keyframes{clip_id, keyframes[]}` (auto-reframe) |
| matte | alpha run (per-frame data, media-hash cached) | `set_effect{clip_id, effect:"matte", ref}` |
| sr_upscale | frame-run ref | `set_effect{clip_id, effect:"sr", ref}` at export |

Rules:
- Proposals keyed by `(media_hash, service, model_id, model_version, params)`; replay of
  the command log never re-runs inference — it replays approved commands only (E-003
  determinism; GPU nondeterminism quarantined inside analysis).
- Cut times / keyframe times are exact rationals on the project tick axis (ADR-007), never
  fp seconds; detector frame indices convert exactly via source rate (n/fps).
- Batches are previewable + cancelable + receipted (ADR-010 transaction shape; kinocut
  receipts precedent S-4f8).

## 7. Licensing blockers (summary)

| Item | License | Consequence |
|---|---|---|
| Ultralytics YOLO | AGPL-3.0 [S-3b6] | no linkage into permissive engine; plugin/process isolation or RT-DETR |
| RobustVideoMatting | GPL-3.0 code [S-3b7] | optional external plugin only |
| @doedja/scenecut-core | GPL-2.0 [S-3b5] | do not bundle into browser app; use native core or WGSL port |
| PySceneDetect / TransNetV2 / av-scenechange / ByteTrack / MODNet / Real-ESRGAN / ONNX Runtime / MediaPipe | BSD/MIT/Apache [S-3b4..S-3b7] | usable (attribution + notice files) |

## 8. UNVERIFIED items

- TransNetV2 throughput claims; SCXvid/WWXD existence/status; PySceneDetect detector
  source-level details (README-level only this pass).
- YOLOX / RF-DETR / TensorFlow.js licenses; MediaPipe solution-level capabilities.
- CapCut Auto Reframe behavior; Opus Clip pipeline internals (no engineering blog found).
- RVM pretrained-weights license; MODNet onnx conversion quality.
- GPU-inference determinism across ORT backends (affects replay strategy).

## 9. Depth remaining (v0.2)

- Read PySceneDetect `content_detector.py`/`adaptive_detector.py` sources; threshold
  semantics doc.
- av-scenechange accuracy spike vs PySceneDetect on a shared sample set; judge with
  human-labeled cuts.
- WGSL content-detector pass prototype (compositor reuse, doc 08); CPU-reference parity.
- RT-DETR → ONNX → ORT-web round-trip spike; ByteTrack Rust port decision.
- ProposalSet schema draft + E-00x experiment: scene_cuts → split batch receipt on real
  media (extends E-007).
