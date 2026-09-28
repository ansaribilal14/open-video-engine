# MASTER ENGINE STATE — single authoritative work order

> RULE (takeover directive §6): this is the ONE current engineering truth.
> Other status documents are historical/evidence records and stay untouched.
> Update this file at the end of every major wave using the §40 report format.

WAVE: 3 COMPLETE (0 / 0.5 / 0.6 / 1 seam / 2 render / 3 encode — all landed)
DATE: 2026-09-29
COMMIT: this wave landed via PR #5 (merge hash recorded in the merge commit)

## Session verification record (2026-09-28, independent takeover continuation)

Prior session claims were re-verified from scratch before any new work:

- HEAD `8b5aa81` on main; takeover baseline `33fde9d` IS an ancestor; tree clean.
- GitHub Actions: post-merge main run 36304812153 GREEN (and PR #1 runs
  36303605420 / 36304226310 GREEN) — W0.5 SIGILL fix confirmed externally.
- Local (system FFmpeg 7.1.5 path, unprivileged recipe per DEV_ENV.md):
  60/60 tests GREEN · fmt GREEN · clippy -D warnings GREEN (pre-W0.6 tree).
- Sources re-read: ove-time (297 l), ove-timeline (2303 l), ove-media (1332 l),
  ove-decode (145 l + conformance/probe suites). Spec set re-read: PROJECT_
  FORMAT / ENCODER / DECODER / FRAME_CONTRACT / MEDIA_ENGINE.

## Session verification record (2026-09-29, fresh continuation #2)

All W0.6/W1/W2 claims re-verified from scratch before Wave 3 work:

- HEAD `16d62a9` on main; baseline `33fde9d` IS an ancestor; tree clean.
- GitHub Actions run 36395951181 on exact HEAD `16d62a9`: SUCCESS (3 jobs:
  rust / audit / libav-confinement). All six recent runs SUCCESS.
- Local (system FFmpeg 7.1.5, unprivileged recipe per DEV_ENV.md, rustc
  1.98.1): 84/84 tests GREEN · fmt GREEN · clippy -D warnings GREEN.
- Source inventory re-counted: 4,223 l (ove-time 378, ove-timeline 1,611,
  ove-media 1,332, ove-decode 145+suites, ove-render 643).

## WAVE 3 deltas (2026-09-29, PR #5)

1. **ove-encode created** (workspace member 6): ENCODER_SPEC §1 traits —
   `Encoder` (configure/feed/drain + the recorded `track_spec()` extension),
   `Muxer` (open/write/finalize), `EncodedPacket` with checkpoint fields;
   pure export planner (`planner.rs`): keyframe-aligned StreamCopy vs
   ReEncode vs Mixed segmentation, snaps always reported (receipt), audio
   grid alignment; FFmpeg adapter (`ffmpeg/`): `FfmpegSwEncoder` (native
   mpeg4, swscale RGB→YUV420P declared-conversion path), `FfmpegMuxer`
   (MP4, faststart + bitexact options, exact tick conversion against the
   EFFECTIVE post-header axis), `FfmpegCopySource` (packet-level demux,
   keyframe-floor landing, 0-start shift, byte-identical passthrough).
2. **Tests 28 new (workspace 112/112 green)**: planner P01–P10 (pure logic:
   snap table, RoundIn validity, ToEof, 23.976 exactness, audio grid,
   mixed coverage, shapes); conformance E-1..E-5 + E-7/E-7b/E-7c + H1–H5 —
   outputs verified through ffprobe CLI (when present), libav probe, and
   decoded-pixel equality; copy-route byte-identity to source GOP range
   (E-007 §3.4) pinned at both packet and pixel level; committed corpus
   (copy24/copyav/copyntsc + golden tables) via
   `scripts/corpus/gen_encode_corpus.py`; byte golden
   `encode_golden.json` keyed on the producing libav identity (7.1.5-0+
   deb13u1), same-run determinism asserted everywhere.
3. **ADR-015** records the v1 decisions: confinement restated as CLOSED
   adapter allowlist {ove-decode, ove-encode} (check script amended and
   green), mpeg4-first codec surface (OpenH264/SVT-AV1 typed Unsupported),
   snap semantics made normative, movenc timescale-widening (24→12288)
   handled by effective-axis conversion, 0-duration packets → None, mp4
   AVIO caller-ownership (the AVFMT_NOFILE inversion SEGFAULT class, found
   and pinned by tests), version-keyed byte goldens with explicit reported
   skips. ADR-005 promoted PROPOSED → ACCEPTED (v1 surface landed).
