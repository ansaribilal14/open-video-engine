# W3d REPORT — wave-3 research agent (docs 30 HEADLESS, 31 STORAGE)

> Task 14-d-retry · 2026-09-24 · GITHUB_TOKEN unavailable (all fetches unauthenticated:
> raw.githubusercontent.com, official doc sites, crates.io API with custom User-Agent
> "ove-research/0.1", npm registry). No engine code touched; no other repo files
> modified. Worklog not appended (read-only).

## Retry-pass summary (this session)

The prior attempt timed out AFTER writing all four deliverables (docs 30/31 were
complete 226/218-line docs, LEDGER_W3d.md had 10 rows, this report existed), but the
worklog never recorded it and the honesty of its VERIFIED markers could not be
assumed. This session therefore re-ran the verification surface fresh (~40 targeted
probes) before adopting the files:

- VERIFIED FRESH (doc 30): Remotion how-lambda-works (Serve URL, Response Streaming,
  progress.json), concurrency (framesPerLambda, default ≥20 frames), limits (1000
  concurrent/region, 10 GB disk, 15 min), cancellation since 4.0.515, webhooks
  (X-Remotion-Signature, customData ≤1KB, isFatal/willRetry), cost example $0.001
  Hello World, random(seed) determinism, two-tier license page, roll-your-own recipe
  (frameRange + pcm-16 + FFmpeg concat, "not recommended"), optimizing-cost
  asset-download hazard. Shotstack OAS at FILE level: package.json 1.18.3, LICENSE
  404, edit/timeline/clip/output YAMLs (deprecated disk, instance s1/s2/a1, {{NAME}}
  merge, seconds-float + auto/end/alias://, client ids) and
  responses/renderresponsedata.yaml (8-state enum queued→fetching→preprocessing→
  rendering→generating→saving→done|failed; temporary URL "deleted after 24 hours").
  Creatomate api.md (POST /v2/renders, 202, dry_run free validation, 30-day expiry,
  {hint,documentation}, 30 req/10s + X-RateLimit-Remaining + Retry-After, v1
  batch-by-tags). JSON2Video llms.txt (v2/movies, quality/draft/cache/exports/
  client-data, duration -1 auto-fit, condition, transition). Diffusion Studio (npm
  dist-tags latest 4.0.3 @ 2025-11-30, MPL-2.0 LICENSE). NVENC app note (qualified vs
  non-qualified; "limited to 12 per system … combined") + container-toolkit
  components. x265 frame-threads description verbatim. SVT-AV1 Docs/CommonQuestions.md
  verbatim ("scale well across many logical processors"; "the video output will be the
  same when using `--lp 1` as `--lp n` in the default CRF configuration"; "about 16
  processor cores when encoding 1080p … 4-6 range"; scene-based chunk splitting) +
  LICENSE.md = BSD-3-Clause Clear + PATENTS.md. Transloadit llms.txt (/video/encode
  robot, ":original"). Netflix Maestro README (WAAS, millions of jobs/day).
- VERIFIED FRESH (doc 31): blake3 1.8.7 (crates.io max_version, 2026-08-20, tri-license
  string, 187,039,163 downloads) + README (Merkle/verified streaming/incremental,
  SIMD incl. NEON+WASM, rayon). web.dev storage (Chrome 80%/60%, incognito 5%, 300MB,
  Firefox 50% free disk). WHATWG fs.spec exclusive-lock sentence verbatim. sqlite.org
  persistence.md verbatim (no "N concurrent readers"; historically ~3, 2026-03 testing
  8-10 workers; non-reset statements lock; never two handles per thread; ms chunks;
  opfs-unlock-asap; sqlite3_js_retry_busy added 3.53.0; Safari <17 + COOP/COEP +
  incognito Achtungs). FFmpeg proresenc_anatoliy.c/proresenc_kostya.c/dnxhdenc.c LGPL
  headers; dnxhr_444/hqx/hq/sq/lb profiles; ffmpeg-codecs.html "9.30 ProRes" +
  prores-aw/prores-ks encoders; ffmpeg.html -bitexact wording. Apple ProRes white
  paper reachable (301→Apple_ProRes.pdf, 206, %PDF): "approximately 45 Mbps at
  1920 x 1080" (422 Proxy) + "Apple has licensed ProRes" (pdftotext extract).
- CORRECTED (1 factual error found by re-verification): doc 30 §1.5 and ledger S-3d4
  claimed a "WebGPU/WebGL renderer" for Diffusion Studio core — the README states
  frames are painted via **Canvas 2D Context** over WebCodecs frames. Fixed in both
  files with an explicit correction note.
- DOWNGRADED TO UNVERIFIED (1 item): Remotion "2048 MB default RAM" — the
  /docs/lambda/memory-size path now 404s and rendermediaonlambda page grep did not
  surface the default; limits page confirms only 10 GB RAM/10 GB disk ceilings. Doc 30
  §1.1 now flags it; ledger S-3d0 annotated; doc 30 UNVERIFIED list extended.
- Search-engine egress still blocked (DDG html returned http 000 after 15 s) — the
  prior NOT-FOUND entries (Netflix "eyrie", etc.) remain honestly NOT-FOUND.

## Docs written

| File | Status | Lines |
|---|---|---|
| docs/research/30_HEADLESS.md | PARTIAL (v0.1 — wave-3 depth pass, retry-verified) replacing PLANNED stub | 230 |
| docs/research/31_STORAGE.md | PARTIAL (v0.1 — wave-3 depth pass, retry-verified) replacing PLANNED stub | 218 |

Both follow the required format: `# NN_TITLE`, `> Status: PARTIAL (v0.1 — wave-3
depth pass 2026-09-24).`, VERIFIED/UNVERIFIED markers, [S-3dx] inline citations,
UNVERIFIED/NOT-FOUND list, depth-remaining list, division-of-labor notes against
docs 18/19/32 (coordination, no duplication: doc 32 §2 kept proxy thresholds; 31
adds the verified codec/license facts and the storage substrate).

## Key findings

Headless (30):
1. Remotion Lambda architecture verified end-to-end (main/renderer Lambda split,
   framesPerLambda chunking ≥20 frames, Response Streaming concat, S3 progress.json,
   typed signed webhooks, cancel, 15-min/10-GB limits, GB-sec pricing, $0.001
   HelloWorld, per-chunk asset fan-out hazard, source-available two-tier license).
2. Request-schema precedents captured from 4 vendors: Shotstack Edit JSON
   (seconds-float timing + 8-state lifecycle incl. mandatory preprocessing),
   Creatomate template+modifications (request = diff vs saved timeline; free
   dry_run validation; hint contracts), JSON2Video scene-sequential movie JSON
   (+client-data passthrough), Transloadit Robot DAG (`use`-edges).
3. NVENC current policy (SDK 13.1 app note): qualified GPUs = resource-limited;
   non-qualified (GeForce) = 12 concurrent sessions per system combined.
   nvidia-container-toolkit components + Apache-2.0 verified.
4. Deterministic CPU fallback: SVT-AV1 output identical for `--lp 1` vs `--lp n`
   (parallelism without bit changes) + ~16-core efficiency at 1080p presets 4–6 +
   scene-chunked parallelism recommended — matches E-007b segment scheduler;
   x265 frame-threads/pools semantics verified; GPL-vs-BSD license split recorded.
5. Determinism hazard table (fonts, color, encoder/thread/GPU nondeterminism,
   randomness, SOURCE_DATE_EPOCH) + FFmpeg `-bitexact` verified; bit-comparability
   scoped per (engine version, output profile, GPU class).
6. Job model: render_id from content hashes for idempotency; done-file-per-segment
   checkpoint format (hybrid frame-range sharding + E-007b keyframe copy);
   Android FGS 6 h/24 h budget (doc 14) makes checkpoints mandatory, not optional.
   No surveyed API has idempotency-key semantics — recorded as an opportunity.

Storage (31):
1. BLAKE3 1.8.7 verified (license `CC0-1.0 OR Apache-2.0 OR Apache-2.0 WITH
   LLVM-exception`; Merkle/verified-streaming; WASM SIMD) = one identity primitive
   across desktop/browser; content-addressed `assets/<xx>/<hash>` pool with
   idempotent ingest and hash-based offline/relink state (extends doc 32 §2 rules).
2. Cache hierarchy L1–L5 defined (GPU in-flight → CPU ring → tiles → proxy/peaks/
   thumbs → originals); decoded frames never persisted; invalidation = append-only
   content keys `(asset-hash, effect-chain-hash, params-hash)` — no timestamp
   invalidation races by construction.
3. Browser deep-dive beyond doc 18: WHATWG FS spec exclusive-lock semantics for
   sync access handles; quota numbers verified (Chrome origin ≤60% disk, incognito
   ~5%, ~300 MB clear-on-close; Firefox ≤50% free); Best-Effort vs Persistent +
   persist() policy; sqlite.org OPFS concurrency (no concurrent readers; 8–10
   workers sustained 2026-03 with minimal locking; practical locking rules).
   Decision rule: SQLite-WASM for the project index, OPFS files for media blobs,
   never blobs-inside-DB.
4. Codec facts upgraded: FFmpeg native ProRes (LGPL headers proresenc_anatoliy/
   kostya) and DNxHR encoder (dnxhdenc.c LGPL; profiles dnxhr_444/hqx/hq/sq/lb —
   resolves doc 32's UNVERIFIED flag); ProRes 422 Proxy ≈45 Mbps @1080p from
   Apple's white paper; H.264 proxies = license-clean platform encoders.
5. Portability: folder-project recommended as native format (zip = derived share
   bundle with dedup'd subset), EDL-style external refs default — consistent with
   doc 19 Candidate A; storage v1 layout sketched (manifest + assets/ + cache/
   disposable + renders/ checkpoint dirs + rebuildable sidecar db).

## ADR-relevant decisions (inputs, not decisions)

- ADR-029 (headless, future): evidence base = doc 30 §1–§5; recommended shape:
  library-first `ove-headless` (command log → render plan → segment scheduler),
  done-file checkpoint format, manifest-scoped bit-comparability.
- ADR-030 (storage, future): folder-project layout + BLAKE3 content addressing;
  manifest+assets are the only non-regenerable data.
- ADR-031 (cache, future): key = (asset-hash, effect-chain-hash, params-hash);
  L1–L5 layering; persist()/evictable split on web.
- ADR-032 (proxy, future): doc 32 §2 remains thresholds owner; doc 31 §2.2 adds
  verified codec/license facts; final codec choice still → doc 35 (license audit).

## NOT-FOUND / fetch failures (all logged honestly)

- Netflix "eyrie": GitHub 404 + search engines (DDG html/lite) time out from this
  environment → NOT-FOUND after genuine attempts (retry pass: DDG still http 000).
- Netflix/video-transcoding-api: GitHub 404 (existed historically; unrecoverable).
- OSS "ffmpeg REST wrapper" survey: probed candidate repos all 404; commercial
  vendors + Remotion FAQ used as evidence instead.
- Adobe helpx (Premiere media cache/relink/proxy docs): HTTP 403 bot-blocked.
- KDE userbase (Kdenlive manual): Cloudflare 403. docs.kde.org paths 404.
- PyPI `diffusionstudio*` (4 name variants): all 404 → Python-era claims marked
  UNVERIFIED in doc 30 (correction vs assignment brief).
- api.github.com: core quota exhausted (concurrent agents) → HTML/raw probes used.
- Apple/Avid SDK licensing pages: Apple licensing article not located (white paper
  fetched instead); Avid 403/404 → left to doc 35.
- DuckDuckGo search: both html and lite endpoints timed out (120 s) — no search
  engine access this session (retry pass: still 000).
- Remotion /docs/lambda/memory-size: now 404 → 2048 MB default-RAM figure moved to
  UNVERIFIED in doc 30.

## Source IDs used

S-3d0 Remotion Lambda docs · S-3d1 Shotstack OAS repo · S-3d2 Creatomate llms docs ·
S-3d3 JSON2Video llms.txt · S-3d4 Diffusion Studio core · S-3d5 NVIDIA SDK 13.1 app
note + container toolkit · S-3d6 x265 CLI + SVT-AV1 · S-3d7 blake3 crates.io + repo ·
S-3d8 web.dev storage + WHATWG FS spec · S-3d9 primary-doc bundle (sqlite.org wasm
persistence, FFmpeg -bitexact/codecs/libavcodec headers, Apple ProRes white paper,
Transloadit llms.txt, reproducible-builds, Netflix Maestro). Rows in
research/sources/LEDGER_W3d.md (SOURCE_LEDGER.md untouched), header annotated with
the retry-pass verification list.
