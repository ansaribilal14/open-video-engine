# ENGINE BUILD PLAN (2026-09-26)

> The concrete, gated build order for the Media Foundation vertical slice and the crates
> it needs. Consumes: ROADMAP_RECONCILIATION (work order), the seven spec contracts, and
> every audit's carry-forward. Rule: a wave may not start until the previous wave's gate
> is green in-repo (tests committed, CI where applicable).

## Wave 0 — Repository integrity (half-day, blocking)

| Task | Detail | Gate |
|---|---|---|
| W0.1 | CI v1: GitHub Actions — `cargo test --release` (ove-time), `cargo clippy -- -D warnings`, `cargo fmt --check`, md-link check; `cargo audit`+`deny` config added | workflow green |
| W0.2 | Re-run E-006a from the record's spec; commit code + raw output; fix 41_EXPERIMENTS row | artifact chain closed (D-1) |
| W0.3 | Add LICENSE-MIT + LICENSE-APACHE + Cargo license fields + SPDX headers habit | LICENSE_AUDIT closed |
| W0.4 | Normalize 50 file modes; refresh ADR-002 header blockers; README dir-structure note; STATUS.md refresh with audit results | clean `git status`, contradictions X-3/X-6/X-7 closed |
| W0.5 | Create engine workspace `engine/Cargo.toml` (members: ove-time) | ready for wave 1 |

## Wave 1 — ove-timeline (the second real crate)

- **Scope:** clip model (explicit ids, Rational in/out per ADR-006), gap buffer + derived
  ordered index (ADR-011), command state machine + verbs {add, remove, resize, split,
  move, retime} each with exact inverse (ADR-009), batch = composite, undo markers,
  track model.
- **Tests (acceptance):** port E-003's invariants — apply==replay (hash), snapshot+suffix==
  full, undo-via-inverse hash-exact; per-verb property "apply→inverse→apply == identity";
  batch atomicity (all-or-nothing); 20k-clip edit perf smoke (E-002c budgets: split
  ≤5ms, resize ≤2ms, move ≤3ms at 20k, container CPU); determinism across seeds.
- **Gate:** property suite green in CI; ove-time consumed (IMPL→IMPLEMENTED).
- **Est. size:** 1,200–2,000 lines + tests.

## Wave 2 — ove-media + ove-decode (decoder-first)

- **ove-media:** MEDIA FILE→PROBE (container, streams, pix_fmt, color tags, keyframe
  index, VFR detection) → ASSET (content-hash identity, metadata cache).
- **ove-decode:** `Decoder` trait per DECODER_SPEC (session state machine, capability
  report, seek Exact/Snap, flush, cancel, error model) + **FFmpeg-SW adapter** (libav*
  linkage confined to this crate) + FrameEnvelope per FRAME_CONTRACT.
- **Tests:** conformance suite on committed tiny media (self-generated): PTS exactness vs
  ffprobe, seek-then-decode frame identity, flush semantics, cancel mid-decode, error
  propagation (truncated/corrupt files), keyframe-index correctness (E-007's lesson).
- **Gate:** conformance green; zero libav* symbols outside ove-decode (link check in CI).

## Wave 3 — ove-render (correctness first, software raster)

- **Scope:** deterministic compile (project state → ordered pass list: per-frame layer
  ordering, transforms, opacity, source rect), software raster path (CPU RGBA), golden
  frames.
- **Tests:** golden-frame hash tests (committed PNGs + hashes); compile purity (same
  project → same passes, property); no GPU code yet (E-005 promotion is wave 6).
- **Gate:** golden frames stable across runs; graph compile fuzz (random timelines →
  valid passes).

## Wave 4 — ove-encode + ove-mux (export leg)

- **Scope:** Encoder trait (configure/feed/drain) + Muxer + **StreamCopy route** chosen by
  export planner (keyframe-aligned, per E-007/E-007b); MP4 via FFmpeg LGPL.
- **Tests:** ffprobe-verified outputs — duration exactness (frame-exact), timestamps,
  codec tags; golden hashes committed (tiny clips); copy-route vs re-encode route both
  covered; mid-timeline cut content-correctness (E-007's wrong-content trap).
- **Gate:** golden MP4 suite green; outputs inspectable by ffprobe (directive requirement).

## Wave 5 — ove-project + integration slice (the quality gate)

- **ove-project:** manifest.json (schema_version, tick axis, asset registry by content
  hash) + commands.jsonl (append-only) + snapshot compaction; folds via ove-timeline.
- **Integration headless CLI (`ove-cli`):** build project: 1 video + 1 audio, multi-clip,
  trim/split/move/undo/redo/save/kill/reopen → decode → preview frames → render → export.
- **Acceptance (directive quality gate):**
  1. save → kill -9 at random point → reopen → replay → state hash == pre-kill hash
  2. exported MP4 duration-exact, ffprobe-clean timestamps
  3. decode→render→export golden chain reproducible in CI (software legs)
- **Gate:** MEDIA+TIMELINE+PROJECT+RENDER+EXPORT cooperating. Only then new features.

## Wave 6 — GPU upgrade (E-005 promotion)

- wgpu compositor consuming the same FrameEnvelope; WGSL passes mirroring the software
  path; correctness suite = same golden frames; **separate** perf bench (committed
  baselines, ratio-gated); feature-detect + fallback.
- **Gate:** golden parity software↔GPU; perf bench committed (correctness ≠ performance).

## Wave 7 — Audio + keyframes (parallelizable)

- Audio: FFmpeg/Symphonia decode → sample-exact mapping on tick axis → mix → WAV/PCM out;
  A/V sync test (sample-count exactness at export; doc 21 rules).
- Keyframes: property→keyframes→interpolation (linear/hold first) →evaluation over exact
  time; property tests (monotonic keys, boundary exactness, split-preserve).

## Wave 8 — Platform legs (evidence collection, parallel where hardware allows)

- Android: E-004c JNI harness on device; MediaCodec adapter spike; checkpoint/resume
  design test (FGS budget).
- Desktop transport: E-006b real-Tauri bench.
- Browser: WASM core (timeline+project) + WebCodecs adapter behind **mandatory runtime
  feature detection**; cross-browser matrix per CONFORMANCE_PLAN.
- Each leg's result flips ADR-002 residuals or triggers the recorded fallbacks.

## Wave 9 — Closure

- ADR-002/003/004/005/006/008/009/011 ACCEPT decisions where gates passed; spec docs get
  promoted per UPDATED_ADR_STATUS; whitepaper 44 finalized; GATE_STATUS + STATUS.md
  updated; OPEN_GAPS.md rows resolved (append-only).

## Anti-overbuild commitments (binding across all waves)

No UI · no plugin system · no scripting runtime beyond command API · no MCP server in
engine binary · no cloud anything · no interchange adapters (OTIO/MLT) · no multi-editor
concurrency · no crate that does not have a failing acceptance test asking for it.
