# E-009: Shared command surface — agent edits and UI edits in ONE transaction log

- **Date**: 2026-09-24 · **Question source**: OPEN_QUESTIONS Q-09 (proposal by agent 3-f) · **Gates**: ADR-010
- **Code**: `scripts/experiments/E-009_shared_command_surface.py` · **Raw output**: `experiments/E-009_result.txt`

## QUESTION
Do AI-agent edits and UI edits share one transaction log without schema divergence?
(Can the same command bus serve a human client and an MCP agent client with hash-identical
core state, owner-agnostic replay, and identical rejection semantics at both edges?)

## HYPOTHESIS
If commands carry explicit ids and exact `(num,den)` time payloads (E-003 lessons), then a
single log schema serves both clients: each command applied through the in-process surface
(UI shape) and through an MCP-stdio wire surface (agent shape) yields identical state;
undo is a logged marker invertible by either client; batches are atomic on both paths;
float payloads are rejected at both edges (fail-closed ADR-007 boundary).

## IMPLEMENTATION
- Engine: E-003 command model extended to **10 command surfaces** (add/remove/move/trim/
  split/merge/rename/set_gain + **batch** transaction + **undo** as logged marker), exact
  rationals only — wire type `{"num":int,"den":int}`; floats rejected by construction.
- Two surfaces, one logical scenario (12 interleaved UI/agent steps, incl. cross-ownership
  undos and an agent batch):
  - **PATH A (UI)** — in-process calls (browser-WASM/desktop-shell shape)
  - **PATH B (agent)** — MCP 2026-07-28 stdio JSON-RPC server (`initialize`, `tools/list`,
    `tools/call`) in a subprocess; schemas integer-only for time fields
- Event sourcing: undo = `{"op":"undo"}` log marker; replay folds forward commands +
  undo markers; batch = atomic (trial-apply on cloned doc before landing).

## HARDWARE
Container CPU; single host. Protocol byte-overhead/latency NOT benchmarked (separate IPC
evidence: E-006). Determinism/process-isolation focus.

## RESULT — 36/36 PASS
- **P7**: `tools/list` conformance — 10 command surfaces; all time/gain fields
  integer-only in schemas. PASS
- **P1**: hash equality (direct vs MCP engine) after **every** one of 12 interleaved
  steps. 12/12 PASS
- **P2**: wire-log replay in a fresh engine == final state hash (`c5ac4a37…`). PASS
- **P2b**: direct-path log carries both owner tags (`ui`, `agent`); **P2c**: direct-path
  log ≡ wire-path log byte-for-byte modulo owner provenance tags. PASS/PASS
- **P3**: cross-ownership undo — UI-issued undo removed the agent's batch; agent-issued
  undo removed the UI's split (LIFO over ONE shared stack; step-5 rename intact). PASS
- **P4**: batch atomicity — invalid inner command ⇒ whole batch rejected on BOTH surfaces
  (`isError` + `EngineError`), state hash and both logs unchanged. 3/3 PASS
- **P5**: float rejection at both API edges — bare `1.5` start, float inside `num`,
  float gain ⇒ wire `isError`, direct `TypeError`; zero state change. 3/3 PASS
- **P6**: full re-run in fresh subprocess + fresh log ⇒ identical final hash. PASS

## LIMITATIONS
- Harness drives the direct engine as two simulated clients (owner tags prove mixed
  ownership); a real UI and a real agent (Claude Code) were not attached — the MCP server
  is protocol-conformant but only exercised by the built-in driver.
- Protocol overhead/latency not measured here (see E-006 for transport cost evidence).
- 10 command surfaces cover the E-003 verb set + batch/undo; richer verbs (retime,
  ripple, keyframes) will need the same treatment at integration time.
- Single-process determinism only; multi-engine concurrency (locking/merge) out of scope.

## DECISION
**Shared command surface VIABLE.** One transaction log serves UI and agent clients without
schema divergence; undo is owner-agnostic; batches are atomic; the exact-time boundary is
enforceable at the API edge. Q-09 answered. ADR-010's gate cleared → **ADR-010 ACCEPTED**.
The MCP tool catalog (10 surfaces) becomes the seed for the ADR-027 tool registry.
