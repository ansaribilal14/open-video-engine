# 32 — PERFORMANCE ENGINEERING (Track S)

> Status: PARTIAL (v0.1 — wave-3 depth pass 2026-09-24).
> Owner: agent W3-e. Scope: frame budgets, proxy workflow, background/smart render,
> memory caps + eviction, parallelism (decode/encode threads, headless sharding,
> GPU async), profiling plan + perf CI. Built on internal experiments E-002c, E-006,
> E-007/E-007b and docs 02/04/05/08/11/12/14/15/17/18. Feeds ADR-036, ADR-032, ADR-031.

## 1. Frame budget: 60 fps = 16.67 ms

Hard deadline math (derived): 60 fps → 16.67 ms/frame; 30 fps → 33.3 ms. Budgets are
per-frame *work* but stages pipeline — the binding constraint is the longest stage
chain, not the sum. Proposed v1 allocation (desktop reference, 2×1080p layers, draft
effects tier — design proposal, to be measured in E-008):

| Stage | Budget | Notes |
|---|---|---|
| decode N layers (+color convert once) | 5–8 ms | SW decode ~3–5 ms/1080p layer on 2 cores; HW decode shifts cost off CPU (doc 04 §2) |
| composite (GPU pass graph, doc 08 §5) | 2–3 ms | pass-graph compile cached; no per-frame allocation (pooled targets) |
| effects (draft tier) | 1–2 ms | tiered: skip costly passes while playing; full quality when idle |
| audio mix | ≤0.5 ms | runs on AudioWorklet/own thread, not on video critical path (doc 18 §1) |
| preview UI + command pump | 1.5–2 ms | commands are 1–24 µs JSON at command scale (E-006) |
| IPC headroom + jitter | 1–2 ms | **1.4 MB JSON payload alone = 42% of a frame** (E-006: 6.95 ms vs 78 µs binary, ~89×) — frame payloads never via JSON |

Grounded anchors: exact rational ADD ~140 ns/op dominates timeline walks (E-002c W5;
prefix-sum caching is the lever, not the container). JSON invoke free at command scale
(128 B → 1 µs), fatal at manifest scale (E-006).

**What surveyed editors achieve (verified where possible):**
- OpenCut (classic): no published benchmarks; code shows texture_pool/texture_store,
  waveform-cache, video-cache — reuse discipline without numbers (doc 02 §1; UNVERIFIED perf).
- Clypra: README claims "sub-10ms frame decoding latency" — no methodology published;
  UNVERIFIED and treated as marketing (doc 02 §4 anti-pattern). Its *telemetry design*
  is reusable: decode µs, compose µs, P95 seek latency, dropped-frame counts; adaptive
  1% sampling at smooth 60 fps, 100% sampling on dropped frames.
- OpenReelio: committed `src/benchmarks/` dir (existence verified, contents unread; doc 02 §5).
- Conclusion: no surveyed FOSS editor publishes a defensible frame budget. OVE's budgets
  must come from its own E-00x runs on pinned reference hardware.

**Dropped-frame recovery (preview vs export asymmetry):** preview drops *quality*,
never *cadence* — tier ladder = effects → proxy resolution → fewer layers (doc 08 §5);
catch-up = jump the playhead to the next presentable frame (keyframe-index seek,
doc 04 §1.6), never queue backlog; render clock decouples from wall clock while
scrubbing. Export is a pull loop, zero drops (doc 08 §5.4). Dropped-frame counters +
sampled telemetry (Clypra pattern, opt-in) are the regression signal. Crash/tab-kill
recovery is a *project-state* problem: continuous snapshots (C-007, R-03) +
checkpointed render segments (Android FGS 6 h budget, docs 14/15, R-07).

## 2. Proxy workflow deep-dive (feeds ADR-032)

**Thresholds (design proposal; no surveyed-editor primary doc survived fetch — see
UNVERIFIED):** generate a proxy when ANY of: height > 1080; bitrate > ~60 Mbps;
codec outside the platform decode ladder (ProRes/DNxHR/10-bit 4:2:2 on Chromium;
doc 11 §2–3); or VFR-heavy phone footage that thrashes decode. Proxy size targets
the preview surface: ≤960×540 mobile-web, ≤1920×1080 desktop-web. Audio needs only
a waveform cache, not a proxy (OpenCut waveform-cache pattern, doc 02 §1).