4. **Bugs found & fixed in-wave** (each now test-pinned): mixed-axis tick
   conversion after movenc widening; stale pict_type=I on the reused
   encoder frame forcing all-intra; codec 0-duration corrupting stts;
   non-refcounted packet aliasing; test fixture pts bug (24k s frames) —
   caught because e2b passed while e2/e3 failed.

## WAVE 2 deltas (2026-09-28)

1. **ove-render created** (workspace member 5): software reference renderer
   per RENDER_GRAPH_SPEC — plan.rs (RenderPlan/Pass/PassKind + canonical
   hash), compile.rs (PURE compiler: compile_frame/compile_span; layer order
   = track order; exact rationals; conservative culling), exec.rs (software
   executor: integer straight-alpha over opaque output, FrameSource D-5
   floor resolution, single color conversion).
2. **RG-1..RG-7 all green** (render_golden.rs): compile purity (runs +
   threads, 4 seeds × 3 tracks × 5 placements), golden frames (3 fixtures,
   blake3-committed, hand-verified pixel semantics asserted alongside),
   layer-order swap, retime 0.5×/2× exact frame prediction (through the
   ADR-013 seam), split continuity END-TO-END through the real Timeline
   (apply Split → walk → compile → render byte-equality across the seam),
   optimizer safety (cullable placements: no passes + byte-identical
   output), color-tag single conversion + TagMismatch honesty error.
3. **ADR-014** records the v1 decisions (opaque integer compositing,
   output-sized surfaces, declared-tag conversion, integer-translate
   geometry, FrameSource decoupling, per-frame compile shape) + reopen
   conditions. Key bug the goldens caught during development: opaque
   transform buffers → fixed to transparent-outside-region.
4. ENGINE_BUILD_PLAN waves 0–3 are now REAL crates; the directive's W2
   "software-first renderer" gate is satisfied; decode binding to the same
   FrameSource trait lands with the W6 vertical slice.

## WAVE 1 deltas (this session)

1. **Forensic finding**: 4 uncompiled files (clip.rs/command.rs/engine.rs/
   state.rs, 1,264 l) orphaned by the v0.7 reconciliation — lib.rs declared
   only avl/gap/oracle, so a "editor-grade state machine" LOOKED implemented
   while compiling zero times with zero running tests (the exact
   documentation≠implementation class the directive forbids). REMOVED with
   full provenance (recoverable from commit ee31fb9); ideas that survive live
   in the compiled lineage (explicit-id log discipline, state hash, exact
   inverses). Recorded in ADR-013.
2. **Seam implemented** (`ove-timeline::mapping`, ADR-013): ClipWindow +
   SourceClock (zero ove-media dependency — layering held); exact
   timeline↔source mapping (roundtrip identity), `frame_span` floor/ceil
   bounds, `validate_ingest` range gate, `plan_seek` keyframe-floor landing
   (D-5/E-007 encoded positively); typed SeamError for all caller-range bugs.
3. **ove-time additions**: `recip()` + `ceil_div_rate()` (exact; unit tests).
4. **Property suite S0–S6** (mapping_properties.rs): roundtrip exactness over
   retimed windows, clip-primitive agreement, ingest gate boundaries, snap
   keyframe membership + monotonicity (20k cases), frame-span exactness at
   23.976 boundaries, exact-plan target identity, clock boundary validation.
5. MEDIA_ENGINE_SPEC §3 annotated "Implemented (2026-09-28, ADR-013)".

## WAVE 0.6 deltas (2026-09-28, PR #2)

1. ove-time overflow policy DECIDED + pinned (ADR-007 amendment 2026-09-28):
   **overflow = contract violation = PANIC (fail-fast); never wraps, never
   saturates.** Rationale: saturation is silent approximation (fatal to E-002
   exactness); fixed-tick-axis discipline makes overflow a caller bug; p3b
   already asserted the panic. Doc drift (crate header said "saturates") fixed.
