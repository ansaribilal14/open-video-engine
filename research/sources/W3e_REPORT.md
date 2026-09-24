# W3e_REPORT — wave-3 agent W3-e (Task 14-e)

Agent: W3-e (research: performance + security) · Date: 2026-09-24 · No engine code touched.

## 1. Docs written (replacing 9-line PLANNED stubs)

| Doc | Title | Lines | Status line |
|---|---|---|---|
| docs/research/32_PERFORMANCE.md | Performance Engineering (Track S) | 240 | `> Status: PARTIAL (v0.1 — wave-3 depth pass 2026-09-24).` |
| docs/research/33_SECURITY.md | Security Model (Track T) | 236 | `> Status: PARTIAL (v0.1 — wave-3 depth pass 2026-09-24).` |

Both follow the established format: VERIFIED/UNVERIFIED markers, inline citations
[S-3eN] + internal doc/E cross-references, Sources, UNVERIFIED/NOT-FOUND list,
depth-remaining list.

## 2. Key findings

Performance (doc 32):
- Frame-budget table proposed (60 fps = 16.67 ms) and grounded in measured anchors:
  E-006 (1.4 MB JSON = 42% of frame vs 0.5% binary; commands 1–24 µs) and E-002c
  (~140 ns rational add dominates walks). No surveyed FOSS editor publishes a
  defensible budget — OpenCut has no benchmarks; Clypra's "sub-10ms" claim is
  UNVERIFIED marketing (its sampled-telemetry design is the reusable part);
  OpenReelio has an unread `src/benchmarks/` dir.
- FFmpeg threading VERIFIED at tag n9.0.2 (S-3e2): FF_THREAD_FRAME adds one frame
  latency per thread → preview/scrub uses thread_count=1/SLICE; export uses frame
  threading. One AVCodecContext per worker.
- Remotion Lambda model VERIFIED (S-3e3): chunked data-parallel frame ranges
  (concurrency = frameCount/framesPerLambda, ≥20 frames/lambda) → OVE headless
  sharding design with E-007b stream-copy stitch.
- wgpu 30.0.1 Features TIMESTAMP_QUERY + TIMESTAMP_QUERY_INSIDE_ENCODERS VERIFIED
  (docs.rs) → per-pass GPU timing; async-compute overlap UNVERIFIED (E-005).
