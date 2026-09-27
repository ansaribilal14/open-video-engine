# 17 — TAURI 2 AS DESKTOP SHELL (vs ELECTRON) FOR A VIDEO EDITOR

> Status: PARTIAL (v0.1).

Scope: architecture, IPC model and its cost for frame-sized payloads, binary protocols,
sidecars (ffmpeg), security capabilities, WebView capabilities per OS (video decode,
WebCodecs), packaging/updater, and a video-editor-specific comparison with Electron.
Evidence: v2.tauri.app docs (fetched), tauri-apps/tauri tags (fetched), MDN
browser-compat-data (fetched). Verdict feeds ADR-003 (desktop shell).

## 1. Architecture (verified)

- Rust core process + system webview: WKWebView (macOS/iOS), WebView2 (Windows,
  Chromium-based, Evergreen runtime), WebKitGTK 4.1 (Linux). Tauri itself is Rust;
  current release line observed: **tauri-v2.11.6** (git tags, 2026 session; S-2e8).
- Process model docs distinguish **Brownfield** (webview + app processes, default) and
  **Isolation Pattern** (protective layer for untrusted frontend code) — relevant to
  track T (security) later.
- No bundled browser engine -> binary sizes in the low MBs vs Electron's bundled
  Chromium (~100MB+ range); exact sizes must be measured in E-00x (claim is common
  knowledge, tier 9, not yet measured for our app).

## 2. IPC model (verified from v2.tauri.app "Calling Rust"/"Calling Frontend")

- **Commands**: `#[tauri::command]` functions invoked from JS with type-safety glue;
  support args, returns, errors, async.
- **Events**: dynamic global/window-scoped event system.
- **Channels**: streaming-style responses from Rust to JS (ordered async stream) —
  the intended primitive for progress reporting and large streaming results.
- **Raw Request / `tauri::ipc::Response`**: escape hatch for raw bytes (seen as
  "Accessing Raw Request" in official docs TOC, S-2e8) — use for binary payloads.
- **Serialization cost**: command arguments/results cross the boundary serialized
  (JSON by default); for video-editor frame traffic (tens of MB/s) this is a hard
  bottleneck. UNVERIFIED this session: whether Tauri 2.x fast-path serializes via
  custom binary protocol now (there were performance reworks around v2.0 — verify
  `tauri::ipc` internals before ADR). Assume JSON-by-default until measured.
- **Custom protocol**: `register_uri_scheme_protocol` handlers serve arbitrary binary
  (e.g., `stream://` thumbnails, partial MP4 ranges) directly into the webview
  without JSON; combined with Range requests this is the right way to feed
  `<video>`/WebCodecs from Rust. Asset-protocol scope is part of the security
  surface (docs: "Asset protocol scope").
- Engine implication: frames/thumbnails/proxy media NEVER go through invoke JSON;
  design the core API as (a) small JSON commands, (b) channels for progress,
  (c) custom-protocol URLs for bulk binary. This constraint must be in ADR-002.

## 3. Sidecars: bundling ffmpeg or engine CLI (verified)

- Sidecar mechanism: external binaries declared in `tauri.conf.json` `externalBin`,
  executed via shell plugin with **capability-gated permissions**: docs show
  `shell:allow-execute` with per-sidecar `args` allowlists (static + regex dynamic
  args) inside `src-tauri/capabilities/default.json` (S-2e8, quoted).
- Use for the engine: either ship a compiled `engine-core` binary (Rust core as CLI/
  stdio JSON-RPC peer) or ffmpeg for utility transcodes. Note licensing: GPL ffmpeg
  in a distributed app has obligations (doc 35, W track).

## 4. Security/capabilities (verified doc structure)

- Tauri 2 ACL: **Permissions** (per-plugin commands) grouped into **Capabilities**
  assigned to windows/webviews; command scopes; CSP config; asset scope. The
  sidecar allowlist above is the same system. Fits track T (security) requirements;
  more granular than Electron's contextIsolation/preload model.

## 5. WebView capabilities per OS — the video-decode question

