# ULTIMATE VIDEO ENGINE — KNOWLEDGE BASE (v0.1 synthesis)

> Status: PARTIAL (v0.1). This is the first cross-track synthesis required by the
> directive. Depth passes and experiments will upgrade sections. Evidence links point to
> docs/research/* and the claim ledger.

## 1. What a video engine actually is

A deterministic machine that (a) references immutable source media, (b) holds an exact
time-addressable model of an edit (timeline), (c) compiles that model into a frame graph
per output frame, (d) executes it against real decoders/GPUs/encoders, and (e) records
every change as reversible commands. The editor UI, the AI agent, the script, and the
server renderer are all clients of (a)–(e). [40_ARCHITECTURE_COMPARISON, 20_TIMELINE]

## 2. How media flows through it

Container → demux (packets, DTS order) → decode (PTS reorder via B-frames) → frame store
→ timeline sampler (per-clip: seek to keyframe, decode forward, discard) → compositor →
encode → mux. Non-negotiables established by research: PTS-driven timeline (VFR is
normal), per-clip keyframe index, codec flush after seeks, remux/smart-render as the
fast path, per-stream hardware-probe with software fallback chain. [04, 05]

## 3. How timeline mathematics works

Time is exact: int64 units in a rational timebase (num/den), never float seconds.
Verified failure modes of fp: boundary floor() off-by-one (3×(1/3)≠1.0), non-associative
sums across renderers, decimal 23.976 vs 24000/1001 ≈ 3.6 ms/h drift, 48 kHz×29.97
sample-boundary math. Model = per-track ordered clip lists with in/out ranges + separate
effect layer (industry consensus: OTIO/MLT/Kdenlive/Shotcut/Flowblade); node graphs for
compositing; interval indices derived, never authoritative. Primitives: move/resize,
splice/extract, retime, split, structural ops. [20, 03] — C-001 HIGH

## 4. How seeking works

Seek → nearest keyframe ≤ target (from per-clip index: MP4 stss / MKV cues /
AV_PKT_FLAG_KEY) → decode forward discarding → emit frame; flush codec state every seek.
Accurate-frame seek costs are why proxies + preview caches exist. [04, 11]

## 5. How compositing works

Compiled pass graph: per-frame target list → layer sort → per-layer effects passes →
blend/transform → output. GPU where possible (wgpu/WebGPU importExternalTexture;
Android SurfaceTexture→GL_OES_EGL_image_external; desktop zero-copy hw frames).
Two portable VideoFrame→GPU paths exist in browsers: importExternalTexture (near
zero-copy, single-use) and copyExternalImageToTexture (RGBA copy). [08, 09, 10, 11, 16]

## 6. How GPU acceleration works — and where its boundary is

The sharpest research result: **sharing succeeds at the model/graph/shader layer, not
the codec/GPU-import layer** (C-002 refined). Layer map:
- SHARED: timeline, project, commands, undo, pass-graph compiler, WGSL library, color math
- PLATFORM-ADAPTER: demux/decode/encode/mux, video-frame import, storage
- PLATFORM-SPECIFIC: capability probing, degraded modes, storage endpoints
[18, 40] — C-002/C-005/C-006 MEDIUM

## 7. How audio works (v0.1)

Resample to a common clock; sample-accurate alignment derives from the rational
timebase (48 kHz ≠ integer frames per NTSC frame — must round by rule, not fp).
Browser: WebAudio/AudioWorklet (Safari <26 lacks audio WebCodecs → bridge needed).
Full doc 21 pending. [11, 20]

## 8. How color works (v1 minimum)

Propagate + persist 4 tags (matrix, primaries, transfer, range) + chroma location per
clip; never silently mix 601/709; upsample 4:2:0→4:4:4 once at ingest; single working
space (full-range, ≥10-bit); tone-map only at output; write tags on export. [22]

## 9. How projects are represented (leading candidate, not decided)

Snapshot + serializable command log: undo log = project log = AI transaction = diff.
Blocked on E-003 replay determinism. Anti-patterns verified from the field: Kdenlive
gen-1 dual-store corruption; decimal-separator crash; OpenCut classic has no portable
format (IndexedDB only) — a lesson, not a model. [19] — C-007 MEDIUM

## 10. How undo works

Command pattern with inverse commands; Kdenlive lambda-pairs and Shotcut QUndoCommand
verbs validate it at NLE scale; selection/ripple semantics are part of command payloads.
[03, 20]

## 11. How AI operates the engine (the safety model)

One command API for humans and agents (C-003 MEDIUM; convergent: Cutlass ToolTier+dry-run,
Diffusion Studio zod tool==command, OpenReelio loopback MCP, EditDuet edits-via-NLE-tools
SIGGRAPH 2025). Shape: analysis commands auto-allowed; destructive/bulk deny-by-default;
validate→preview→approve→apply with inverse-batch undo + receipts; MCP is a thin versioned
shell, never the project model; tool descriptions and media-derived text are untrusted
input (tool-poisoning/prompt-injection defenses). [25, 26, 27, 38]

## 12. Platform truths (v0.1)

- **Android**: Media3 Transformer = export MVP backend, not a timeline engine; MediaCodec
  surface zero-copy path is the real compositing route; Android 14+ mediaProcessing FGS
  6h/24h budget ⇒ resumable/checkpointed export is mandatory. Bridge = UniFFI domain API
  + raw JNI for Surface/AHardwareBuffer + cargo-ndk. [14, 15, 16]
- **Web**: WebCodecs video is Baseline everywhere (Safari 16.4+, audio since Safari 26);
  WebGPU now Chrome/Firefox(141+ Win)/Safari 26; Firefox Android has neither ⇒ mobile-web
  is Chromium-only; iOS jetsam ~2048 MB ⇒ snapshotting + sub-GB pools; wasm decode 2–5×
  slower ⇒ probe/demux/mux only; Mediabunny closes the mux/demux gap. [10, 11, 12, 18]
- **Desktop**: Tauri 2 viable — but frames must never cross `invoke` JSON; channels +
  custom-protocol for bulk binary; sidecar ffmpeg/ingine binary pattern; WebKitGTK
  WebCodecs status unverified ⇒ webview decode never load-bearing. [17]

## 13. What the field's editors teach (top lessons)

1. Command/tool surface unifies UI, CLI, AI — three projects converged independently. [02]
2. Strict UI-state vs core-logic split; OpenCut-classic is the cleanest undo/selection
   reference; its rewrite (Rust core) validates the direction. [02]
3. Rust core + thin shells is the new-gen consensus; wgpu compositors + dual export
   paths (WebCodecs browser / FFmpeg+GPU desktop). [02, 40]
4. Licensing is an architecture input: default builds LGPL-only; GPL/nonfree streams are
   build variants, never defaults. [05]
5. Honest non-existence: Kerf, Velocut, OpenTake, Frontstage could not be found (39) —
   the directive's named list contains aspirational entries; evidence over authority.

## 14. What we still do not know (blocking)

Experiments E-001..E-006 (see 41/risk register) · audio/captions/CV/plugin/headless docs ·
depth passes on OpenCut-classic internals + OTIO + FFmpeg hw paths ·
capability matrix v1 (directive CROSS-PLATFORM REALITY) · licensing audit.
