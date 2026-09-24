# ADR-009: Undo/redo

- **Status**: ACCEPTED (2026-09-24) — acceptance condition met: inverse-command
  mechanics exercised experimentally at schema level. Evidence: E-003 9/9 (undo via
  exact inverses restores state hash-exact), E-009 36/36 (cross-ownership undo over
  ONE LIFO stack; composite inverse for batch atomicity; undo markers replay in a
  fresh engine). The former blocker "per-verb property tests in CI" is encoded as
  standing rule T-1 in docs/research/45_TESTING_STRATEGY.md and applies to every
  verb added henceforth — it is a forward obligation, not an open decision.
- **Date**: 2026-09-24 · **Confidence**: MEDIUM-HIGH

## CONTEXT
Undo must be exact (no fp), work across UI/agent/scripting clients, survive batches as
single steps, and compose with the command log being the project record (ADR-008).

## OPTIONS
A. Full-state snapshots per step · B. **Inverse commands** (exact), batches = composite
inverse, undo = logged marker · C. Piece-table add/remove undo (E-002c option D) ·
D. Replay-from-empty to any point (pure event-sourcing undo)

## EVIDENCE
- E-003 9/9: undo via exact inverses restores state; replay==apply; snapshot+suffix
  equivalence → undo need not discard history
- E-009 36/36: cross-ownership undo (UI undo removes agent batch; agent undo removes UI
  split) over ONE LIFO stack; composite inverse for batch atomicity; undo markers replay
  hash-exact in a fresh engine
- NLE survey: Kdenlive lambda-pairs (closure inverses), Shotcut QUndoCommand,
  Olive NodeUndo — inverse-command pattern is the industry norm — [03, 02]
- doc 34: UX requires transaction labels + grouping levels → commands carry labels;
  engine exposes grouping primitives

| Criterion | A snapshots | B inverse commands | C piece-table | D replay-to-point |
|---|---|---|---|---|
| Performance | O(state) per step | O(1) typical | O(1) structural | O(log) recompute |
| Portability | high | high | ties model to structure | high |
| Complexity | memory blowup | inverse discipline per verb | rework of model | replay cost |
| Security | — | same validation as forward | — | same |
| License | — | — | — | — |
| Maintenance | every verb twice (serialize) | every verb once + inverse | — | cheap |

## DECISION (provisional)
B — every command ships an exact inverse; batch = composite inverse (reversed sub-inverses);
undo appends a logged marker (redo history = un-undo); transaction labels ride command
metadata for UX grouping (doc 34); undo stack is per-session, NOT persisted state — the
log itself persists (ADR-008). C remains the noted alternative if piece-identity undo
proves superior at Phase 2 integration (carried from ADR-011).

## REJECTED ALTERNATIVES
A (memory + serialization cost at editor scale); D (replay latency per undo step is
avoidable and would punish deep histories).

## CONFIDENCE & RISK
MEDIUM-HIGH. Risks: inverse correctness for every future verb (mitigate: property tests
like E-003 per verb — "apply then inverse then apply = identity" invariant in CI);
undo of commands with external effects (renders/analysis) = cancel-or-orphan policy
deferred to ADR-029 headless design.
