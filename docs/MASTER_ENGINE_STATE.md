# MASTER ENGINE STATE — single authoritative work order

> RULE (takeover directive §6): this is the ONE current engineering truth.
> Other status documents are historical/evidence records and stay untouched.
> Update this file at the end of every major wave using the §40 report format.

WAVE: 2 COMPLETE (0 / 0.5 / 0.6 / 1 seam / 2 render — all landed)
DATE: 2026-09-28
COMMIT: this wave landed via PR #4 (merge hash recorded in the merge commit)

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

## WAVE 2 deltas (this session)

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
| ove-time | TESTED — 9 unit + 18 property (P1–P12 + recip/ceil) | 77/77 local (2026-09-28) + CI |
| ove-timeline | TESTED — 10 properties (AVL primary, ADR-011) + 7 seam properties (S0–S6, ADR-013) | CI + local; bench in ADR-011 |
| ove-media | TESTED — 13 unit (asset hash, FrameEnvelope, pool, probe types) | CI + local |
| ove-decode | TESTED — 14 conformance (D-1..D-12) + 6 probe; libav confined | CI + local, both system and bundled libav |
| ove-render | TESTED — RG-1..RG-7 (purity, goldens, layer order, retime, split continuity, optimizer safety, color) | 84/84 local (2026-09-28) + CI; ADR-014 |
| ove-encode / ove-project / ove-engine / ove-cli | PLANNED (waves 3–5 per ENGINE_BUILD_PLAN) | — |

## Current research / architecture status

- ADR-001/002/007/008/009/010/011/012/013/014 ACCEPTED; ADR-003/004/005/006
  PROPOSED with named evidence legs. ADR-007: overflow policy; ADR-013: seam
  + dead-leg removal; ADR-014: software renderer v1.
- 45 research docs + 137-source ledger + gates (13 UNDERSTOOD / 1 PARTIAL env-bound).
- Architecture residuals (named, not hidden): E-005 real-GPU perf, E-004c device
  runtime, E-006b real-Tauri transport — all hardware/environment-bound.

## Current validation status

- 84/84 workspace tests GREEN locally (system FFmpeg 7.1.5 path), fmt GREEN,
  clippy -D warnings GREEN, after W2 changes (2026-09-28).
- Bundled path: exercised by CI on the PR for this wave (same pure-Rust diff —
  no libav interaction; PR run is the gate).
- CI: ALL GREEN — W2 merge 32c15406 (run 36395655326), W1 merge 96763dcf
  (run 36392979166), W0.6 merge 9383d14 (run 36391223192). PR runs:
  36391084190 (PR #2), 36392782833 (PR #3), 36395480815 (PR #4).

## Current CI state

- Workflow: fmt + clippy + tests (bundled) + portability guard + cargo-audit
  + libav-confinement. Cache prefix `v2-portable-ffmpeg` (poisoned caches
  unreachable).

## Current blockers

- None environmental.

## Current open gaps (top)

1. WAVE 3 ove-encode + mux + ExportPlanner (BUILD_PLAN W4; ffprobe-verified goldens).
2. Mutation-style "test the tests" not yet systematic.
3. Hardware-bound experiment residuals (E-004c/E-005/E-006b).
4. GitHub PAT used across chat sessions must be rotated by the owner
   (standing security rule, DEV_ENV.md) — outside engine scope, flagged.
5. docs/VERTICAL_SLICE_TRACE.md (directive deliverable) not yet written —
   scheduled with W6 vertical slice when the chains exist end-to-end.

## Current wave order (directive §37, unchanged)

0 audit ✓ → 0.5 SIGILL ✓ → 0.6 reconcile ✓ → 1 seam ✓ → 2 render ✓ →
3 encode+mux+export → 4 project → 5 engine+cli → 6 vertical slice →
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

## Unresolved decisions (registry)

- NLE derived-verb surface (ripple/roll/slip/slide/insert/overwrite/extract/lift):
  record primitive-vs-derived table when timeline work resumes (directive §6.2) —
  not before.

## Explicit next action

WAVE 3 — ove-encode + muxer + ExportPlanner (BUILD_PLAN W4): Encoder trait
(configure/feed/drain) + Muxer + stream-copy route chosen by the export
planner (keyframe-aligned per E-007/E-007b); MP4 via FFmpeg LGPL inside an
adapter crate (libav confinement holds); ffprobe-verified outputs — duration
exactness (frame-exact), timestamps, codec tags; golden hashes committed.
Land via PR with fmt/clippy/tests green, update this file, then WAVE 4
(ove-project persistence: manifest + commands.jsonl + snapshot compaction).
