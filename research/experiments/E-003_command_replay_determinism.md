# E-003 — Command log + snapshot replay determinism (project format core)

> Status: RUN-COMPLETE (9/9 property checks).

- **QUESTION**: Can a project be snapshot + serializable command log with exact replay,
  exact undo (hash-equality), JSON round-trip, snapshot+suffix equivalence — and must the
  schema forbid float time payloads?
- **HYPOTHESIS**: Yes with exact (num,den) payloads; float payloads break replay determinism.
- **IMPLEMENTATION**: `scripts/experiments/E-003_command_replay_determinism.py` — document
  model (tracks/clips, exact Fraction times), commands AddClip/RemoveClip/MoveClip/TrimClip/
  SplitClip with exact inverses; JSON log schema; 300–400-command random streams (3 seeds)
  with a simulated validity-guaranteeing generator.
- **HARDWARE**: container CPU (schema/determinism focus; no perf claims).
- **RESULT** (9/9 PASS):
  - P1 (×3 seeds): apply == replay-from-log (exact canon equality) at ~290 commands each.
  - P1b: JSON round-trip replay is hash-identical.
  - P2/P2b/P2c: undo via exact inverses; hash-equality restore of single-clip base state;
    base-clip identity preserved through 200 ops incl. splits/merges.
  - P3: snapshot + suffix-log replay == full replay (hash equality).
  - P5: **300/300 reconstructed starts diverge under two legal fp policies** (float
    payloads); rational schema diverges 0/300 by construction.
  - **Schema lessons recorded**: (1) every id must be explicit in the log (AddClip cid,
    SplitClip right-id) — apply-time id derivation makes logs interpretation-dependent;
    (2) RemoveClip inverse needs saved state; (3) log schema v0: entries carry explicit
    ids + (num,den) pairs, floats forbidden.
- **LIMITATIONS**: model is minimal (no ripple/track-wide ops, no effects); Python speed
  irrelevant to schema conclusions; inverse-fidelity for remove-add cycles needs the Rust
  property suite.
- **DECISION**: **C-007 upgraded: command log + snapshot is VIABLE as the project-format
  core** (was MEDIUM hypothesis). ADR-008 direction set: snapshot + versioned command log;
  blocked items for ACCEPT: Rust property suite + E-002c structure + migration story.
