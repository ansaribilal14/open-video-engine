> Status: PARTIAL (v0.1 first-pass scan; repo source-code depth study still required).

# 02 — EXISTING EDITORS (new-gen open-source scan)

Verified: 2026-09-21 by agent 3-a. All repos below were either shallow-cloned and
inspected on disk, or verified via GitHub HTML/API metadata. Every factual claim in
this document comes from files actually fetched during this scan; anything else is
marked UNVERIFIED. Star counts are point-in-time (2026-09-21) and drift constantly.

Companion doc: `39_GITHUB_RESEARCH.md` (repo index, search methodology log, NOT-FOUND list).

---

## 1. OpenCut (deep study)

### Identity
- Repo (rewrite): https://github.com/OpenCut-app/OpenCut — verified via GitHub HTML.
  ~90,169 stars (HTML aria-label, 2026-09-21). License: MIT (LICENSE + README badge).
- Repo (legacy): https://github.com/OpenCut-app/opencut-classic — verified. ~254 stars
  (the legacy split carries only its own stars; the classic README's star-history chart
  points at the main repo). README states: "This is the original OpenCut codebase.
  It's archived and no longer maintained." Website opencut.app still runs classic.
- Description: "The open-source CapCut alternative. A free and open source video editor
  for web, desktop, and mobile."
- Positioning (classic README): privacy (footage stays on device), free features
  (CapCut paywall response), simplicity.

### Architecture summary
The project is mid-flight through a full rewrite; both generations were inspected.

**Rewrite (main repo, cloned):**
- Cargo workspace: `members = ['apps/desktop']` — note `'crates/*'` is COMMENTED OUT;
  the platform-agnostic Rust core has not landed on main yet.