**Proxy formats + license implications:**
- H.264/H.265 proxies via platform encoders (WebCodecs ladder, doc 11 §3; MediaCodec,
  doc 16) — no license exposure.
- ProRes proxies: FFmpeg ships a native `prores` encoder family (prores-ks options
  documented in ffmpeg-codecs.html §9.30 — VERIFIED 2026-09-24); it is part of the
  base library, not the GPL external-wrapper set (doc 05 §5). Apple's own ProRes
  SDK/licensing program page: NOT-FOUND (devimages/prores URLs 404/DNS-fail) — any
  "ProRes" branding/SDK redistribution question goes to doc 35.
- DNxHR: FFmpeg native dnxhd/dnxhr encode UNVERIFIED this pass (0 hits in fetched
  ffmpeg-codecs.html — re-check); Avid SDK terms NOT-FOUND → doc 35.
- Rule: proxy format choice is a licensing decision, not only a perf one (R-05).

**Auto-generation policies (proposal):** generate at ingest in a background worker,
priority = clips near the playhead; cache key = (source content-hash, proxy params);
invalidate on source-hash change; cap total proxy cache (evictable — regenerable, §4);
never block the preview loop (play original until proxy ready). Browser storage:
OPFS + `navigator.storage.persist()` (doc 18 §2).

**Relink correctness (hash-based, mandatory):** media identity = content hash
(BLAKE3; blake3 1.8.7 VERIFIED on crates.io — chunked/merkle design fits large files).
Manifest lives in the project file: (clip_id → {hash, size, duration, fps}). Relink
procedure: match by hash; verify hash after path match; fall back to (path+size+mtime)
only as a *hint*, always confirmed by hash. Never relink on path alone — the E-007
lesson generalizes: silently selecting wrong media is the failure mode that must be
impossible (mid-GOP copy cut selected the wrong content; keyframe index became
mandatory). E-009 must fuzz the relinker (hash collisions, same-hash-different-codec).

## 3. Background render / smart render (extends E-007b)

E-007b established the economics: keyframe-aligned stream copy = 67–69 ms per segment
vs 458–815 ms re-encode at 1080p → **6.8–11.9×**; full re-encode baseline 1897 ms/10 s.
E-007 established the safety rule: copy cuts are keyframe-quantized; mid-GOP copy
selects wrong content → per-clip keyframe index mandatory.

Extended design (proposal):
1. **Dirty-region tracking:** every applied command batch (C-007 log) computes the
   affected time ranges per track; the union = dirty set. Clean regions keep their
   rendered segments.
2. **Idle pre-render:** when no user input for T ms, the renderer works the dirty set
   at the *final* rung (deadline-free — export semantics, not preview); priority by
   (distance to playhead, recency of edit).
3. **Render-cache tiles:** segments keyed (track-set, time-range, fidelity-rung,
   layer-stack hash); LRU-evictable like any cache (ADR-031); bit-identical segments
   are reusable between preview and export.
4. **Fidelity ladder:** proxy+draft-effects (live preview) → proxy+final-effects
   (scrub preview) → full+final (export). Each rung change is a cache-key change, not
   a pipeline rewrite.
5. **Export = promote-or-render:** walk the timeline; copy cached/clean segments
   (E-007b planner, duration-exact), render only dirty tails; stitch by remux;
   checkpoint per segment for resume (Android 6 h FGS budget, docs 14/15).

## 4. Memory: caps, pools, streaming, eviction

| Platform | Cap (VERIFIED unless noted) | Design consequence |
|---|---|---|
| iOS Safari WebContent | jetsam hard kill ~2 GB ("ActiveHard 2048 MB"; docs 12/18) | working set target < ~1 GB; pools capped; snapshot often (R-03) |
| wasm32 browser | 4 GiB addressable ceiling; wasm64 shipped Chrome 133 / Firefox 134, Safari UNVERIFIED, Rust nightly-only (doc 12) | media buffers live OUTSIDE wasm linear memory (JS/GPU land); heap stays < 4 GB (doc 12 §4) |
| Android app | hard heap limit per app/device; heap not defragmented (developer.android.com memory-overview, fetched 2026-09-24) | query limit at runtime; MediaCodec buffers live in platform memory (doc 16) |
| Desktop native | physical RAM; no hard API cap | still cap pools (configurable) — decoded-frame arithmetic below |

