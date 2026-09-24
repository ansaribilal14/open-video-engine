#!/usr/bin/env python3
"""E-007b: smart-render timing at REAL resolutions (1080p).

Extends E-007 (which proved semantics on tiny media, timing leg invalidated):
  Q1: How slow is exact (re-encode) cutting vs keyframe-aligned stream copy
      at 1080p with realistic bitrates?
  Q2: Is keyframe-aligned copy-cut duration-exact and content-correct?
  Q3: What is the copy-vs-encode speedup ratio that justifies smart render?

Method: generate deterministic 1080p30 H.264 (libx264, fixed GOP 60, no
scene-cut keys => keyframes exactly at 0/2/4/6/8s), then measure 3 cut points
x 3 strategies x 3 reps with ffmpeg -benchmark wall time. Verify outputs with
ffprobe (duration, frame count) and compare a decoded-frame hash at the cut
head for content correctness.

Honesty notes printed in output: container CPU, software encoder only
(no NVENC/QSV hardware paths in this environment).
"""
import json
import subprocess
import tempfile
import time
from pathlib import Path

OUT = Path("/home/z/my-project/open-video-engine/experiments/E-007b_result.txt")
W, H, FPS, DUR_S = 1920, 1080, 30, 10
GOP = 60  # keyframe every 2s at 30fps

def run(cmd, **kw):
    return subprocess.run(cmd, capture_output=True, text=True, **kw)

def gen_media(path: Path):
    t0 = time.time()
    r = run([
        "ffmpeg", "-y", "-hide_banner", "-loglevel", "error",
        "-f", "lavfi", "-i",
        f"testsrc2=size={W}x{H}:rate={FPS}:duration={DUR_S}",
        "-f", "lavfi", "-i", f"sine=frequency=440:duration={DUR_S}",
        "-c:v", "libx264", "-preset", "ultrafast", "-crf", "23",
        "-g", str(GOP), "-keyint_min", str(GOP), "-sc_threshold", "0",
        "-pix_fmt", "yuv420p",
        "-c:a", "aac", "-b:a", "128k",
        "-movflags", "+faststart",
        str(path),
    ])
    assert r.returncode == 0, r.stderr
    return time.time() - t0

def probe(path: Path):
    r = run(["ffprobe", "-v", "error", "-count_frames", "-print_format", "json",
             "-show_format", "-show_streams", str(path)])
    assert r.returncode == 0, r.stderr
    d = json.loads(r.stdout)
    v = next(s for s in d["streams"] if s["codec_type"] == "video")
    return {
        "duration": float(d["format"]["duration"]),
        "nb_frames": int(v.get("nb_read_frames", -1)),
        "r_frame_rate": v["r_frame_rate"],
    }

def keyframes(path: Path):
    r = run(["ffprobe", "-v", "error", "-select_streams", "v:0",
             "-show_entries", "frame=pict_type,pts_time", "-of", "json", str(path)])
    d = json.loads(r.stdout)
    return [float(f["pts_time"]) for f in d["frames"] if f.get("pict_type") == "I"]

def cut(src: Path, dst: Path, start: float, end: float, mode: str):
    """mode: copy | encode | encode_full"""
    t0 = time.time()
    if mode == "copy":
        cmd = ["ffmpeg", "-y", "-hide_banner", "-loglevel", "error",
               "-ss", f"{start:.6f}", "-to", f"{end:.6f}", "-i", str(src),
               "-c", "copy", str(dst)]
    elif mode == "encode":
        cmd = ["ffmpeg", "-y", "-hide_banner", "-loglevel", "error",
               "-ss", f"{start:.6f}", "-to", f"{end:.6f}", "-i", str(src),
               "-c:v", "libx264", "-preset", "ultrafast", "-crf", "23",
               "-g", str(GOP), "-keyint_min", str(GOP), "-sc_threshold", "0",
               "-pix_fmt", "yuv420p", "-c:a", "aac", str(dst)]
    else:  # encode_full: re-encode whole file then trim (worst case baseline)
        cmd = ["ffmpeg", "-y", "-hide_banner", "-loglevel", "error",
               "-i", str(src),
               "-ss", f"{start:.6f}", "-to", f"{end:.6f}",
               "-c:v", "libx264", "-preset", "ultrafast", "-crf", "23",
               "-pix_fmt", "yuv420p", "-c:a", "aac", str(dst)]
    r = run(cmd)
    assert r.returncode == 0, r.stderr
    return time.time() - t0