- `apps/desktop`: Rust + GPUI 0.2.2 (Zed's UI framework). Panels scaffold only:
  `panels/{timeline,inspector,browser,preview}.rs`, components, theme, shell.
  WSLg workaround in main.rs (removes WAYLAND_DISPLAY when both X11+Wayland present).
- `apps/web`: TanStack Start + Vite 8 + React 19 + Tailwind 4 (moved OFF Next.js from
  classic). Mostly shadcn-style UI primitives; `routes/editor.tsx` renders
  "Editor / Coming soon." — the rewrite editor does not exist yet on main.
- `apps/api`: Elysia on Cloudflare Workers (wrangler.jsonc).
- Tooling: moonrepo `proto` + `moon` tasks (replaced Bun workspaces+turbo of classic).
- README declares the roadmap: Editor API, plugin-first architecture, one codebase for
  desktop/mobile/browser from a Rust core, MCP server for AI agents, headless mode
  (automation/batch rendering), and a scripting tab inside the editor.
- Changelog versions 0.1.0–0.3.0 (dated 2026-02-23, 2026-03-01, ...) describe property
  panel overhaul, 1,000+ fonts, blend modes, keyframes, ripple editing.

**Legacy (opencut-classic, cloned) — this is the running product:**
- Monorepo: `apps/web` (Next.js 16, React 19), `apps/desktop` (GPUI, in progress),
  `rust/` (core), `packages/*`, Bun + Turborepo, Docker compose for Postgres + Redis
  (accounts/projects backend; drizzle-orm, better-auth, Upstash ratelimit; deployed on
  Cloudflare via wrangler/@opennextjs/cloudflare).
- `AGENTS.md` states the migration thesis: "An ongoing migration is moving all business
  logic into `rust/`. Each app under `apps/` is a UI shell — it owns rendering,
  interaction, and platform-specific concerns, but never owns logic. The UI framework
  for any given app is a replaceable detail." (Verified verbatim from AGENTS.md.)
- `rust/crates`: `gpu` (context), `effects` (types/pipeline/effects), `masks`
  (masks/sdf/feather), `compositor` (compositor/blend_mode/texture_pool/frame/
  texture_store), `bridge`, `time` (timecode/media_time/time/frame_rate). Published to
  npm as `opencut-wasm` (root package.json pins `opencut-wasm ^0.2.10`; wasm-pack build,
  `bun dev:wasm` watch loop, `bun link` flow for local dev).
- `docs/` architecture notes: `actions.md` (action system), `keyframes.md`
  (keyframe system), `effects-renderer.md`, plus `notes/primitives-vs-domains.md`
  (an engineering-culture note on keeping primitive value types out of domain folders).
- Key web deps: mediabunny ^1.29.1 (export muxing), @huggingface/transformers ^3.8.1
  (in-browser Whisper transcription), soundtouchjs (audio time-stretch), wavesurfer.js,
  zod 4, zustand 5, Tailwind 4, Radix UI.

### Timeline/state model (classic, read from source)
- Two-tier state: Zustand stores hold UI state ONLY. `timeline/timeline-store.ts`
  header comment: "UI state for the timeline. For core logic, use EditorCore instead."
  It holds snappingEnabled, rippleEditingEnabled, expandedElementIds (persisted subset).
- `core/index.ts` defines `EditorCore` — a singleton class aggregating 12 managers:
  CommandManager, TimelineManager, PlaybackManager, ScenesManager, ProjectManager,
  MediaManager, RendererManager, SaveManager, AudioManager, SelectionManager,
  ClipboardManager, DiagnosticsManager. Constructor registers default effects/masks,
  wires a "prune empty tracks" reactor into the command manager, and starts autosave.
- Data model (`project/types.ts`, `timeline/types.ts`): `TProject { metadata, scenes[],
  currentSceneId, settings (fps/canvasSize/background), version, timelineViewState }`.
  `TScene { id, name, isMain, tracks: SceneTracks, bookmarks[] }`.
  `SceneTracks { overlay: OverlayTrack[], main: VideoTrack, audio: AudioTrack[] }` —
  one main video track plus ordered overlay tracks (video/text/graphic/effect types) and
  audio tracks. Element types: video, image, text, sticker, graphic, effect, audio
  (upload|library). Elements carry optional `animations.channels` (keyframe channels
  keyed by property path: number/color/discrete types, registry-driven).
- Time is Rust-owned: `FrameRate`, `MediaTime`, `TICKS_PER_SECOND` come from
  opencut-wasm; TS code converts via `mediaTimeToSeconds`.

### Command/undo model (classic, read from source)
- `commands/base-command.ts`: abstract `Command { execute(): CommandResult|undefined;
  undo(); redo() = execute() }`. `CommandResult` may carry a selection patch.
- `core/managers/commands.ts`: CommandManager keeps `history` + `redoStack` of
  `CommandHistoryEntry { command, previousSelection, selectionOverride }`.
  - execute(): snapshot selection → run → optionally apply ripple adjustments
    (computed from before-tracks when ripple mode is on) → apply selection override →
    run reactors → push history, clear redo.
  - undo(): only restores selection for commands that declared selection intent;
    UI-driven selection changes between commands survive undo. Commands that removed
    selection targets must declare a `selectionOverride` to clear stale refs.
  - `push()` adds externally-mutated history entries; `registerReactor()` lets
    invariants (e.g. pruning empty tracks) run after every command.
- Action layer (`docs/actions.md`): user-triggered operations go through named ACTIONS
  (category, defaultShortcuts, args) with `useActionHandler` registrations and
  `invokeAction()` calls; docs explicitly forbid UI calling `editor.xxx()` directly
  because it would bypass toasts/validation/keybindings. Keybindings persist in
  localStorage with versioned migrations.
- Keyframe doc (`docs/keyframes.md`): three layers — data model (channels on elements),
  property registry (which paths are animatable, value kind, interpolation, ranges),
  resolver (`lib/animation/resolve.ts` effective value at local time) — renderers call
  the resolver before drawing, both preview and export.

### Project format (classic)
- In-memory `TProject` (see above) persisted by SaveManager; projects listed in
  `app/projects/store.ts` (server-backed: Postgres via drizzle when logged in; local
  persistence otherwise). No standalone interchange format (no OTIO import/export
  found in the scan). `version` field exists on projects.

### Render/export strategy (classic)
- Export module (`export/index.ts`): formats mp4|webm, quality low..very_high,
  optional audio; downloads an ArrayBuffer blob in-browser.
- `services/renderer/scene-exporter.ts` uses **mediabunny**: `Output` +
  `Mp4OutputFormat`/`WebMOutputFormat` + `BufferTarget` + `CanvasSource` +
  `AudioBufferSource` — i.e. frames rendered to canvas are muxed client-side through
  mediabunny (mediabunny drives WebCodecs encoders internally; UNVERIFIED detail which
  codec path it picks per browser).
- Rendering is a node graph: `services/renderer/nodes/` (root-node, video-node,
  image-node, text-node, sticker-node, graphic-node, effect-layer-node,
  blur-background-node, color-node) + `canvas-renderer.ts`, `gpu-renderer.ts`,
  `scene-builder.ts`, `wasm-compositor.ts` (bridge into the Rust compositor).
- Contributing note in README: "Avoid for now: ... export functionality - we're
  refactoring these with a new binary rendering approach."

### AI/MCP/agent features
- Classic: in-browser Whisper transcription (`transcription/` module +
  @huggingface/transformers), captions. No MCP, no agent API found (grep for
  MCP/model-context-protocol in classic: no hits).
- Rewrite roadmap (README): MCP server for AI agents, headless mode, scripting tab,
  Editor API, plugin-first architecture. None of this is on main yet.

### PERFORMANCE notes
- Rust WASM compositor for effects/masks/blending; texture_pool/texture_store in the
  compositor crate suggest reuse of GPU textures (not further measured here).
- Waveform caching (`services/waveform-cache`), video caching (`video-cache`),
  font sprite generation script (generate-font-sprites) to speed font rendering.
- `perf.rs` exists in rust/wasm. No published benchmarks found in the scan (UNVERIFIED
  performance claims absent — no numbers to cite).

### WHAT THEY GOT RIGHT
- Strict UI-state vs core-state split (Zustand = UI only; EditorCore = logic) — the
  cleanest separation we observed in a browser editor.
- Command-pattern undo with selection snapshot semantics and post-command reactors —
  pragmatic and small (single file, ~150 lines) yet covers ripple editing.
- Action layer decoupling keyboard/menu/button from handlers, with persisted-keybinding
  migrations — directly reusable design for our engine's command API.
- Time primitives in Rust (`time` crate: timecode/frame_rate/media_time) so TS and Rust
  agree on time.
- Honest docs for AI agents (AGENTS.md, docs/actions.md) that state invariants instead
  of hope — keeps contributors and copilots from breaking the architecture.
- Node-graph renderer with per-element-type nodes; easy to reason about, test, and
  extend (keyframe resolver sits in front of every node).

### WHAT THEY GOT WRONG
- The rewrite reset: 90k-star audience is staring at "Coming soon." while the product
  lives in an archived repo. The Rust core (`crates/*`) is not even enabled in the
  workspace on main. Big-bang rewrite risk made visible.
- Client-only project storage split across local + Postgres/better-auth/Upstash
  inflates the codebase for what is, at core, a browser editor (accounts infra in a
  tool whose README pitch is "your videos stay on your device" — a tension the
  community has flagged; source: filmora review, SECONDARY).
- Export path was marked "do not contribute" for months (README), a bus-factor smell
  for a critical subsystem.

### WHAT THEY SIMPLIFIED (deliberate scope cuts, from source)
- Single main video track + overlay/audio stacks; no magnetic/multi-track V1..Vn model.
- Effect/transition catalog is thin; blend modes shipped text-only first (changelog 0.1.0).
- No proxy/intermediate media management; media handled via caches instead.
- Project format is an internal typed object, not a portable interchange document.

### WHAT WE CAN REUSE
- The action/commands/managers taxonomy (nearly 1:1 with our planned command API).
- Rust time-crate shape (ticks, timecode, frame rate) as our own `engine-time` crate.
- The "primitives vs domains" folder test (notes/primitives-vs-domains.md) as a repo
  hygiene rule from day one.
- Keyframe data model: channels keyed by property path with typed value kinds +
  registry + resolver — small and portable.
- AGENTS.md pattern: write architecture contracts for AI contributors.

### WHAT WE SHOULD NOT COPY
- The two-codebase limbo (product in `classic`, promise in main repo). Ship the engine
  first; rehome the UI in place.
- Server-coupled project persistence in the core editor (keep accounts optional).
- Zustand-persisted keybindings with manual migration chain (we: persist at the
  command/config layer, version the project file, not UX prefs chains).

### OPEN ISSUES
- Rewrite completion timeline unknown; no public ADRs found (UNVERIFIED whether design
  docs exist privately).
- Classic export path stability during the "binary rendering" refactor.
- opencut-wasm versioning vs main-branch core (crates still uncommitted).

### Sources
- https://github.com/OpenCut-app/OpenCut (README, HTML metadata, clone)
- https://github.com/OpenCut-app/opencut-classic (README, AGENTS.md, docs/actions.md,
  docs/keyframes.md, notes/primitives-vs-domains.md, package.json, apps/web/src source
  files cited above, rust/ tree, CHANGELOG 0.1.0–0.3.0)
