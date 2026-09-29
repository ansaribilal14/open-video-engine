#!/usr/bin/env bash
# REAL-WORLD REFERENCE MEDIA acquisition — the permanent OVE proof workflow.
# See docs/REALWORLD_VALIDATION.md. The download goes through ytagent
# (bilal140202/ytagent) — OVE never grows its own downloader.
#
# Usage: acquire_source.sh <youtube-url-or-id> <out-dir>
#
# Environment:
#   YTAGENT_GITHUB_TOKEN  optional; enables ytagent's github_actions_farm
#                         method (Tier 13) when direct methods are blocked.
#                         NEVER written to disk or committed.
#   FARM_OWNER/FARM_REPO  optional personal deployment of ytagent's
#                         yt-download-farm.yml workflow (see the doc).
#
# Acquisition tiers observed working (2026-09-29, datacenter IP):
#   1. `ytagent download <url>` — the 13-method fallback chain (first try).
#   2. github_actions_farm via a personal deployment of ytagent's
#      yt-download-farm.yml workflow (WARP + android_vr + manual GVS PO token
#      from the BGutil server). The farm run downloads on GitHub runners and
#      returns the artifact; ytagent's own method fetches it back.
# Either way the result is verified by ytagent (ffprobe + magic bytes) and
# then re-verified here independently.
set -euo pipefail

URL="${1:?youtube url required}"
OUT="${2:?out dir required}"
mkdir -p "$OUT"

echo "== Tier 1: ytagent download chain =="
if ytagent download "$URL" --out-dir "$OUT" --json > "$OUT/ytagent_result.json" 2> "$OUT/ytagent.log"; then
  echo "ytagent chain succeeded; see ytagent_result.json"
else
  echo "ytagent chain exhausted (see ytagent.log)"
  if [ -n "${YTAGENT_GITHUB_TOKEN:-}" ]; then
    echo "== Tier 2: ytagent github_actions_farm =="
    VID="${URL##*=}"; VID="${VID: -11}"
    FARM_OWNER="$FARM_OWNER" FARM_REPO="$FARM_REPO" \
    YTAGENT_GITHUB_TOKEN="$YTAGENT_GITHUB_TOKEN" \
    python3 - "$VID" "$OUT" << 'PYEOF'
import os, sys
from pathlib import Path
sys.path.insert(0, "/home/z/.local/lib/python3.13/site-packages")
from ytagent.methods.github_actions_farm import download
vid, out = sys.argv[1], Path(sys.argv[2])
opts = {"github_token": os.environ["YTAGENT_GITHUB_TOKEN"],
        "github_wait_timeout": 900}
for k in ("FARM_OWNER", "FARM_REPO"):
    if os.environ.get(k):
        opts["github_owner" if k == "FARM_OWNER" else "github_repo"] = os.environ[k]
r = download(vid, out, opts=opts)
print("FARM_OK:", r.ok, getattr(r, "path", ""), r.reason)
sys.exit(0 if r.ok else 1)
PYEOF
  else
    echo "No YTAGENT_GITHUB_TOKEN set; cannot run the farm tier." >&2
    exit 1
  fi
fi

echo "== Normalize to one MP4 (stream copy only; no re-encode) =="
cd "$OUT"
PARTS=$(ls video_* 2>/dev/null || true)
if echo "$PARTS" | grep -q "f[0-9]*\.mp4"; then
  # DASH parts came back unmerged (runner ffmpeg absent): merge with -c copy
  V=$(echo "$PARTS" | grep "f[0-9]*\.mp4" | head -1)
  A=$(echo "$PARTS" | grep "\.m4a" | head -1 || true)
  ffmpeg -y -v error -i "$V" ${A:+-i "$A"} -c copy -movflags +faststart source.mp4
else
  SRC=$(echo "$PARTS" | head -1)
  cp "$SRC" source.mp4
fi

echo "== Identity + independent baseline =="
sha256sum source.mp4 | tee identity_sha256.txt
ffprobe -v error -show_format -show_streams -of json source.mp4 > baseline_streams.json
echo "ACQUISITION COMPLETE: $OUT/source.mp4"
