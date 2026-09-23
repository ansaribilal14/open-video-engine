# E-002 — Time representation + timeline structure (Python property tests + native Rust bench)

> Status: RUN-COMPLETE (13/13 property checks + native Rust benchmark).

- **QUESTION**: Is float64 seconds safe as the authoritative timeline time? What does an
  exact-rational timeline cost natively?
- **HYPOTHESIS**: fp is unsafe (boundary errors, policy-dependence, rate drift); exact
  int64/rational time is deterministic and cheap.
- **IMPLEMENTATION**: `scripts/experiments/E-002_time_representation.py` (IEEE-754
  binary64 semantics — identical to Rust f64/JS Number) + `E-002b_timeline_bench.rs`
  (native exact-rational timeline ops, rustc -O).
- **HARDWARE**: container CPU (correctness results hardware-independent; perf indicative).
- **RESULT** (13/13 PASS):
  - T1: 10×(0.1s) in fp = 0.9999999999999999 → frame index 29 instead of 30 (off-by-one
    at the boundary). NOTE recorded: the popular "3×(1/3)≠1" example is FALSE in binary64
    (round-to-even tie → exactly 1.0) — good thing we tested before believing folklore.
  - T2: decimal "23.976" vs 24000/1001 → 7.2 ms drift after 2h (172,627 frames).
  - T3: 2000/2000 clip starts order-dependent in fp; rational sums order-invariant.
  - T4: 178 sample-boundary mismatches (fp vs exact floor) across 6 rate-pairs × 20k
    frames — audio alignment MUST be ruled by exact rationals.
  - T5: 1753/2000 starts differ between two legal fp accumulation policies.
  - T6: 192 kHz tick axis handles 23.976 but NOT 29.97 (32032/5 ticks); even a 30.03 MHz
    axis fails arbitrary rates (17000/999) → per-clip rationals remain necessary.
  - Rust bench: 20k-clip rational build **3.41 ms**; BTreeMap index 2.27 ms; order-
    invariance assert over 25k clips 3.4 ms; **5000 Vec-splits = 45.6 ms** →
    naive Vec+insert is NOT editor-grade; primary structure needs gap-buffer/order-stat
    design → new experiment E-002c.
- **LIMITATIONS**: perf numbers indicative (container CPU); property suite not yet a
  permanent Rust test crate (E-002c scope: structure choice + property tests in `ove-time`).
- **DECISION**: **C-001 CONFIRMED experimentally → HIGH.** ADR-007 evidence complete:
  exact rational/integer authoritative; fp display-only. Timeline primitive bench
  baseline recorded; structure redesign before Phase 2.