- SECONDARY: coddykit.com "OpenCut ... 87K+ stars" (2026-08-28); filmora.wondershare
  review; discuss.privacyguides.net thread (2025-07-12). Used only for positioning.

---

## 2. Diffusion Studio (diffusionstudio/editor) — agent-first editor

### Identity
- Repo: https://github.com/diffusionstudio/editor — verified, cloned. ~2,997 stars
  (2026-09-21). License: MPL-2.0 (LICENSE + package.json). Version 0.205.2.
- Description: "An open-source video editor built for agents. Edits become code, code
  becomes video." YC F24 company. macOS Apple Silicon download only (README badge).

### Architecture summary
- Monorepo: `apps/{desktop,web,cli}`, `packages/{runtime,jsx,reconciler,koota-solid,
  agent-chat,dapi,encoder,assets}`. Electron desktop app; Node agent host.
- The project source of truth is CODE: compositions are authored in a JSX vocabulary
  (`packages/jsx` — "Types and authoring API for Diffusion Studio compositions: the JSX
  element vocabulary, declarative generated assets"), compiled with babel-preset-solid
  (`packages/reconciler` — "evaluates a compiled project bundle"), and executed by a
  headless ECS runtime (`packages/runtime` — "Headless editor runtime: koota traits,
  world, actions, systems, media decoding, and capture. No DOM, no solid-js.").
  koota = ECS library; `koota-solid` = Solid bindings port.
- Encoding offline over runtime "worlds" via **mediabunny** (`packages/encoder`).
- `packages/assets`: asset library with `assets.yml` manifest, content hashing, media
  probing — an explicit, inspectable asset ledger.
- `apps/desktop` deps include `@modelcontextprotocol/sdk ^1.30.0`, `ts-morph` (code
  editing), `esbuild`, `@diffusionstudio/agent-chat`.

### Timeline/state model
- ECS, not tracked-tree: koota traits/world/actions/systems (from runtime package
  description). UI is a projection of the same code document the agent edits — "Every
  edit you make is written to real code, so the agent always sees the latest version"
  (README).

### Command/undo model
- Not command-stack based (no undo model documented in what we fetched); the unit of
  edit is a code change to the project bundle, so history is presumably file/VCS-level
  (UNVERIFIED — not found in fetched files).

### Project format
- Code (JSX/TS project bundle) + `assets.yml` content-hashed manifest. Portable,
  diffable, reviewable — literally source code.

### Render/export strategy
- Headless runtime executes compositions; encoder package renders/encodes via
  mediabunny offline; headless mode is a first-class product feature (README).

### AI/MCP/agent features (the core of the product)
- App registers an MCP server with the user's coding agent (Claude Code, Codex,
  Cursor, Copilot, Gemini CLI) — README.
- `dapi` tool catalog: "every tool the Diffusion Studio app exposes to agents and the
  CLI, as a name, a description, and zod schemas" (package description). Same tools
  exposed as MCP tools AND shell commands (`media grab` == `media_grab` MCP tool);
  every result is one JSON object; errors to stderr with exit code 1; "built to be
  piped, grepped, and driven by a program" (docs/INSTRUCTIONS.md + README).
- Agent perception tools: "Cutting footage requires understanding it. The app exposes
  the inspection tools an agent needs to work with media it cannot watch — as MCP
  tools, and as the same commands in a shell" (README, media_inspect/contact-sheet
  family — per-sheet flag observed).
