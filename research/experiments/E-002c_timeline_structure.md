# E-002c — Timeline primary structure (gap-buffer vs BTreeMap vs piece-table vs Vec)

> Status: RUN-COMPLETE (3 reps × 4 workloads × 4 structures; hard correctness gates PASS).

- **QUESTION**: E-002b showed 5000 Vec-splits = 45.6 ms on a 20k-clip track — not
  editor-grade. Which primary per-track sequence structure should the timeline use?
- **HYPOTHESIS**: editing is cursor-local (Kdenlive/Shotcut op patterns, doc 20), so a
  gap buffer should dominate; an ordered map should win scattered edits; a piece table
  trades identity stability for move cost; naive Vec loses everywhere.
- **IMPLEMENTATION**: `scripts/experiments/E-002c_timeline_structure.rs` (rustc -O, std
  only). Four structures under IDENTICAL logical op sequences:
  1. `Vec<Clip>` baseline; 2. `GapTrack` (Vec + moving gap, 256-slot chunks);
  3. `BTreeTrack` (BTreeMap with fractional midpoint **u128** keys + `id→key` aux map,
  counted full-rekey fallback); 4. `PieceTrack` (append-only clip arena + 16-byte pieces).
  Design honesty: structures store `(dur, id)` only — absolute starts are DERIVED prefix
  sums (eager start maintenance would add identical O(n) rational arithmetic to every
  candidate and drown the structural signal). Timing wraps only the op loop; BTree aux
  maintenance is included (part of that structure's real cost); driver order-vectors are
  not.
- **HARDWARE**: container CPU (2 cores); rustc 1.98.1 -O. Numbers indicative.
- **RESULT** (medians over 3 reps, ms; full output `experiments/E-002c_result.txt`):

  | workload (20k clips)        | Vec  | Gap  | BTree | Piece |
  |---|---|---|---|---|
  | W1 cursor-split ×5000       | 37.4 | **1.1** | 8.7 | 13.0 |
  | W3 fixed-point delete ×2500 | 18.5 | **0.0** (<0.1) | 3.4 | 6.2 |
  | W4 scatter-delete ×2500     | 9.8  | 7.3 | **1.8** | 3.0 |
  | W5 walk ×200 (render pass)  | 561.8 | 563.0 | 604.1 | 569.6 |

  - Cross-structure FNV hash equality over final `(dur,id)` sequence: **PASS** for all
    workloads × reps (all four structures implement identical edit semantics).
  - Fractional-key rekey events: **0** across all reps (u128/2^60 spacing absorbs the
    benchmark; adversarial same-gap re-splitting would trigger O(n log n) rekeys —
    counted, not hidden).
  - W5 surprise: walk cost is dominated by exact rational ADDITION (~140 ns/op, gcd
    path), not traversal — BTreeMap pointer-chasing costs only ~7% over contiguous
    storage. **Implication**: prefix-sum caching/incremental maintenance is the walk
    optimization lever, not the container.
- **LIMITATIONS**: two adversarial patterns tested (cursor-local, uniform-random);
  adversarial far-jump cursor (gap buffer worst case) not benchmarked; no memory
  profiling beyond struct sizes (Clip=24B, Piece=8B); single-threaded.
- **DECISION**: **Gap buffer is the provisional primary per-track structure** (34×
  faster than Vec on the dominant edit pattern; O(1) cursor deletes), with a derived,
  lazily-rebuilt ordered index for random access — recorded as ADR-011 (PROPOSED).
  Vec rejected definitively; piece table noted as the stable-identity/undo alternative;
  BTreeMap reserved for derived indexes, not the primary store. Feed into Phase 2.
