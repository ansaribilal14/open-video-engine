# E-007 — Smart-render / stream-copy cut semantics (real H.264 media)

> Status: RUN-COMPLETE (4/5 checks; 1 check invalidated honestly — see LIMITATIONS).

- **QUESTION**: Are stream-copy cuts frame-exact only at keyframes? What do mid-GOP copy
  cuts actually produce? Is copy actually faster than re-encode?
- **HYPOTHESIS**: Copy cuts are keyframe-quantized; mid-GOP copy cuts yield wrong content;
  re-encode is exact but slower.
- **IMPLEMENTATION**: `scripts/experiments/E-007_smart_render.py` — ffmpeg-generated
  320×180 24fps 6s H.264 (GOP=24, 6 keyframes); ffprobe keyframe/packet analysis; three
  cut modes: copy-at-keyframes, copy-mid-GOP, re-encode-mid-GOP.
- **HARDWARE**: container CPU, ffmpeg 7.1.5.
- **RESULT**:
  1. Copy cut at keyframe boundaries (1.0s→4.0s): **frame-exact 72/72**, 123 ms.
  2. **Mid-GOP copy cut: requested 1.5s → output starts at 0.0s** (snapped back two
     keyframes, wrong content selected) — 48 frames covering 0–2s. CONFIRMS doc 04:
     stream copy without a keyframe index silently selects wrong media.
  3. Re-encode mid-GOP cut: frame-exact 48/48, 89 ms.
  4. Timing comparison INVALID at this scale: copy (123 ms) was slower than re-encode
     (89 ms) because 320×180 re-encode is nearly free and the copy path demuxed more
     stream. On real footage the relationship inverts (copy = O(bytes), re-encode =
     O(pixels×frames)). Recorded as a micro-benchmark limitation, not a contradiction.
- **LIMITATIONS**: toy resolution; single codec (H.264 baseline); no audio; timing leg
  non-representative (rerun at 1080p multi-Mbps in E-007b).
- **DECISION**: Engine requirements confirmed with primary evidence: (a) per-clip
  keyframe index is MANDATORY (MP4 stss/MKV cues); (b) smart render = stream copy with
  boundary-GOP re-encode for mid-GOP edit points; (c) never trust copy cuts without
  index verification. Doc 04 claims upgraded from literature-sourced to
  experiment-confirmed.