- Skills docs loaded at session start (`docs/skills/editor.md`, `docs/skills/watch.md`)
  — INSTRUCTIONS.md instructs the agent to read the relevant skill first.
- `agent-chat` package: "a Node host that drives the user's Claude Code or Codex
  against a project over a WebSocket" — the editor can host the agent loop itself.

### PERFORMANCE notes
- Nothing published in fetched files (UNVERIFIED). mediabunny decode/encode path is
  the same browser-grade stack OpenCut uses.

### WHAT THEY GOT RIGHT
- One tool surface, three transports (MCP tool == CLI command == editor action) —
  eliminates the "agent API is a second-class API" problem before it exists.
- Code-as-timeline makes every agent edit reviewable, diffable, undoable via VCS, and
  gives LLMs a native medium (text) instead of opaque JSON graphs.
- Giving agents EYES (contact sheets / media inspection tools) acknowledges that
  editing requires watching — most competitors expose only mutation tools.
- Headless runtime with zero DOM deps makes the engine testable and embeddable.
- Asset manifest with content hashing solves the "agent hallucinated a filename"
  class of bugs.

### WHAT THEY GOT WRONG / RISKS
- Single-platform (macOS Apple Silicon first) shrinks community feedback.
- Code-as-timeline is hostile to pure-UI creators; the "fully featured editing
  environment" must stay in sync with two mediums (UI and code) — sync cost UNVERIFIED.
- MPL-2.0 file-level copyleft is fine, but company-backed repo with closed backend
  services possible (UNVERIFIED).

### WHAT WE CAN REUSE
- The dapi pattern: single zod-schema tool catalog projected to MCP + CLI + internal
  API. This is the single most transferable idea in this entire scan for our mission.
- Skills/INSTRUCTIONS.md pattern for agent onboarding.
- assets.yml content-hashed manifest.
- koota-style ECS runtime as headless core (we should evaluate vs our planned design).

### WHAT WE SHOULD NOT COPY
- macOS-only-first distribution; JSX-authoring as the only authoring medium.

### OPEN ISSUES
- Undo model, licensing of server components, Windows/Linux builds — UNVERIFIED
  (not visible in fetched files).

### Sources
- https://github.com/diffusionstudio/editor (README, LICENSE, package.json files of
  root + packages/{runtime,jsx,reconciler,agent-chat,dapi,encoder,assets} +
  apps/desktop, docs/INSTRUCTIONS.md, docs tree)

---

## 3. Cutlass (1mrnewton/cutlass)

### Identity
- Repo: https://github.com/1mrnewton/cutlass — verified, cloned. ~55 stars (2026-09-21;
  HTML title also references 1Mr-Newton/releases). License: `MIT OR Apache-2.0`
  (workspace Cargo.toml). Version 0.7.0-alpha.0. Description: "An open-source Rust
  video editor where you edit by describing what you want."
- Small stars but the most complete Rust-native engine architecture in this scan.

### Architecture summary
- 20-crate Cargo workspace: cutlass-core, models, engine, commands, decoder,
  compositor, render, encoder, text, shapes, analysis, jobs, storage, transcription,
  mobile, settings, ai, cloud, cli, ui-gallery (+ cutlass-py PyO3 crate excluded from
  the default workspace and shipped to PyPI separately via maturin).
- UI is **Slint 1.17** (not React/Svelte): workspace comment — "wgpu 29 matches Slint
  1.17's `unstable-wgpu-29` so the desktop UI can hand the compositor its own
  device/queue and share textures (zero-copy present) instead of round-tripping pixels
  through the CPU." (Verified from Cargo.toml comment.)
- Per-OS media backends: AVFoundation on macOS, Media Foundation on Windows, Linux
  backend "isn't implemented yet" (README).
- Apps: cutlass-desktop (default member), cutlass-ios-macos, cutlass-android.

### Timeline/state model
- Multi-lane timeline; features per README: cut/trim/split/move/duplicate/link-unlink/
  ripple-delete/multi-select, speed (flat + ramped), reverse, crop, flip, transforms,
  opacity, styled text, shapes, stickers, entrance/exit/combo animations, keyframes
  with graph editor + easing presets + bezier motion paths, blend modes, layer styles,
  animatable crop, per-clip motion blur, per-clip masks (linear/mirror/circle/rect/
  heart/star), chroma key, lane-wide adjustment/effect/filter passes grading everything
  beneath, transitions (crossfade + wipe-left implemented; others play as crossfade),
  volume envelopes, stereo pan, fade handles, RNNoise noise reduction per clip.

### Command/undo model
- Dedicated `cutlass-commands` crate (crates/cutlass-commands/src/{command.rs,lib.rs}).
- README on the AI assistant: "the assistant applies it through the same commands the
  UI uses, so its work shows up on the timeline like yours would and undoes in one
  step. Dry-run preview is on by default: you see the plan before anything changes."
  This is the same command-layer-is-agent-API thesis we hold, implemented.

