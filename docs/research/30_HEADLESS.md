# 30 — HEADLESS / SERVER RENDERING (Track R)

> Status: PARTIAL (v0.1 — wave-3 depth pass 2026-09-24).
> Owner: agent W3-d. Scope: render-as-a-service survey (request schemas = command-API
> precedents), server GPU strategy, deterministic render contract, job model
> (idempotency/progress/cancel/resume), OVE proposal. Respects ADR-007 exact time,
> E-003 command log, E-007b segment-copy economics, doc 14 Android FGS 6 h/24 h
> budget (exports MUST be checkpointed/resumable). Feeds ADR-029 (future). Doc 32 §5
> owns the perf-side sharding overlap; this doc owns the server/service column.
> Cites S-3d0..S-3d9 (research/sources/LEDGER_W3d.md).

## 1. Render-as-a-service survey

### 1.1 Remotion Lambda — the most-documented distributed renderer (VERIFIED, S-3d0)

- One **main function** (`renderMediaOnLambda()`) opens the *Serve URL* in headless
  Chrome and runs props resolution → duration in frames; video is **split into
  chunks** (`concurrency = frameCount / framesPerLambda`, default ≥ 20 frames per
  lambda) and one **renderer function** per chunk renders it in a headless browser.
- Renderer functions stream binary chunks + progress back via **AWS Lambda Response
  Streaming**; the main function concatenates (algorithm NOT public API: "Building a
  distributed renderer is hard, and not recommended") and uploads the final video to S3.
- Progress = `progress.json` on S3 polled by `getRenderProgress()`; webhooks are
  **typed** (`success` w/ outputUrl+costs+timeToFinish+lambdaErrors{frame, chunk,
  isFatal, willRetry, attempt} | `error` | `timeout`), signed, with `customData`
  ≤ 1 KB passthrough. Cancelation since v4.0.515.
- Pricing = Lambda GB-sec + requests (10 GB disk from Remotion 5.0 — VERIFIED on
  limits page; 2048 MB default-RAM figure NOT re-confirmed this pass, see
  UNVERIFIED); measured HelloWorld render $0.001; documented levers: less memory,
  fewer functions (cold starts/browsers dominate), cheaper regions, precomputed props.
- Limits: 1000 concurrent/region default (quota-increase flow documented), 10 GB disk,
  10 GB RAM, **15 min execution**. License = **source-available two-tier** (free ≤ 3
  employees; company license above) → doc 35.
- Stated escape hatch: roll your own with `frameRange` + `audioCodec: "pcm-16"` + FFmpeg
  concat. Documented hazard: **each chunk downloads all assets it references** (fan-out
  bandwidth; CDN advised).

### 1.2 Shotstack — Edit JSON (VERIFIED from OAS repo, S-3d1)

`shotstack/oas-api-definition` main (@shotstack/schemas 1.18.3; **no LICENSE file in
repo** — reuse question → doc 35):

- `Edit = {timeline, output, merge?, callback?, disk(deprecated), instance(s1|s2|a1)}`;
  `Timeline = {tracks[{clips[]}], fonts[], background, cache}`; `Clip = {asset, start,
  length, …}` with **seconds-float timing** or smart-clip strings (`auto`, `end`,
  `alias://clip-name`); client-generated clip `id` for cross-edit references.
- `Output = {format: mp4|gif|mp3|jpg|png|bmp, resolution preset|size}`.
- Lifecycle (VERIFIED response schema): `queued → fetching → preprocessing → rendering
  → generating → saving → done|failed`; POST → 201 + id; result URL **temporary
  (24 h)**. `merge` = template placeholder substitution ({{NAME}}-style).
- Documented **automatic preprocessing** of video assets "to fix common compatibility
  issues" (per-asset `transcode` flag) — vendor-level admission that raw user inputs
  break naive FFmpeg pipelines (§1.6 evidence).

### 1.3 Creatomate — template + modifications (VERIFIED, S-3d2)

- Model: template = stored RenderScript (their JSON timeline); render = `POST /v2/renders
  {template_id, modifications}` or inline RenderScript; `modifications` = fill elements /
  change properties / remove elements / replace scene children — the request is a **diff
  against a saved timeline** (strong precedent for AI-agent render requests, docs 26/27).
- `202` → poll `GET /v2/renders/:id`; artifacts **expire after 30 days**; `dry_run: true`
  validates without credits/queueing (`{valid, errors, warnings}`).
- Every error returns `{hint, documentation}`; rate limit 30 req/10 s per account with
  `X-RateLimit-Remaining` + `Retry-After`; v1 adds batch-by-tags.

### 1.4 JSON2Video — movie JSON (VERIFIED, S-3d3)

- `Movie = {resolution preset|custom, quality, scenes[], elements[] (global overlay),
  template?, variables?, exports[], draft, cache, client-data}`; `POST /v2/movies`;
  status **pending|running|done|error**.
- `Scene` = sequential segment ("scenes play sequentially and cannot overlap"), `duration:
  -1` = auto-fit, `condition` skip-expression, `transition`, scene-local `variables`.
- Typed elements: video/image/text/component (HTML5 animated)/audio/voice (TTS azure|
  elevenlabs)/audiogram/subtitles. `client-data` passthrough ≡ Remotion `customData`.

### 1.5 Diffusion Studio core (VERIFIED with corrections, S-3d4)

- `@diffusionstudio/core` 4.0.3 (npm 2025-11-30, MPL-2.0; npm description: "A fast,
  browser based video compositing engine powered by WebCodecs"); README highlights
  declarative timeline, rich text/captions, silence removal, font management,
  **checkpoints**, "high fidelity rendering mode"; paint path = **Canvas 2D Context**
  over WebCodecs frames (NOT WebGPU/WebGL — corrected in retry pass). Headless use =
  the Remotion-class pattern: **drive a browser engine from headless Chrome/Playwright**.
- CORRECTION vs brief: current repo is TypeScript end-to-end (tags v1.0.0-rc…v2.0.0 all
  TS); "Python+Rust" era NOT verifiable — PyPI probes `diffusionstudio*` all 404. Mark
  Python-era claims UNVERIFIED.

### 1.6 Orchestrators, wrappers, NOT-FOUND, failure modes (S-3d9, S-3d5)

- **Transloadit** (VERIFIED llms.txt): request schema = **Robot DAG** —
  `{"steps": {"encoded": {"robot": "/video/encode", "use": ":original", "preset": "mp4"}}}`,
  INPUT→PROCESS→OUTPUT with explicit `use` edges + export robots. Precedent: jobs as
  declarative step graphs, not imperative scripts.
- **Netflix Maestro** (VERIFIED README): general-purpose **data/ML workflow orchestrator**
  (Java, WAAS, "millions of jobs every day", K8s/AWS extensions). NOT a video render
  service; relevant only as the orchestration layer above a render farm.
- NOT-FOUND after genuine attempts: **Netflix "eyrie"** (GitHub 404; DDG search engines
  time out from this env); **Netflix/video-transcoding-api** (GitHub 404 — existed
  historically, content unrecoverable); OSS "ffmpeg REST wrapper" probes (all 404).
- Failure modes distilled from VERIFIED vendor design (analysis): (a) input heterogeneity
  → mandatory preprocessing stage (Shotstack); (b) per-chunk asset fan-out → bandwidth
  blowups (Remotion FAQ); (c) chunk stitching is the hard part (Remotion keeps concat
  private); (d) artifacts ephemeral (24 h/30 d) — copy out immediately; (e) webhooks
  typed+signed with frame/chunk error detail; (f) no surveyed API has Stripe-style
  idempotency keys — an opportunity, not a convention.

## 2. Server GPU strategy

- **NVENC session policy (VERIFIED, Video Codec SDK 13.1 NVENC Application Note):**
  GPUs are "qualified" (data-center/pro) vs "non-qualified" (GeForce). Qualified:
  concurrent sessions limited only by resources (encoder capacity, system/video memory).
  Non-qualified: **12 concurrent encode sessions per system**, combined across ALL
  non-qualified cards. Historical 3→5→8 progression: UNVERIFIED. Scheduler consequence:
  per-node consumer-GPU encode budgets are required; never assume unlimited sessions.
- **Containerized rendering (VERIFIED):** NVIDIA Container Toolkit (Apache-2.0) =
  `nvidia-container-runtime`, `nvidia-ctk`, `nvidia-cdi-hook`,
  `nvidia-container-runtime-hook`, `nvidia-container-cli`, `libnvidia-container1`;
  CDI mode is the docs' current headline path for GPU containers.
- **Scale-to-zero economics (VERIFIED model):** Lambda bills per GB-second (1 ms
  granularity) + requests; Remotion's own cost docs admit cold-start/browser/asset
  overheads. Serverless wins spiky sub-minute jobs; GPU VMs win long exports (no
  15-min/10-GB caps, real NVENC); CPU-only fleet as overflow:
- **No-GPU fallback thread scaling (VERIFIED docs):** x265 `--frame-threads` (1
  frame-thread = better compression; over-allocation "will not improve performance, it
  will generally just increase memory use") + `--pools` NUMA pools + WPP. SVT-AV1:
  "designed to scale well across many logical processors", ~**16 cores at 1080p presets
  4–6**; output is **the same for `--lp 1` as `--lp n`** in default CRF config
  (parallelism without bit changes — server-grade determinism); recommends **scene-based
  chunk splitting** for more parallelism = exactly the E-007b segment scheduler. License:
  SVT-AV1 BSD-3-Clause-Clear + AOM Patent License 1.0 (VERIFIED) vs x264/x265 GPL-2.0+
  (S-2c1) — CPU pool can be GPL-free if AV1-based; H.264/5 fallback = separate GPL build
  (doc 05 §5).

## 3. Deterministic render contract

Definition: same (project snapshot + command-log prefix + assets by content hash +
engine + output profile) ⇒ bit-comparable output. Hazard list + mitigations:

| Hazard | Failure | Mitigation |
|---|---|---|
| fp-seconds time math | frame-selection drift (ADR-007) | integer/rational frames only in the render plan |
| font versions/fallback | glyph rasterization differs | fonts are hashed project assets (Shotstack `fonts[]` precedent) |
| color stack | 601/709 mix; tone-map/libplacebo version skew | fixed working space + tags (doc 22); effect-graph hash in manifest |
| encoder version/settings | bitstream + encoder-tag drift | pin encoder per profile; FFmpeg `-bitexact` (VERIFIED: "Enable bitexact mode for (de)muxer and (de/en)coder") |
| thread nondeterminism | output varies with thread count | SVT-AV1 `--lp` invariance (VERIFIED); x264 cross-thread bit-equality UNVERIFIED (E-00x) |
| GPU nondeterminism | reduction/atomic order in blend passes | deterministic kernels for export; CPU reference path for certification runs |
| randomness | `Math.random()` breaks reruns | seeded PRNG only (Remotion `random(seed)` VERIFIED: same seed → same output) |
| plugins/AI models | versioned behavior | plugin + model hashes recorded (docs 28/25) |
| clock/timestamps | volatile container metadata | `SOURCE_DATE_EPOCH` convention (reproducible-builds.org, VERIFIED) |
| media identity | wrong source bytes | hash-addressed assets; relink rules (doc 32 §2) |

Bit-comparability is claimable only per (engine version, output profile, GPU class) —
the artifact manifest records all three. Cross-version byte equality is NOT promised;
replay determinism of the *plan* is (E-003).

## 4. Job model

- **Idempotency:** `render_id = hash(snapshot id, log position, output profile, engine
  version)` → resubmission converges on the same id and short-circuits on a completed
  checkpoint dir. Precedents: Shotstack client clip ids; Creatomate dry-run; no surveyed
  API implements idempotency-key headers (§1.6-f).
- **Progress:** coarse state machine (Shotstack's 8 states = most granular VERIFIED) +
  fine counters (frames_done/total, per-shard heartbeats = Remotion progress.json);
  typed+signed webhooks carrying frame/chunk/isFatal/willRetry (Remotion VERIFIED);
  ≤ ~1 KB `customData/client-data` passthrough.
- **Cancelation:** stop scheduling shards, drain in-flight, finalize partial manifest
  (Remotion cancel since 4.0.515; partial ≡ resume state below).
- **Partial results:** output is always an **artifact manifest** (segment list +
  concatenation plan + sidecars poster/peaks) — partial and resumable are the same
  mechanism.
- **Checkpoint/resume (hybrid, E-007b + doc 14):** shards are frame-range windows
  (Remotion-verified shape) with per-segment strategy `copy|encode`; untouched regions
  are keyframe-indexed stream copies (6.8–11.9× at 1080p, E-007b; keyframe index
  mandatory per E-007); every completed segment writes a **done-file with hash** (§5).
  Resume = load manifest, verify done-files, re-run missing shards only. Mandatory, not
  optional: Android mediaProcessing FGS 6 h/24 h budget + onTimeout() (doc 14) forces
  checkpointed exports; the same mechanism gives spot-instance tolerance server-side.

## 5. OUR proposal: `ove-headless`

- One core, two surfaces: **library mode** (`render_plan(project, profile) -> Plan`,
  `execute(plan, checkpoint_dir)`) and **CLI mode** (`ove render project.ove --profile …
  --resume`). The HTTP wrapper (job queue, storage, webhooks) is a thin server crate;
  multi-tenant service is a v1 non-goal.
- Pipeline: **command log → render plan → segment scheduler**. The plan is pure data:
  exact integer frame ranges (ADR-007), per-segment strategy from the E-007b planner,
  per-segment asset set (Remotion fan-out lesson: planner prefetches/dedupes per shard).
- Checkpoint format v0 (proposal):
  `renders/<render_id>/manifest.json` (plan hash, profile, engine, shard table) +
  `segments/<n>.mp4` (copy-segments stay bit-exact) + `segments/<n>.done.json`
  (`{plan_hash, range, BLAKE3, bytes, strategy}`). Done-file = idempotency marker;
  concatenation consumes only hash-verified segments → a killed worker resumes
  without trust.
- ADR-029 (future) should decide: library-first API (recommended), CLI stability policy,
  checkpoint-format versioning (Olive per-version serializer pattern, doc 19 §1.7), and
  whether Maestro-class DAG orchestration / Transloadit-style robot graphs are an
  integration target or out of scope.

## UNVERIFIED / NOT-FOUND (this pass)

- Netflix "eyrie" (404 + no search-engine access); Netflix/video-transcoding-api (404);
  OSS ffmpeg-REST wrappers (probes 404).
- Historical NVENC session progression (3/5/8); only the current 12-session policy is
  verified. Toolkit CDI-on-k8s behavior at scale unprobed.
- Remotion 2048 MB default-RAM figure (memory-size doc path 404s this pass; limits
  page confirms only 10 GB RAM ceiling + 10 GB disk).
- Diffusion Studio Python+Rust era (PyPI 404 ×4; TS at all fetched tags).
- Shotstack OAS repo license (no LICENSE file) → doc 35.
- x264/x265 cross-thread-count bit-equality (only SVT-AV1 `--lp` invariance verified).

## Depth remaining (v0.2)

- E-00x determinism harness: render one project on two runners (CPU + GPU pool), byte-
  compare segments under the §3 manifest.
- Cost curves: serverless vs GPU-VM vs CPU-pool at 1/10/60-min jobs.
- Study Remotion's concat boundary via the public frameRange+pcm-16 recipe; design the
  OVE stitch (remux copy) against E-007b planner output.
- Scale-API schema comparison (ByteDance/Pixocial-class) — blocked on search egress;
  retry from another network.

## Sources (fetched 2026-09-24; rows in LEDGER_W3d.md)

S-3d0 Remotion Lambda docs bundle · S-3d1 Shotstack OAS repo · S-3d2 Creatomate llms
docs · S-3d3 JSON2Video llms.txt · S-3d4 Diffusion Studio core repo/npm · S-3d5 NVIDIA
SDK 13.1 app note + container-toolkit docs · S-3d6 x265 CLI docs + SVT-AV1 repo ·
S-3d7 BLAKE3 crates.io + repo · S-3d8 web.dev storage + WHATWG FS spec (shared with 31)
· S-3d9 primary-doc bundle (sqlite.org wasm persistence, ffmpeg.org -bitexact/codecs +
libavcodec headers, Apple ProRes white paper, Transloadit llms.txt, reproducible-builds,
Netflix Maestro README). Internal: E-003, E-007, E-007b; docs 14/15, 19, 22, 26/27, 32;
ADR-007.