Decoded-frame arithmetic (derived): 1080p YUV420 8-bit ≈ 3.1 MB/frame; 4K ≈ 12.4 MB.
A 48-frame 1080p pool ≈ 150 MB; a naive 300-frame 4K hold ≈ 3.7 GB — instant jetsam.

Rules:
1. **Decoded-frame pools everywhere** — WebCodecs `VideoFrame.close()` discipline
   (leak = pipeline stall; doc 11 §7); OpenCut's texture_pool/texture_store is the
   GPU-side analog (doc 02 §1); libobs pools textures per tick (doc 08 §3).
2. **Streaming decode, no full-file RAM:** demux→decode→consume frame-by-frame
   (Mediabunny streaming I/O, doc 11 §5; ffmpeg.wasm 4 GB lesson, doc 12 §5-R2) —
   "load the file" architectures are non-viable under the wasm ceiling.
3. **Eviction = LRU keyed on (track, time) window around the playhead** (±W seconds,
   W from playback rate); pinned = anything in the in-flight composite; proxy media and
   render-cache tiles are always evictable (regenerable, §2–3). Persisted (OPFS +
   persist()) vs evictable split per doc 18 §2.
4. Per-command and per-clip memory caps in the core API (doc 33 §9 rule 3) — memory
   exhaustion is also a security failure mode.

## 5. Parallelism

**Decode/encode threading (VERIFIED from FFmpeg n9.0.2 libavcodec/avcodec.h, S-3e2):**
- `FF_THREAD_FRAME`: decodes multiple frames at once — **adds one frame of latency per
  thread**; clients that cannot provide future frames must not use it. → scrubbing and
  low-latency preview: thread_count=1 or SLICE.
- `FF_THREAD_SLICE`: decodes parts of one frame at once → latency-friendly parallelism
  where the codec supports it (per-codec support varies; UNVERIFIED per codec).
- One `AVCodecContext` is not thread-safe for concurrent sends → decoder pool, one
  context per worker (doc 05 §6). Encoders are mostly frame-threaded internally;
  export path = frame threading + throughput.

**Pipeline stage overlap (doc 18 §1 shape):** decode workers ∥ compositor worker ∥
export workers with bounded queues; backpressure is the app's job (WebCodecs gives
`decodeQueueSize`, doc 11 §1); audio runs on the worklet thread. Never run decode or
composite on the UI thread (doc 18 rule).

**Headless data-parallel (VERIFIED model, S-3e3):** Remotion Lambda = "distributed
video rendering system. The video is split into chunks that can be rendered
concurrently"; concurrency = frameCount / framesPerLambda, default ≥20 frames per
chunk. OVE headless (doc 30, ADR-029) adopts the shape without the serverless parts:
deterministic i64 frame-range sharding → per-range segment renders → stitch via
stream copy where codec params match (E-007b); determinism from exact time (ADR-007)
+ replayable log (E-003) means any shard reruns bit-exact.

**GPU async compute (wgpu):** wgpu 30.0.1 `Features::TIMESTAMP_QUERY` and
`TIMESTAMP_QUERY_INSIDE_ENCODERS` VERIFIED (docs.rs) → per-pass GPU timing in the
pass graph (doc 08 §5). Async-compute overlap (compute on a second queue behind
graphics) is a native-backend idea; WebGPU's single-queue model makes the portable
version "reorder passes + rely on driver overlap" — UNVERIFIED, measure in E-005.

## 6. Profiling plan + perf CI

- **Core instrumentation:** `tracing` 0.1.44 + `tracing-subscriber` 0.3.23 (VERIFIED
  crates.io 2026-09-24). Spans: per command apply, per decode GOP, per pass-graph
  execution, per export segment; no sampling inside inner loops (rational add, blend)
  — those are criterion territory.
- **Chrome tracing integration:** `tracing-chrome` 0.7.2 emits Chrome Trace Event
  Format (S-3e7: phases B/E/X, counters, async events; consumed by chrome://tracing
  and Perfetto). Crate is stale (last publish 2024-03) — pin or vendor a fork.
  Webview-side profiling reads the same format as Chrome DevTools (E-006 harness).
- **GPU timing:** wgpu timestamp queries (above) reported next to tracing spans;
  readback rate-limited (doc 08 §5 rule 7 — readback is the expensive boundary).
- **Microbenchmarks:** criterion 0.8.2 (VERIFIED) for rational/gap-buffer/IPC kernels;
  baselines = E-002/E-002c (3.41 ms 20k-clip build; ~140 ns/add; 1.1 ms/5000 splits).
