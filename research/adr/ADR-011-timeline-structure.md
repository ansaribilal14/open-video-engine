# ADR-011: Timeline primary data structure

- **Status**: ACCEPTED (2026-09-26) — **REVISED AT ITS OWN INTEGRATION REVISIT GATE (E-012)**:
  the provisional two-layer decision (gap buffer primary + AVL derived index) is replaced
  by the single-layer augmented order-statistic AVL, measured in the production crate
  `engine/ove-timeline`. Decision text history preserved below (2026-09-24 fold of the
  E-002c2 addendum; original 2026-09-24 provisional).
- **Date**: 2026-09-24 · **Confidence**: MEDIUM-HIGH (integration-measured)

## CONTEXT
Per-track clip sequence needs editor-grade edit performance. E-002b measured naive
Vec+insert at 45.6 ms for 5000 splits on a 20k track — rejected. E-002c benchmarked
four candidates under identical logical op sequences with hard cross-structure
correctness gates (research/experiments/E-002c_timeline_structure.md).

## OPTIONS
A. `Vec<Clip>` — simplest; O(n) memmove per mid-track insert/delete.
B. Gap buffer — O(1) at cursor, O(dist) to move the gap; contiguous iteration.
C. Ordered map (BTreeMap, fractional midpoint u128 keys + id→key aux) — O(log n)
   insert/delete anywhere; pointer-chasing iteration; rekey fallback on key-space
   exhaustion (counted).
D. Piece table — immutable clip arena + 16 B pieces; stable identities; O(n) memmove
   of small pieces; natural fit for undo-by-piece-add/remove.

## EVIDENCE (E-002c medians, 20k clips, container CPU)
- cursor-split ×5000: Vec 37.4 ms | **gap 1.1 ms** | BTree 8.7 ms | piece 13.0 ms
- fixed-point delete ×2500: Vec 18.5 ms | **gap <0.1 ms** | BTree 3.4 ms | piece 6.2 ms
- scatter-delete ×2500: Vec 9.8 ms | gap 7.3 ms | **BTree 1.8 ms** | piece 3.0 ms
- walk ×200: 562–604 ms across ALL structures — bounded by exact rational addition
  (~140 ns/op), not traversal ⇒ prefix-sum caching is the optimization lever.
- Hash equality across structures PASS (all workloads × reps); 0 rekey events.

## DECISION (provisional, to verify at integration)
**Gap buffer as the primary per-track sequence**, matching the cursor-local edit
pattern observed in surveyed NLEs (doc 20); plus a **derived ordered index that is an
augmented order-statistic AVL tree** (subtree count + duration sum, implicit order
key), incrementally maintained — NOT lazily rebuilt. E-002c2 (addendum in
research/experiments/E-002c_timeline_structure.md) showed the AVL wins every verb
under uniform-random edit positions (split 2.1 ms / resize 0.4 ms / move 0.9 ms @20k),
giving O(log n) point/rank queries without an O(n) index rebuild after every cursor
edit — the strongest known shape for agent/script-driven batch access (E-009 surface)
where no cursor locality exists. Time-keyed BTreeMaps remain DISQUALIFIED (the
E-002c2 single-pass rekey collision bug — 17 silent clip losses — independently
reconfirmed the derived-starts rule; two-phase rekey mandatory if ever used). Clip
identity = explicit ids already mandated by E-003. Piece table remains the noted
alternative if undo-by-piece proves superior at integration.

## REJECTED ALTERNATIVES
A: measured non-editor-grade (45.6 ms/5000 splits; 37.4 ms cursor splits).

## INTEGRATION EVIDENCE (E-012, 2026-09-26 — production crate engine/ove-timeline)

E-012 built the verb surface as a real crate (exact Rational time, explicit ids,
exact inverses, batch atomicity, undo/redo, typed errors) with three interchangeable
containers behind one `TrackOps` surface — the provisional shape (GapTrack: gap buffer
+ lazily rebuilt prefix index), the E-002c2 shape (AvlTrack: augmented AVL alone), and
a naive oracle — cross-validated by hash+count+total after every workload. Property
suite 10/10. Bench @20k clips (container CPU, single-run indicative):

| Workload | GapTrack (gap+lazy index) | AvlTrack (AVL alone) | Verdict |
|---|---|---|---|
| W1 cursor-local splits ×5000 | **5.7 ms** | 47.4 ms | gap 8× (its home turf) |
| W2 random storm ×9500 | 502.2 ms | **83.9 ms** | avl 6× |
| W3 far-jump ×5000 | 471.9 ms | **47.2 ms** | avl 10× |
| W2' by-id command leg ×2000 | **106.7 ms** | 360.1 ms | gap 3.4× (contiguous id scan) |
| **W4 scrub-burst p99 (1 ms budget, doc 34)** | **3.62 ms — FAIL** | **0.52 ms — PASS** | **DECISIVE** |
| W5 mixed by-id edits @2.5k/track ×800 | **6.6 ms** | 21.1 ms | gap 3.2× at small N |

## DECISION (final, revised at integration)
**The augmented order-statistic AVL ALONE is the primary per-track structure.** It is
the only measured shape that satisfies the 1 ms scrub-burst budget in the edit→scrub
alternation regime — the regime E-009 makes first-class (agents edit through the same
surface humans scrub on). The gap buffer's residual advantages (cursor-local 8×,
contiguous id scan 3.4×, small-track 3.2×) are real but do not justify the two-layer
design's costs (double bookkeeping, memory, and — with a lazily rebuilt index — the
W4 budget breach; an incrementally-maintained AVL index alongside the gap buys the
AVL's query speed at AVL's cost PLUS the gap's, with nothing left that the AVL alone
doesn't provide). The gap buffer remains the NOTED alternative, retained in the crate
(`GapTrack`) with the oracle (`OracleTrack`), interchangeable behind `TrackOps` —
revisit if render-walk profiling shows cursor-local storms dominating. Absolute starts
are never stored; time-keyed indexes stay DISQUALIFIED (E-002c2 rekey-collision rule).
E-012 additionally fixed schema rule #4 (engine-allocated ids ride the log explicitly,
ADR-008) — found by the oracle-equivalence property, not by review.

## CONFIDENCE & RISK
MEDIUM-HIGH (integration-measured). Named residuals: (1) AVL by-id scan is
pointer-chase-bound (~3.4× slower than contiguous) — accepted on the edit path at v1;
Phase 2 may add a side index under explicit invalidation discipline (E-002c2 rekey
rules). (2) Single-writer stands (ADR-008); E-012 P8 proves disjoint-track sharding
hashes sequentially. (3) Keyframe/effect-layer representation stays with ADR-006.
Revisit triggers: W4 regime breach at real-world track sizes; render-walk profiles
showing AVL in-order walk dominating frame budget; concurrency beyond sharding.
