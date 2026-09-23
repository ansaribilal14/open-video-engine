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
  research/                   # Numbered research documents (01..44, per directive)
research/
  sources/                    # SOURCE_LEDGER.md — every source, recoverable
  claims/                     # CLAIM_EVIDENCE_LEDGER.md — beliefs tracked to evidence
  github/                     # Per-repository analysis
  papers/ youtube/ transcripts/ conferences/
  experiments/ benchmarks/    # EXPERIMENT_RECORDS per directive format
  architecture/               # Architecture options + dependency graphs
  adr/                        # Architecture Decision Records (36 planned)
  comparisons/ licenses/ risks/ unresolved/ synthesis/
engine/                       # (future) engine implementation — empty until gates pass
apps/                         # (future) editor clients — empty until engine proves itself
scripts/                      # research + benchmark tooling
STATUS.md                     # Honest subsystem status (IMPLEMENTED/TESTED/.../PLANNED)
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

**PHASE 0 — Research infrastructure + evidence gathering (v0.1 in progress).**
See `STATUS.md` and `docs/research/01_RESEARCH_INDEX.md` for what is actually known,
partially known, or open.

## License

TBD — pending the licensing audit (directive TRACK W). Do not assume permissive
licensing is compatible with all candidate dependencies (notably GPL/LGPL codec stacks).
