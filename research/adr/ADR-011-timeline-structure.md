# ADR-011: Timeline primary data structure

- **Status**: PROPOSED (evidence complete; integration pending Phase 2)
- **Date**: 2026-09-24 · **Confidence**: MEDIUM-HIGH

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
pattern observed in surveyed NLEs (doc 20); plus a **derived, lazily-rebuilt ordered
index** (BTreeMap or interval index) for random/agent access and hit-testing; clip
identity = explicit ids already mandated by E-003. Piece table remains the noted
alternative if undo-by-piece proves superior at integration.

## REJECTED ALTERNATIVES
A: measured non-editor-grade (45.6 ms/5000 splits; 37.4 ms cursor splits).

## CONFIDENCE & RISK
MEDIUM-HIGH. Risks: (1) gap-buffer far-jump worst case unbenchmarked — mitigation:
derived index + benchmark at integration; (2) memory: 256-slot gap chunks per track —
bounded, small; (3) multi-track concurrency untested — Phase 2 gate.
Revisit trigger: integration benchmark fails editor budgets (scrub burst ~1 ms,
doc 18) or E-006b changes IPC picture.
