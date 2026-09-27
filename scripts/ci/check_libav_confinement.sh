#!/bin/sh
# DECODER_SPEC §5 gate: ZERO libav* linkage outside engine/ove-decode.
# (a) dependency graph: no package other than ove-decode may depend on
#     ffmpeg-sys-next (cargo metadata — deterministic, no compilation)
# (b) source scan: no ffmpeg_sys_next usage outside ove-decode sources
set -e
cd "$(dirname "$0")/../../engine"

echo "== (a) dependency graph check (cargo metadata)"
python3 - <<'EOF'
import json, subprocess, sys
meta = json.loads(subprocess.check_output(
    ["cargo", "metadata", "--format-version", "1", "--no-deps"]))
pkgs = meta["packages"]
if not any(p["name"] == "ove-decode" for p in pkgs):
    print("FAIL: ove-decode not a workspace member"); sys.exit(1)
# full metadata (with deps) for the reverse-dependency rule
full = json.loads(subprocess.check_output(
    ["cargo", "metadata", "--format-version", "1"]))
viol = []
for p in full["packages"]:
    if p["name"] == "ffmpeg-sys-next":
        continue
    for d in p["dependencies"]:
        if d["name"] == "ffmpeg-sys-next" and p["name"] != "ove-decode":
            viol.append(f"{p['name']} depends on ffmpeg-sys-next")
if viol:
    print("VIOLATION: libav* reachable outside ove-decode:")
    for v in viol:
        print("  -", v)
    sys.exit(1)
print("dependency graph: ffmpeg-sys-next reached only via ove-decode")
EOF

echo "== (b) source scan outside engine/ove-decode"
VIOL=$(grep -rln "ffmpeg_sys_next" --include='*.rs' --include='*.toml' . 2>/dev/null | grep -v './ove-decode/' || true)
if [ -n "$VIOL" ]; then
  echo "VIOLATION: libav references outside ove-decode:"; echo "$VIOL"; exit 1
fi

echo "libav confinement: PASS (libav* linkage confined to engine/ove-decode)"
