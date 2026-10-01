#!/usr/bin/env python3
"""INDEPENDENT corpus output verification + visual frames (RLW-8).

Verifies every corpus A/V export and WAV with ffprobe/ffmpeg only — OVE is
never the authority — and extracts representative frames for visual
inspection. One verification JSON per corpus item, plus one combined
summary printed to stdout.

Usage: corpus_verify.py <corpus_proof_dir> <frames_out_dir>
"""
import json
import subprocess
import sys
from pathlib import Path

PROOF = Path(sys.argv[1] if len(sys.argv) > 1 else ".")
FRAMES = Path(sys.argv[2] if len(sys.argv) > 2 else "frames")
FRAMES.mkdir(parents=True, exist_ok=True)

ITEMS = {
    # id: (out_frames, rate_str, frame_spots timeline-seconds)
    "portrait_true": (96, "30/1", [0.0, 0.5, 1.0, 1.6, 2.3, 3.1]),
    "rotation_metadata": (80, "24000/1001", [0.0, 0.5, 1.05, 1.5, 2.7, 3.2]),
    "long_gop": (80, "24000/1001", [0.0, 0.5, 1.05, 1.5, 2.7, 3.2]),
    "audio_48k": (80, "24000/1001", [0.0, 0.5, 1.05, 1.5, 2.7, 3.2]),
    "audio_mono": (80, "24000/1001", [0.0, 0.5, 1.05, 1.5, 2.7, 3.2]),
    "audio_51": (80, "24000/1001", [0.0, 0.5, 1.05, 1.5, 2.7, 3.2]),
    # RLW-8-F3 FIXED: the VFR item now exports (96 f @ 30/1) and is verified
    # like the others; it is SILENT, so its audio probe legitimately returns
    # no streams (recorded as absent, never an error).
    "vfr_constructed": (96, "30/1", [0.0, 0.5, 1.0, 1.6, 2.3, 3.1]),
}


def run(cmd):
    return subprocess.run(cmd, capture_output=True, text=True)


summary = {}
for item, (n_frames, rate, spots) in ITEMS.items():
    rep = {}
    av = PROOF / f"corpus_{item}_av.mp4"
    if av.exists():
        r = run(["ffmpeg", "-v", "error", "-xerror", "-i", str(av), "-f", "null", "-"])
        rep["decode_clean"] = r.returncode == 0
        rep["decode_stderr"] = r.stderr.strip()[:200]
        r = run(["ffprobe", "-v", "error", "-select_streams", "v:0", "-count_frames",
                 "-show_entries", "stream=nb_read_frames,avg_frame_rate,width,height",
                 "-of", "json", str(av)])
        s = json.loads(r.stdout)["streams"][0]
        rep["nb_read_frames"] = s.get("nb_read_frames")
        rep["avg_frame_rate_out"] = s.get("avg_frame_rate")
        rep["out_dims"] = f'{s.get("width")}x{s.get("height")}'
        rep["frames_exact"] = s.get("nb_read_frames") == str(n_frames)
        r = run(["ffprobe", "-v", "error", "-show_entries", "format=duration", "-of", "json", str(av)])
        rep["container_duration"] = json.loads(r.stdout)["format"]["duration"]
        r = run(["ffprobe", "-v", "error", "-select_streams", "a:0", "-count_frames",
                 "-show_entries", "stream=nb_read_frames,codec_name,channels,sample_rate",
                 "-of", "json", str(av)])
        # silent exports (vfr_constructed) legitimately have no audio stream:
        # record absence, never crash (F3-fixed wave)
        streams = json.loads(r.stdout).get("streams") or []
        ast = streams[0] if streams else {}
        rep["audio_out"] = {k: ast.get(k) for k in ("codec_name", "channels", "sample_rate")}
        if not streams:
            rep["audio_absent_expected"] = item == "vfr_constructed"
        for name, t in spots.items() if isinstance(spots, dict) else [(f"{t:.2f}s", t) for t in spots]:
            r = run(["ffmpeg", "-v", "error", "-y", "-ss", f"{t:.3f}", "-i", str(av),
                     "-frames:v", "1", str(FRAMES / f"out_{item}_{name}.png")])
            rep[f"frame_{name}"] = r.returncode == 0
    else:
        rep["av_export"] = "ABSENT (typed failure recorded by the engine — see the item record)"
        # visual evidence from the SOURCE for the failed item (the portrait of
        # what OVE was asked to process)
        src = Path("/home/z/my-project/corpus/corpus_items") / f"{item}.mp4"
        if src.exists():
            for name, t in [("1.00s", 1.0), ("2.60s", 2.6)]:
                r = run(["ffmpeg", "-v", "error", "-y", "-ss", f"{t:.3f}", "-i", str(src),
                         "-frames:v", "1", str(FRAMES / f"src_{item}_{name}.png")])
                rep[f"src_frame_{name}"] = r.returncode == 0

    wav = PROOF / f"corpus_{item}.wav"
    if wav.exists():
        r = run(["ffprobe", "-v", "error", "-show_entries",
                 "stream=sample_rate,channels,duration", "-of", "json", str(wav)])
        rep["wav"] = json.loads(r.stdout)["streams"][0]

    summary[item] = rep

ok_counts = {}
for item, rep in summary.items():
    bools = [v for v in rep.values() if isinstance(v, bool)]
    ok_counts[item] = (sum(bools), len(bools))
    print(f"== {item}: booleans {sum(bools)}/{len(bools)}")
    print(json.dumps(rep, indent=1)[:900])

json.dump(summary, open(PROOF / "corpus_independent_verification.json", "w"), indent=1)
print("written:", PROOF / "corpus_independent_verification.json")
