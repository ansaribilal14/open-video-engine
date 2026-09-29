#!/usr/bin/env python3
"""Build the INDEPENDENT baseline facts JSON for the real-world proof test.

Reads the ffprobe-derived baseline JSON (produced by baseline_analysis.sh)
and writes the compact expectation file the ove-engine realworld test
consumes. OVE is never the authority: every fact here comes from ffprobe.
"""
import json
import sys

baseline_dir, out_path = sys.argv[1], sys.argv[2]

s = json.load(open(f"{baseline_dir}/baseline_streams.json"))
v = json.load(open(f"{baseline_dir}/baseline_vcount.json"))["streams"][0]
a = json.load(open(f"{baseline_dir}/baseline_acount.json"))["streams"][0]
vs = [x for x in s["streams"] if x["codec_type"] == "video"][0]
as_ = [x for x in s["streams"] if x["codec_type"] == "audio"][0]

facts = {
    "container": s["format"].get("format_name"),
    "video": {
        "codec": vs.get("codec_name"),
        "profile": vs.get("profile"),
        "width": vs["width"],
        "height": vs["height"],
        "pix_fmt": vs.get("pix_fmt"),
        "avg_frame_rate": vs.get("avg_frame_rate"),
        "r_frame_rate": vs.get("r_frame_rate"),
        "time_base": vs.get("time_base"),
        "nb_frames_decoded": v.get("nb_read_frames"),
    },
    "audio": {
        "codec": as_.get("codec_name"),
        "sample_rate": int(as_["sample_rate"]),
        "channels": int(as_["channels"]),
        "channel_layout": as_.get("channel_layout"),
        "nb_samples_decoded": a.get("nb_read_samples"),
    },
    "duration": s["format"].get("duration"),
    "sha256": open(f"{baseline_dir}/identity_sha256.txt").read().split()[0],
    "keyframes": len(open(f"{baseline_dir}/baseline_keyframes.txt").read().split()),
    "decode_clean": "DECODE_CLEAN" in open(f"{baseline_dir}/decode_errors.txt").read(),
}
json.dump(facts, open(out_path, "w"), indent=1)
print(json.dumps(facts, indent=1))
