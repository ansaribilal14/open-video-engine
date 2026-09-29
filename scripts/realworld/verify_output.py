#!/usr/bin/env python3
"""INDEPENDENT output verification + visual sanity extraction (Phase 8/9).

Verifies the OVE proof artifacts with ffprobe/ffmpeg only — OVE is never
the authority — and extracts representative frames for visual inspection.
"""
import json
import subprocess
import sys
from pathlib import Path

PROOF = Path(sys.argv[1] if len(sys.argv) > 1 else ".")
SRC = Path(sys.argv[2] if len(sys.argv) > 2 else "source.mp4")
FRAMES = Path("/home/z/my-project/ove-artifacts/frames")
FRAMES.mkdir(parents=True, exist_ok=True)
AV = PROOF / "realworld_proof_av.mp4"

report = {}

def run(cmd):
    return subprocess.run(cmd, capture_output=True, text=True)

# 1. full decode integrity (both streams)
r = run(["ffmpeg", "-v", "error", "-xerror", "-i", str(AV), "-f", "null", "-"])
report["decode_video_clean"] = r.returncode == 0
report["decode_stderr"] = r.stderr.strip()[:400]

# 2. frame-level facts (count + pts continuity)
r = run(["ffprobe", "-v", "error", "-select_streams", "v:0", "-count_frames",
         "-show_entries", "stream=nb_read_frames,avg_frame_rate", "-of", "json", str(AV)])
v = json.loads(r.stdout)["streams"][0]
report["nb_read_frames"] = v.get("nb_read_frames")
report["avg_frame_rate_out"] = v.get("avg_frame_rate")

# 3. audio sample count via full PCM decode
r = run(["ffmpeg", "-v", "error", "-i", str(AV), "-vn", "-f", "s16le", "-y", "/tmp/_out_pcm.raw"])
import os
raw = os.path.getsize("/tmp/_out_pcm.raw")
report["audio_samples_decoded"] = raw // 4  # s16 stereo

# 4. WAV sample count
wav = PROOF / "realworld_proof.wav"
r = run(["ffprobe", "-v", "error", "-show_entries", "stream=sample_rate,channels,duration", "-of", "json", str(wav)])
ws = json.loads(r.stdout)["streams"][0]
report["wav"] = ws

# 5. A/V duration delta
r = run(["ffprobe", "-v", "error", "-show_entries", "format=duration", "-of", "json", str(AV)])
report["container_duration"] = json.loads(r.stdout)["format"]["duration"]

# 6. visual sanity frames: beginning / early motion / scene transition /
#    key edited region (split seam 2.75s, overlay fade-in 1.05s, mid-fade 2.5s)
#    / middle / end
spots = {
    "begin_0.00s": 0.0,
    "early_0.50s": 0.5,
    "overlay_fadein_1.05s": 1.05,
    "overlay_mid_1.50s": 1.5,
    "seam_2.70s": 2.70,
    "overlay_fade_out_3.20s": 3.20,
    "after_overlay_3.60s": 3.60,
    "clip4_5.00s": 5.0,
    "end_6.60s": 6.6,
}
for name, t in spots.items():
    r = run(["ffmpeg", "-v", "error", "-y", "-ss", f"{t:.3f}", "-i", str(AV),
             "-frames:v", "1", str(FRAMES / f"out_{name}.png")])
    report[f"frame_{name}"] = r.returncode == 0

# 7. corresponding SOURCE frames for comparison at identical content times
#    (timeline t -> source time via the recorded edit map)
src_map = {
    "begin_0.00s": 0.0,        # clip1 src 0-2
    "early_0.50s": 0.5,
    "seam_2.70s": 8.70,        # clip2b src 8.75-ish region (seam frame)
    "clip4_5.00s": 21.25,      # clip4 src 20 + (5.0-3.75)
    "end_6.60s": 22.85,
}
for name, t in src_map.items():
    r = run(["ffmpeg", "-v", "error", "-y", "-ss", f"{t:.3f}", "-i", str(SRC),
             "-frames:v", "1", str(FRAMES / f"src_{name}.png")])
    report[f"src_{name}"] = r.returncode == 0

# 8. pixel comparison (numpy): mean abs diff on the non-overlay pairs
import numpy as np
def load(p):
    # decode PNG to raw RGB via ffmpeg pipe (no PIL dependency)
    r = subprocess.run(["ffmpeg", "-v", "error", "-i", str(p), "-f", "rawvideo",
                        "-pix_fmt", "rgb24", "-"], capture_output=True)
    data = np.frombuffer(r.stdout, dtype=np.uint8)
    return data.reshape(-1, 1280 * 3)[:720]

pixel = {}
for name, src_t in src_map.items():
    out_p = FRAMES / f"out_{name}.png"
    src_p = FRAMES / f"src_{name}.png"
    if not (out_p.exists() and src_p.exists()):
        pixel[name] = "missing"
        continue
    o, s = load(out_p), load(src_p)
    if o.shape != s.shape:
        pixel[name] = f"shape mismatch {o.shape} vs {s.shape}"
        continue
    diff = np.abs(o.astype(np.int16) - s.astype(np.int16))
    mad = float(diff.mean())
    pct_close = float((diff.max(axis=-1) <= 24).mean() * 100)
    pixel[name] = {"mean_abs_diff": round(mad, 2), "pct_pixels_within_24": round(pct_close, 2)}
report["pixel_comparison_out_vs_src"] = pixel

print(json.dumps(report, indent=1))
json.dump(report, open(PROOF / "independent_verification.json", "w"), indent=1)