| OS | Engine | HW video decode in webview | WebCodecs |
|---|---|---|---|
| Windows | WebView2 (Chromium) | yes (Media Foundation via Chromium stack) | yes — Chromium shipped WebCodecs in Chrome 94 (MDN BCD: chrome 94, edge 94, S-2e9) |
| macOS | WKWebView | yes (VideoToolbox) | yes — Safari/WebKit 16.4+ (MDN BCD, S-2e9) |
| Linux | WebKitGTK | only with user-installed GStreamer codec plugins (H.264 needs gst-libav etc.); distro-dependent, common pain point | PARTIAL/UNVERIFIED — WebKitGTK implements WebCodecs over GStreamer (freedesktop talk "WebCodecs in WebKit, with GStreamer!"); shipping status/version gate UNVERIFIED; verify on target distro before ADR |
- Consequence: a Tauri front-end cannot rely on the webview for timeline preview on
  Linux, and even on Windows/macOS the webview's decoder gives timeline scrubbing
  but not the same color/timing guarantees as the engine's own decoder. Preview
  should be engine-rendered (wgpu/WebGPU canvas or engine-decoded frames via
  custom protocol) with webview decode reserved for casual playback.
  (Cross-check docs 09/10/11 from agent 3-d.)

## 6. Packaging, updates

- Bundlers (NSIS/MSI, DMG/App Store, AppImage/deb/rpm) + official **updater plugin**
  (tauri-plugin-updater) with signed artifacts. UNVERIFIED this session: plugin
  exact config surface; verify in tauri-plugin-workspace before ADR-003.
- Linux packaging inherits WebKitGTK version fragmentation; Tauri docs keep
  prerequisites per-distro (webkit2gtk-4.1 + ayatana deps) — support-matrix risk.

## 7. Tauri 2 vs Electron for a video editor (analysis, not marketing)

| Criterion | Tauri 2 | Electron |
|---|---|---|
| Bundle size / memory | small binary; one webview + Rust core; lower idle RAM | Chromium bundled; multi-process; 100MB+ class payloads; higher RAM baseline |
| WebView consistency | varies per OS (esp. Linux WebKitGTK) — QA matrix cost | one Chromium version everywhere |
| GPU decode in webview | strong on Win/mac; fragile on Linux | uniform Chromium decode incl. WebCodecs |
| Rust core integration | native (commands run in Rust core process) | needs Node native modules or sidecar |
| Binary data path | custom protocols + channels; JSON default on invoke must be avoided for frames | same class of problem; developers often use WebSocket/IPC workarounds |
| Security model | capabilities/ACL per window; isolation pattern | contextIsolation, CSP, third-party (electron-sandbox) |
| Ecosystem maturity for pro apps | younger; fewer pro-app precedents | many shipped pro apps (VS Code class) |
| Updater | official plugin, signed updates | electron-updater (mature) |

- Honest read: for THIS project the decisive factors are (1) engine core is Rust
  anyway (doc 13 F-arguments), so Tauri's Rust core removes a process/FFI hop that
  Electron would impose (Node <-> Rust sidecar), and (2) the Linux webview codec
  fragility — but our architecture (doc 5/§5) makes webview decode non-critical.
  Electron remains the fallback if webview inconsistencies burn the UI track.

## 8. Where the engine core sits (options for ADR-003)

| Option | Shape | Pros | Cons |
|---|---|---|---|
| Tauri core process | engine core compiled INTO the Tauri Rust process | no extra process; commands are direct calls | engine crash takes shell; rebuild couples releases |
| Sidecar engine binary | engine core as separate `engine-core` binary, JSON-RPC over stdio | crash isolation; CLI/CI reuses same core (headless track R); sandboxable via ACL | one more process; serialization at command boundary (small payloads only) |
| FFmpeg sidecar only | Tauri + web UI + ffmpeg utility binary only; no Rust core yet | fastest MVP | violates engine-first mission directive |

Recommendation shape (pending ADR): sidecar engine binary as the default, with the
option to link the core into the Tauri process later behind the same command API.
This preserves the directive's "headless/agent parity" requirement (same core API
for human UI and AI agents) at near-zero extra design cost.

## 9. Risks (feed research/risks/RISK_REGISTER.md)

- R-TAURI-1: invoke JSON serialization if a future dev naively passes frame data —
  mitigate with lint rule + core API review checklist.
- R-TAURI-2: WebKitGTK version drift on Linux distros (preview rendering bugs).
- R-TAURI-3: sidecar ffmpeg licensing + update channel complexity.
- R-TAURI-4: Tauri 2.x API still evolving (2.11 line observed); pin versions.

## 10. Depth remaining (v0.2)

- E-00x: measure invoke JSON vs custom-protocol throughput for 10MB/100MB blobs on
  Win/macOS/Linux; measure idle/scrubbing RAM Tauri vs Electron with same UI.
- Verify WebCodecs-in-WebKitGTK on 2 target distros; document gst plugin install.
- Verify tauri-plugin-updater config + signing flow end-to-end.
- Prototype: engine-core sidecar speaking JSON-RPC over stdio + thumbnail custom
  protocol (gates ADR-002/ADR-003).
