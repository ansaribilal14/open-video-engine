#!/usr/bin/env bash
# REAL-WORLD REFERENCE MEDIA acquisition — the permanent OVE proof workflow.
# See docs/REALWORLD_VALIDATION.md. The download goes through ytagent
# (Bilal140202/ytagent) — OVE never grows its own downloader.
#
# Usage: acquire_source.sh <youtube-url-or-id> <out-dir>
#
# Environment:
#   YTAGENT_GITHUB_TOKEN  optional; enables ytagent's github_actions_farm
#                         method when direct methods are blocked or
#                         byte-wrong. NEVER written to disk or committed.
#   FARM_OWNER/FARM_REPO  optional personal deployment of ytagent's
#                         yt-download-farm.yml workflow (see the doc).
#   EXPECTED_SHA256       optional identity gate; defaults to the §2
#                         certified reference identity (RLW-1..6 trail).
#
# HARDENED (RLW-6, 2026-09-30): the identity gate lives INSIDE this
# script now. History that forced this: ytagent's method set grew a
# cobalt fallback that returns a RE-MUXED variant (real footage, WRONG
# bytes — `d7e3019a…` ≠ `2d315daf…`), and its output naming changed
# (`<videoid>.mp4`, not `video_*`). A tier-1 "success" therefore used to
# defeat both the gate (which ran nowhere) and the normalizer (which
# globbed video_*). Now: every tier result is normalized, gated against
# EXPECTED_SHA256, and a mismatch ESCALATES to the next tier instead of
# being accepted. The script only succeeds on the certified identity.
set -euo pipefail

URL="${1:?youtube url required}"
OUT="${2:?out dir required}"
mkdir -p "$OUT"

EXPECTED_SHA256="${EXPECTED_SHA256:-2d315daf6130366d9a98c9f49716aa263036ba5fef8f241cefff727bc3b4705f}"

# Normalize whatever the current tier produced into $OUT/source.mp4
# (stream copy only; never re-encode) and print its sha256.
# LOUD-FAILURE RULE (RLW-6): incomplete DASH parts (video without audio
# or vice versa) are an ERROR, never a silent video-only remux — the
# certified reference is a merged A/V artifact and the identity gate
# deserves an honest error message.
normalize_and_hash() {
  cd "$OUT"
  local -a dash_v=() dash_a=() singles=()
  local p fp=""
  # 1st choice: what ytagent itself says it produced (any naming scheme)
  if [ -f ytagent_result.json ]; then
    fp=$(python3 -c '
import json,sys
try:
    d=json.load(open("ytagent_result.json"))
    p=d.get("final_path") or ""
    print(p if p else "")
except Exception:
    print("")' 2>/dev/null || true)
  fi
  # classify candidates: DASH parts vs single files
  for p in video_*; do
    [ -e "$p" ] || continue
    case "$p" in
      *f[0-9]*.mp4) dash_v+=("$p") ;;
      *.m4a)        dash_a+=("$p") ;;
      *)            singles+=("$p") ;;
    esac
  done
  [ -n "$fp" ] && [ -f "$fp" ] && singles+=("$fp")

  if [ "${#dash_v[@]}" -gt 0 ]; then
    if [ "${#dash_a[@]}" -eq 0 ]; then
      echo "INCOMPLETE_DASH_PARTS: video part(s) ${dash_v[*]} but NO audio part — refusing to produce a video-only source (re-run to dispatch a fresh farm attempt)" >&2
      return 1
    fi
    ffmpeg -y -v error -i "${dash_v[0]}" -i "${dash_a[0]}" -c copy -movflags +faststart source.mp4
  elif [ -e "${singles[0]:-}" ]; then
    cp -f "${singles[0]}" source.mp4
  else
    echo "NO_CANDIDATE_FILE" >&2
    return 1
  fi
  # belt & suspenders: the normalized file must carry BOTH streams
  local v a
  v=$(ffprobe -v error -select_streams v:0 -show_entries stream=codec_name -of csv=p=0 source.mp4 2>/dev/null || true)
  a=$(ffprobe -v error -select_streams a:0 -show_entries stream=codec_name -of csv=p=0 source.mp4 2>/dev/null || true)
  if [ -z "$v" ] || [ -z "$a" ]; then
    echo "NORMALIZED_FILE_INCOMPLETE: video='$v' audio='$a' — refusing" >&2
    return 1
  fi
  sha256sum source.mp4 | cut -d' ' -f1
}

gate_check() { # $1 = actual sha256
  [ "$1" = "$EXPECTED_SHA256" ]
}

run_farm_tier() {
  [ -n "${YTAGENT_GITHUB_TOKEN:-}" ] || { echo "No YTAGENT_GITHUB_TOKEN; farm tier unavailable" >&2; return 1; }
  echo "== Tier 2: ytagent github_actions_farm =="
  local VID
  VID="${URL##*=}"; VID="${VID: -11}"
  FARM_OWNER="${FARM_OWNER:-}" FARM_REPO="${FARM_REPO:-}" \
  YTAGENT_GITHUB_TOKEN="$YTAGENT_GITHUB_TOKEN" \
  python3 - "$VID" "$OUT" << 'PYEOF'
import os, sys
from pathlib import Path
for sp in ("/home/z/.local/lib/python3.13/site-packages",):
    if sp not in sys.path:
        sys.path.insert(0, sp)
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
}