### Project format
- "expect rough edges and a project format that hasn't settled yet" (README). Auto-save
  always-on, CapCut-style project ownership, launch-screen project list.

### Render/export strategy
- Live GPU preview with scrubbing (wgpu compositor); export to H.264/AAC MP4 via
  cutlass-encoder over the OS media backend.

### AI/MCP/agent features
- Built-in assistant, no bundled model: Local (Ollama/LM Studio), OpenRouter, or any
  OpenAI-compatible endpoint via `~/.cutlass/config.toml`; API keys never written into
  project files (README).
- `cutlass-ai` crate: agent loop with `ToolHost`, `HostToolSpec`, `ToolTier::ReadOnly`
  "sense tools" that travel with a sandbox bridge (screenshots of engine state),
  `commit_progress` tool for phase breaks in long tasks, and a "hard cap on edit-tool
  calls per prompt (the runaway-loop fuse)" (crate source comments, verified).
- Small-model caveat in README: "Small local models work but their tool calling is less
  reliable, which is part of why dry-run is the default." — honest UX for weak models.

### PERFORMANCE notes
- Zero-copy GPU texture present UI->compositor (Cargo comment); typed GPU effect
  passes (gaussian blur, vignette, pixelate + typed params/duotone); several catalog
  effects render as no-op until shaders land (README — honest status).

### WHAT THEY GOT RIGHT
- Cleanest crate decomposition seen: engine/commands/decoder/encoder/compositor/
  render/storage split with a PyO3 headless consumer (cutlass-py) proving the core is
  UI-independent.
- Agent edits ride the exact command layer the UI uses, with dry-run + one-step undo.
- Agent-loop safety engineering is explicit: read-only sense tier, authorize/call
  split, result hooks, runaway-loop fuse.
- Platform-native decode/encode instead of bundling FFmpeg everywhere.

### WHAT THEY GOT WRONG / RISKS
- Three OS media backends means Linux (a dominant FOSS contributor OS) is last.
- Project format unsettled at v0.7 — data loss/migration risk for early users.
- 55 stars: bus factor ~1; Slint choice is elegant but niche (smaller plugin ecosystem).

### WHAT WE CAN REUSE
- Crate-per-subsystem decomposition and the engine/commands separation.
- ToolTier (read-only sense vs mutating act) + fuse + dry-run default: adopt as our
  agent-API safety model.
- PyO3/Python headless consumer as a test harness idea.

### WHAT WE SHOULD NOT COPY
- Unsettled project format shipping to real users; per-OS proprietary backend matrix
  without a portable fallback (we plan FFmpeg/WebCodecs fallbacks).

### OPEN ISSUES
- Linux media backend; encoder caps (H.264/AAC only); how cloud crate participates
  (cutlass-cloud) — UNVERIFIED beyond name.

### Sources
- https://github.com/1mrnewton/cutlass (README, Cargo.toml workspace, crates/
  cutlass-commands + cutlass-ai source listing, package structure)

---

## 4. Clypra (AIEraDev/Clypra)

### Identity
- Repo: https://github.com/AIEraDev/Clypra — verified, cloned. ~3,246 stars
  (2026-09-21). License: MIT (package.json `license: MIT`, LICENSE file). Version 1.5.1.
- Description: "A hardware-accelerated video editor built on Rust, Tauri v2, and
  React 19. Sub-10ms frame decoding, GPU-native rendering, and a frame-accurate
  timeline — all free and open source under MIT."

### Architecture summary
- Tauri v2 shell (`tauri 2.11` with protocol-asset/macos-private-api features) +
  React 19 + Vite frontend; Capacitor for iOS/Android (android/, capacitor.config.ts).
- Rust side (`src-tauri/src`): `wgpu_compector`/`wgpu_compositor.rs` (wgpu 24),
  `shaders/`, `thumbnail_engine`, `ai/`, `clymatte/`, `native_audio.rs` (cpal 0.18 +
  rtrb lock-free ring buffer), `commands/`, `diagnostics`, `golden_harness` +
  `preview_golden.rs` (golden-image preview tests), `transfer/`, `models/`.
- Separate crates: `clypra-native-core`, `clypra-native-cli`, `clypra-render-wasm`
  (browser rendering path via WASM).
- FFmpeg used for decode with hw accel: VideoToolbox, D3D11VA, VAAPI (README).
- Embedded axum HTTP server (axum 0.7, multipart, CORS, qrcode) — used for device
  transfer/QR workflows (UNVERIFIED purpose beyond deps).
- Frontend store zoo (`src/store`): projectStore, timelineStore, timelineDraftStore,
  historyStore, exportStore/exportHistoryStore, cameraStore, captionStore,
  dragStateStore, mediaJobStore, recordingStore, settingsStore, shortcutStore,
  favoritesStore, presetStore, uiStore (+ middleware dir) — 15+ Zustand stores.
- i18n infra (`src/i18n`), workers dir (`src/workers`), services, features, core dirs.

### Timeline/state model
- Multi-track frame-accurate timeline with draft store (drag previews separated from
  committed state — timelineDraftStore.ts exists), undo/redo history store.
- Filmstrip cache on the frontend (README architecture diagram).