- **Perf CI budgets (regression gates):** fail PR when any of: 20k-clip build/walk
  regresses >10%; 1.37 MB IPC payload binary path exceeds 200 µs (E-006 baseline 78 µs);
  decode fps floor per tier missed on reference runner; composite frame-time p95
  > 16.6 ms budget share. Absolute numbers are runner-specific — gate on ratios and
  recorded baselines (E-007b lesson: "RATIOS are the durable result").
- **Field telemetry (opt-in only):** Clypra's sampling shape — 1% when smooth at
  60 fps, 100% on dropped frames; fields: decode µs, compose µs, P95 seek, drop
  counters, OS/GPU vendor (doc 02 §4). Default-on telemetry is an anti-pattern we
  explicitly reject.

## 7. Engine rules (v1, performance)

1. Preview drops quality tiers; export never drops frames.
2. No steady-state per-frame allocations: pools for frames, textures, segments.
3. JSON IPC ≤ ~10 KB; ≥100 KB payloads take the binary path; never frames via JSON (E-006).
4. Color/format conversion exactly once, at the boundary (doc 04 §1.9).
5. Copy-not-re-encode whenever a region is untouched (E-007/E-007b planner).
6. Incremental/prefix-sum start maintenance; rational adds are the walk cost (E-002c).
7. Platform-aware pool caps; LRU eviction on the (track, time) playhead window.
8. Proxy generation by threshold, hash-keyed, always relink-verified.
9. Headless renders shard by frame range; stitch via stream copy; reruns are bit-exact.
10. tracing spans always compiled in; sampled at runtime; criterion gates in CI.

## 8. ADR relevance

- **ADR-036 (Performance):** this doc is the evidence base — budgets §1, threading §5, gates §6.
- **ADR-032 (Proxy system):** §2 thresholds/policies/relink.
- **ADR-031 (Cache):** §3 render-cache tiles + §4 eviction are the cache spec's first draft.
- Feeds ADR-002 (platform adapter: pool/cap contracts live in the adapter trait).
- PROPOSED risk-register additions (register not edited by this agent): R-13
  proxy/relink cache-consistency bugs (wrong media silently shown); R-14 perf
  regressions across the platform matrix without reference-hardware CI.

## Sources (fetched 2026-09-24 unless noted)

- S-3e2 FFmpeg n9.0.2 libavcodec/avcodec.h — FF_THREAD_FRAME/FF_THREAD_SLICE semantics.
- S-3e3 Remotion Lambda concurrency docs — chunked data-parallel render model.
- S-3e7 Chromium "Trace Event Format" — Chrome tracing JSON spec.
- S-3e9 crates.io API sweep — tracing 0.1.44, tracing-subscriber 0.3.23, tracing-chrome
  0.7.2, criterion 0.8.2, blake3 1.8.7, wgpu 30.0.1.
- ffmpeg.org/ffmpeg-codecs.html §9.30 ProRes (prores-ks) — native encoder; developer.android.com
  memory-overview — hard heap limits. Internal: E-002c, E-006, E-007, E-007b records;
  docs 02/04/05/08/11/12/14/15/17/18.

## UNVERIFIED / NOT-FOUND (this pass)

- OpenCut and Clypra performance claims (no methodology; treated as absent).
- Kdenlive proxy settings docs, Premiere smart-rendering helpx (403), FCP proxy page
  (redirects away) — proxy *thresholds in shipped editors* have no fetched primary doc.
- Apple ProRes SDK licensing page (NOT-FOUND); DNxHR encode/SDK terms (NOT-FOUND).
- FF_THREAD_SLICE per-codec support table; per-codec wasm decode costs (E-001 pending);
  wgpu multi-queue async-compute overlap; exact current iOS jetsam thresholds (device test).

## Depth remaining (v0.2)

- E-008 (PROPOSED): measured frame-budget breakdown per stage on pinned reference
  hardware (desktop + Android + browser) — replaces the proposed §1 table.
- E-009 (PROPOSED): proxy throughput + relink fuzzing (hash/size/mtime collisions,
  mid-GOP traps). E-010 (PROPOSED): range-parallel scale-out crossover.
- Measure iOS Safari jetsam on device (R-03); tracing-chrome maintenance check;
  run E-005 for GPU timestamp-query pass timing.
