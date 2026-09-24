# 37_TRANSCRIPTS — Transcription/alignment pipeline (Track Z)

> Status: PARTIAL (v0.1 — wave-3 depth pass 2026-09-24).

## 0. Method

Primary fetches 2026-09-24, unauthenticated: raw GitHub LICENSE/README (ggml-org/whisper.cpp,
SYSTRAN/faster-whisper, OpenNMT/CTranslate2, m-bain/whisperX, pyannote/pyannote-audio,
huggingface/transformers.js, ml-explore/mlx-examples, openai/whisper, ReadBeyond/aeneas,
jianfch/stable-ts, alphacep/vosk-api), Hugging Face model API JSON, PyPI JSON, arXiv
abstract pages. New rows S-3b8/S-3b9 in `research/sources/LEDGER_W3b.md`. The directive's
original "per-video transcript extraction" stub scope is superseded here by the engine-relevant
transcription pipeline scope per mission context; honesty rule (TRANSCRIPT_UNAVAILABLE)
is preserved in §7. Overlap: caption rendering = doc 23; agent governance = docs 26/27.

## 1. Engine survey (licenses verified from repo license files)

| Engine | License (verified) | Backend / notes | Version evidence |
|---|---|---|---|
| whisper.cpp | MIT ("The ggml authors") [S-3b8] | GGML C/C++; CPU + Vulkan GPU support + OpenVINO; WebAssembly [S-3b8] | release v1.9.4 (2026-09-11) via GitHub API [S-3b8] |
| faster-whisper | MIT (SYSTRAN) [S-3b9] | CTranslate2 (MIT) [S-3b9]; "up to 4 times faster than openai/whisper for the same accuracy while using less memory"; 8-bit quantization [S-3b9] | README; deps used by whisperX [S-3b9] |
| WhisperX | **BSD-2-Clause** ("(c) 2024, Max Bain") [S-3b9] | faster-whisper backend; batched inference "70x realtime ... large-v2"; word-level timestamps via **wav2vec2 forced alignment**; diarization via pyannote; VAD preprocessing default-on | PyPI 3.8.6; paper arXiv 2303.00747 verified [S-3b9] |
| Transformers.js | Apache-2.0 [S-3b9] | ONNX Runtime in-browser; `device: 'webgpu'`; dtype fp32/fp16/q8 [S-3b9] | README (WebGPU guide; experimental warning) |
| mlx-whisper (Apple) | MIT ("© 2023 Apple Inc.", mlx-examples) [S-3b9] | MLX on Apple silicon; default model `mlx-community/whisper-tiny`; HF collection of pre-converted models [S-3b9] | README |
| Vosk | Apache-2.0 (COPYING) [S-3b9] | offline non-Whisper (Kaldi) toolkit, 20+ languages; small-footprint alternative | README [S-3b9] |
| OpenAI Whisper (orig.) | MIT ("(c) 2022 OpenAI") [S-3b9] | reference weights + PyTorch impl | LICENSE |
| NVIDIA NeMo | NOT FETCHED this pass — license/versions UNVERIFIED | candidate diarization/ASR toolkit | — |

WhisperX licensing nuance: the *code* is BSD-2, but its pipeline depends on gated HF models
(§2) and wav2vec2 alignment models (facebook/wav2vec2-* card licenses UNVERIFIED) — engine
distribution must separate code license from model terms [S-3b9].

## 2. Diarization: code vs model licenses (verified distinction)

- **pyannote.audio code: MIT** ("Copyright (c) 2020 CNRS") [S-3b9].
- **Models: gated on Hugging Face.** HF API for `pyannote/segmentation-3.0` and
  `pyannote/speaker-diarization-3.1`: `gated: "auto"`, `cardData.license: "mit"` (both)
  [S-3b9]. So: MIT terms, but download requires accepting user conditions with an HF token.
- Current pipeline is `speaker-diarization-community-1` (pyannote README + WhisperX docs
  both require accepting its user conditions + HF token) [S-3b9]. pyannoteAI sells a
  premium hosted tier [S-3b9].
- Engine stance: diarization = local analysis service that loads user-accepted models;
  the engine ships no weights and stores only speaker labels in the project (§6).
- NVIDIA NeMo diarization: UNVERIFIED (not fetched) — depth item.

## 3. Word-level timestamps and forced alignment (statuses, honestly)

- faster-whisper: `word_timestamps=True` option exists in README example [S-3b9] (Whisper
  cross-attention DTW; accuracy analysis UNVERIFIED beyond README).