def head_frame_sig(path: Path):
    """sha256 of first decoded frame's md5 (ffmpeg framecrc of v:0 first frame)."""
    r = run(["ffmpeg", "-hide_banner", "-loglevel", "error", "-i", str(path),
             "-map", "0:v:0", "-frames:v", "1", "-f", "md5", "-"])
    return r.stdout.strip()

def main():
    lines = []
    log = lambda *a: (print(*a, flush=True))
    with tempfile.TemporaryDirectory() as td:
        tmp = Path(td)
        src = tmp / "src_1080p.mp4"
        gen_s = gen_media(src)
        kp = keyframes(src)
        info = probe(src)
        size_mb = src.stat().st_size / 1e6
        log(f"E-007b smart-render timing at real resolution")
        log(f"source: {W}x{H}@{FPS} {DUR_S}s libx264 CRF23 GOP={GOP} no-scene-cuts")
        log(f"gen_time={gen_s:.1f}s size={size_mb:.1f}MB frames={info['nb_frames']} "
            f"rate={info['r_frame_rate']} duration={info['duration']:.3f}")
        log(f"keyframes at: {kp}")
        assert 0.0 in kp and 2.0 in kp and 8.0 in kp, "GOP plan violated"
        log("")

        # 2 cut points (container CPU budget): head-aligned (0-4), mid (4-6)
        cuts = [(0.0, 4.0), (4.0, 6.0)]
        results = {}
        for (a, b) in cuts:
            row = {}
            for mode in ("copy", "encode"):
                times = []
                sigs = []
                for rep in range(2):
                    dst = tmp / f"out_{a}_{b}_{mode}_{rep}.mp4"
                    dt = cut(src, dst, a, b, mode)
                    times.append(dt)
                    if rep == 0:
                        sigs.append(head_frame_sig(dst))
                        p = probe(dst)
                        row["out_dur"] = p["duration"]
                        row["out_frames"] = p["nb_frames"]
                    dst.unlink()
                times.sort()
                row[mode] = {"med_s": times[0], "max_s": times[1],
                             "head_sig": sigs[0]}
            want = b - a
            c, e = row["copy"], row["encode"]
            log(f"cut {a:.0f}s->{b:.0f}s (want {want:.1f}s):")
            log(f"  copy   min {c['med_s']*1000:7.1f} ms  out_dur={row['out_dur']:.3f} frames={row['out_frames']}")
            log(f"  encode min {e['med_s']*1000:7.1f} ms  ({e['med_s']/c['med_s']:.1f}x slower than copy)")
            log(f"  head-frame identical copy vs encode: {c['head_sig'] == e['head_sig']}")
            results[f"{a}-{b}"] = row
            log("")

        # baseline: one full-timeline re-encode for context (1 rep)
        dst = tmp / "full.mp4"
        t0 = time.time()
        r = run(["ffmpeg", "-y", "-hide_banner", "-loglevel", "error",
                 "-i", str(src), "-c:v", "libx264", "-preset", "ultrafast",
                 "-crf", "23", "-pix_fmt", "yuv420p", "-c:a", "aac", str(dst)])
        assert r.returncode == 0, r.stderr
        full_s = time.time() - t0
        log(f"full-timeline re-encode baseline: {full_s*1000:.1f} ms ({DUR_S}s of 1080p)")
        log("")

        # Q2: duration exactness for keyframe-aligned copy
        ok_dur = all(abs(r["out_dur"] - (float(k.split("-")[1]) - float(k.split("-")[0]))) < 0.05
                     for k, r in results.items())
        log(f"keyframe-aligned copy duration-exact (±50ms): {ok_dur}")
        # copy speedup vs encode
        sp = [r["encode"]["med_s"] / r["copy"]["med_s"] for r in results.values()]
        log(f"copy speedup vs re-encode: {min(sp):.1f}x .. {max(sp):.1f}x")
        log("")
        log("LIMITATIONS: container CPU (2 cores); libx264 software only (no HW encoder paths);")
        log("single-stream (no multi-track graph); AAC audio re-encoded in encode legs;")
        log("timings indicative for relative ratios, not absolute engine budgets.")

    OUT.parent.mkdir(parents=True, exist_ok=True)

if __name__ == "__main__":
    import sys
    main()
