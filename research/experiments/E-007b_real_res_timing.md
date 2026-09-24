# E-007b — Smart-render timing at real resolutions (1080p H.264)

> Status: RUN-COMPLETE (supersedes E-007's invalidated timing leg; semantics unchanged).

- **QUESTION**: E-007 proved smart-render SEMANTICS on tiny media but its timing leg
  was honestly invalidated (micro-scale). At real resolution, how large is the
  stream-copy vs re-encode win that justifies a smart-render renderer?
- **HYPOTHESIS**: keyframe-aligned copy cuts are orders of magnitude cheaper than
  re-encode at 1080p, and duration/content-exact when keyframe-aligned.
- **IMPLEMENTATION**: `scripts/experiments/E-007b_real_res_timing.py`. Deterministic
  1080p30 source (testsrc2 + 440Hz sine, libx264 CRF23 ultrafast, GOP 60, scene-cut
  keys disabled ⇒ keyframes exactly at 0/2/4/6/8 s — asserted). Strategies per cut:
  `-c copy` vs `-c:v libx264` re-encode (both `-ss X -to Y -i src`), plus a
  full-timeline re-encode baseline. Verification: ffprobe duration/frame-count,
  first-decoded-frame md5 per strategy. ffmpeg 7.1.5.
- **HARDWARE**: container CPU, 2 cores, software encode only (no NVENC/QSV in this
  environment).
- **RESULT** (`experiments/E-007b_result.txt`):
  - source: 300 frames, 19.7 MB, keyframes at [0,2,4,6,8] s.
  - cut 0→4 s: copy **68.7 ms** vs encode 815.0 ms → **11.9×**.
  - cut 4→6 s: copy **67.0 ms** vs encode 458.0 ms → **6.8×**.
  - full re-encode baseline: 1897 ms per 10 s of 1080p.
  - keyframe-aligned copy is **duration-exact** (±50 ms check: PASS; 120/60 frames).
  - head-frame md5 differs copy-vs-encode: expected — copy is bit-exact passthrough,
    re-encode re-quantizes (generational loss); not an error.
- **LIMITATIONS**: 2 cuts × 2 reps (container CPU budget; medians reported as min);
  single-stream only (no multi-track graph, no compositing); AAC re-encoded in encode
  legs; absolute numbers are container-specific — RATIOS are the durable result.
- **DECISION**: Smart render is economically decisive at real resolutions: keyframe-
  aligned copy is ~7–12× cheaper than re-encode per segment at 1080p. Combined with
  E-007's keyframe-index requirement (mid-GOP copy cuts select wrong content), the
  renderer design is confirmed: **segment planner = keyframe-indexed copy where
  untouched, re-encode only at edit boundaries**, with checkpointed segments (Android
  FGS 6 h budget, doc 15). Feeds ADR-002 (architecture) Phase-4 renderer plan.
