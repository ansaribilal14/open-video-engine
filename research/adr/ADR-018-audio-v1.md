# ADR-018: Audio leg v1 (decode, encode, assembly, export)

- **Status**: ACCEPTED (2026-09-29) — acceptance condition met: ENGINE_
  BUILD_PLAN wave-7 audio gate green (decode A-1..A-4, encode E-8..E-10,
  engine W7 integration: sample-exact assembly, A/V export with ± 0 sample
  drift ffprobe-verified, WAV/PCM out exact; workspace 141/141).
- **Date**: 2026-09-29 · **Confidence**: 0.88 (the canonical-surface and
  priming-trim decisions are durable; the single-audio-lane and no-mixing
  v1 notes are named, short-lived limitations)

## CONTEXT

ENGINE_BUILD_PLAN wave 7: audio decode → sample-exact mapping on the tick
axis → mix → WAV/PCM out; A/V sync at export (ENCODER_SPEC §2 sample-count
authority, §3.3 seam re-encode). The planner shipped `Deferred` for audio
seams at W3 with the recorded promise that the route becomes executable at
the audio wave. Constraints unchanged: exact rationals, typed errors, libav
confined to {ove-decode, ove-encode}, software path is the reference.

## DECISION

1. **Canonical decoded PCM surface = planar f32 ("fltp") at the SOURCE
   rate and layout** (ove-decode). swresample performs FORMAT conversion
   only — rate and channel layout are structurally unchanged (a resample
   would break sample-count exactness and is therefore impossible in the
   adapter, not merely forbidden). s32→f32 precision narrowing is accepted
   and declared: the audio contract is TIMING exactness (sample counts,
   durations, cut points); lossy codecs already bound value fidelity.
   Audio frames are NOT pooled (tiny, variable-size; the video pool
   contract is untouched) — `reclaim()` on an audio session is a typed
   internal error.
2. **Audio seek = floor frame + sample trim (the D-5 rule applied to
   audio)**. Video D-4 drops whole frames before the Exact target; audio
   frames are 1024-sample granular, so the frame STRADDLING the target is
   delivered whole and the CALLER trims the exact sample in-point
   (target − frame.pts) × rate, which is always an exact integer for
   rate-aligned seeks. Whole frames whose END is ≤ target are dropped.
   Pinned by A-3.
3. **Per-frame duration for audio is nb_samples/rate, never the container
   field** (FRAME_CONTRACT §3.2 already normative; now enforced in the
   decode adapter). Decoded AAC facts: priming IS trimmed by the
   demuxer/decoder (first pts 0 via skip_samples); the tail is NOT
   sample-trimmed (the last frame extends up to the AAC grid) — callers
   trim to the exact duration (A-1 pins the corpus numbers).
4. **AAC encoder (ove-encode, native libavcodec aac — LGPL)**: input =
   the canonical surface; the adapter buffers caller samples to the codec
   grid (frame_size 1024) and stamps input frames at the chunk's real
   sample start; the codec's audio frame queue subtracts
   `initial_padding` on the FIRST output packet (pts −1024), which the
   container expresses as the trim edit list (media_time 1024) —
   verified against a CLI-produced reference file (first packet pts −1024,
   elst 1024, file duration sample-exact). `TrackSpec` gains
   `initial_padding` so the muxer records it on the codec parameters;
   `ParsedStream::track_spec` propagates it for copy routes. The
   small-last-frame branch is DETECTED from
   `AV_CODEC_CAP_SMALL_LAST_FRAME` (present on the pinned 7.1.x, both
   local and CI) and reported by the adapter; the absent-capability
   branch (zero-pad the last frame) is implemented and declared so a
   different encoder build degrades honestly. Feed contiguity is
   ENFORCED: pts must equal the sample cursor or the feed is a typed
   `FrameMismatch`. AAC v1 is CBR-only (CRF = typed `Unsupported`).
5. **WAV/PCM out is pure Rust** (no libav): RIFF/WAVE PCM16 LE with the
   pinned conversion `s16 = round(clamp(x, −1, 1) × 32768)` (the libav
   fltp→s16 convention; deterministic bytes; v1 mono/stereo).
6. **Engine audio v1 = single lane**: audio follows the FIRST track's
   video placements (the W5/W6 session model), assembled as a
   concatenation of per-clip sample ranges with every cut required to
   land on a sample boundary (`NonExactSampleCut` otherwise) and the
   span authority enforced (exactly span × rate samples; short sources
   are a typed error, never a silent pad). Retimed clips are video-only
   (`AudioRetimeUnsupported`); multi-track audio mixing and
   overlap/fade legs are NAMED GAPS for the render-graph audio wave.
   `export_reencode` pairs video re-encode with audio re-encode
   automatically when the sources carry audio (ENCODER_SPEC §3.3);
   `export_wav` delivers PCM out.
7. **Test IDs**: decode A-1..A-4 (audio_conformance.rs); encode E-8/E-8b
   (AAC exactness + typed rejections), E-9 (A/V mux, two streams,
   ffprobe + libav verified, drift ± 0 samples), E-10 (WAV); engine
   w7_audio_assembly_and_av_export (assembly, A/V export, WAV, hash
   stability across export work).

## CONSEQUENCES

- The planner's `reencode_available` flip makes the W3 `Deferred` audio
  route executable with zero planner changes (the pure logic was already
  complete).
- Byte goldens for AAC output are NOT committed in v1 (encoder delay +
  edit-list interactions are version-sensitive); duration/count exactness
  and round-trip decode are pinned instead. Byte goldens land with the
  E-5 keying work when the encoder identity table is stable.
- Named gaps (not hidden): audio mixing/overlaps, audio retiming,
  multi-lane assembly, E-6 audio checkpoint execution.

## REOPEN CONDITIONS

- An encoder without `AV_CODEC_CAP_SMALL_LAST_FRAME` in the supported set
  forces the padded-tail policy to be exercised and its container story
  (edit list end trim) verified.
- A codec whose audio time base is not 1/rate forces the NonExactSampleCut
  path to be re-examined (currently a typed error; may become exact
  tick-domain cuts).
- A mixing requirement (overlapping audio clips) opens the render-graph
  audio legs decision (float mixing graph vs. integer fixed-point).
