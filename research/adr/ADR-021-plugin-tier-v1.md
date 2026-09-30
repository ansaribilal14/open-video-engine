# ADR-021 — Plugin tier v1 (process plugins as typed command-bus clients)

Date: 2026-09-30 · Status: **ACCEPTED** (wave 17) · Confidence: 0.9

## Context

The directive names the plugin tier: "the plugin-tier research (28_PLUGINS)
becomes a typed plugin client at the same command-bus boundary (process/WASM
tier behind the same Engine surface; no new semantics outside the command
grammar)." 28_PLUGINS surveys four models (native in-process, declarative,
in-process embed, out-of-process, WASM) and proposes three tiers: A (native
Rust trait effects), B (WASM sandboxed effects), C (out-of-process helper
services). §8 fixes the boundary law: frames cross into EFFECTS; effects
NEVER emit commands; findings reach the project as proposed commands through
the command bus. §9 fixes the security law: manifest-declared capabilities,
default DENY. §10 fixes the evidence law: every invocation attributable to
plugin id + version.

## Decision

1. **Tier C (out-of-process plugins) lands first**, as the `ove-plugin`
   crate. Tier A (native trait effects) and Tier B (WASM) stay future work
   — the research itself marks the WASM leg unverified (§11: fuel/epoch
   API names unread) and §12 scopes a wasmtime prototype as its own
   experiment. Landing the process tier first gives the boundary law a
   real conformance surface without new heavy dependencies.
2. **The ONLY write path across the plugin boundary is a proposal** — a
   command in the same typed grammar the CLI/batch/MCP/script clients use
   (add_track/add_clip/split/resize/move_clip/remove_clip/set_keyframes),
   applied through the SAME engine surface. The plugin boundary adds zero
   new engine semantics. import_media is NOT in the grammar — it is a
   filesystem grant (later capability, default DENY).
3. **Protocol v1** is line-delimited JSON over the plugin's stdio:
   manifest (required first) → hello; context (host push, read-only
   document view: state hash, tick axis, per-track clip windows with
   exact-rational durations); proposals; proposal_result receipts; log
   relay; done. Protocol version mismatches are typed rejections — no
   compatibility shims between generations (§10; N-1 policy is later).
4. **Rejection taxonomy** (mirrors the MCP split): protocol violations
   ABORT as typed errors (non-JSON line, unknown message type, manifest
   ordering/duplicates, version mismatch, unknown capability name, EOF
   before done); payload problems REJECT the individual proposal in-band
   (bad rational shape — ME-7: a JSON number in a rational field never
   coerces; unknown verb/property/interp; undeclared capability; engine
   rejection) and the session continues.
5. **Default-DENY capabilities** (§9): a plugin may propose only what its
   manifest declared; host v1 knows exactly one capability
   (`propose.timeline`); unknown capability NAMES abort the manifest.
6. **Receipts are deterministic evidence** (§10): every proposal is
   receipted (id, verb, applied/error, state hash after); the session
   report carries plugin name+version and carries NO timestamps — same
   project + same plugin behavior → byte-identical receipts.
7. **Plugins never see frames, the filesystem, the project uuid, or the
   clock.** The context is document state only — exactly what batch
   `status` sees (client parity discipline).

## Consequences

- A plugin edit is an ordinary engine edit: validated, journaled,
  undoable, state-hashed, replay-deterministic. The conformance tests pin
  undo/redo over a plugin-applied Split, including the ADR-016/E-012 rule
  that id-state is document state (undoing an allocation-consuming command
  restores structure but never the pre-command hash; redo identity is
  exact).
- Reference plugin `ove-plugin-stub` (deterministic; derives one
  content-neutral structural proposal from the context) doubles as the
  real-media gate instrument.
- The user-approval step of the 26_AI_AGENTS ladder is NOT in v1: validated
  proposals auto-apply with receipts. The approval gate is a UI/agent-wave
  concern and is recorded as a reopen item.

## Reopen

- User-approval ladder step (validate → approve → apply) at the plugin edge.
- Tier B WASM prototype (wasmtime, component model, `ove:effect` WIT) as its
  own experiment; Tier A native trait effects with the pass graph.
- Capability grants beyond `propose.timeline` (import, filesystem, network)
  with a real permission broker.
- Plugin distribution/signing/notarization (§9: recorded UNVERIFIED).
- Protocol N-1 compatibility policy.

## Falsification evidence

- `ove-plugin/tests/plugins.rs` — 5 conformance tests: command-bus flow
  (apply → receipt → undo/redo identity), deterministic receipts, typed
  protocol aborts (9 violation shapes), in-band payload rejections with
  session continuation, default-DENY.
- `ove-plugin/tests/realworld_gate.rs` + REALWORLD_VALIDATION §5 W17 rows —
  the certification loop on the real reference media.
