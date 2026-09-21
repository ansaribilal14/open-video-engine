# 39 — GITHUB RESEARCH (repository index + methodology log)

> Status: PARTIAL (v0.1 first-pass scan; deep source study queue at bottom).

Owner: agent 3-a. Verified date for all entries: 2026-09-21. Stars are point-in-time
snapshots taken from GitHub repo HTML (aria-label "N users starred this repository");
they drift and should be re-sampled on each revision. "Cloned" means a `git clone
--depth 1` was inspected on disk during this task. No fabricated repos, stars, or
features; anything not directly observed is marked UNVERIFIED.

Environment note: the provided GITHUB_TOKEN returned HTTP 401 (Bad credentials) on
this run, so all API-based metadata came from unauthenticated endpoints where the
per-IP rate limit allowed, and otherwise from GitHub HTML pages and raw file fetches.
The token value was never written to any file or output.

---

## 1. Repository index

| # | Repo | Verified URL | Stars (2026-09-21) | Language/Stack | License | Tier | Evidence depth |
|---|------|--------------|--------------------:|----------------|---------|------|----------------|
| R-101 | OpenCut (rewrite) | https://github.com/OpenCut-app/OpenCut | ~90,169 | Rust (GPUI desktop) + TanStack Start/Vite web + Elysia/CF Workers API | MIT | PRIMARY | README + clone (full tree read) |
| R-102 | OpenCut classic (archived) | https://github.com/OpenCut-app/opencut-classic | ~254 | Next.js 16 + React 19 + Zustand + rust/wasm (opencut-wasm) | MIT | PRIMARY | README + AGENTS.md + docs/ + clone (editor core, commands, types, exporter read) |
| R-103 | Diffusion Studio | https://github.com/diffusionstudio/editor | ~2,997 | TS monorepo: Electron + koota ECS runtime + Solid reconciler + MCP SDK | MPL-2.0 | PRIMARY | README + package.json manifests + docs/INSTRUCTIONS.md |
| R-104 | Cutlass | https://github.com/1mrnewton/cutlass | ~55 | Rust workspace (20 crates) + Slint UI + wgpu 29 | MIT OR Apache-2.0 | PRIMARY | README + Cargo.toml + crates listing + cutlass-ai source comments |
| R-105 | Clypra | https://github.com/AIEraDev/Clypra | ~3,246 | Tauri v2 + React 19 + Rust (wgpu 24, cpal/rtrb, FFmpeg hw-decode) + WASM render crate + Capacitor mobile | MIT | PRIMARY | README + Cargo.toml + trees |
| R-106 | OpenReelio | https://github.com/openreelio/openreelio | ~75 | Tauri 2 + React 18 + Zustand/Immer + FFmpeg sidecar + Wasmtime + SQLite + loopback MCP | MIT | PRIMARY | README + lib.rs excerpts + package.json |
| R-107 | Katana | https://github.com/milanofthe/katana | ~28 | Tauri 2 + SvelteKit + Rust (5-file core) | present, type UNVERIFIED this pass | SECONDARY | README + tree |
| R-108 | LosslessCut | https://github.com/mifi/lossless-cut | ~43,929 | Electron + FFmpeg (execa) | GPL-2.0 (LICENSE read) | SECONDARY | README + package.json + LICENSE |
| R-109 | Remotion | https://github.com/remotion-dev/remotion | ~59,899 | React programmatic video | source-available company license (UNVERIFIED this pass) | SECONDARY | HTML metadata only |
| R-110 | Kinocut | https://github.com/KyaniteLabs/kinocut | ~160 | MCP server + Python lib + CLI over FFmpeg | UNVERIFIED | BONUS (MCP pattern) | HTML metadata only |

