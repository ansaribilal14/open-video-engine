#!/bin/sh
# DECODER_SPEC §5 + ENCODER_SPEC §5 gate (ADR-015): libav* linkage is confined
# to the NAMED ADAPTER CRATES — currently {ove-decode, ove-encode}. Every core
# crate (time/timeline/media/render/project/engine/cli) must stay libav-free.
# (a) dependency graph: only adapter crates may depend on ffmpeg-sys-next
#     (cargo metadata — deterministic, no compilation)
# (b) source scan: no ffmpeg_sys_next usage outside adapter crate sources
#
# History: through Wave 2 the allowlist was {ove-decode} only. Wave 3 (ADR-015)
# added ove-encode — the encode/mux adapter crate — keeping the INVARIANT that
# matters: libav never leaks into core. The allowlist is explicit and closed;
# adding a crate to it requires an ADR.
set -e
cd "$(dirname "$0")/../../engine"

ADAPTERS="ove-decode ove-encode"

echo "== (a) dependency graph check (cargo metadata)"
python3 - "$ADAPTERS" <<'EOF'
import json, subprocess, sys
adapters = set(sys.argv[1].split())
meta = json.loads(subprocess.check_output(
    ["cargo", "metadata", "--format-version", "1", "--no-deps"]))
pkgs = {p["name"] for p in meta["packages"]}
missing = adapters - pkgs
if missing:
    print(f"FAIL: adapter crate(s) {sorted(missing)} not workspace members")
    sys.exit(1)
# full metadata (with deps) for the reverse-dependency rule
full = json.loads(subprocess.check_output(
    ["cargo", "metadata", "--format-version", "1"]))
viol = []
for p in full["packages"]:
    if p["name"] == "ffmpeg-sys-next":
        continue
    for d in p["dependencies"]:
        if d["name"] == "ffmpeg-sys-next" and p["name"] not in adapters:
            viol.append(f"{p['name']} depends on ffmpeg-sys-next")
if viol:
    print(f"VIOLATION: libav* reachable outside adapter crates {sorted(adapters)}:")
    for v in viol:
        print("  -", v)
    sys.exit(1)
print(f"dependency graph: ffmpeg-sys-next reached only via {sorted(adapters)}")
EOF

EXCLUDE=$(for a in $ADAPTERS; do printf -- "-e ./%s/ " "$a"; done)

echo "== (b) source scan outside adapter crates"
VIOL=$(grep -rln "ffmpeg_sys_next" --include='*.rs' --include='*.toml' . 2>/dev/null \
  | grep -v $EXCLUDE || true)
if [ -n "$VIOL" ]; then
  echo "VIOLATION: libav references outside adapter crates ($ADAPTERS):"
  echo "$VIOL"; exit 1
fi

echo "libav confinement: PASS (libav* linkage confined to adapter crates: $ADAPTERS)"
