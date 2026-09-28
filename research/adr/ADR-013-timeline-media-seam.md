# ADR-013: Timeline↔media seam + dead state-machine leg removal

- **Status**: ACCEPTED (2026-09-28) — acceptance condition met: seam property
  suite S0–S6 lands green in-repo (engine/ove-timeline/tests/mapping_properties.rs).
- **Date**: 2026-09-28 · **Confidence**: 0.88 (seam contract 0.9; removal
  decision 0.85 — both falsifiable, see REOPEN below)

## CONTEXT

WAVE 1 (directive §37) is the media↔timeline seam. MEDIA_ENGINE_SPEC §3
requires: clip in/out are project-axis rationals; source pts are source-rate
rationals; mapping is exact rational arithmetic at query time (Q-06 v1:
provenance stored, no converted copies); keyframe index drives copy-route seek
(D-5, E-007). Nothing in the shipped tree implemented that mapping.

Forensic finding while implementing (this wave): the repo carried a **dead
uncompiled state-machine leg** — `clip.rs`, `command.rs`, `engine.rs`,
`state.rs` (1,264 lines) — orphaned by the v0.7 reconciliation merge: lib.rs
declares only `avl/gap/oracle`, so these files compile ZERO times and their
designs (separate `ClipEntry`/`TimelineState`/`TimelineEngine`, journal,
10-variant error model) are NOT covered by any running test. They look
implemented (and mislead source audits) while being neither compiled nor
tested — the exact "documentation ≠ implementation" failure class the
takeover directive forbids carrying silently.

## DECISION

1. **Remove the dead leg** (git-rm, provenance commit ee31fb9 keeps it
   recoverable). Its still-valuable ideas live on in the compiled lineage:
   explicit-id allocation riding the log (E-012 finding, now in
   `Timeline::alloc_id`/`Split` discipline), state hash (lib.rs), exact
   inverses + batches (Command/UndoStack). The richer pieces that die with it
   (speed-carrying entries, resolved-command journal) return with the waves
   that actually consume them (retime verb → W5+; persisted journal →
   ove-project W4), rebuilt against the LIVE model and tested when they land.
2. **Seam contract** (new `ove-timeline::mapping`, zero ove-media dependency):
   - `ClipWindow { timeline_start, dur, src_in, speed }` — the seam's view of
     one placement; `From`-style lift from live `Clip` at speed 1/1 (the
     current verb set has no retime; the speed≠1 math is pinned now so the
     future retime verb inherits a tested seam, per BUILD_PLAN Wave-1 scope).
   - `SourceClock { duration, keyframes }` — the media side as plain exact
     time; constructed from probe records by the layer above (layering:
     ove-timeline never imports ove-media).
   - Exact mappings: `timeline_to_source`, `source_to_timeline` (roundtrip
     identity S1), `frame_span` (floor/ceil frame bounds, S5),
     `plan_seek` (Snap = keyframe floor landing, D-5/E-007 encoded positively;
     Exact = decoder-owned drop, D-4; S4/S6), `validate_ingest` (speed/dur/
     range gate against `SourceClock.duration`, S3).
   - Typed `SeamError` for ALL caller-range bugs — validation happens before
     arithmetic; panics remain reserved for unrepresentable values (ADR-007
     amendment contract).
3. **ove-time additions**: `recip()` (exact reciprocal; zero and i64::MIN
   panic per contract) and `ceil_div_rate()` (exact ceil via −floor(−x));
   unit-tested; consumed by the seam.

## ALTERNATIVES CONSIDERED

- Wire the dead leg into lib.rs: rejected — it duplicates the compiled model
  (two clip types, two command systems, two error enums) and its P1–P8 tests
  were already replaced by the E-012 suite; resurrecting doubles surface
  before any consumer exists.
- Keep dead files "for reference": rejected — uncompiled code cannot be kept
  honest by clippy/tests and rots silently; git history IS the reference.
- Put the seam in ove-media: rejected — ADR-002 layering (timeline above
  media); the seam parameterized by SourceClock keeps the dependency arrow
  pointing the right way.

## REOPEN CONDITIONS

- A retime verb lands with semantics contradicting `ClipWindow::retimed`
  math (S1 fails on a real verb) → renegotiate the window shape.
- A decoder backend cannot honor keyframe-floor landing (D-5) → `plan_seek`
  gains backend capability input (DECODER_SPEC capability report).
- ove-project needs persisted journals richer than the command log → the
  resolved-command journal design returns from ee31fb9 with tests.