- WhisperX: word-level timestamps come from **wav2vec2 forced alignment on top of Whisper
  segments** — README defines "Forced Alignment ... orthographic transcriptions aligned to
  audio ... to generate phone level segmentation" and shows `--align_model
  WAV2VEC2_ASR_LARGE_LV60K_960H` [S-3b9]. Paper: "WhisperX: Time-Accurate Speech
  Transcription of Long-Form Audio" (arXiv 2303.00747, verified) [S-3b9].
- whisper.cpp: token-level timestamp support (DTW) NOT verified this pass — UNVERIFIED;
  README-verified features: quantization, Vulkan, OpenVINO, WASM [S-3b8].
- Post-processors: stable-ts MIT ("(c) 2022 jian") — OpenAI-whisper-adjacent timing
  refinement; maintenance status UNVERIFIED [S-3b9]. **aeneas: AGPL-3.0** (LICENSE
  verified) — copyleft blocker; project staleness UNVERIFIED [S-3b9]. **gentle: MIT**
  (GitHub repo license label; LICENSE file 404 at both default-branch paths — flag);
  maintenance status UNVERIFIED [S-3b9].
- VAD preprocessing "reduces hallucination & batching with no WER degradation" (WhisperX
  README) [S-3b9] — adopt VAD before transcription in our pipeline.

## 4. Model sizes / resource envelope (verified numbers)

whisper.cpp README "Memory usage" table (disk / runtime mem) [S-3b8]:

| Model | Disk | Mem |
|---|---|---|
| tiny | 75 MiB | ~273 MB |
| base | 142 MiB | ~388 MB |
| small | 466 MiB | ~852 MB |
| medium | 1.5 GiB | ~2.1 GB |
| large | 2.9 GiB | ~3.9 GB |

- Quantization: integer-quantized ggml models (e.g. `q5_0`) shrink disk/mem further
  [S-3b8]. whisper.cpp build matrix includes large-v3 and large-v3-turbo targets [S-3b8].
- In-browser: whisper.wasm example README claims real-time for tiny/base on a modern
  CPU/browser ("transcribe a 60 seconds audio in about ..." — full sentence truncated in
  our extract; claim scope: tiny/base real-time) [S-3b8].
- faster-whisper README benchmark tables list ~3.6–6.1 GB GPU memory for batched inference
  (fp16/int8, batch_size=8 rows) — model attribution of each row NOT re-verified; treat as
  large-class envelope [S-3b9].
- OpenAI cloud API: pricing page fetch returned HTTP 403 — **no cost/latency numbers
  recorded; UNVERIFIED** (deliberately not filled from memory) [S-3b9 limitations].

## 5. Privacy / local-first stance

- Local-first default is technically credible across all three adapters: desktop =
  whisper.cpp (CPU/ANE via MLX on Apple) [S-3b8/S-3b9]; browser = Transformers.js whisper
  via ONNX Runtime + WebGPU [S-3b9]; Android = engine core + small ggml models (memory
  table fits mid-range devices at tiny/base/small) [S-3b8].
- Cloud transcription = opt-in only, explicit per-job consent, no media leaves the project
  implicitly; cloud vs local runs are recorded in the transcript asset (`origin` field, §6)
  so provenance survives replay (E-003).
- Media-derived text is untrusted data, never instructions (MCPXKIT / S-4f9; doc 27) —
  applies to transcript strings fed into any AI feature.

## 6. Pipeline design for the engine (transcript as first-class asset)

```
Transcript { id, media_hash, model_id, model_rev, quantization, language,
             origin: local|cloud{provider}, created_at,
             segments: [ { id, start: Rational, end: Rational, text,
                           speaker: Option<SpeakerId>, words: Option<[WordTime]> } ] }
WordTime { span: [start,end] Rational, word: string, conf: Option }   // exact times
```

Chain (each arrow is a command-producing step, never direct mutation):

1. `transcribe.run{media_hash, model, params}` → Transcript asset (analysis cache), keyed
   by media hash + model + revision; re-runs are explicit, not replay-time (E-003).
2. transcript → captions: segmentation service (doc 23 §8) → `caption.import` batch.
3. **cut-by-text**: user selects text range → map to word/segment intervals →
   `remove_interval{clip_id, [start,end] exact rationals}` command batch (ripple via
   normal timeline ops); undo = inverse batch. Text range → interval mapping must hit
   word boundaries; falls back to segment boundaries when `words` absent.
4. speaker labels: group per speaker → caption track grouping / per-speaker styling
   (doc 23 §9 `caption.set_style`).
5. Relink rule: media_hash mismatch ⇒ transcript marked stale; re-transcribe or re-align
   (forced alignment of old text against new audio — tool support UNVERIFIED, §3).

Command schema conformance: all times (num,den) rationals on the project tick axis
(ADR-007 + refinement); explicit ids; floats forbidden (E-003 P5).

## 7. Honest gaps

- TRANSCRIPT_UNAVAILABLE rule: when no transcript can be produced (no speech, failed
  model load, denied model gate), the engine records an explicit unavailable marker, not
  an empty transcript — callers must branch on it.
- No numbers cited for OpenAI API cost/latency (fetch 403) — §4.
- Diarization quality tradeoffs (DER, speaker counts) not benchmarked here.

## 8. UNVERIFIED items

- whisper.cpp token-level timestamps (`--dtw`) and per-token confidence outputs.
- NeMo license/versions; wav2vec2 alignment-model card licenses; distil-whisper model
  card terms (checkpoint compatibility verified only).
- faster-whisper benchmark-row model attribution; VAD implementation identity (silero
  per community — UNVERIFIED).
- aeneas/gentle/stable-ts maintenance status; gentle LICENSE file absence.
- Browser whisper accuracy vs native; WebGPU availability on user machines (doc 10 matrix).

## 9. Depth remaining (v0.2)

- E-00x experiment: whisper.cpp tiny/base on sample audio → transcript asset →
  cut-by-text command batch → replay determinism check (ties E-003 to a real model).
- Benchmark word-level accuracy: whisper.cpp vs faster-whisper vs WhisperX on the same
  clip (WER + boundary error vs manual marks).
- Diarization UX: speaker-label correction commands (rename/merge/split speakers).
- Local model manager UX: download/verify/acceptance for gated models (doc 31 storage).
- Forced-alignment service decision (for re-transcribe/relink path).
