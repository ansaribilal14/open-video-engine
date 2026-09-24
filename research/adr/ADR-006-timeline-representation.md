# ADR-006: Timeline representation

- **Status**: PROPOSED (subordinate structure decided in ADR-011)
- **Date**: 2026-09-24 · **Confidence**: MEDIUM-HIGH

## CONTEXT
The timeline is the engine's central state. It must support exact time (ADR-007),
editor-grade edits (E-002c), derived query (hit-testing, agents), keyframed effect
parameters, and both human and agent clients (ADR-010).

## OPTIONS
A. Absolute fp seconds everywhere · B. Per-track ordered clip lists, rational times,
separate effect layer · C. Everything-is-a-node graph (Olive/Blender style) ·
D. **Hybrid**: B for the editing model + node graph only for the compositor render graph

## EVIDENCE
- Industry consensus = per-track ordered clip lists with in/out + separate effect layer
  (OTIO, MLT playlist, Kdenlive, Shotcut, Flowblade) — [03, 19, 20, S-2b*]
- fp timeline failure modes measured: floor() off-by-one, non-associative sums,
  23.976 drift 3.6ms/h; E-002 13/13; C-001 HIGH — [E-002]
- E-002c: gap-buffer primary structure 34× Vec on cursor-local edits; BTreeMap wins
  scattered; walk bounded by rational add (~140ns) — [E-002c, ADR-011]
- E-003/E-009: commands with explicit ids + exact rationals replay hash-exact — the
  timeline state must be a pure fold of the command log
- doc 20 edit-op matrix: move/resize, splice/extract, retime, split, structural ops map
  1:1 onto Kdenlive request*Action / Shotcut QUndoCommand verbs

| Criterion | A fp | B clip lists | C all-node | D hybrid |
|---|---|---|---|---|
| Performance | n/a (incorrect) | best for edits (E-002c) | graph overhead per edit | best of both |
| Portability | broken cross-engine | high | high | high |
| Complexity | low but wrong | moderate | high (editing UX hard) | moderate |
| Security | — | payload-validated commands | — | payload-validated commands |
| License | — | none implicated | — | none implicated |
| Maintenance | — | matches NLE norms | niche | two representations, one boundary |

## DECISION (provisional)
D — timeline = per-track ordered clip sequences (primary structure: gap buffer per
ADR-011) + derived ordered index for hit-testing/agents; clips carry explicit ids,
exact rational in/out, keyframe tracks per property; compositing relationships expressed
in a render graph compiled per frame (doc 08 pass-graph), NOT stored as the editing
model. State = fold(command log) per E-003/E-009.

## REJECTED ALTERNATIVES
A (experimentally falsified); C (node-graph editing UX is the documented weak point for
timeline-centric NLE work — Olive keeps tracks for editing despite node compositor).

## CONFIDENCE & RISK
MEDIUM-HIGH. Risks: keeping the two representations in sync (clip effects → graph nodes)
— mitigate: graph is derived/compiled, never user-persistent; keyframe model details
(assignment/interpolation) deferred to Phase 2 design with doc-29 expressions evidence.
