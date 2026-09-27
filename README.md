# Open Video Engine (OVE)

A research-driven, open-source **universal video engine**: media pipeline + timeline +
compositor + project model + render engine + AI/agent API + plugin system + headless
automation — designed so that editors (mobile / browser / desktop), AI agents, scripts,
and servers are all **clients of the same engine**.

> **Mission directive**: [`docs/MISSION_DIRECTIVE.md`](docs/MISSION_DIRECTIVE.md)
>
> Operating principle: `LEARN → VERIFY → COMPARE → EXPERIMENT → BENCHMARK → ARCHITECT →
> PROTOTYPE → IMPLEMENT → TEST → BENCHMARK → AUDIT → FIX → REPEAT`

## Repository layout

```
docs/
  MISSION_DIRECTIVE.md        # The governing mission directive (verbatim)
  DEV_ENV.md                  # Canonical unprivileged build recipe (60/60 reproducible)
  specs/                      # Normative contracts (FRAME_CONTRACT, DECODER_SPEC, ...)
  research/                   # Numbered research documents (01..45, per directive)
research/
  adr/                        # Architecture decision records (001..012)
  audit/                      # Forensic audits incl. FRESH_TAKEOVER_AUDIT.md
  sources/                    # SOURCE_LEDGER.md — every source, recoverable
  claims/                     # CLAIM_EVIDENCE_LEDGER.md — beliefs tracked to evidence
  experiments/ gates/ risks/ unresolved/ synthesis/      # evidence + decision tracking
engine/                       # The engine workspace (Rust):
  ove-time                    # exact rational time (TESTED)
  ove-timeline                # edit verbs, undo/redo, AVL structure (TESTED)
  ove-media                   # asset identity, FrameEnvelope, frame pool (TESTED)
  ove-decode                  # decoder abstraction + FFmpeg-SW adapter (TESTED, libav-confined)
experiments/                  # experiment harness sources
scripts/                      # corpus generation, CI guards, env recipes
STATUS.md                     # Honest subsystem status (IMPLEMENTED/TESTED/.../PLANNED)
docs/MASTER_ENGINE_STATE.md   # The single current work order
```

## Rules this repository enforces on itself

1. **No architecture without evidence.** Major claims live in the claim/evidence ledger
   with confidence levels, counter-evidence, and (where feasible) experiments.
2. **No fake completion.** Every subsystem is classified honestly in `STATUS.md`.
   Documentation is never substituted for engineering.
3. **No architectural fashion.** Subsystems justify themselves; smallest sufficient
   architecture wins.
4. **Engine first.** Applications, AI, scripting, MCP, and plugins are clients.
5. **Non-destructive by construction.** Edits are references + ranges + transforms +
   effects + commands; source media is never modified.
6. **Human and AI share one command API.** No secret alternate editing path for agents.

## Current phase

**Waves 0–2 landed (v0.8): four engine crates TESTED, decoder conformance
D-1..D-12 green, CI-enforced libav confinement.** Current work order:
`docs/MASTER_ENGINE_STATE.md` (single source of truth). Detailed per-subsystem
status: `STATUS.md`.

## License

The engine's own code is dual-licensed `MIT OR Apache-2.0` (LICENSE-MIT,
LICENSE-APACHE, per-crate `license` fields). Dependency license discipline is
audited in `docs/research/35_LICENSES.md`; libav* linkage is isolated to
`engine/ove-decode` and configured LGPL-only (DECODER_SPEC §4). Distribution
strategy for codec-dependent artifacts remains gated on the wave-21
production-readiness audit.
