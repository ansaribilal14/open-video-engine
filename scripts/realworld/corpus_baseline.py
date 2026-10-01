#!/usr/bin/env python3
"""Build the INDEPENDENT baseline facts JSON for one corpus media item.

Like baseline_facts.py (primary reference) but corpus-aware: OVE is never
the authority, so every fact comes from ffprobe/ffmpeg:

  identity (sha256) · container · streams (codec/dims/cadence) ·
  keyframe index (count, pts list, max GOP seconds) ·
  frame-pts delta analysis (VFR truth: distinct deltas) ·
  rotation / display-matrix side data ·
  audio geometry (rate/channels/layout) + sample count via full PCM decode ·
  decode integrity under -xerror.

Usage: corpus_baseline.py <media-file> <out-baseline.json>
"""
import json
import subprocess
import sys
import hashlib
from pathlib import Path

media = Path(sys.argv[1])
out = Path(sys.argv[2])


def run(cmd):
    return subprocess.run(cmd, capture_output=True, text=True)


facts = {"file": media.name, "size_bytes": media.stat().st_size}

# 1. identity
h = hashlib.sha256(media.read_bytes()).hexdigest()
facts["sha256"] = h

# 2. container + streams
r = run(["ffprobe", "-v", "error", "-show_format", "-show_streams", "-of", "json", str(media)])
streams = json.loads(r.stdout)
facts["container"] = streams["format"].get("format_name")
facts["container_duration"] = streams["format"].get("duration")
vs = [s for s in streams["streams"] if s.get("codec_type") == "video"]
as_ = [s for s in streams["streams"] if s.get("codec_type") == "audio"]
v = vs[0] if vs else None
a = as_[0] if as_ else None
facts["video"] = None if v is None else {
    "codec": v.get("codec_name"),
    "profile": v.get("profile"),
    "width": v.get("width"),
    "height": v.get("height"),
    "pix_fmt": v.get("pix_fmt"),
    "r_frame_rate": v.get("r_frame_rate"),
    "avg_frame_rate": v.get("avg_frame_rate"),
    "time_base": v.get("time_base"),
    "nb_frames_meta": v.get("nb_frames"),
    "duration": v.get("duration"),
    "side_data": v.get("side_data_list", []),
}
facts["audio"] = None if a is None else {
    "codec": a.get("codec_name"),
    "profile": a.get("profile"),
    "sample_rate": int(a["sample_rate"]),
    "channels": int(a["channels"]),
    "channel_layout": a.get("channel_layout"),
}

# 3. rotation (display matrix) truth
rot = None
if v and v.get("side_data_list"):
    for sd in v["side_data_list"]:
        if sd.get("side_data_type") == "Display Matrix":
            rot = sd.get("rotation", 0.0)
facts["display_rotation_deg"] = rot

# 4. keyframe truth: pts (in stream timebase) of keyframe packets
r = run(["ffprobe", "-v", "error", "-select_streams", "v:0",
         "-show_entries", "packet=pts_time,flags", "-of", "csv=p=0", str(media)])
kfs = []
for line in r.stdout.splitlines():
    parts = line.split(",")
    if len(parts) < 2:
        continue
    pts_s, flags = parts[0], parts[-1]
    if pts_s in ("", "N/A"):
        continue
    if "K" in flags:
        kfs.append(float(pts_s))
facts["keyframe_count"] = len(kfs)
facts["keyframe_pts_first8"] = kfs[:8]
gaps = [round(kfs[i + 1] - kfs[i], 6) for i in range(len(kfs) - 1)]
facts["max_gop_seconds"] = max(gaps) if gaps else None
facts["keyframe_ptss_are_sorted"] = kfs == sorted(kfs)