2. ove-time i64::MIN holes FIXED (real bugs found by the takeover re-audit):
   - `neg()`: `-i64::MIN` wrapped silently (release) — now checked_neg + panic
     with explicit contract message.
   - `gcd64/gcd128`: `.abs()` on i64::MIN/i128::MIN overflowed — gcd now
     computed on unsigned magnitudes.
   - `Rational::new(i64::MIN, n)` normalizes correctly end-to-end now.
3. Property suite extended P1–P10 → P1–P12: P11 extreme-safety (MIN/MAX
   construct/cmp/half/floor_div_rate/split/add/serde), P12 overflow-panics
   (neg/sub/add/mul/floor_div_rate) + must-NOT-panic exact-boundary guard.
4. Hash terminology single-policy reconciliation (audit gap #1b):
   PROJECT_FORMAT_SPEC §2 now carries a normative HASH POLICY (BLAKE3-256,
   64 lowercase hex, = ove-media ContentHash; sha256 confined to golden-
   artifact TEST tooling), §2 manifest example `sha256`→`content_hash`,
   §5 `assets/<sha256>/`→`assets/<content-hash>/`; ENCODER_SPEC E-5 +
   muxer finalize() annotated "artifact integrity, not identity".

## CI CONFIRMATION (2026-09-27, prior wave)

- PR #1 run 36303605420: ALL 3 JOBS GREEN (fresh portable build; wrapper
  marker present; guard scanned 1445 objects PASS; conformance 14/14).
- Post-merge main run 36304226310: ALL 3 JOBS GREEN — the original failure
  scenario (cache restored from another runner) now passes with portable
  artifacts; conformance 14/14; portability guard PASS.

## Current implementation inventory

| Crate | State | Evidence |
|---|---|---|
| ove-time | TESTED — 9 unit + 18 property (P1–P12 + recip/ceil) | 112/112 workspace local (2026-09-29) + CI |
| ove-timeline | TESTED — 10 properties (AVL primary, ADR-011) + 7 seam properties (S0–S6, ADR-013) | CI + local; bench in ADR-011 |
| ove-media | TESTED — 13 unit (asset hash, FrameEnvelope, pool, probe types) | CI + local |
| ove-decode | TESTED — 14 conformance (D-1..D-12) + 6 probe; libav confined | CI + local, both system and bundled libav |
| ove-render | TESTED — RG-1..RG-7 (purity, goldens, layer order, retime, split continuity, optimizer safety, color) | CI + local; ADR-014 |
| ove-encode | TESTED — P01–P10 planner + E-1..E-5/E-7/H1–H5 conformance (28 tests; ffprobe/libav/pixel-verified, goldens committed) | 112/112 workspace local (2026-09-29); ADR-015 |
| ove-project / ove-engine / ove-cli | PLANNED (waves 4–5 per ENGINE_BUILD_PLAN) | — |

## Current research / architecture status

- ADR-001/002/005/007/008/009/010/011/012/013/014/015 ACCEPTED
  (005 promoted 2026-09-29, v1 surface landed; 015 encode leg v1);
  ADR-003/004/006
  PROPOSED with named evidence legs. ADR-007: overflow policy; ADR-013: seam
  + dead-leg removal; ADR-014: software renderer v1.
- 45 research docs + 137-source ledger + gates (13 UNDERSTOOD / 1 PARTIAL env-bound).
- Architecture residuals (named, not hidden): E-005 real-GPU perf, E-004c device
  runtime, E-006b real-Tauri transport — all hardware/environment-bound.

## Current validation status

- 112/112 workspace tests GREEN locally (system FFmpeg 7.1.5 path, rustc
  1.98.1), fmt GREEN, clippy -D warnings GREEN, after W3 (2026-09-29).
- Bundled path: exercised by CI on this wave's PR (bundled libav now also
  links swscale via the union feature set; portability guard scans the
  same object tree; E-5 byte gate self-skips with an explicit report on a
  different libav identity, structure gates always run).
- CI: ALL GREEN through W2 (runs 36395655326 / 36392979166 /
  36391223192; PRs 36391084190 / 36392782833 / 36395480815; HEAD run
  36395951181). W3 PR #5 run: recorded at merge.

## Current CI state

- Workflow: fmt + clippy + tests (bundled) + portability guard + cargo-audit
  + libav-confinement. Cache prefix `v2-portable-ffmpeg` (poisoned caches
  unreachable).

## Current blockers

- None environmental.

## Current open gaps (top)

