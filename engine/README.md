# engine/ — locked design facts

> Everything below is EVIDENCE-LOCKED: proven by a committed, re-runnable experiment
> or property suite in this repository. Do not weaken these rules without a new
> experiment that falsifies the evidence. Statuses per the directive's honest classes.

## Crates

| Crate | Status | What it locks |
|---|---|---|
| `ove-time` | **TESTED** (15/15) | Exact rational time (ADR-007): i64 num/den, den > 0, normalized; i128 intermediates; overflow = typed error, never wrap; fp never on the authoritative path (`from_f64_seconds_quantized` is legacy-interop only); tick-axis overflow guard (E-002c P3b). |
| `ove-timeline` | **TESTED** (10/10 properties + cross-validated bench, E-012) | Per-track sequences, edit verbs with exact inverses, batch atomicity, LIFO undo/redo, typed errors, replay determinism. Primary container: **augmented order-statistic AVL** (ADR-011 revised at integration); gap buffer = noted alternative; naive oracle mandatory for property tests (doc 45 rule 4). |

## Non-negotiable invariants (E-003 + E-009 + E-012)

1. **Explicit ids everywhere** — every logged command references clips by id.
2. **Exact `(num, den)` pairs** — floats are forbidden in command/log payloads
   (float payloads diverged 300/300 in E-003).
3. **Absolute starts are DERIVED** (prefix sums of durations) — never stored
   authoritatively; time-keyed indexes that rekey on edit hit the silent-collision
   trap (E-002c2) — forbidden.
4. **Engine-allocated ids ride the log explicitly** — `Split.new_id` is allocated at
   command-construction time; the engine never allocates during apply. A log with
   implicit allocation diverges on replay once removed ids leave holes (E-012 Finding 1).
5. **Every command ships an exact inverse**; batch inverse = reversed sub-inverses;
   undo stack is per-session, the log is the record (ADR-008/009).
6. **Per-verb property triple in CI** (doc 45 rule 1): apply → inverse → identity,
   re-apply exactness, oracle-model hash equality — see `ove-timeline/tests/properties.rs`.
7. **Single-writer** per timeline (ADR-008 v1); disjoint-track sharding is proven
   hash-equivalent to sequential application (E-012 P8).
8. **Typed errors, no panics** on user-input paths (E-004a discipline); `panic!` is
   reserved for engine-invariant violations (e.g. batch rollback failure).

## Measured performance anchors (container CPU, relative order)

- Exact rational add ≈ 140 ns/op — dominates timeline walks (E-002c).
- AVL vs gap vs oracle @20k clips (E-012): AVL wins random storms 5.5× and
  far-jumps 7.8×; gap wins cursor-local 8×, by-id scan 3×, small-N (2.5k) 3.6×.
- **Scrub-burst budget (1 ms, doc 34)**: AVL PASS (p99 0.37 ms) — gap+lazy-index
  FAIL (p99 4.58 ms). This is why ADR-011's provisional decision was revised.

## Build & test

```
cargo test --release          # in engine/ove-time and engine/ove-timeline
cargo run --release --bin e012_bench   # E-012 benchmark (writes nothing; stdout only)
```
