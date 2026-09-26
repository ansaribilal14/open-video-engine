# 26 — AI AGENT ARCHITECTURE FOR AN EDITOR (Track M)

> Status: PARTIAL (v0.1).

## 1. Scope

Architecture research for making AI agents first-class clients of the same command API
humans use (claim C-003). Sources verified by fetch on 2026-09-22: EditDuet paper
(S-4f1), Timeline Assembler paper (S-4f3), MCP spec + security best practices (S-4f5..),
Claude Agent SDK permissions doc (S-4f7), kinocut guardrailed MCP server (S-4f8), MCP
security papers (S-4f9). No application code written.

## 2. The canonical agent editing loop

Synthesis of EditDuet, Timeline Assembler, CineAgents, and production agent-tool
documentation (Claude Agent SDK). The loop an editor-hosted agent must run:

```
observe -> understand -> plan -> validate -> preview -> approve -> apply -> undo
```

1. **Observe**: read project state through the engine's query surface (timeline
   structure, clip metadata, transcripts, detected shots, audio analysis). The agent
   sees the SAME project document the human UI renders — no shadow state.
2. **Understand**: parse the user instruction against observed state (EditDuet's Editor
   takes NL instruction + clip collection + A-roll; CineAgents builds a hierarchical
   narrative memory of shot/event/story/character before planning).
