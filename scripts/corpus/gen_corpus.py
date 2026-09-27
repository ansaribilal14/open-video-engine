#!/usr/bin/env python3
"""Wave 2 conformance corpus generator (DECODER_SPEC §2 test media).

Self-generated tiny media, committed to the repo so CI decodes (never encodes).
Codec choice: MPEG-4 Part 2 (`-c:v mpeg4`, libavcodec-native, LGPL build ok,
no codec-patent-pool entanglement for the corpus). Containers: MP4 with exact
per-track timescales so pts are exactly representable rationals (ADR-007).

Run from repo root:  python3 scripts/corpus/gen_corpus.py
Outputs:             engine/ove-decode/tests/media/*.mp4
                     engine/ove-decode/tests/media/golden/*.ffprobe.json
"""
import json
import subprocess
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
MEDIA = ROOT / "engine" / "ove-decode" / "tests" / "media"
GOLDEN = MEDIA / "golden"

COMMON = ["-bitexact", "-hide_banner", "-loglevel", "error", "-y"]


def run(cmd):
    # sandbox quirk: standalone ffmpeg gets killed after ~5s CPU; via python
    # subprocess it survives (documented in worklog, session 3, E-007b)
    p = subprocess.run(cmd, capture_output=True, text=True, timeout=120)
    if p.returncode != 0:
        print("CMD FAILED:", " ".join(map(str, cmd)), file=sys.stderr)
        print(p.stderr[-2000:], file=sys.stderr)
        sys.exit(1)
    return p


def gen_cfr24():
    # 8s @ 24fps, keyframe every second (g=24), timescale 24000 -> pts = n/24 exact
    out = MEDIA / "cfr24.mp4"
    run(["ffmpeg", *COMMON, "-f", "lavfi", "-i",
         "testsrc2=size=320x240:rate=24:duration=8",
         "-c:v", "mpeg4", "-q:v", "4", "-g", "24", "-keyint_min", "24",
         "-video_track_timescale", "24000", str(out)])
    return out


def gen_ntsc():
    # 3s @ 30000/1001, timescale 30000 -> pts = n*1001/30000 exact
    out = MEDIA / "ntsc.mp4"
    run(["ffmpeg", *COMMON, "-f", "lavfi", "-i",
         "testsrc2=size=320x240:rate=30000/1001:duration=3",
         "-c:v", "mpeg4", "-q:v", "4", "-g", "30", "-keyint_min", "30",
         "-video_track_timescale", "30000", str(out)])
    return out


def gen_vfr():
    # VFR: 90fps source, select keeps 10 frames whose input pts yield output
    # durations alternating 1/30 s and 1/15 s — every pts an exact multiple of
    # 1/90000 s (3000/6000 ticks). select preserves pts; -fps_mode vfr stops
    # the muxer resampling to CFR.
    out = MEDIA / "vfr.mp4"
    sel = "+".join(f"eq(n,{i})" for i in (0, 3, 9, 12, 18, 21, 27, 30, 36, 39))
    run(["ffmpeg", *COMMON, "-f", "lavfi", "-i",
         "testsrc2=size=160x120:rate=90:duration=0.5",
         "-vf", f"select='{sel}'", "-fps_mode", "vfr",
         "-c:v", "mpeg4", "-q:v", "4", "-g", "5", "-keyint_min", "5",
         "-video_track_timescale", "90000", str(out)])
    return out


def gen_vidaud():
    # video + AAC audio stream: negative test (audio stream open -> typed
    # Unsupported in v1; audio decode is Wave 7)
    out = MEDIA / "vidaud.mp4"
    run(["ffmpeg", *COMMON,
         "-f", "lavfi", "-i", "testsrc2=size=160x120:rate=24:duration=1",
         "-f", "lavfi", "-i", "sine=frequency=440:duration=1",
         "-c:v", "mpeg4", "-q:v", "4", "-g", "24",
         "-video_track_timescale", "24000",
         "-c:a", "aac", "-b:a", "64k", "-shortest", str(out)])
    return out


def corrupt_variants():
    # D-8 error-model corpus from cfr24.mp4
    src = (MEDIA / "cfr24.mp4").read_bytes()
    (MEDIA / "truncated.mp4").write_bytes(src[: int(len(src) * 0.4)])
    (MEDIA / "garbage.mp4").write_bytes(
        bytes((i * 31 + 7) % 256 for i in range(4096)))


def probe_tables():
    files = sorted(p for p in MEDIA.glob("*.mp4"))
    for f in files:
        # format + streams (corrupt files: record the expected failure itself)
        p = subprocess.run(
            ["ffprobe", "-v", "error", "-show_format", "-show_streams",
             "-of", "json", str(f)], capture_output=True, text=True)
        if p.returncode != 0:
            (GOLDEN / f"{f.stem}.ffprobe.json").write_text(json.dumps({
                "expected": "unreadable",
                "ffprobe_stderr": p.stderr.strip(),
            }, indent=1))
            continue
        (GOLDEN / f"{f.stem}.ffprobe.json").write_text(p.stdout)
        # packet-level truth: pts + key flags (container/index layer, D-3)
        p = run(["ffprobe", "-v", "error", "-select_streams", "v:0",
                 "-show_packets",
                 "-show_entries",
                 "packet=pts,pts_time,dts,duration,duration_time,flags,pos,size",
                 "-of", "json", str(f)])
        (GOLDEN / f"{f.stem}.packets.json").write_text(p.stdout)
        # frame-level table (D-1/D-2): pts, duration, key_frame, pict_type
        if f.name not in ("garbage.mp4", "truncated.mp4"):
            p = run(["ffprobe", "-v", "error", "-select_streams", "v:0",
                     "-show_frames",
                     "-show_entries",
                     "frame=pts,pts_time,duration,duration_time,key_frame,pict_type,width,height,pix_fmt",
                     "-of", "json", str(f)])
            (GOLDEN / f"{f.stem}.frames.json").write_text(p.stdout)


def main():
    MEDIA.mkdir(parents=True, exist_ok=True)
    GOLDEN.mkdir(parents=True, exist_ok=True)
    gen_cfr24()
    gen_ntsc()
    gen_vfr()
    gen_vidaud()
    corrupt_variants()
    probe_tables()
    for f in sorted(MEDIA.glob("*.mp4")):
        print(f"{f.name:16s} {f.stat().st_size:8d} bytes")
    print("golden tables:", len(list(GOLDEN.glob('*.json'))))


if __name__ == "__main__":
    main()
