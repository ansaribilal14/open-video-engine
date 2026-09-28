# ADR-016: Project persistence v1 (ove-project — manifest, log, snapshots)

- **Status**: ACCEPTED (2026-09-29) — acceptance condition met: PROJECT_
  FORMAT_SPEC §7 P-1..P-8 green in-repo (engine/ove-project/tests/
  acceptance.rs, 11 tests) including the real subprocess kill-9 drill.
- **Date**: 2026-09-29 · **Confidence**: 0.9 (authority order and crash
  model are the durable decisions; the log grammar grows additively)

## CONTEXT

WAVE 4 (ENGINE_BUILD_PLAN W5, pulled forward per the directive's wave order)
= ove-project: manifest.json + commands.jsonl + snapshot compaction, folding
through ove-timeline commands. Constraints: single-writer v1; the acceptance
gate is save → kill -9 → reopen → replay → state-hash equality; the log is
the project (corruption = hard stop + explicit repair, never auto-edit);
BLAKE3-256 is the only identity hash; core crates stay libav-free.

## DECISION

1. **Authority order (crash safety)**: (1) `commands.jsonl` is the RECORD
   (append-only, flushed to the OS before execute returns — the on-disk
   state after each call IS the kill-9 state); (2) `snapshot/meta.json` is
   the snapshot authority, written atomically AFTER `state-<seq>.json` is
   complete; (3) `manifest.json` is a HINT (identity, assets, track
   registry, last-known state), reconciled on open — a stale manifest is
   rewritten from the replayed truth, never trusted. Snapshot order:
   state file → meta → log rewrite → manifest. A crash at ANY point leaves
   a loadable project (event-sourcing safety argument, pinned by P-3).
2. **Log grammar (v1 refinement of the spec sketch)**: entries are
   `{seq, owner, kind, payload, undo?}` — `exec` entries carry the exact
   inverse in `undo`; `undo`/`redo` markers EMBED the inverse/forward
   command so a compaction suffix is self-contained (a marker may target a
   compacted-away exec; replay never reads below the snapshot seq).
   `target` (the exec seq) is informational. Timestamps are OMITTED in v1
   (nondeterministic content must not enter the replayed record; adds when
   a consumer needs them, stored outside the hash).
3. **Seq contiguity, anchored**: a log starts at `snapshot_seq + 1` (1 when
   no snapshot) and each entry is previous+1. Any gap = LOST COMMANDS =
   typed corruption error (hard stop) — truncation-tolerance would be
   silent data loss.
4. **Track registry in the manifest**: ove-timeline has no AddTrack command
   (tracks are project scaffolding; clips are the command surface), so the
   track structure `{id, kind}` lives in the manifest and is applied to the
   timeline before log replay; a loaded snapshot mirror (the state
   authority) refreshes it. Limitation, named: track-structure EDITS are
   not commands yet — when the keyframe/track model grows them (W8 era),
   they must become commands or the registry must version.
5. **Id-allocation state is document state (E-012 carry-over)**:
   `next_id` + `used_ids` are serialized in the state mirror and restored
   exactly (`Timeline::next_id_value()/used_ids()/from_parts()` added to
   ove-timeline — read-only accessors + constructor, no semantic change).
   Rationale: a snapshot does not replay removed-id history; losing the
   allocation cursor would diverge post-reopen `alloc_id()` from the live
   session (the exact E-012 divergence class).
6. **State hash**: BLAKE3-256 over the canonical serde serialization of the
   state mirror (declaration-order keys, BTreeMap tracks, sorted used_ids,
   {num,den} rationals, no floats). Covers the DOCUMENT (tracks, clips,
   id state, asset identities); per ADR-008 the undo stack is session state
   rebuilt by replay and is NOT hashed. `Timeline::state_hash()` (FNV u64)
   remains the internal oracle/property-test hash.
7. **Float rejection at load**: rationals deserialize as {num: i64, den:
   i64}; a float payload is a serde type error → `LogCorruption` → load
   aborts (P-5). Never repaired, never rounded.
8. **Repair path**: `Project::repair_log` quarantines the damaged log
   VERBATIM (`commands.corrupt-<stamp>.jsonl`) and rewrites the good
   prefix; callers re-open explicitly. The load abort is the gate; repair
   is never automatic (spec §3.3).
9. **Assets by hash**: import copies bytes into `assets/<blake3-hex>/`
   (dedupe), optional probe sidecar supplied by an ADAPTER layer
   (ove-project is core: no libav). `asset_status()` = Present | Missing |
   Corrupt (bytes re-hashed against the registry name — relink-by-path is
   forbidden, R-13); render/export refusal wires at W6.
10. **Kill-9 model**: P-2 is tested with a real subprocess abort (re-exec of
    the test binary in child mode, `--exact`, abort() = process death
    without destructors) plus drop-without-close equivalence at random
    points (write-through makes them identical by construction).
    fsync-to-disk (power-loss durability) is OUT of the v1 claim and
    documented as such in log.rs.

## NON-GOALS (deferred, named)

- Multi-writer/concurrency (ADR-008 v1 rule), fsync-grade durability claims,
  binary/zstd snapshot encoding (`.json(.zst)` hook exists in the spec
  layout), migration serializers (schema_version gates the reader; v1 → v2
  migration lands when a second version exists), track-structure commands.

## REJECTED ALTERNATIVES

- Persisting the undo stack in the snapshot (contradicts ADR-008; replay
  rebuilds it — P-7 verifies).
- Manifest as the loader authority (crash between append and manifest
  rewrite would drop commands; the log is the record, the manifest a hint).
- Auto-repair on load (spec: corruption is a hard stop with an explicit
  repair path — silent edits are the documentation≠implementation class).
- Adding AddTrack to ove-timeline commands (inverses/oracle properties
  ripple for zero Wave-4 value; registry covers the scaffolding honestly).

## CONFIDENCE & RISK

0.9. Risks: canonical JSON serialization is version-coupled (serde field
order changes change hashes — mitigated by schema_version + committed
fixtures when v2 arrives); the child-mode acceptance test depends on
`current_exe()` semantics (standard libtest behavior, CI-safe on
ubuntu/bundled paths); compaction of very large logs rewrites the file
(O(n) — acceptable at v1 scale, chunked rewrite is the known upgrade).
