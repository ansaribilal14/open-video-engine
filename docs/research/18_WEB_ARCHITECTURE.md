> Status: PARTIAL (v0.1).
# 18 — WEB ARCHITECTURE (browser column of OVE)

> Owner: agent 3-d · Tracks: E, Q · Date: 2026-09-21
> Scope: local-first app shape (threads, canvases, audio), browser storage stack
> (OPFS/IndexedDB/FSA/SQLite-WASM/quotas), where the shared engine core can and cannot be
> shared, and mobile-browser constraints. Feeds C-002/C-006 and ADR-001.

## 1. Process/thread shape (verified support data, 2026-09-21)

- **Main thread**: UI, timeline interaction, state machine. Never decode or compose here.
- **Workers** (Dedicated): decode workers (own `VideoDecoder`s), export workers, project
  persistence. `VideoFrame`/`AudioData` are Transferable + Serializable and WebCodecs is
  `Exposed=(Window, DedicatedWorker)` (doc 11 §1/§7) — frames move worker-to-worker by
  transfer without pixel copies.
- **OffscreenCanvas**: Chrome 69, Firefox 105, **Safari 16.4** (MDN BCD, fetched today) —
  a full canvas/WebGL/WebGPU composition surface inside a worker. Compositor can live
  entirely off the main thread.
- **WebGPU in workers**: available in dedicated workers (GPU contexts except service
  workers; BCD notes apply, e.g. Firefox 141 note "all contexts except service workers").
- **AudioWorklet**: Chrome 66, Firefox 76, Safari 14.1 (MDN BCD today) — audio graph /
  mixer runs here; sample-accurate scheduling with `AudioContext.currentTime` as the
  playback clock. Audio decode bridge on Safari <26: WebAudio decodeAudioData (doc 11 §10).
- **Storage access from workers**: OPFS sync access handles are **worker-only**
  (`createSyncAccessHandle`: Chrome 102, Firefox 111, Safari 15.2 — MDN BCD today), so the
  persistence layer must run in a worker regardless.

Resulting shape:

```
Main thread:            UI (egui-on-wgpu or DOM) + command dispatch
Decode worker(s):       Mediabunny Input (demux) -> VideoDecoder/AudioDecoder -> frames
Compositor worker:      OffscreenCanvas + WebGPU, pass graph (doc 08), preview loop
Audio worklet:          mixer, fades, per-track gains
Export worker(s):       deterministic render -> VideoEncoder/AudioEncoder -> Mediabunny Output
Persistence worker:     OPFS + SQLite-WASM (project, cache, proxy media)
```

## 2. Storage stack (verified support data, 2026-09-21)

| Layer | Since | Notes for OVE |
|---|---|---|
| **OPFS** (origin-private FS) | Chrome 86/102+, Firefox 111, Safari 15.2+ (access-handle path) | Private-to-origin; **sync access handles only in workers**; the only web FS suitable for multi-GB project media cache with real file semantics |
| **createSyncAccessHandle** | Chrome 102, Firefox 111, Safari 15.2 | In-place read/write, synchronous within worker = no Asyncify tax |
| **SQLite-WASM over OPFS** | official sqlite.org Wasm build (2023, Chrome first; Safari/Firefox VFS progress per sqlite.org notes; PowerSync Nov 2025: "latest versions of Chrome, Safari and Firefox all support" sync access handles) | Candidate for project index/db inside OPFS-backed VFS |
| **IndexedDB** | everywhere | Blobs + structured clones; fine for project files/snapshots, awkward for huge media |
| **File System Access API** (user-visible files) | **Chromium-only** (showOpenFilePicker etc.) | Use where present (open/save native-feeling project + media); fall back to OPFS + download everywhere else |
| **navigator.storage.persist()** | Chrome 55, Firefox 57 (MDN BCD today) | Request persistence so eviction does not eat the media cache; Safari/WebKit: install/HOME-screen prompts |

Quota/persistence reality: quotas are origin-based and engine-specific (Chrome historically
~% of disk; Firefox/Safari smaller and prompt-driven); exact current numbers UNVERIFIED at
v0.1 — treat quota as a *runtime query* (`navigator.storage.estimate()`) and design
proxy/trim policies around it. Persisted storage must be requested, not assumed.

## 3. Where the "shared engine core" can and cannot be shared (browser column)

Classification draft (SHARED = one implementation for all platforms; PLATFORM-ADAPTER =
shared interface, per-platform implementation; PLATFORM-SPECIFIC = browser-only logic):

