# ADR-007: Time representation

- **Status**: PROPOSED (target: ACCEPT after property tests land in E-002)
- **Date**: 2026-09-21 · **Confidence**: HIGH

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
(Provisional) D-shape: exact integer/rational authoritative everywhere; fp seconds are
display-only. Source PTS maps stay exact (Q-06 open: map-in-format vs normalize).

## REJECTED ALTERNATIVES
A: unfixable determinism. B: breaks on mixed-rate + audio-sample alignment. C: workable
but loses source-rate provenance; may combine with D as the project-time axis.

## CONFIDENCE & RISK
HIGH. Risk low; E-002 adds drift property tests to close.