### Command/undo model
- historyStore.ts present; mechanism not read in depth this pass (UNVERIFIED whether
  command-pattern or snapshot-based).

### Project format
- Not documented in fetched files (UNVERIFIED). projectStore exists.

### Render/export strategy
- GPU-native wgpu compositor; export MP4/MOV/WebM/MKV/MP3/WAV/PNG; "high-quality
  ProRes/H.264 export" (README). Hardware decode/encode pipelines.

### AI/MCP/agent features
- `src-tauri/src/ai/` module exists; no MCP found; features not documented in README
  (UNVERIFIED — flagged for depth study).

### PERFORMANCE notes
- Marketing claims "sub-10ms frame decoding latency" — no methodology published in
  the repo scan (UNVERIFIED). Telemetry: production-only frame-timing collector
  (decode us, compose us, P95 seek latency, dropped-frame counts, OS/GPU vendor),
  adaptive 1% sampling at smooth 60fps, 100% on dropped frames (README +
  docs/performance-telemetry.md referenced). Golden-image preview harness
  (preview_golden.rs) is a strong engineering signal.

### WHAT THEY GOT RIGHT
- Draft-store pattern for drag interactions (never commit mid-gesture).
- Golden-image testing of the compositor (preview_golden.rs) — rare and valuable.
- Lock-free audio (cpal + rtrb), real-time peak+RMS waveform rendering.
- Tri-platform desktop + mobile via one Tauri/Capacitor codebase; WASM render crate
  for browser reuse of the same core.

### WHAT THEY GOT WRONG / CONCERNS
- Ships performance telemetry by default in an MIT FOSS editor — README argues zero
  PII, but it is opt-out-by-default data collection in a creative tool; we should NOT
  copy this default.
- 15+ top-level Zustand stores with no visible core/editor boundary suggests state
  sprawl (opposite of OpenCut's discipline); architecture doc depth is marketing-heavy.
- Unverifiable performance numbers in README erode trust ("sub-10ms" without bench).

### WHAT WE CAN REUSE
- Golden-image preview testing approach; timelineDraftStore pattern; wgpu_compositor +
  separate render-wasm crate split for desktop/browser parity; hardware decode matrix
  (VideoToolbox/D3D11VA/VAAPI) as a target list.

### WHAT WE SHOULD NOT COPY
- Default-on telemetry; unverifiable perf claims in README; store sprawl.

### OPEN ISSUES
- AI module scope, project format, license of bundled FFmpeg builds (GPL/LGPL
  questions), ProRes licensing reality (ProRes encode is Apple-licensed; verify how
  they ship it — UNVERIFIED).

### Sources
- https://github.com/AIEraDev/Clypra (README, package.json, src-tauri/Cargo.toml,
  src-tauri/src tree, crates/ tree, src/store listing)

---

## 5. OpenReelio (openreelio/openreelio)

### Identity
- Repo: https://github.com/openreelio/openreelio — verified, cloned. ~75 stars
  (2026-09-21). License: MIT. Version 0.1.13, self-described Pre-Alpha
  ("(0.0.0 <= v < 0.5.0: [Pre-Alpha])" in repo title).
- Description: "Prompt-driven AI video editor for Shorts and long-form."

### Architecture summary
- Tauri 2 + React 18 + TypeScript + Zustand + Immer frontend; Rust backend; FFmpeg
  bundled as sidecar binaries (src-tauri/binaries; installers bundle FFmpeg/FFprobe +
  OpenReelio CLI); SQLite storage; Wasmtime for a WASM plugin runtime.
- Rust `src-tauri/src`: `core/`, `ipc/`, `bin/` — plus a lazily-started LOOPBACK MCP
  SERVER: `ipc::openreelio_mcp::OpenReelioMcpServer`, field
  `openreelio_mcp: Mutex<Option<Arc<...>>>` in lib.rs, with IPC commands
  `respond_openreelio_mcp_call`, `wait_openreelio_mcp_ready` (verified in lib.rs).
  README: "Installers bundle ... the OpenReelio CLI used for local MCP integration."
- TS side: `src/agents/` (ContextBuilder.ts, ToolRegistry.ts, toolOutputContracts.ts
  with tests, toolNameNormalization.ts, tools/, workflow/, core/, engine/, external/),
  `src/core/TimelineEngine.ts` (+ tests), `src/benchmarks/`.
- Event sourcing: "Complete edit history with unlimited undo/redo" (README Key
  Concepts) — the event log is the state mechanism (mechanism file not read this pass;
  UNVERIFIED detail).
- Quality-control framework: black-frame/audio-peak/caption checks, currently heuristic
  (README — honest status). Plugin system: WASM-based runtime, provider interfaces for
  assets/presets/templates; "Backend integration in place; plugin UX is planned."

### Timeline/state model
- "Non-linear timeline with multi-track support" (README); TimelineEngine.ts core with
  tests on the TS side. Zustand + Immer immutable state.

### Command/undo model
- Event sourcing for unlimited undo/redo (README). Rollback note in lib.rs: "the
  CLI/MCP plan path — must call this, or the rollback silently reverts" (comment at
  src-tauri/src/lib.rs:955) — implies plan/commit semantics over the event log.

### Project format
- Not documented in fetched files (UNVERIFIED). SQLite backing.