| Capability | Class | Browser implementation |
|---|---|---|
| Timeline/project model, command log, undo/redo | SHARED (Rust->wasm or TS) | wasm module in worker(s) |
| Command API for human+AI agents | SHARED | same wasm module |
| Interchange import/export (OTIO/FCPXML/SRT) | SHARED | wasm/TS |
| Pass-graph compiler (doc 08) | SHARED | TS/Rust emitting pass lists |
| GPU execution of pass graph | PLATFORM-ADAPTER | wgpu/WebGPU backend (+WebGL2 fallback) |
| Shader library (WGSL) | SHARED via naga | WGSL sources, naga/native both consume |
| Blend/mask/LUT/text algorithms | SHARED (as WGSL + host logic) | same |
| Decode (elementary) | PLATFORM-ADAPTER | WebCodecs VideoDecoder; native: FFmpeg/MediaCodec |
| Demux | PLATFORM-ADAPTER | Mediabunny/mp4box.js; native: FFmpeg |
| Mux | PLATFORM-ADAPTER | Mediabunny Output; native: mp4 mux libs |
| HW frame zero-copy import | PLATFORM-SPECIFIC (per backend) | importExternalTexture; native external memory |
| Color space conversion at ingest | PLATFORM-ADAPTER | browser does it in copy/external-texture paths; native: swscale/libplacebo |
| Audio graph/mixing | SHARED core + PLATFORM-ADAPTER out | AudioWorklet; native: cpal/audio graph (3-e) |
| Text shaping | SHARED core | harfbuzz-wasm or browser text stack decision (doc 08 §5.6) |
| Export scheduling | SHARED (pull loop) | workers + WebCodecs encoders |
| Storage | PLATFORM-SPECIFIC | OPFS/IndexedDB/FSA vs desktop FS vs Android SAF |
| Capability probing (codecs, GPU, quotas) | PLATFORM-SPECIFIC | isConfigSupported/adapter/estimate |

Key boundary statement (C-002, refined): the **shared core is everything that is
deterministic, pure data, or pure math** (timeline, commands, pass-graph compilation, WGSL,
color math, text layout). The **platform adapters are everything that touches OS codecs,
OS GPU, OS files**. The browser is unusual only in that its adapters are JS API surfaces
instead of native libs — and in that decode/mux adapters are *libraries we choose*
(Mediabunny) rather than OS services.

## 4. Mobile browser constraints (iOS Safari)

- **Jetsam hard memory limit ~2 GB per WebContent process**: "ActiveHard 2048 MB" kill
  observed (developer.apple.com forum, Apr 2026; catchmetrics WebKit deep dive, Jan 2026);
  practitioners hit ~2GB ceilings on iOS (Babylon forum). Tab eviction under memory
  pressure means: project state must be snapshotted continuously (C-007 alignment), and
  in-flight media pools must be capped well below the limit (target < ~1 GB working set).
- No Firefox WebCodecs on Android (doc 11 §4): mobile-web editing today = Chromium browsers
  only; iOS Safari 26 gives video WebCodecs (16.4+) + audio (26+) + WebGPU (26) — workable
  but newest-only.
- Practical mobile posture: mobile = *review/light-edit* tier in v1; full editing on
  desktop-web; keep single codebase via capability probing.

## 5. Risks

- R1: Safari <26 users (audio WebCodecs gap) — WebAudio bridge required until Apple's
  baseline moves.
- R2: OPFS quota + eviction on non-persisted origins can silently drop media caches —
  persist + estimate + proxy-trim policy.
- R3: worker-COOP/COEP constraints on multithreaded wasm (doc 12) can conflict with embeds.
- R4: OffscreenCanvas+WebGPU-in-worker combos differ per engine — must runtime-probe and
  fall back to main-thread rendering.

## Sources (fetched 2026-09-21)
- MDN BCD: OffscreenCanvas, AudioWorklet, FileSystemFileHandle.createSyncAccessHandle,
  StorageManager.persist, VideoDecoder (mobile rows) — raw.githubusercontent fetch.
- developer.chrome.com "SQLite Wasm in the browser backed by the OPFS" (Jan 11, 2023);
  sqlite.org "SQLite Wasm for Safari And Firefox progress" (Mar 11, 2023).
- powersync.com "The Current State Of SQLite Persistence On The Web" (Nov 11, 2025).
- MDN "Origin private file system" (Jul 2025).
- developer.apple.com WebContent jetsam thread (ActiveHard 2048 MB, Apr 2026);
  catchmetrics.io "Deep Dive: RAM Internals in WebKit" (Jan 5, 2026);
  lapcatsoftware.com (Jan 22, 2026); forum.babylonjs.com iOS ~2GB thread (Mar 2023).
- caniuse sharedarraybuffer.json; W3C WebCodecs TR (worker exposure).

## Depth tracking (v0.1)
- TODO: quota measurement matrix across engines (E-003-adjacent); OPFS throughput bench
  (sequential vs random, handle reuse); SQLite-WASM vs IndexedDB project-store decision
  experiment; worker-GPU contention study; verify iOS Safari tab-kill thresholds on device.
