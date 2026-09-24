# W3-c REPORT — wave-3 research agent (docs 28 PLUGINS, 29 SCRIPTING)

Task ID: 14-c · Date: 2026-09-24 · Scope: research + docs only (no engine code).

## 1. Docs written (line counts)

| File | Lines | Replaces |
|---|---|---|
| docs/research/28_PLUGINS.md | 197 | 9-line PLANNED stub |
| docs/research/29_SCRIPTING.md | 189 | 9-line PLANNED stub |

Both carry `> Status: PARTIAL (v0.1 — wave-3 depth pass 2026-09-24).`, VERIFIED/
UNVERIFIED markers, inline citations ([S-3cN] new + existing rows S-2b*/S-2c*/S-2d*/
S-10x/S-4f*), and end sections: UNVERIFIED list + depth-remaining list.

## 2. Ledger

research/sources/LEDGER_W3c.md — 10 new rows S-3c0..S-3c9 in the exact 14-column
pipe-table format. SOURCE_LEDGER.md untouched (read-only for citation IDs).

## 3. Key findings (verified this pass)

- **wasmtime is 49.0.0** (crates.io, updated 2026-09-21), Apache-2.0 WITH
  LLVM-exception — the "33.x?" guess in the task brief was stale. wasmer 7.4.2 (MIT);
  extism 1.30.0 (BSD-3) explicitly targets "execute arbitrary, untrusted code from
  your users" with runtime limiters/timers and host-controlled HTTP.
- **VST3 SDK 3.8.x is now MIT** — "Licensing under GPLv3 and the Steinberg proprietary
  license is no longer available" (README §700). CLAP is MIT with a stable 1.x ABI +
  capability-extension negotiation — the best-in-class pattern for our effect ABI.
