# E-012: Timeline structure INTEGRATION — ADR-011 revisit gate

- **Date**: 2026-09-26 · **Question source**: 41_EXPERIMENTS (defined wave-3; ADR-011's own revisit trigger) · **Gates**: ADR-011 (revisit), ADR-009 (per-verb property CI), ADR-006/008 (command schema)
- **Code**: `engine/ove-timeline/` (production crate: `src/lib.rs`, `src/gap.rs`, `src/avl.rs`, `src/oracle.rs`, property suite `tests/properties.rs`, bench `src/bin/e012_bench.rs`) · **Raw output**: `experiments/E-012_result.txt`

## QUESTION

ADR-011 provisionally decided: **gap buffer primary + derived lazily-rebuilt ordered
index** for random/agent access and hit-testing, with an explicit revisit trigger:
"integration benchmark fails editor budgets (scrub burst ~1 ms, doc 18/34)". Does the
provisional design survive integration into a real crate with the production verb
surface — under cursor-local storms, random agent storms, far-jump worst cases, the
edit→scrub alternation regime, and multi-track interleaving?

## HYPOTHESIS

If the gap+lazy-index shape meets the 1 ms scrub-burst budget and stays competitive on
edit verbs, ADR-011 flips as-proposed. If the edit→scrub alternation regime breaches the
budget (each edit dirties the index; the next burst pays a full O(n) exact-prefix
rebuild ≈ 20k rational adds), the E-002c2 "derived-index shape" (augmented order-
statistic AVL with exact Rational subtree sums) becomes the evidence-backed primary.

## IMPLEMENTATION

- **First engine crate since `ove-time`**: `ove-timeline` — per-track sequences with
  exact `Rational` durations on the 24 kHz tick grid (ADR-007), explicit ids (E-003),
  verbs = Insert/Remove/Split/Resize/Move + atomic Batch, every command's exact inverse
  computed at apply time (ADR-009 option B), LIFO undo/redo stack, typed errors
  (E-004a discipline: no panics on user input paths), deterministic FNV state hashes.
- **Three interchangeable containers behind one `TrackOps` surface**: `GapTrack`
  (gap buffer + lazy prefix-sum index — ADR-011 provisional shape), `AvlTrack`
  (augmented order-statistic AVL, exact Rational subtree sums — E-002c2 shape),
  `OracleTrack` (naive Vec — the doc-45-rule-4 oracle).
- **Property suite 10/10** (P1 apply→inverse→identity per verb; P1b undo/redo
  round-trips; P2 oracle equivalence under shared randomized scripts; P3 replay
  determinism on fresh engines; P4 split conservation + exact unsplit; P5 hit-test
  total coverage vs linear oracle incl. exact boundaries; P6 batch atomicity +
  rollback; P7 cross-track move exactly-once invariant; P8 threaded disjoint-track
  storms == sequential hashes; P9 typed errors leave state unchanged).
- **Bench** (release, N=20k clips/track unless noted, identical scripts cross-validated
  by hash+count+total after EVERY workload): W1 cursor-local splits ×5000; W2 random
  storm (split/resize/move/remove-insert ×9500); W3 far-jump ×5000 (random positions in
  front/back quarters); W2' by-id command leg ×2000; W4 scrub-burst ×1000 rounds of
  (1 by-id edit + 100 hit-tests) vs the 1 ms budget; W5 multi-track (8 × 2500 clips,
  800 interleaved by-id edits + cross-track moves).

## HARDWARE

Container CPU (same class as E-002c/E-002c2; compare relative order, not absolutes).
Single-run timings, release profile.

## RESULT — property suite 10/10 PASS; bench cross-validation PASS everywhere; budget verdict SPLIT

| Workload (20k clips) | GapTrack | AvlTrack | Oracle | Verdict |
|---|---|---|---|---|
| W1 cursor-local splits ×5000 | **5.7 ms** | 47.4 ms | 70.4 ms | gap 8× (its home turf) |
| W2 random storm ×9500 | 502.2 ms | **83.9 ms** | 74.6 ms | avl 6.0× |
| W3 far-jump ×5000 | 471.9 ms | **47.2 ms** | 40.6 ms | avl 10× |
| W2' by-id command leg ×2000 | **106.7 ms** | 360.1 ms | 21.6 ms | gap 3.4× (contiguous id scan; AVL in-order scan is pointer-chase-bound) |
| **W4 scrub-burst p99** (1 ms budget) | **3.62 ms — FAIL** | **0.52 ms — PASS (1.9× headroom)** | n/a | **DECISIVE** |
| W5 build 8×2500 | **3.0 ms** | 78.0 ms | — | gap (contiguous build) |
| W5 800 mixed by-id edits @2.5k/track | **6.6 ms** | 21.1 ms | — | gap 3.2× — crossover at small N |

(A second full run agreed on every verdict; single-run indicative timings, hash gates
exact. AVL W4 max 1.66 ms shows occasional outlier bursts — p99 is the budget metric
per doc 34.)

W4 is the regime the ADR-011 decision text did NOT anticipate as binding: **agent-era
editing interleaves edits with scrubs through the same surface (E-009)**. Gap+lazy pays
one full O(n) exact-prefix rebuild per burst after ANY edit (measured p50 3.40 ms,
p99 3.62 ms across two runs — 3.4× over budget). AVL's weighted descent needs NO
rebuild after edits (p50 0.33 ms — structural O(log n) on cached Rational subtree sums).

## FINDINGS (beyond the headline)

1. **ALLOCATION MUST BE EXPLICIT IN THE LOG (extends E-003's rule set).** The first P2
   run diverged from the oracle: `Split` allocated its right-half id inside `apply()`,
   so replay depended on the allocation sequence, which breaks once removed ids leave
   holes in the used-id set (the allocator's skip-used lands differently than the
   generator's counter). Fix: `Split.new_id` is allocated by the CALLER at command-
   construction time and rides the log explicitly; the engine never allocates during
   apply. This additionally makes mid-session re-apply EXACT (P1's re-apply leg now
   passes bit-exact). Schema rule #4 for ADR-008: **engine-allocated ids ride the log**.
2. **By-id addressing does not need the ordered index.** Every structure pays an O(n)
   scan for `index_of` (positions shift on every structural edit — the E-002c2 rekey
   trap forbids id→position maps). Contiguous scans (gap/oracle ≈ 10–20 µs @20k) beat
   the AVL's cache-hostile in-order scan (~150 µs) by ~3×; at 20k this is editor-grade
   for the edit path. The ordered index is reserved for TIME queries, where it cannot
   be avoided.
3. **Small-N crossover**: at 2.5k clips/track (W5), gap wins mixed by-id edits 3.6× —
   the memmove tax shrinks with N while AVL's per-op Rational-sum maintenance is
   N-independent. Per-track structures may legitimately differ by track scale (Phase 2
   option, not v1 complexity).
4. **Honest bench bugs found and fixed during the run** (recorded per doc 45 rule 2):
   (a) W1/W3 initially degenerated — re-splitting halved the same clips until sub-tick
   (guards added: min 3-tick split, attempt caps, reported attempt counts);
   (b) the script generator's shadow model gave split right-halves the ORIGINAL id,
   making later by-id ops ambiguous → InvalidSplitPoint (fixed: right halves carry
   their real engine id).

## LIMITATIONS

- Single-threaded container CPU; absolute numbers indicative (E-002c methodology).
  The budget verdict (W4) is a 3.5–4.6× margin breach — robust to hardware class.
- True concurrent multi-track WRITING is not claimed: P8 proves disjoint-track shard
  hashing == sequential; the single-writer rule (ADR-008) stands.
- AVL by-id scan cache-hostility is measured but its mitigation (side index) is
  deferred to Phase 2 — v1 verbs are id-addressed through the scan.
- Real-GPU/real-device legs (E-005/E-004c) and real-Tauri IPC (E-006b) are unaffected
  by this record; E-006b re-attempted 2026-09-26: still BLOCKED (webkit2gtk-4.1 needs
  root; Xvfb present; no container runtime).

## DECISION IMPACT

- **ADR-011: ACCEPTED (REVISED at its own revisit gate)** — primary per-track structure
  = augmented order-statistic AVL (exact Rational subtree sums); gap buffer demoted to
  noted alternative (cursor-local sessions, small tracks, contiguous scans); the
  "lazily-rebuilt derived index" of the provisional decision is REJECTED by W4 evidence.
- **ADR-009: ACCEPTED** — per-verb property triples (apply/inverse/identity + batch
  composite + redo) now exist as committed CI-runnable tests (P1/P1b/P6), satisfying
  ADR-009's own named risk mitigation; E-009 36/36 covers cross-ownership.
- **ADR-008: ACCEPTED** — command-log schema now exercised by a real crate; schema
  rules gain #4 (explicit allocation) from Finding 1.
- Phase 2 legally open for `ove-project`/render layers on top of this crate.
