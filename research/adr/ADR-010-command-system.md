# ADR-010: Command system (human + AI share one API)

- **Status**: ACCEPTED (2026-09-24 — gate cleared by E-009, 36/36 PASS)
- **Date**: 2026-09-21 · **Confidence**: MEDIUM-HIGH

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
- **E-009 (2026-09-24, 36/36 PASS)**: 10 command surfaces wrapped in an MCP 2026-07-28
  stdio server; UI-shaped (in-process) and agent-shaped (wire) invocations produce
  hash-identical state at every step; one transaction log, owner-agnostic replay,
  cross-ownership undo, atomic batches; float payloads rejected at both API edges —
  Q-09 answered [research/experiments/E-009_shared_command_surface.md]
- **E-003 (9/9)**: replay/undo/snapshot hash-exact; explicit ids + (num,den) payloads
  mandatory — schema basis for the command bus

## DECISION
**B — single ordered command stream**: schema-versioned commands, inverse commands for
undo, batch transactions (validate→preview→approve→apply→receipt), tiered permissions
(analysis auto-allow; destructive deny-by-default). MCP = thin versioned shell. ACCEPTED
on the strength of E-009 + E-003 + convergent industry implementations.

## REJECTED ALTERNATIVES
A: violates directive + re-creates OpenCut-classic's early split pain — [02].

## CONFIDENCE & RISK
MEDIUM-HIGH. Proven: schema determinism, shared-surface equivalence, undo semantics,
batch atomicity (E-009/E-003). Open at integration: full verb set (retime, ripple,
keyframes) through the same bus; approval-tier table (Q-10) for destructive batches;
real Claude Code end-to-end run over the E-009 server (follow-up).
