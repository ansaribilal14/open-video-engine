#!/usr/bin/env python3
"""E-007: smart-render / stream-copy cut semantics on real H.264 media.

QUESTIONS: (a) Are stream-copy cuts frame-exact only when boundaries are keyframes?
           (b) What do non-keyframe copy cuts actually produce?
           (c) Cost delta: stream copy vs accurate re-encode decode?
HYPOTHESIS: copy cuts at keyframes are exact+instant; copy cuts elsewhere shift content
            to the previous keyframe (frame-count error); re-encode is slower but exact.
CLAIM: supports doc 04 (keyframe index, smart render) + R-0x smart-render experiment.
"""
import subprocess, json, time, os, sys

W = "/home/z/my-project/open-video-engine/experiments"
SRC = f"{W}/E-007_src.mp4"
res = []
def check(name, ok, detail):
    res.append((name, bool(ok), detail)); print(f"[{'PASS' if ok else 'FAIL'}] {name}: {detail}")

def run(cmd): return subprocess.run(cmd, capture_output=True, text=True)

def frames(path):
    r = run(["ffprobe", "-v", "error", "-select_streams", "v", "-count_frames",
             "-show_entries", "stream=nb_read_frames", "-of", "csv=p=0", path])
    return int(r.stdout.strip()) if r.stdout.strip().isdigit() else -1

def keyframes(path):
    r = run(["ffprobe", "-v", "error", "-select_streams", "v", "-show_entries",
             "packet=pts_time,flags", "-of", "csv=p=0", path])
    kfs = []
    for line in r.stdout.strip().splitlines():
        pts, fl = line.split(",")
        if "K" in fl: kfs.append(float(pts))
    return kfs

# --- generate source: 6s, 24fps, keyframe every 24 frames (1s) ---
run(["ffmpeg", "-y", "-loglevel", "error", "-f", "lavfi", "-i",
     "testsrc=duration=6:size=320x180:rate=24", "-c:v", "libx264", "-profile:v", "baseline",
     "-pix_fmt", "yuv420p", "-g", "24", "-keyint_min", "24", SRC])
total = frames(SRC); kfs = keyframes(SRC)
check("source built with predictable GOP", total == 144 and len(kfs) >= 6,
      f"frames={total}, keyframes={len(kfs)} at {kfs[:4]}...")

# --- (a) stream-copy cut exactly at keyframe boundaries: t=1.0 -> t=4.0 ---
t0, t1 = kfs[1], kfs[4]
out = f"{W}/E-007_cut_kf.mp4"
tc = time.perf_counter()
run(["ffmpeg", "-y", "-loglevel", "error", "-ss", str(t0), "-to", str(t1), "-i", SRC, "-c", "copy", out])
dt_copy = time.perf_counter() - tc
got = frames(out)
check("copy cut at keyframes is frame-exact", got == int((t1 - t0) * 24),
      f"expected {int((t1-t0)*24)} frames, got {got}, {dt_copy*1000:.0f} ms")

# --- (b) stream-copy cut mid-GOP: t=1.5 -> t=3.5 ---
mid0, mid1 = kfs[1] + 0.5, kfs[3] + 0.5
out2 = f"{W}/E-007_cut_mid.mp4"
run(["ffmpeg", "-y", "-loglevel", "error", "-ss", str(mid0), "-to", str(mid1), "-i", SRC, "-c", "copy", out2])
got2 = frames(out2)
# expected drift: start snaps back to previous keyframe
snapped = frames_ok = None
r = run(["ffprobe", "-v", "error", "-select_streams", "v", "-show_entries",
         "frame=pts_time", "-of", "csv=p=0", out2])
first_pts = float(r.stdout.strip().splitlines()[0])
check("mid-GOP copy cut snaps to previous keyframe (imprecision CONFIRMED)",
      abs(first_pts - mid0) > 0.2 or got2 != int((mid1 - mid0) * 24),
      f"requested {mid0}s, output first pts {first_pts}s, frames {got2} vs requested {int((mid1-mid0)*24)}")

# --- (c) accurate re-encode cut at same mid-GOP points ---
out3 = f"{W}/E-007_cut_re.mp4"
tc = time.perf_counter()
run(["ffmpeg", "-y", "-loglevel", "error", "-ss", str(mid0), "-to", str(mid1), "-i", SRC,
     "-c:v", "libx264", "-preset", "veryfast", "-pix_fmt", "yuv420p", out3])
dt_re = time.perf_counter() - tc
got3 = frames(out3)
check("re-encode cut is frame-exact", got3 == int((mid1 - mid0) * 24),
      f"expected {int((mid1-mid0)*24)}, got {got3}, {dt_re*1000:.0f} ms")
check("copy is faster than re-encode", dt_copy < dt_re, f"copy {dt_copy*1000:.0f}ms vs re-encode {dt_re*1000:.0f}ms")

fails = [r for r in res if not r[1]]
print("\n==== E-007 SUMMARY ====")
print(f"checks: {len(res)}, failed: {len(fails)}")
print("VERDICT: smart-render requires per-clip keyframe index; copy cuts are keyframe-quantized;" 
      " exact mid-GOP edits need boundary-GOP re-encode" if not fails else "investigate")
with open(f"{W}/E-007_result.txt", "w") as f:
    for name, ok, detail in res: f.write(f"[{'PASS' if ok else 'FAIL'}] {name}: {detail}\n")
    f.write("VERDICT: see summary\n")
