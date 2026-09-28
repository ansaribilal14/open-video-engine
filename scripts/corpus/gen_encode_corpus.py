#!/usr/bin/env python3
"""Wave 3 (encode leg) conformance corpus generator.

Generates the committed fixtures the ove-encode conformance suite needs for
the STREAM-COPY routes (ENCODER_SPEC §3). Codec choice mirrors the Wave 2
decode corpus: MPEG-4 Part 2 (`-c:v mpeg4`, libavcodec-native, LGPL-safe)
plus native AAC for the audio grid. Exact per-track timescales so every pts
is an exactly-representable rational (ADR-007).

Outputs:
    engine/ove-encode/tests/media/copy24.mp4        6s @ 24fps video, kf=1s
    engine/ove-encode/tests/media/copyav.mp4        6s @ 24fps video + AAC 48k
    engine/ove-encode/tests/media/copyntsc.mp4      30000/1001 exact
    engine/ove-encode/tests/media/golden/*.json     committed ffprobe tables

The RE-ENCODE route needs no fixture (synthetic frames are generated in the
test itself, mirroring the render goldens); its byte goldens are keyed on
the producing libav identity (see gen_encode_golden / conformance notes).

Run from repo root:  python3 scripts/corpus/gen_encode_corpus.py
"""
import json
import subprocess
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
MEDIA = ROOT / "engine" / "ove-encode" / "tests" / "media"
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


def gen_copy24():
    # 6s @ 24fps, keyframe every second (g=24), timescale 24000
    out = MEDIA / "copy24.mp4"
    run(["ffmpeg", *COMMON, "-f", "lavfi", "-i",
         "testsrc2=size=320x240:rate=24:duration=6",
         "-c:v", "mpeg4", "-g", "24", "-pix_fmt", "yuv420p",
         "-video_track_timescale", "24000", str(out)])
    return out


def gen_copyav():
    # video as copy24 + AAC 48k stereo; audio shorter than video on purpose
    # (checks per-track end handling)
    out = MEDIA / "copyav.mp4"
    run(["ffmpeg", *COMMON,
         "-f", "lavfi", "-i", "testsrc2=size=320x240:rate=24:duration=6",
         "-f", "lavfi", "-i", "sine=frequency=440:sample_rate=48000:duration=5",
         "-c:v", "mpeg4", "-g", "24", "-pix_fmt", "yuv420p",
         "-video_track_timescale", "24000",
         "-c:a", "aac", "-b:a", "96k", "-shortest", str(out)])
    return out


def gen_copyntsc():
    # 4s @ 30000/1001, keyframe every 30 frames, timescale 30000
    out = MEDIA / "copyntsc.mp4"
    run(["ffmpeg", *COMMON, "-f", "lavfi", "-i",
         "testsrc2=size=320x240:rate=30000/1001:duration=4",
         "-c:v", "mpeg4", "-g", "30", "-pix_fmt", "yuv420p",
         "-video_track_timescale", "30000", str(out)])
    return out


def probe(out_name, src):
    """Commit ffprobe -show_streams -show_packets JSON as the golden table."""
    p = run(["ffprobe", "-v", "error", "-show_streams", "-show_packets",
             "-of", "json", str(src)])
    (GOLDEN / f"{out_name}.ffprobe.json").write_text(p.stdout)
    return p.stdout


def main():
    MEDIA.mkdir(parents=True, exist_ok=True)
    GOLDEN.mkdir(parents=True, exist_ok=True)
    for name, gen in [("copy24", gen_copy24), ("copyav", gen_copyav),
                      ("copyntsc", gen_copyntsc)]:
        src = gen()
        probe(name, src)
        print(f"generated {name}: {src.stat().st_size} bytes")
    print("corpus complete")


if __name__ == "__main__":
    main()