# 5. VFR truth from FRAMES (decode order — pts IS monotonic; packet order
#    with B-frames is not). EXACT integer analysis in the stream timebase:
#    OVE's VfrReport (D-12) computes deltas in exact rationals, so ±1-tick
#    timebase jitter on nominal-CFR media IS distinct at the container level.
#    The corpus baseline therefore reports BOTH the exact-integer verdict
#    (is_vfr_exact — the OVE-comparable truth) and the nominal human verdict
#    (deltas clustered at 0.1 ms — jitter-tolerant).
r = run(["ffprobe", "-v", "error", "-select_streams", "v:0",
         "-show_entries", "frame=pts", "-of", "csv=p=0", str(media)])
all_pts_i = []
for line in r.stdout.splitlines():
    line = line.strip().rstrip(",")
    if line and line != "N/A":
        all_pts_i.append(int(line))
if len(all_pts_i) > 2:
    exact_deltas = sorted({all_pts_i[i + 1] - all_pts_i[i] for i in range(len(all_pts_i) - 1)})
    tb = (v.get("time_base") or "1/1").split("/")
    tb_den = float(tb[1]) if len(tb) > 1 else 1.0
    tol_ticks = max(1.0, round(0.0001 * tb_den))  # 0.1 ms in timebase ticks
    clustered = []
    for d in exact_deltas:
        if not any(abs(d - c) <= tol_ticks for c in clustered):
            clustered.append(d)
    clustered.sort()
    base = exact_deltas[0]
    facts["vfr"] = {
        "time_base": v.get("time_base"),
        "distinct_delta_count_exact": len(exact_deltas),
        "exact_deltas_sample": exact_deltas[:16],
        "is_vfr_exact": len(exact_deltas) > 1,
        "distinct_delta_count_jitter_tolerant": len(clustered),
        "is_vfr": len(clustered) > 1,
        "frame_count_from_pts": len(all_pts_i),
        "uneven_pairs_pct": round(
            100.0 * sum(1 for d in (all_pts_i[i + 1] - all_pts_i[i]
                                    for i in range(len(all_pts_i) - 1)) if d != base)
            / max(1, len(all_pts_i) - 1), 3),
    }

# 6. frame count via decode
if v:
    r = run(["ffprobe", "-v", "error", "-select_streams", "v:0", "-count_frames",
             "-show_entries", "stream=nb_read_frames", "-of", "json", str(media)])
    facts["video"]["nb_frames_decoded"] = json.loads(r.stdout)["streams"][0].get("nb_read_frames")

# 7. audio sample count via full independent PCM decode (byte-derived)
if a:
    ch = int(a["channels"])
    r = run(["ffmpeg", "-v", "error", "-i", str(media), "-vn", "-f", "s16le", "-y", "/tmp/_corpus_pcm.raw"])
    raw = Path("/tmp/_corpus_pcm.raw")
    n = raw.stat().st_size if raw.exists() else 0
    raw.unlink(missing_ok=True)
    facts["audio"]["nb_samples_decoded"] = n // (2 * ch)  # s16

# 8. decode integrity
r = run(["ffmpeg", "-v", "error", "-xerror", "-i", str(media), "-f", "null", "-"])
facts["decode_clean"] = r.returncode == 0
facts["decode_stderr"] = r.stderr.strip()[:300]

json.dump(facts, open(out, "w"), indent=1)
print(json.dumps({k: facts[k] for k in ("file", "sha256", "display_rotation_deg",
                                        "keyframe_count", "max_gop_seconds",
                                        "decode_clean") if k in facts}, indent=1))
if facts.get("vfr"):
    # A-1 fix (RLW-8 audit observation): the summary referenced a stale key
    # `distinct_delta_count` and crashed AFTER the baseline JSON was written.
    # Print the two real keys, guarded, so the script exits clean.
    v = facts["vfr"]
    print("vfr:", v.get("is_vfr"),
          "deltas_exact:", v.get("distinct_delta_count_exact"),
          "deltas_jitter_tolerant:", v.get("distinct_delta_count_jitter_tolerant"))
