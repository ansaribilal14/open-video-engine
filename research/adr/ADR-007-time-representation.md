# ADR-007: Time representation

- **Status**: ACCEPTED (2026-09-24) — acceptance condition met: property tests landed
  and pass. Evidence: E-002 13/13 (research/experiments/E-002_time_representation.md),
  permanent Rust property suite engine/ove-time **15/15 PASS**
  (experiments/ove-time_property_suite.txt), E-002c P3b accumulator finding
  (research/experiments/E-002c_timeline_structure.md), E-004b 8/8 exact values
  verified end-to-end Rust→Kotlin across FFI (research/experiments/E-004b_uniffi_kotlin.md).
- **Date**: 2026-09-21 · accepted 2026-09-24 · **Confidence**: HIGH

## CONTEXT
Timeline must be frame-accurate across mixed rates (23.976/25/29.97/30/60), VFR sources,
and 48 kHz audio, and deterministic across renderers/platforms.

## OPTIONS
A. float64 seconds · B. int64 frame index at project fps · C. int64 ticks in fixed tick
rate (e.g., 1/MHz) · D. rational (int64 num, int32 den) per clip + project ticks

## EVIDENCE
- fp failures (verified math): floor() off-by-one at exact boundaries (3×1/3), order-
  dependent sums, 23.976-as-decimal ≈ 3.6 ms/h drift, cross-renderer divergence — [20]
- Industry: MLT int positions + rational profile rates; OTIO RationalTime (value, rate)
  discipline; Olive AVRational; FCPXML rational seconds — [03, 07, 19, 20]
- C-001 raised to HIGH by 3-b with primary-source verification.

## DECISION
D-shape: exact integer/rational authoritative everywhere; fp seconds are
display-only. Source PTS maps stay exact (Q-06 open: map-in-format vs normalize).

Refinement (2026-09-24, from E-002c P3b): **aggregates (track duration, prefix sums,
render seeks) MUST accumulate on a fixed rational tick axis** (int64 ticks, axis den
fixed per project) — free num/den rationals overflow i64 via lcm-chain growth after
~8–10 coprime-denominator additions (Rust i64; Python bigint hid this in E-002).
Free rationals remain the ingest/boundary representation for source PTS and mixed-
rate clip edges; they are converted to the project axis at ingest, exactly.
engine/ove-time encodes this as a regression guard (test `p3b`).

## REJECTED ALTERNATIVES
A: unfixable determinism. B: breaks on mixed-rate + audio-sample alignment. C: workable
but loses source-rate provenance; may combine with D as the project-time axis.

## CONFIDENCE & RISK
HIGH. Risk low; E-002 adds drift property tests to close.
