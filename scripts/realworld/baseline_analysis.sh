#!/usr/bin/env bash
# Independent ffprobe baseline for the real-world proof
# (REALWORLD_VALIDATION §3/§9). OVE is never the authority: every file
# produced here comes from ffprobe/ffmpeg directly.
#
# Produces the five inputs consumed by baseline_facts.py:
#   baseline_streams.json   format + stream metadata
#   baseline_vcount.json    decoded video frame count  (-count_frames)
#   baseline_acount.json    decoded audio sample count (-count_samples)
#   baseline_keyframes.txt  packet-level keyframe pts_time list (whitespace-separated)
#   decode_errors.txt       full-decode health (-xerror) + DECODE_CLEAN marker
#   identity_sha256.txt     source identity gate
#
# Usage: baseline_analysis.sh <source.mp4> <out-dir>
set -euo pipefail

SRC="${1:?source mp4 required}"
OUT="${2:?out dir required}"
mkdir -p "$OUT"

echo "== streams =="
ffprobe -v error -show_format -show_streams -of json "$SRC" > "$OUT/baseline_streams.json"

echo "== decoded frame count (video) =="
ffprobe -v error -count_frames -select_streams v:0 -show_streams -of json "$SRC" > "$OUT/baseline_vcount.json"

echo "== decoded sample count (audio) =="
# ffprobe has no -count_samples; the exact sample count comes from a FULL
# independent PCM decode (ffmpeg, s16le) — bytes / (2 * channels). Emitted
# in the ffprobe stream shape so baseline_facts.py needs no branching.
ffmpeg -v error -i "$SRC" -map 0:a:0 -f s16le - 2>/dev/null | wc -c \
  | python3 -c '
import json, sys
# channel count from the ffprobe stream dump (already written above)
streams = json.load(open("'"$OUT"'/baseline_streams.json"))["streams"]
ch = int([s for s in streams if s.get("codec_type") == "audio"][0]["channels"])
raw = sys.stdin.read().strip()
assert raw.isdigit(), f"PCM byte count not numeric: {raw!r}"
samples = int(raw) // (2 * ch)
assert int(raw) % (2 * ch) == 0, f"PCM bytes not whole samples: {raw} / {2*ch}"
json.dump({"streams": [{"nb_read_samples": str(samples)}]}, open("'"$OUT"'/baseline_acount.json", "w"))
print(f"audio samples (decoded): {samples}")
'

echo "== keyframe truth (packet flags) =="
ffprobe -v error -select_streams v:0 -show_entries packet=pts_time,flags -of csv=p=0 "$SRC" \
  | awk -F, 'BEGIN{ORS=" "} $2 ~ /K/ {print $1}' > "$OUT/baseline_keyframes.txt"

echo "== decode health (-xerror) =="
if ffmpeg -v error -xerror -i "$SRC" -f null - 2> "$OUT/decode_errors.txt"; then
  echo "DECODE_CLEAN" >> "$OUT/decode_errors.txt"
  echo "decode: CLEAN"
else
  echo "decode: ERRORS (see decode_errors.txt)"
fi

echo "== identity =="
sha256sum "$SRC" | tee "$OUT/identity_sha256.txt"
echo "BASELINE COMPLETE: $OUT"