- Proxy workflow: thresholds/formats/policies/relink are design proposals (no
  shipped-editor primary doc survived fetch); ProRes native encoder VERIFIED in
  ffmpeg-codecs.html §9.30; Apple ProRes SDK licensing NOT-FOUND → doc 35.
  Relink correctness = BLAKE3 content-hash manifest (blake3 1.8.7 VERIFIED), never
  path-only (generalizes E-007's mid-GOP lesson).
- Memory caps table per platform (iOS jetsam ~2 GB — docs 12/18; wasm32 4 GiB vs
  wasm64 status — doc 12; Android hard heap limit VERIFIED on
  developer.android.com); LRU eviction on (track, time) window around playhead.
- Profiling plan: tracing 0.1.44 / tracing-subscriber 0.3.23 / tracing-chrome 0.7.2
  (stale — vendor-or-fork) / criterion 0.8.2 all VERIFIED via crates.io (S-3e9);
  Chrome Trace Event Format VERIFIED (S-3e7); perf CI gates defined on E-002/E-006/
  E-007b baselines, ratio-based (E-007b lesson: ratios are durable).

Security (doc 33):
- FFmpeg security posture VERIFIED (S-3e0): 513 CVE ids on the official page with
  per-fix hashes (CVE-2025-59733/59734 tagged BIGSLEEP-*, CVE-2026-8461,
  CVE-2026-30998/99); page warns of AI-generated false-positive reports. OSS-Fuzz
  FFmpeg VERIFIED (S-3e1): libFuzzer/AFL/honggfuzz + ASan/MSan/UBSan. Conclusion:
  media parsing = top untrusted surface → boundary-1 rule (sidecar/worker/wasm only).
- Malicious project files: E-003 schema lessons extended into 7 validation rules
  (explicit ids; (num,den) i64 rationals, floats forbidden; referential+range
  checks; fail-closed; version-tagged schema; no code execution; provenance/
  receipts against log forgery). Zip-slip rules for project archives (S-3e6 VERIFIED).
- Sandboxing ladder: media bytes (untrusted, boundary-1) → project files (data,
  validator) → wasm plugins (semi-trusted; wasmtime sandbox VERIFIED S-3e5) →
  scripts (command batches) → native dylib plugins (excluded from v1) → MCP tools
  (untrusted services; docs 26/27 referenced, not duplicated).
- Supply chain: cargo-audit/cargo-deny (RustSec, S-3e8) + cargo-auditable; FFmpeg/
  GStreamer pinning + per-release security-page watch; Tauri updater signatures
  "cannot be disabled" + public key in tauri.conf.json + per-artifact .sig
  (VERIFIED S-3e4); minisign-verify 0.2.5 available, exact primitive UNVERIFIED.
- IPC/local: Tauri capabilities/ACL pattern VERIFIED; E-006 numbers double as
  attack-surface rule; CSP (MDN page verified present) + COOP/COEP posture; secrets
  NEVER in project files/command log (replay would re-execute them) → keychain/
  origin storage.
- v1 rule list: 12 concrete rules (doc 33 §9), incl. no network from core, resource
  caps per command batch, crash-only checkpoint+replay (E-003).

## 3. ADR-relevant decisions / inputs

- ADR-036 (Performance): doc 32 is the evidence base — budget table, threading
  split, parallelism model, CI gates.
- ADR-032 (Proxy system): thresholds, hash-keyed cache, relink procedure.
- ADR-031 (Cache): render-cache tile keys + eviction policy first draft.
- ADR-033 (Security): doc 33 is the evidence base; sandboxing ladder = proposed
  core decision; §9 rule list = proposed acceptance checklist.
- PROPOSED risk-register additions (register intentionally NOT edited): R-13
  (hostile-media parser exploit / proxy-relink cache consistency), R-14 (perf
  regressions across platform matrix; malicious project-file import covered by
  doc 33 §3). Existing R-04/R-08/R-09 get rule-level mitigations, not duplicates.
- PROPOSED experiments: E-008 (measured frame-budget breakdown), E-009 (proxy
  throughput + relink fuzzing), E-010 (range-parallel scale-out crossover) —
  defined in doc 32 depth-remaining; numbering avoids E-001..E-012 in use.

## 4. NOT-FOUND / failed-fetch log (all genuine attempts)

- Apple ProRes SDK/licensing page: devimages.apple.com.edgekeys.net DNS fail,
  images.apple.com + apple.com/prores 404 → NOT-FOUND (doc 35 owner).
- Avid DNxHR SDK terms: NOT-FOUND; native dnxhd encoder docs: 0 hits in fetched
  ffmpeg-codecs.html → UNVERIFIED.
- Premiere Pro "smart rendering" helpx: HTTP 403 (blocked).
- FCP "playback and proxy files" support page: URL resolves but content redirects
  (0 "proxy" hits in body) → unusable.
- Kdenlive proxy docs: docs.kde.org stable5/stable6 paths 404 (guesses; no
  working index found) → NOT-FOUND.
- GStreamer SECURITY/CVE policy: raw.githubusercontent 404; gitlab.freedesktop.org
  raw fetch blocked by anti-bot challenge (Anubis) → NOT-FOUND (doc 06 owner).
- FFmpeg trac Fuzzing wiki: blocked by anti-bot (Anubis) — covered instead by
  S-3e1 (oss-fuzz config) and S-3e0.
- snyk.io/research/zip-slip-attack 404 → corrected URL security.snyk.io/research/
  zip-slip-vulnerability (VERIFIED).
- GitHub API (OTIO contrib listing): unauthenticated rate limit exceeded → skipped.
- NVD CVE API not needed (official ffmpeg.org/security.html provided ids+hashes).

## 5. Source IDs used

- Ledger rows (research/sources/LEDGER_W3e.md, new file): S-3e0..S-3e9.
- Inline, status-verified without dedicated rows (URLs given in docs):
  developer.android.com/topic/performance/memory-overview,
  ffmpeg.org/ffmpeg-codecs.html §9.30, docs.rs/wgpu Features (30.0.1),
  MDN CSP + COOP header pages (200-status topic verification).
- Internal evidence cited: E-002c, E-003, E-004a, E-006, E-007, E-007b records;
  docs 02/04/05/06/08/11/12/14/15/16/17/18/19/26/27/28/30; C-001/C-003/C-007;
  R-03/R-04/R-05/R-07/R-08/R-09.

## 6. Honesty notes

- No fabricated citations: every VERIFIED marker maps to a fetch on 2026-09-24
  (or an internal repo record); inferences are marked INFERENCE (e.g., BIGSLEEP
  tag meaning in doc 33 §2).
- No repo files other than the two stub docs + two new research/sources files
  were modified.
