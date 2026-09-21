# ADR-010: Command system (human + AI share one API)

- **Status**: PROPOSED (GATE-9 pending experiment Q-09)
- **Date**: 2026-09-21 · **Confidence**: MEDIUM

## CONTEXT
AI agents must not have a secret alternate editing path; undo/redo, autosave, diff,
and AI transactions all want the same primitive.

## OPTIONS
A. UI events + separate AI planner calls into model directly · B. One command API; UI,
CLI, scripting, MCP, agents all emit commands; undo = inverse commands · C. Pure
event-sourcing (commands are the storage)

## EVIDENCE
- Convergent independent implementations: Cutlass (AI rides UI command layer, ToolTier +
  dry-run + runaway fuse), Diffusion Studio (zod tool catalog = MCP tool == CLI command),
  OpenReelio (event-sourced undo + loopback MCP server) — [02, 39]
- Research: EditDuet (SIGGRAPH 2025) agents edit only via NLE tool surface; Timeline
  Assembler (NeurIPS 2024 ws) emits timeline diffs — [25, 38] → C-003 MEDIUM
- NLE validation of command-verb undo at scale: Kdenlive lambda-pairs, Shotcut
  QUndoCommand — [03]
- B ⊃ C: command log can BE the project log (C-007) without forcing event-sourcing now.

## DECISION
(Provisional) B: single ordered command stream with schema-versioned commands, inverse
commands for undo, batch transactions (validate→preview→approve→apply→receipt), tiered
permissions (analysis auto-allow; destructive deny-by-default). MCP = thin versioned shell.

## REJECTED ALTERNATIVES
A: violates directive + re-creates OpenCut-classic's early split pain — [02].

## CONFIDENCE & RISK
MEDIUM. Open: exact command schema (Q-05), approval tier table (Q-10), MCP wrap
experiment (Q-09).
