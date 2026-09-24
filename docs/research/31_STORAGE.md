# 31 — STORAGE: project, cache, proxy substrate (Track Q)

> Status: PARTIAL (v0.1 — wave-3 depth pass 2026-09-24).
> Owner: agent W3-d. Scope: local-first project layout + content-addressed asset
> identity, cache hierarchy (decoded frames, proxy, waveform/peaks, thumbnails) and
> invalidation keys, browser storage deep-dive (beyond doc 18), project portability,
> storage v1 proposal. Division of labor: doc 19 owns serialization formats; doc 32
> §2 owns proxy thresholds/policies and §4 memory caps; doc 30 §5 owns render
> checkpoints; this doc owns the storage substrate. Feeds ADR-030 (storage),
> ADR-031 (cache), ADR-032 (proxy) — all future. Cites S-3d0..S-3d9
> (research/sources/LEDGER_W3d.md).

## 1. Local-first desktop: media identity + dedup

- **Content hash = media identity (VERIFIED tooling):** `blake3` crate 1.8.7
  (crates.io, 2026-09-24), license `CC0-1.0 OR Apache-2.0 OR Apache-2.0 WITH
  LLVM-exception`; BLAKE3 is a Merkle tree → **verified streaming + incremental
  updates** (repo README), SIMD (SSE2/4.1, AVX2/512, NEON) + **WASM** feature →
  one hash implementation desktop/browser/Android. Doc 32 §2 already fixed
  relink-by-hash rules (never path-only); storage adds the address space.
- **Asset dedup:** blobs stored content-addressed (`<hex[0:2]>/<hex>`), so dedup
  within a project is automatic and cross-project **media pool** dedup is the same
  lookup; ingest is idempotent (hash exists → no copy). Cheap thanks to BLAKE3
  multithread (`rayon` feature) for multi-GB files.
- **Project file = media + sidecar db:** the manifest references assets by hash +
  probe metadata; a per-project SQLite sidecar holds indexes (clips, keyframe
  indices, peaks pointers) — matching doc 19 Candidate A (snapshot + command log)
  and Resolve's DB precedent (doc 19 §1.10). Nothing embeds large media (VERIFIED
  across OTIO "not a container for media", Kdenlive "stores only a reference",
  FCPXML asset refs — S-2bx, doc 19 §2 notes).
- **Offline/missing-asset workflow (proposal, consistent with doc 32 relink):**
  missing = hash not found in pool/paths → clip enters offline state (timeline
  stays intact, placeholder renders); relink = match by hash, confirm hash after
  path match; (path+size+mtime) is a *hint* only. Offline copies stay in the pool
  as first-class blobs so projects keep working after source-media removal.
- Shipped-editor offline/cache docs: Adobe helpx (media cache, relink, scratch
  disks) → **403 bot-blocked this pass**; KDE userbase → **Cloudflare 403**;
  Resolve manual PDF too large to fetch. Behaviors above are therefore design
  proposals anchored on doc 19's verified format properties, NOT surveys of
  vendor docs (log in UNVERIFIED).

## 2. Cache hierarchy

Layered (L1–L5), hot→cold, each layer regenerable except L5:

| Layer | Content | Resident | Notes |
|---|---|---|---|
| L1 | in-flight decoded frames/textures | GPU | only the in-flight composite (WebGPU guaranteed limits: 256 MiB buffers, 8192-tex, doc 10 §3) |
| L2 | decoded-frame ring (playhead window ±W s) | CPU RAM | pooled; caps per doc 32 §4 (1080p ≈ 3.1 MB/frame; iOS jetsam ~2 GB, doc 18 §4) |
| L3 | render-cache tiles / preview segments | disk (project cache/) | keyed (§2.3); LRU; preview+export reuse when bit-identical (doc 32 §3) |
| L4 | proxy media, waveform peaks, thumbnails | disk (OPFS on web) | doc 32 §2 thresholds; evictable |
| L5 | original media | external refs / assets/ pool | the only non-regenerable layer besides manifest |

2.1 **Decoded frames are never persisted** (GPU-resident vs CPU ring is a memory-
model decision per platform, not a persistence one). L1 residency is bounded by
the in-flight composite; everything else lives in L2's ring. Rationale: frame
caches are cheap to recompute (decode) relative to their size.

2.2 **Proxy format + license implications (extends doc 32 §2; VERIFIED this pass):**
- FFmpeg native ProRes encode: `libavcodec/proresenc_anatoliy.c` and
  `proresenc_kostya.c` = **LGPL** headers (VERIFIED at FFmpeg master); documented
  encoder family in ffmpeg-codecs.html §9.30 (prores-ks options).