### Render/export strategy
- FFmpeg pipelines; GPU-accelerated encoding via NVENC, AMF, QSV, VideoToolbox;
  parallel proxy generation; memory pooling; LRU/LFU cache eviction (README).

### AI/MCP/agent features
- Local loopback MCP server fronting editor tools (verified in Rust source), CLI for
  MCP integration, agent TS stack with tool output contracts (typed results) and a
  context builder for the editor state.

### PERFORMANCE notes
- Has a committed `src/benchmarks/` directory (existence verified; contents not read).

### WHAT THEY GOT RIGHT
- MCP server co-resident with the editor (loopback) + CLI over the same tools — same
  single-surface thesis as Diffusion Studio, self-hosted.
- Event sourcing as undo substrate for an agent-driven editor (agent edits become
  events; replay/diff/audit are natural).
- Tool output contracts (typed, tested) — prevents LLM-facing API drift.
- Honest pre-alpha self-labeling; QC-check framework idea is novel for FOSS editors.

### WHAT THEY GOT WRONG / RISKS
- 75 stars, pre-alpha: API churn guaranteed; wide surface (plugins, QC, agents,
  proxies) with thin core — feature-first risk.
- FFmpeg-sidecar bundling per platform is heavy; SQLite event log portability
  (UNVERIFIED).

### WHAT WE CAN REUSE
- Loopback MCP server pattern; toolOutputContracts (typed, versioned agent results);
  event-sourced project history; GPU encode matrix (NVENC/AMF/QSV/VideoToolbox).

### WHAT WE SHOULD NOT COPY
- Pre-alpha breadth-before-depth; framework-of-everything README before a stable core.

### OPEN ISSUES
- Event log schema, plugin SDK shape, whether agents can act headlessly without the
  desktop app — UNVERIFIED.

### Sources
- https://github.com/openreelio/openreelio (README, package.json, src-tauri/src/lib.rs
  excerpts, src/agents + src/core listings)

---

## 6. Katana (milanofthe/katana)

### Identity
- Repo: https://github.com/milanofthe/katana — verified, cloned. ~28 stars
  (2026-09-21). License file present (type not read in full this pass — UNVERIFIED;
  NOTICE.md exists). Description: "A fast, minimalist, open source video editor.
  Multitrack compositing, lossless export, snappy UI. A lightweight Clipchamp
  alternative built with Tauri, SvelteKit and Rust."

### Architecture summary
- Tauri 2 + SvelteKit frontend; tiny Rust core: `src-tauri/src/{lib,project,export,
  thumbnails}.rs` — the whole backend is 4-5 files.
- Native FFmpeg for heavy lifting; "lossless stream-copy where possible, so a simple
  cut is I/O-bound rather than a full re-encode" (README).

### Timeline/state model
- Multitrack compositing on a viewport canvas (PiP, split screen, overlays); text
  overlays as clips; mixed frame rates: "the timeline runs at the fastest clip's frame
  rate; slower clips are frame-held to match" (README) — a simple, honest rule.
- Filmstrip previews per clip; audio detach to own track; waveforms; audio scrubbing.

### Command/undo model
- Undo/redo + command palette (Cmd/Ctrl+K) (README); mechanism not read (UNVERIFIED).

### Project format
- "Project save/load to a small `.katana` file" (README) — small file format.

### Render/export strategy
- MP4 (H.264/H.265), WebM (VP9), MOV, GIF with lossless stream-copy fast path;
  frame-accurate playhead stepping/split.

### AI/MCP/agent features
- None (README shows no AI features).

### WHAT THEY GOT RIGHT
- Ruthless scope: the CapCut-for-quick-cuts use case, nothing else; lossless-first
  export makes the common case instant.
- Mixed-frame-rate rule is defined in one sentence and implemented cheaply.
- Small Rust core is auditable in an afternoon.

### WHAT THEY GOT WRONG / LIMITS
- No compositing depth, effects, or engine claims; scales to hobby use only (by
  design).

### WHAT WE CAN REUSE
- Stream-copy fast path as our quick-cut/export optimization; `.katana`-style minimal
  project file; filmstrip generation pattern (thumbnails.rs).

### WHAT WE SHOULD NOT COPY
- Single fast-clip frame-rate rule for a professional engine (fine for its scope).

### Sources
- https://github.com/milanofthe/katana (README, src-tauri/src listing, package.json)

---

## 7. LosslessCut (secondary)

### Identity
- Repo: https://github.com/mifi/lossless-cut — verified, cloned. ~43,929 stars
  (2026-09-21). License: GPL-2.0 (LICENSE file read). Version 3.69.0.
- Description: "The swiss army knife of lossless video/audio editing."

### Architecture summary
- Electron (electron.vite; src/{main,preload,renderer,common}); Node main process
  spawns FFmpeg/FFprobe (execa). Deps are utilities only (i18next, electron-store,
  zod, cue-parser, octokit for updates) — no media framework beyond ffmpeg.
- Model: multiple named segments per file (trim/cut lists), keyframe-cut vs normal-cut
  modes, stream copy; merge/concat of segments; export of project EDL-like files
  (llc project format; UNVERIFIED detail this pass — docs/ + docs.md exist in repo).
- Long-proven ux for fast rough-cutting; huge format coverage via ffmpeg.

### WHAT THEY GOT RIGHT
- Perfect fit of model to job: a segments list + ffmpeg stream copy beats any timeline
  engine for rough cutting; instant results, zero quality loss.