1. WAVE 4 ove-project persistence (manifest + commands.jsonl + snapshot
   compaction), then W5 ove-engine + ove-cli.
2. AAC seam re-encode (audio wave W7); OpenH264/SVT-AV1 encoder legs;
   E-6 kill-resume execution at segment granularity (checkpoint fields
   exist).
3. Mutation-style "test the tests" not yet systematic.
4. Hardware-bound experiment residuals (E-004c/E-005/E-006b).
5. GitHub PAT used across chat sessions must be rotated by the owner
   (standing security rule, DEV_ENV.md) — outside engine scope, flagged.
6. docs/VERTICAL_SLICE_TRACE.md (directive deliverable) not yet written —
   scheduled with W6 vertical slice when the chains exist end-to-end.

## Current wave order (directive §37, unchanged)

0 audit ✓ → 0.5 SIGILL ✓ → 0.6 reconcile ✓ → 1 seam ✓ → 2 render ✓ →
3 encode+mux+export ✓ → 4 project → 5 engine+cli → 6 vertical slice →
7 audio → 8 keyframes → 9 GPU → 10–12 platforms → 13 conformance →
14 headless → 15 AI/MCP → 16 scripting → 17 plugins → 18 security →
19 perf → 20 docs/release → 21 production audit.

## Resolved decisions (registry)

- ADR-012 (2026-09-27): CI bundled-FFmpeg portability — wrapper + cache prefix
  + guard. Confidence 0.92. Reopen condition: ffmpeg-sys-next flag change (guard
  fails loudly) or CI SIGILL recurrence.
- ADR-007 amendment (2026-09-28): ove-time overflow policy = intentional panic
  (fail-fast); i64::MIN safe except negation (panics); checked variants deferred
  to first non-panicking consumer. Confidence 0.93. Reopen condition: an FFI
  consumer that cannot tolerate panics forces the checked-variant conversation.
- Hash policy (2026-09-28): BLAKE3-256 is the ONLY identity hash (assets +
  state); sha256 exists only in golden-artifact test tooling / render-record
  integrity fingerprints. Confidence 0.97 (code truth was already BLAKE3).
- ADR-013 (2026-09-28): timeline↔media seam = ove-timeline::mapping
  (ClipWindow/SourceClock, query-time mapping = Q-06 v1 resolution, typed
  seam errors, keyframe-floor seek planning) + dead state-machine leg removed
  (provenance ee31fb9). Confidence 0.88. Reopen: retime-verb semantics
  contradict S1; a backend cannot honor D-5 landing; ove-project needs
  persisted resolved-command journals.
- ADR-014 (2026-09-28): software reference renderer v1 — opaque integer
  straight-alpha compositing, output-sized surfaces, declared-tag single
  conversion, integer-translate geometry, FrameSource decoupling from
  ove-decode. Confidence 0.9. Reopen: intentional golden diff without
  byte-level note; scaling/affine need; stacked partial-coverage banding.
- ADR-015 (2026-09-29): encode leg v1 — adapter allowlist {ove-decode,
  ove-encode}; mpeg4-first codec surface; snap semantics normative
  (RoundIn start validity, RoundOut/ToEof end, Mixed inner core);
  effective-axis exact tick conversion; version-keyed byte goldens with
  explicit skips; refcounted packets; mp4 AVIO caller-ownership.
  Confidence 0.88. Reopen: a libav build whose movenc does NOT widen
  timescales still passes (conversion is effective-axis based); an
  encoder that needs B-frames forces the dts/ctts conversation; openh264
  licensing form (external lib vs bundled) at its adapter leg.

## Unresolved decisions (registry)

- NLE derived-verb surface (ripple/roll/slip/slide/insert/overwrite/extract/lift):
  record primitive-vs-derived table when timeline work resumes (directive §6.2) —
  not before.

## Explicit next action

WAVE 4 — ove-project persistence (BUILD_PLAN W5): manifest.json
(schema_version, tick axis, asset registry keyed by BLAKE3 content hash) +
commands.jsonl (append-only command journal) + snapshot compaction; folds
through ove-timeline commands (explicit IDs, exact inverses, state hash).
Acceptance: save → kill -9 → reopen → replay → state hash == pre-kill hash.
Land via PR with fmt/clippy/tests green, update this file, then WAVE 5
(ove-engine + ove-cli integration slice).