# Complete the farm artifact fetch-back. UPSTREAM DEFECT (found by the
# RLW-6 gate, 2026-09-30): ytagent's `_download_artifact` extracts only
# the FIRST media file in the artifact zip and breaks — DASH artifacts
# (f136.mp4 + f140.m4a) come back video-only. The artifact itself is
# complete (verified: zip listing carries both parts). This step fetches
# the latest completed farm run's artifact directly and extracts ALL
# media parts. It is artifact fetch-back only — the YouTube download
# itself stays in the farm/ytagent mechanism (OVE grows no downloader).
complete_farm_artifact() {
  [ -n "${YTAGENT_GITHUB_TOKEN:-}" ] || return 1
  local OWNER="${FARM_OWNER:-Bilal140202}" REPO="${FARM_REPO:-ytagent}"
  python3 - "$OWNER" "$REPO" "$OUT" << 'PYEOF'
import os, sys, time, zipfile
from pathlib import Path
import urllib3
urllib3.disable_warnings()
import requests

owner, repo, out = sys.argv[1], sys.argv[2], Path(sys.argv[3])
token = os.environ["YTAGENT_GITHUB_TOKEN"]
H = {"Authorization": f"token {token}", "Accept": "application/vnd.github+json"}
S = requests.Session(); S.verify = False

def get(path, **kw):
    return S.get(f"https://api.github.com{path}", headers=H, timeout=60, **kw)

runs = get(f"/repos/{owner}/{repo}/actions/workflows/yt-download-farm.yml/runs?per_page=5").json()
run = next((r for r in runs.get("workflow_runs", [])
            if r["status"] == "completed" and r["conclusion"] == "success"), None)
if run is None:
    print("complete_farm: no completed successful run", file=sys.stderr); sys.exit(1)
arts = get(f"/repos/{owner}/{repo}/actions/runs/{run['id']}/artifacts").json().get("artifacts", [])
if not arts:
    print("complete_farm: run has no artifacts", file=sys.stderr); sys.exit(1)
r = get(f"/repos/{owner}/{repo}/actions/artifacts/{arts[0]['id']}/zip")
zf = zipfile.ZipFile(__import__("io").BytesIO(r.content))
out.mkdir(parents=True, exist_ok=True)
n = 0
for name in zf.namelist():
    if name.endswith((".mp4", ".m4a", ".webm", ".mkv")):
        (out / Path(name).name).write_bytes(zf.read(name)); n += 1
print(f"complete_farm: extracted {n} media parts from run {run['id']} artifact")
sys.exit(0 if n else 1)
PYEOF
}

ACTUAL=""

echo "== Tier 1: ytagent download chain =="
if ytagent download "$URL" --out-dir "$OUT" --json > "$OUT/ytagent_result.json" 2> "$OUT/ytagent.log"; then
  echo "ytagent chain succeeded; see ytagent_result.json"
  if ACTUAL=$(normalize_and_hash); then
    if gate_check "$ACTUAL"; then
      echo "IDENTITY GATE: PASS (tier 1 bytes are the certified reference)"
    else
      echo "IDENTITY GATE: FAIL — tier 1 returned $ACTUAL, expected $EXPECTED_SHA256" >&2
      echo "  (a byte-wrong variant — likely a re-mux fallback method). Escalating." >&2
      ACTUAL=""
    fi
  fi
else
  echo "ytagent chain exhausted (see ytagent.log)"
fi

if [ -z "$ACTUAL" ]; then
  if ! run_farm_tier; then echo "farm tier failed" >&2; exit 1; fi
  # ytagent extraction defect: complete the fetch-back if parts are missing
  if ! ACTUAL=$(normalize_and_hash); then
    echo "== Completing farm artifact fetch-back (ytagent extracts only the first media part) =="
    complete_farm_artifact || { echo "artifact completion failed" >&2; exit 2; }
    ACTUAL=$(normalize_and_hash) || { echo "normalization failed after completion" >&2; exit 2; }
  fi
  if gate_check "$ACTUAL"; then
    echo "IDENTITY GATE: PASS (farm bytes are the certified reference)"
  else
    echo "IDENTITY GATE: FAIL — farm returned $ACTUAL, expected $EXPECTED_SHA256" >&2
    exit 2
  fi
fi

echo "== Independent baseline (ffprobe authority) =="
cd "$OUT"
sha256sum source.mp4 | tee identity_sha256.txt
ffprobe -v error -show_format -show_streams -of json source.mp4 > baseline_streams.json
echo "ACQUISITION COMPLETE: $OUT/source.mp4 (identity gate PASS)"