3. **Plan**: produce an ordered batch of candidate commands (CineAgents: blueprint ->
   compiled script refinement; C-007's command log is the natural plan format).
4. **Validate**: dry-run the plan against engine invariants (frame-accurate time math
   per C-001, no dangling references, track/clip type rules) WITHOUT mutating state.
5. **Preview**: render a proxy of the result (playhead scrub, waveform, thumbnails).
6. **Approve**: human gate — see section 4; approval is per-transaction, not per-pixel.
7. **Apply**: commit the batch as one transaction to the command log (C-007).
8. **Undo**: rollback is just an inverse command batch in an event-sourced log (C-007);
   EditDuet's critic loop shows why iterations must be cheap and reversible.

## 3. What the published systems do (evidence)

- **EditDuet** (S-4f1): two agents — Editor manipulates the timeline through NLE tools
  (search, trim, add/remove clips); Critic returns NL feedback or approves by calling
  render. Iteration replaces one-shot planning; communication learned via synthetic
  in-context demonstrations. Lesson: specialize agents per concern; gate the RENDER
  call, not the intermediate attempts.
- **Timeline Assembler** (S-4f3): one multimodal LLM emits an edited timeline
  representation directly. Lesson: if the project format is serializable and diffable,
  the model output IS a project diff — then validation reduces to schema + invariant
  checks on the diff.
- **CineAgents** (S-4f2): plan-then-assemble with hierarchical memory beats
  retrieve-and-rank on long-form content. Lesson: agents need structured project
  context (events, characters) exposed by the engine, not raw shot captions.
- **Claude Agent SDK** (S-4f7): production permission pipeline evaluated in a fixed
  order: hooks -> deny rules -> ask rules -> permission mode -> allow rules ->
  canUseTool callback. Modes: default / acceptEdits / plan / bypassPermissions /
  dontAsk / auto. MCP tools can demand interaction via
  `_meta["anthropic/requiresUserInteraction"]`. Lesson: deterministic, ordered,
  declarative gating beats ad-hoc prompting.
- **kinocut** (S-4f8): an MCP video-editing server (Apache-2.0) whose pitch is
  "typed tools and Video Receipts — not invented FFmpeg flags": i.e., constrain the
  agent to validated tool schemas, emit receipts for every applied operation, quality
  gates between steps. Lesson: receipts = command-log audit trail; quality gates =
  validate/preview stages.

## 4. Approval patterns for destructive operations (surveyed)

| Pattern | Where seen | Editor mapping |
|---|---|---|
| Per-tool approval callback (canUseTool) | Claude Agent SDK (S-4f7) | risky commands (delete track, overwrite audio) trigger UI dialog |
| Declarative allow/deny/ask rules with fixed evaluation order | Claude Agent SDK + Claude Code settings (S-4f7) | engine command ACL: read-only commands allowlisted; deletions ask; bulk ops deny by default |
| Permission modes incl. read-only "plan" mode | Claude Code plan mode (S-4f7) | agent proposes; writes disabled; UI shows diff of would-be commands |
| Human-in-the-loop SHOULD on all tool calls | MCP spec tools/sampling warnings (S-4f5) | default stance: AI output requires explicit apply |
| Per-transaction approval (batch, not per command) | EditDuet critic->render gate (S-4f1) | one approval for a whole edit batch; undo returns whole batch |
| Receipts / audit log per applied op | kinocut Video Receipts (S-4f8) | command log entries rendered as human-readable receipts |
| Confirm-before-commit for irreversible external side effects | MCP elicitation URL-mode rules (S-4f5) | export/publish/share = always ask; in-project edits = preview suffices |

## 5. Deterministic command resolution (hard requirement)

AI output must resolve to editor commands; nothing may bypass the engine.

- Resolution contract: model emits intents against the SAME command schema humans use.
  Suggested shape: `{intent, arguments, target_refs, provenance}` where target_refs are
  engine-issued IDs (clip_id, track_id, timeline_range) — never free-form filenames or
  time strings parsed ad hoc.
- Validation ladder before apply: schema validation -> referential integrity ->
  time-base validation (rational seconds, C-001) -> policy check (section 4 rules) ->
  human approval (if policy says so) -> transactional apply (C-007).
- Any output that cannot resolve to a command is rejected as a model error (this is
  kinocut's "not invented FFmpeg flags" principle at the protocol level).
- Provenance: every AI-applied command records model identity, prompt hash, and the
  approval decision — enabling undo, review, and benchmark replay (CineBench-style).

## 6. Safety model summary (the eight gates)

- Observe: read-only queries; the agent cannot write during observation.
- Understand/Plan: pure functions over project state; no side effects.
- Validate: deterministic engine-side invariant checks; cheap; always run.
- Preview: render proxy; no state mutation; user sees result before approval.
- Approve: policy-driven; default = human applies; auto-apply only allowlisted
  read-committed analysis commands (detect shots, transcribe, tag).
- Apply: single transaction; command log entry with provenance (C-007).
- Undo: inverse batch; agent-initiated undo requires the same approval policy.
- Audit: immutable log + receipts (kinocut pattern, S-4f8).

## 7. Threats specific to an editor host (from S-4f9 + MCP security docs)

- **Prompt injection via media metadata**: captions, transcripts, EXIF, subtitles, or
  even model-generated summaries of footage are untrusted input. A poisoned subtitle
  track could carry "instructions" the agent reads. Mitigation: treat all media-derived
  text as data, never as instructions; strip/escape before model context.
- **Tool poisoning / rug pull**: malicious or compromised MCP tool servers redefine
  tools after approval (S-4f9: Tool Poisoning Attacks; MCP security best practices
  call out change notification + re-approval). Mitigation: pin tool servers, show diff
  on tool-list change notifications (`listChanged`), require re-approval.
- **Confused deputy**: agent holding user credentials performs actions the user never
  intended (MCP security best practices, S-4f5). Mitigation: least-privilege scopes per
  session; scope minimization section of the spec.
- **Destructive mass operations**: delete-all-tracks or export-overwrite storms.
  Mitigation: deny-by-default on bulk/irreversible commands; transaction caps.

## 8. Open questions (to unresolved register)

- Q: Should auto-apply exist at all for analysis commands, or should every command be
  user-visible in a pending tray? (proposal: two-tier policy)
- Q: One agent or a small team of specialists (EditDuet's duet vs CineAgents' studio)?
  Need an experiment on long-documentary projects before deciding.
- Q: Plan-mode UX for edits — diff visualization of command batches needs design input
  (track U / 34_UX).