- **LuaJIT is NOT dormant** (corrects the task brief's working assumption):
  luajit.org/status.html states "actively developed and maintained", rolling releases
  from versioned branches, README copyright 2005-2026, MIT. Bellard's original
  QuickJS, by contrast, is confirmed dormant by the quickjs-ng fork README (qjs-ng
  v0.17.0 active, MIT). mlua 0.12.1 (MIT) supports Lua 5.5→5.1/LuaJIT/Luau, vendored,
  async, send.
- **Adobe CEP lesson verified in source**: CEP 12 = CEF3 embedded in the host process
  + bundled Node.js (Node 8.6.0→17.7.1 across CEP 9–12) → in-process JS is the host's
  crash/security domain; UXP is the vendor's narrower-runtime pivot. Figma shows the
  minimal sandbox: main-thread minimal-JS + iframe UI + message passing + manifest
  network allowlist (CSP-enforced). Deno: secure-by-default capability grants.
- **OBS is the two-tier precedent**: full C API for in-process modules
  (obs_open_module, obs_source_info) vs documented RESTRICTED Python/Lua scripting
  surface ("Other Differences From the C API").
- **GStreamer verified from gst-docs clone** (freedesktop.org PWG HTML 404s; GitLab
  raw blocked by Anubis): GObject model, GST_ELEMENT_REGISTER registration flow.
- **Kdenlive per-effect XML verified**: 91 declarative effect XML files in
  data/effects/ (box_blur, brightness, …); UI generated from schema, processing
  stays in MLT.
- **FFmpeg filtergraph-as-file verified**: `-/filter:v filter.script` option-value-
  from-file syntax loads a filtergraph from a file (current docs).
- **AE expressions verified verbatim**: per-property JavaScript "evaluate[s] to a
  single value for a single layer property at a specific point in time" — the exact
  shape for scripted param sources on exact rational ticks (ADR-007).
- **Resolve scripting**: official docs are locked in a 209MB manual PDF (reachable,
  not parsed — honest); verified via community MCP wrapper that exposes the
  "complete" API (25 project / 59 timeline / 27 media-pool tools) over a
  fusionscript bridge with an in-memory object-ID registry — the object-RPC
  anti-pattern our command bus avoids.

## 4. ADR-relevant decisions proposed (feeds future ADR-025; respects ADR-007/010)

1. **Three plugin tiers** (28_PLUGINS §7): (a) native Rust trait effects, in-core,
   WGSL for GPU, params keyframed on exact rational ticks, catch_unwind boundary;
   (b) WASM sandboxed effects/analysis via wasmtime + component model (WIT ABI
   `ove:effect@X.Y`), extism-style manifest + limits, deterministic, no ambient IO;
   (c) out-of-process helper services for heavy AI models, JSON + frame-hash
   receipts, MCP-wrappable. Declarative Kdenlive-XML-like tier as B0.
2. **Boundary rule**: frames cross INTO effect code; effects never emit commands;
   commands are emitted only by UI/scripts/agents/MCP (one command bus, ADR-010).
   Analysis results become commands only via validate→approve→apply (doc 26).
3. **Scripting surface** (29_SCRIPTING §4/§5): scripts emit command batches through
   the same validated ladder as agents; scripts get read-only snapshots; split
   script file into pure `plan()` (emits commands) + pure per-time/per-frame
   functions; IO quarantined behind capability grants and materialized as logged
   commands/records (E-003 replay safety).
4. **v1 ships JSON command batches + ove-cli** (`render project.ove`, `apply
   batch.json`, `validate batch.json`); v2 = embedded Lua 5.4 via mlua (leading
   candidate) or QuickJS-ng via rquickjs; decision gates G1–G4 defined (batch
   latency, sandbox fuzz, replay hash-equality, license firewall — bpy's GPL wheel
   is the cautionary case).

## 5. NOT-FOUND / fetch failures (all logged honestly)

- GITHUB_TOKEN absent; GitHub REST rate-limited once (commits API 403) → worked via
  raw.githubusercontent, HTML pages, atom feeds, shallow git clone.
- gstreamer.freedesktop.org PWG intro URL → 404 (site reorganization); GitLab raw
  blocked by Anubis anti-bot → solved by shallow-cloning gst-docs (PWG markdown).
- docs.obsproject.com PWG-adjacent dead end: obsproject.com/kb/lua-scripting → 404
  (official /scripting doc worked instead).
- DaVinci Resolve official standalone scripting docs: NOT-FOUND as fetchable docs;
  Reference Manual PDF reachable but 209MB (not parsed). No fabricated API claims.
- Adobe CEP_12.x/README.md path → 404 (used repo-root README + actual cookbook path).
- Kdenlive data/render → 404 (render-profile file location UNVERIFIED).
- Shotcut CLI job-control flags: NOT-FOUND in FAQ (only QT_SCALE_FACTOR documented);
  melt CLI verified instead.
- pydavinci (rekyuu) README → 404; Excalidraw plugin model not attempted (scope cut,
  listed UNVERIFIED); Adobe UXP permission/manifest details not fetched.
- wasi.dev / component-model docs reviewed contain no GPU/zero-copy frame interface
  (absence stated as UNVERIFIED-absence, not fact).

## 6. Source IDs used

New: S-3c0 (component model/WASI 0.3), S-3c1 (wasmtime/wasmer/extism), S-3c2 (OBS
plugin+scripting docs), S-3c3 (GStreamer PWG), S-3c4 (VST3/CLAP), S-3c5 (Resolve
bundle), S-3c6 (Adobe CEP/UXP/AE), S-3c7 (Blender bpy), S-3c8 (Kdenlive XML +
Audacity), S-3c9 (mlua/LuaJIT/qjs-ng/rquickjs/Deno/Figma).
Existing rows cited: S-2b0..S-2b6 (OTIO/MLT/Kdenlive/Shotcut/Olive), S-2c0..S-2c5
(FFmpeg/GStreamer), S-2d1/S-2d5/S-2d6/S-2d9 (wgpu/libplacebo/OBS/wasm-perf), S-106
(OpenReelio), S-110/S-4f8 (kinocut), S-4f5 (MCP).