- Per-format quirks handled empirically for years (keyframe snapping, merge modes).

### WHAT WE CAN REUSE
- Stream-copy fast path + keyframe-aware cut planning for our "quick cut" tier;
  the idea that our engine should offer a lossless ops layer (cut/merge/strips)
  separate from the compositor.

### WHAT WE SHOULD NOT COPY
- GPL-2.0 license if we want permissive licensing (keep code-level reuse to design
  lessons, not code); Electron-only shell; no timeline model (out of scope for them).

### Sources
- https://github.com/mifi/lossless-cut (README head, package.json, LICENSE,
  src/ listing)

---

## 8. Remotion (secondary, programmatic video)

### Identity
- Repo: https://github.com/remotion-dev/remotion — verified via HTML. ~59,899 stars
  (2026-09-21). Description: "Make videos programmatically with React."
- License: source-available custom license with free tier for individuals/small
  companies and paid company licenses (well-known; NOT re-verified this pass —
  UNVERIFIED, flagged for doc 35 LICENSES).

### Relevance to the engine (2 paragraphs, secondary)
- Remotion proved that code-first video authoring scales to a large audience when the
  authoring medium is a familiar UI framework (React) and preview is frame-exact.
  Its execution model (render loops, deterministic frames, CLI rendering, parametric
  compositions) is the reference point for our headless/scripting tier, and
  Diffusion Studio's "edits become code" shows the same idea migrating into
  agent-driven editing.
- Its licensing model (free for individuals, paid for companies) is exactly what our
  mission must avoid per the directive ("universal open-source engine"); the technical
  lessons are reusable, the license is not.

### Sources
- https://github.com/remotion-dev/remotion (HTML metadata only this pass)

---

## 9. Kinocut (bonus find: MCP video-editing server)

### Identity
- Repo: https://github.com/KyaniteLabs/kinocut — verified via HTML. ~160 stars
  (2026-09-21). Description: "Guardrailed video editing MCP server for AI agents.
  FFmpeg, Hyperframes, repurposing tools, Python client, and CLI. Local, fast..."
- Not a timeline editor; an MCP server + Python lib + CLI wrapping ffmpeg for
  agent-driven editing with guardrails ("Hyperframes" concept UNVERIFIED — not
  inspected deeper this pass).

### Why it matters
- Evidence that MCP-wrapped ffmpeg is the current minimal viable "AI editor" pattern;
  guardrail framing (restricting agents to safe op sets) matches Cutlass's ToolTier
  fuse and supports our planned command-policy layer.

### Sources
- https://github.com/KyaniteLabs/kinocut (HTML metadata only)

---

## Cross-cutting synthesis (agent 3-a view)

1. Convergent thesis across 4 of 8 verified projects (Diffusion Studio, Cutlass,
   OpenReelio, and OpenCut's rewrite roadmap): the editor exposes ONE tool/command
   surface consumed identically by UI, CLI, and AI agents (MCP or OpenAI-style tools).
   Cutlass and Diffusion Studio both route agent edits through the exact same command
   layer the UI uses — undo, validation, and dry-run come for free. This validates our
   mission's "timeline+media+compositor+AI agent API" core: build the command API as
   the product, not a feature.
2. Rust-core + thin-UI-shells is now the dominant new-gen pattern (OpenCut rewrite,
   Clypra, Cutlass, OpenReelio, Katana). Two transport choices: Tauri IPC (Clypra,
   OpenReelio, Katana) vs process/WASM boundary (OpenCut). The compose-everything move
   is a wgpu compositor (Clypra wgpu 24, Cutlass wgpu 29 zero-copy present,
   OpenCut WASM compositor).
3. Export stack: browser projects converge on mediabunny (OpenCut classic, Diffusion
   Studio encoder) — WebCodecs + muxing handled; desktop projects converge on FFmpeg
   or OS-native backends (AVFoundation/Media Foundation) with GPU encode
   (NVENC/AMF/QSV/VideoToolbox). A dual-path (WebCodecs/mediabunny in browser,
   FFmpeg native on desktop) is the evidence-backed design.
4. Undo models split: command-stack with selection snapshots (OpenCut), event sourcing
   (OpenReelio), code/VCS history (Diffusion Studio). Command stack is the smallest
   common denominator; event sourcing is the agent-friendliest (audit + diff).
5. Agent safety patterns to adopt: read-only sense tools vs mutating act tools
   (Cutlass ToolTier), dry-run plan preview default (Cutlass, OpenReelio plan/rollback),
   runaway-loop fuse (Cutlass), typed tool-output contracts (OpenReelio), asset
   manifest against hallucinated paths (Diffusion Studio), media inspection tools so
   agents can "watch" (Diffusion Studio).
6. Honesty patterns worth copying: Cutlass publishes which effects are no-ops;
   OpenReelio labels checks heuristic; OpenCut classic documented architecture for AI
   contributors. Anti-patterns: unverifiable perf claims (Clypra), default-on
   telemetry (Clypra), "Coming soon." rewrites (OpenCut main).

Depth-study queue (for v0.2): OpenCut classic renderer/export internals; Cutlass
cutlass-commands + cutlass-engine + storage; Diffusion Studio runtime + dapi catalog;
Clypra wgpu_compositor + golden harness; OpenReelio event log schema. See
39_GITHUB_RESEARCH.md for the full index and methodology log.
