# ADR-008: Project format

- **Status**: ACCEPTED (2026-09-24) — acceptance condition met: the format's core
  mechanics were exercised experimentally, not just surveyed. Evidence: E-003 9/9
  (snapshot+log replay == apply; undo via inverses hash-exact; log JSON round-trip;
  snapshot+suffix equivalence; float payloads diverge 300/300 → schema rules), E-009
  36/36 (one log serves UI+agent owners; undo markers replay). Tracked engineering
  rules: T-6/T-8/T-9 in docs/research/45_TESTING_STRATEGY.md (replay corpora,
  crash-only persistence tests, migration fixtures). Revisit trigger: concurrent
  multi-editor scope change, or relational-query needs at scale (option D).
- **Date**: 2026-09-24 · **Confidence**: MEDIUM-HIGH

## CONTEXT
Projects must survive crashes (crash-only design), open across engine versions, support
diff/merge and agent tooling, and avoid the documented disasters of prior formats
(Kdenlive dual-store corruption, decimal-separator crash; OpenCut's no-portable-format).

## OPTIONS
A. Opaque binary blob per save · B. **Snapshot (manifest) + append-only command log**
(event-sourced, schema-versioned) · C. XML document like MLT/Kdenlive (full state per save)
· D. SQLite database project (Resolve-style)

## EVIDENCE
- E-003 9/9: snapshot + log replay == apply, undo via inverses restores state, log
  round-trips JSON, snapshot+suffix == full replay; float payloads diverge 300/300 →
  schema rules: explicit ids, (num,den) pairs, floats forbidden
- E-009 36/36: one log serves UI+agent; undo markers replay; owner-agnostic
- doc 19 survey: OTIO (interchange, not editing model), FCPXML rational seconds,
  OpenShot .osp JSON, Kdenlive XML+`.storable` dual-store failure history, Olive
  per-version serializers = cheapest migration pattern
- doc 31 (wave-3): folder layout manifest.json + content-addressed assets/ + disposable
  cache/ — regenerable-everything-except-manifest-and-assets
- doc 33: project files are an attack surface → schema validation, no code execution,
  zip-slip rules for archives

| Criterion | A binary | B snapshot+log | C XML state | D SQLite |
|---|---|---|---|---|
| Performance | fast | log grows; compact snapshots | slow big writes | good |
| Portability | poor | JSON = diff/merge/tool-friendly | good | tool-dependent |
| Complexity | low | event-sourcing discipline | serializer drift | schema migrations |
| Security | opaque | schema-validated, no exec | XXE-class risks | SQL not a risk here |
| License | — | none implicated | — | SQLite public domain |
| Maintenance | dead ends | per-version serializers (Olive pattern) | version pain | migrations |

## DECISION (provisional)
B — project = folder: `manifest.json` (schema version, asset registry by content hash,
tick rate) + `commands.jsonl` (append-only, schema-versioned, explicit ids, exact
rationals, undo markers per E-009) + `assets/` (content-addressed) + `cache/`+`renders/`
(disposable). Periodic snapshot compaction = manifest + log suffix (E-003 P3 equivalence).
Per-version serializers for forward migration; XML export adapters (OTIO/MLT) are
interchange, never the stored format.

## REJECTED ALTERNATIVES
A (undiffable, dead-end); C (Kdenlive's documented dual-store + locale disasters — [19]);
D (v1 defer; revisit if relational queries dominate at scale).

## CONFIDENCE & RISK
MEDIUM-HIGH. Risks: log growth (mitigate: snapshot compaction, E-003 P3); schema
evolution discipline (version field + migration tests from day one); concurrent
multi-editor editing is explicitly OUT of scope v1 (single-writer lock).