- FFmpeg native DNxHD/**DNxHR** encode: `libavcodec/dnxhdenc.c` = **LGPL**
  (VERIFIED); profile options `dnxhr_444|dnxhr_hqx|dnxhr_hq|dnxhr_sq|dnxhr_lb`
  VERIFIED in encoder source. (Doc 32 flagged this UNVERIFIED — now resolved.)
- Apple ProRes 422 Proxy data-rate anchor: **≈ 45 Mbps at 1920×1080** (official
  white paper PDF, VERIFIED); Apple "has licensed ProRes" to authorized licensees
  — trademark/SDK redistribution question → doc 35.
- H.264 low-bitrate proxies via platform encoders (WebCodecs/MediaCodec) carry no
  license exposure (doc 32 §2). Codec-string support ≠ HW support (doc 11).
- 10-bit 4:2:2 sources (ProRes/DNxHR inputs) decode outside the Chromium WebCodecs
  comfort zone (doc 11 §2–3) — proxying is a *decode-capability* decision too, not
  just throughput.

2.3 **Cache invalidation = keys, never timestamps.** Key = `(asset-hash,
effect-chain-hash, params-hash)` where effect-chain-hash = compiled pass/WGSL
list (doc 08 §5) and params-hash = canonicalized effect parameters (E-003
discipline: explicit ids, no floats). All layers L2–L4 use this scheme; entries
are **append-only** (new key = new entry; old evicted by LRU cost model), which
removes invalidation races by construction. Browser mapping: OPFS filenames are
the keys (§3.4).

2.4 **Waveform/peak caches:** per (asset-hash, samples-per-pixel tier), computed
at ingest, stored as blob per bucket; OpenCut's waveform-cache is the surveyed
precedent (doc 02 §1). **Thumbnail strips:** per-clip filmstrip keyed
(asset-hash, interval, cell size); one OPFS/file per strip (sequential write).

## 3. Browser storage deep-dive (beyond doc 18)

3.1 **OPFS sync-access-handle semantics (VERIFIED, WHATWG File System spec):**
"Creating a FileSystemSyncAccessHandle takes an **exclusive lock** on the file
entry … prevents the creation of further FileSystemSyncAccessHandles or
FileSystemWritableFileStreams for the entry, until the access handle is closed";
sync methods exist "for higher performance … e.g., WebAssembly"; in-place writes
are available for files in a bucket file system (OPFS). Worker-only (doc 18 §2).
Design consequence: one persistence worker owns each file; long-lived handles
must be explicitly closed or a crash leaks the lock until tab death.

3.2 **Quota + eviction semantics (VERIFIED, web.dev storage article):**
- Two buckets: **"Best Effort"** (default; browser may evict under pressure) vs
  **"Persistent"** (not auto-cleared; user must clear manually);
  `navigator.storage.persist()` requests the upgrade.
- Quota model (current web.dev numbers): Chrome — browser up to 80% of disk,
  **origin up to 60% of total disk**; incognito ≈ 5%; ~300 MB cap with
  "clear on close". Firefox — browser up to 50% of *free* disk. Quota is a
  runtime query: `navigator.storage.estimate()` → `{usage, quota}`; Safari
  prompts the user when quota is exceeded (most engines no longer prompt).
- OVE policy (proposal): manifest + assets must never live ONLY in best-effort
  storage — either `persist()` granted, or mirrored out (export/FSA path), or
  accepted-lossy with re-download (web tier); cache/ and proxy/ are explicitly
  evictable. `estimate()` gates proxy-trim (doc 32 §2 policy).

3.3 **OPFS concurrency budgets (VERIFIED, sqlite.org Wasm persistence doc):**
"there's no such thing as 'N concurrent readers' in OPFS-via-VFS" (read locks
file). Reliable concurrent-connection limit "unknown"; historically docs
suggested ~3, but 2026-03 testing sustained **8–10 concurrent workers** when
(A) locking minimized and (B) client handles SQLITE_BUSY. Rules given by
sqlite.org: open DBs lazily; keep statements reset (non-reset statements lock);
work in millisecond-scale chunks; never hold transactions open; never two
handles per file in one thread; `opfs-unlock-asap` as last resort;
`sqlite3_js_retry_busy()` added in 3.53.0. Safari < 17 caveat for SAH pool;
COOP/COEP required (docs 12/18). Incognito/guest = reduced or absent
persistence ("Achtung", same doc).

3.4 **IndexedDB metadata vs SQLite-WASM (deepens doc 18 §2):**
- IndexedDB: everywhere, structured-clone + blob storage, global indexes;
  awkward for multi-GB media and for SQL-shaped metadata; blobs are
  unaddressable in place (copy-on-read).
- SQLite-WASM over OPFS (SAH pool VFS): real SQL + transactions + WAL
  (WAL has caveats under OPFS per sqlite.org), the 8–10 worker budget (§3.3),
  and the SAME blocking primitives wasm needs. Cost: worker-only, header
  sensitivity, SQLite file is opaque to diff tools.
- Decision rule (proposal): **project index (clips, indices, receipts) in
  SQLite-WASM; media blobs as OPFS files content-addressed by name** — never
  blobs-inside-DB. Blob dedup in-browser = BLAKE3-WASM (SIMD feature VERIFIED,
  S-3d7) over chunked reads → same address space as desktop (§1).

## 4. Project portability

(Format surveys live in doc 19 — referenced, not repeated: OTIO per-type
versioning §1.1, Kdenlive generations §1.5, Olive per-version serializers §1.7,
OpenShot truncation cases §1.8, OpenCut IndexedDB lock-in §1.9, Resolve
DB storage §1.10.)

- **Single-file .ove (zip: snapshot + log + manifest) vs folder project:** the
  storage-layer constraint that decides it is media size. A single file forces
  bundle-in/bundle-out of media (copy storms for >100 MB projects) or degenerates
  into "zip + external media refs" anyway (Kdenlive/FCPXML pattern). Folder
  project = copy only what changed, snapshot = manifest diff. Recommendation
  aligned with doc 19 Candidate A: **folder is the native format; zip export is
  a derived artifact.**
- **Large-media strategies:** EDL-style external refs (default — hashes in
  manifest, bytes in pool/external paths) + optional **share bundle** = zip
  containing manifest + command log + the *referenced subset* of content-
  addressed assets (dedup makes the subset minimal; hash manifest verifies
  partial transfers). Bundled mode is a distribution artifact, never the
  working format.
- Cross-platform: same folder layout maps to desktop FS (doc 17 Tauri shell),
  OPFS (§3), Android SAF (docs 14/15) behind the platform storage adapter
  (doc 18 §3 PLATFORM-ADAPTER row).

## 5. Proposal: storage v1

```
project.ove/                     # folder project (native format)
  manifest.json                  # snapshot + command-log refs; OTIO-style per-type
                                 # schema versions (doc 19 Candidate A); asset table
                                 # {clip_id -> {hash, size, duration, fps, paths[]}}
  assets/<hex[0:2]>/<hex>        # content-addressed blobs (BLAKE3-256), deduped
  cache/                         # DISPOSABLE, regenerable:
    proxy/  peaks/  thumbs/  tiles/   # keys per §2.3; LRU-evictable
  renders/<render_id>/           # doc 30 §5 checkpoint format (manifest + segments)
  project.db (sidecar)           # SQLite index; WAL; rebuildable from manifest+log
```

- Invariant: **everything regenerable except manifest + assets**; `cache/` and
  `renders/` may be deleted at any time; backup/sync scope = manifest + assets
  (+ sidecar db optionally). Crash story: manifest+log replay (E-003) rebuilds
  index sidecar; doc 19 §2's hybrid-package trade-offs apply.
- Ingest receipt per asset (hash, size, codec, duration, fps, keyframe index) is
  the unit E-007/E-007b rendering depends on; receipts are cache (recomputable)
  but live beside assets for cheap validation.
- Future ADRs: ADR-030 (this layout + hash discipline), ADR-031 (cache keys +
  LRU + persist policy §2/§3), ADR-032 (proxy system — doc 32 §2 remains the
  thresholds owner; §2.2 here adds the verified codec/license facts).

## UNVERIFIED / NOT-FOUND (this pass)

- Premiere Pro media-cache/relink/scratch docs (helpx 403 bot-blocked); Kdenlive
  userbase (Cloudflare 403); DaVinci Resolve manual (too large) — shipped-editor
  offline/cache workflows rest on doc 19 verified format properties, not vendor docs.
- OPFS sequential-vs-random throughput numbers; per-engine OPFS write budgets
  (needs E-00x bench; doc 18 TODO stands).
- Chrome/Firefox quota percentages on mobile web; Safari `persist()` prompt UX
  current behavior.
- BLAKE3 GB/s rates per architecture (README chart is a 2019 CPU example — not
  restated as a current number).
- SQLite-WASM SAH-pool VFS on Safari < 17 remains a hard caveat (sqlite.org).

## Depth remaining (v0.2)

- E-00x: OPFS bench — SAH sequential/random read-write, handle reuse vs reopen,
  8–10-worker concurrency ceiling on real disks; SQLite-WASM vs IndexedDB
  metadata-store decision experiment (doc 18 TODO).
- E-00x: BLAKE3 ingest throughput at 4/64 GB file sizes native vs wasm-SIMD.
- Fetch Adobe/KDE offline-workflow docs from a non-403 network; add Resolve
  media-management page from the official manual.
- Design review: zip-bundle manifest (subset selection, streaming zip for 4 GB+
  bundles); crash-consistency test for `assets/` + sidecar db pairs.

## Sources (fetched 2026-09-24 unless noted; rows in LEDGER_W3d.md)

S-3d7 BLAKE3 crates.io + repo · S-3d8 web.dev "Storage for the web" + WHATWG File
System spec · S-3d9 bundle: sqlite.org Wasm persistence.md, FFmpeg libavcodec
proresenc_anatoliy/kostya + dnxhdenc headers (LGPL), ffmpeg-codecs.html §9.30,
Apple ProRes White Paper PDF (45 Mbps @1080p Proxy), Kdenlive/OTIO/FCPXML refs via
doc 19 · S-3d4 Diffusion Studio (waveform/thumbnail cache analogs). Internal:
E-003, E-007, E-007b; docs 02 §1, 08 §5, 10 §3, 11, 12/18, 14/15, 17, 19, 32.