Forks/variants observed but not analyzed: `Ekaanth/OpenCut-AI` ("Open-source AI video
editor" — search hit only; relationship to upstream UNVERIFIED). SourceForge mirrors
of OpenCut exist and link back to OpenCut-app/OpenCut.

---

## 2. Search methodology log

Tooling: `z-ai function web_search` (web search API), `curl` of GitHub HTML pages and
raw.githubusercontent.com, `git clone --depth 1`, GitHub REST API (rate-limited to
zero this run; one authenticated probe returned 401 Bad credentials).

### 2.1 OpenCut location
| Query | Result |
|-------|--------|
| web_search "OpenCut github open source video editor" | Found Ekaanth/OpenCut-AI (fork/variant), coddykit article (87K+ stars claim), filmora review, sourceforge mirror naming github.com/OpenCut-app/OpenCut |
| web_search "OpenCut-app/OpenCut github repository CapCut alternative" | Confirmed OpenCut-app/OpenCut as canonical ("The open-source CapCut alternative"); privacyguides thread (2025-07-12) |
| GitHub HTML https://github.com/OpenCut-app/OpenCut | title + ~90,169 stars; clone succeeded |
| GitHub HTML https://github.com/OpenCut-app/opencut-classic | archived, ~254 stars; clone succeeded |

### 2.2 Named-project verification
| Target | Queries (verbatim) | Outcome |
|--------|--------------------|---------|
| Kerf (video editor MCP) | "Kerf video editor MCP github"; "Kerf video editor github"; "kerf ffmpeg timeline agent" | No video editor named Kerf found. Adjacent finds: KyaniteLabs/kinocut (MCP server), VFX MCP Server (mcpservers.org), mcp-video (mcpmarket.com). Recorded as NOT FOUND. |
| Clypra | "Clypra video editor" | FOUND: AIEraDev/Clypra (GitHub), x-cmd listing, Pinokio page, notes.nicolasdeville.com |
| Cutlass (Rust) | "Cutlass video editor Rust github" | FOUND: 1mrnewton/cutlass |
| Velocut | "Velocut video editor github"; "velocut video editor"; "velocit OR velocut video editor open source" | Only unrelated hits (Velomingo/velocity-edit apps). NOT FOUND. |
| OpenReelio | "OpenReelio github" | FOUND: openreelio/openreelio + openreelio.com |
| OpenTake | "OpenTake video editor github"; "opentake video editor open source" | Only OpenShot/other noise + diffusionstudio/editor discovery. NOT FOUND. |
| Frontstage | "Frontstage video editor github"; "frontstage video editor app" | Only unrelated noise. NOT FOUND. |
| Katana (video editor) | "Katana video editor open source github"; "katana video editor open source rust" | FOUND: milanofthe/katana (projectdiscovery/katana is an unrelated web crawler). |

### 2.3 Secondary/bonus discoveries during the scan
| Query | Discovery |
|-------|-----------|
| "opentake video editor open source" | diffusionstudio/editor — "An open-source video editor built for agents. Edits become code, code becomes video." (cloned, analyzed) |
| "Kerf video editor MCP github" | KyaniteLabs/kinocut — guardrailed video editing MCP server (HTML-verified) |

### 2.4 Verification commands pattern (for reproducibility)
- Repo HTML: `curl -sL https://github.com/OWNER/REPO` then regex
  `aria-label="N users starred this repository"` and `<title>` for the description.
- Source: `git clone --depth 1 https://github.com/OWNER/REPO /tmp/NAME` and read
  README.md / package.json / Cargo.toml / trees on disk.
- Raw files: `curl -sL https://raw.githubusercontent.com/OWNER/REPO/main/PATH`.

---

## 3. NOT-FOUND list (honest record)

| Mission-supplied name | Status | Searches performed | Best alternative evidence |
|-----------------------|--------|--------------------|---------------------------|
| Kerf | NOT FOUND | 3 web searches (see 2.2) | KyaniteLabs/kinocut is the closest real MCP video-editing server found; no project named Kerf located |
| Velocut | NOT FOUND | 3 web searches | Only unrelated "velocity edit" mobile apps (Velomingo etc.) |
| OpenTake | NOT FOUND | 2 web searches | No matching project; search surfaced diffusionstudio/editor instead |
| Frontstage | NOT FOUND | 2 web searches | No matching project |

These names may refer to private repos, renamed projects, social-media mentions, or
nonexistent projects. No repo was force-matched by name similarity. If the principal
has source links (e.g. Twitter/X or blog posts), provide them and this list will be
re-checked against those leads.

---

## 4. Recommended repos for later deep source study (v0.2 queue)

Priority order, with the specific files/subsystems to read:

1. **OpenCut classic** (R-102): `apps/web/src/core/managers/timeline-manager.ts`,
   `playback-manager.ts`, `renderer-manager.ts`, `save-manager.ts`;
   `services/renderer/scene-builder.ts` + nodes; `rust/crates/compositor`;
   export refactor state (open PRs). Why: closest to our browser-first architecture.
2. **Diffusion Studio** (R-103): `packages/dapi` tool catalog (zod schemas),
   `packages/runtime` (koota world/actions/systems, capture), `agent-chat` WebSocket
   protocol, `docs/skills/editor.md`. Why: the reference implementation of
   "one tool surface for UI + MCP + CLI".
3. **Cutlass** (R-104): `crates/cutlass-commands` (command.rs/lib.rs full read),
   `crates/cutlass-engine`, `cutlass-storage` (project format), `cutlass-ai`
   (ToolHost/ToolTier/fuse), `cutlass-py` (headless consumer). Why: cleanest Rust
   crate decomposition + agent safety model.
4. **Clypra** (R-105): `src-tauri/src/wgpu_compositor.rs` + `shaders/`,
   `preview_golden.rs` golden harness, `crates/clypra-render-wasm`, telemetry spec
   `docs/performance-telemetry.md`. Why: wgpu compositor reference + golden-image
   testing pattern.
5. **OpenReelio** (R-106): event log schema in `src-tauri/src/core/`,
   `ipc/openreelio_mcp.rs`, `src/agents/toolOutputContracts.ts`, `src/benchmarks/`.
   Why: event-sourced undo + loopback MCP as an agent substrate.
6. **Remotion** (R-109): render CLI + deterministic-frame execution model (for our
   headless tier design). Why: 59.9k-star proof of programmatic video demand.
7. **LosslessCut** (R-108): keyframe-cut planning logic (renderer ffmpeg command
   assembly). Why: stream-copy fast path details.
8. **Katana** (R-107): `src-tauri/src/export.rs` + `project.rs`. Why: minimal
   stream-copy export + tiny project file reference.

Cross-reference: per-editor findings in `02_EXISTING_EDITORS.md`; media/GPU details
belong to docs 05/09/10/11 (agents 3-c/3-d) and MCP/AI agent details to docs 26/27
(agent 3-f) — handoff noted in the worklog.

---

## 5. Gaps / limitations of this pass

- GitHub REST API was rate-limited (0 core remaining) and the provided token was
  invalid (401), so: no fork/issue/PR counts, no commit activity graphs, no license
  API cross-checks; star counts came from HTML and are approximate (+/- rounding).
- Repos were read via shallow clones (depth 1): no history/blame insights, no tag
  archaeology (e.g. when OpenCut classic was split).
- Remotion, Kinocut: metadata-only; claims about internals are UNVERIFIED.
- Clypra: undo mechanism, AI module scope, ProRes licensing path not inspected.
- OpenReelio: event-log schema not read; "unlimited undo" is README-claimed.
- Diffusion Studio: undo/history model not found in fetched files (likely VCS-level;
  UNVERIFIED).
