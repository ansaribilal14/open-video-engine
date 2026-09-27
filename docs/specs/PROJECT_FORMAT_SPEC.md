# PROJECT FORMAT SPEC — manifest + command log + snapshots (v1, 2026-09-26)

> Parent: ADR-008 + E-003 + E-009 + doc 19/31/33. The format is a folder; the acceptance
> gate is save/kill/reopen/replay hash-equality. Single-writer v1 (concurrency explicitly
> out of scope).

## 1. On-disk layout

```
my-project.ove/
├── manifest.json          // the ONLY hand-editable-ish file; schema_version + registry
├── commands.jsonl         // append-only command log (post-snapshot suffix)
├── snapshot/
│   ├── state.json(.zst)   // compacted state at snapshot_seq
│   └── meta.json          // {snapshot_seq, state_hash, engine_version, created}
├── assets/
│   └── <content-hash>/    // content-addressed original media (+ sidecar probe cache)
│       └── probe.json     // streams, keyframe index, color tags, duration (ove-media)
├── renders/               // export outputs (disposable, hash-recorded in manifest)
└── cache/                 // any derived data (disposable, regenerable)
```

- Disposable rule (doc 31): deleting cache/ + renders/ must never change state_hash.
- Assets are referenced by content hash only — relink-by-path is forbidden (R-13
  proxy-relink rule: wrong media after relink; hash is the identity).

## 2. manifest.json (normative fields)

```json
{
  "schema_version": 1,
  "format": "ove/project",
  "tick_axis": { "rate_num": 48000, "rate_den": 1 },
  "created_by": { "engine": "ove", "version": "0.1.0" },
  "state": { "snapshot_seq": 12, "log_len": 340, "state_hash": "blake3:…" },
  "assets": [ { "id": "a-1", "sha256": "…", "path": "assets/<sha>/src.mp4",
                "probe": "assets/<sha>/probe.json" } ],
  "uuid": "…"
}
```

- `tick_axis` is the project-wide fixed rational axis for aggregates (ADR-007
  refinement; P3b rule). Per-clip times may carry their own denominators; accumulation
  happens on the axis.
- `state_hash` is the canonical hash of the folded state (same canonicalization as
  receipts) — it is what the acceptance suite compares.

## 3. commands.jsonl (the log)

1. One JSON object per line: `{seq, id, owner, kind, payload, undo: Marker|Inverse, ts?}`.
2. Schema rules from evidence (E-003/E-009, binding):
   - explicit ids mandatory (ids in payload, no positional references);
   - times as {num, den} pairs; floats FORBIDDEN (rejected at load, not repaired);
   - `owner` distinguishes human/agent/script (owner-agnostic replay — E-009);
   - batch = `{batch: [sub-commands], atomic: true}` with composite inverse;
   - undo markers are logged entries (redo = un-undo), not out-of-band state.
3. Validation: on load, every line is schema-validated; first invalid line → typed error,
   load aborts (no silent truncation). The log is the project; corruption is a hard stop
   with a repair path (truncate-to-last-good + quarantine tail file), never auto-edit.

## 4. Snapshot protocol (compaction + equivalence)

1. Snapshot at seq N = full state fold at N + `meta.json`; log keeps suffix > N.
2. Equivalence invariant (E-003 P3): state(snapshot) + replay(log suffix) ==
   replay(log from empty) — hash-checked on every compaction and in CI property tests.
3. Snapshot triggers: configurable (every K commands / MB log / on demand / on close).
4. Crash during compaction: write-temp + atomic rename; manifest's `snapshot_seq` only
   advances after rename — a torn compaction is discarded (old snapshot + full suffix
   still replay — the safety argument of event sourcing).

## 5. Asset pipeline

1. Import = copy into `assets/<sha256>/` (dedupe by hash) + run probe → probe.json.
2. Probe record includes: container, streams {codec, profile, rate rationals, duration,
   color tags}, keyframe index (pts list), VFR flag — everything DECODER_SPEC §2 needs.
3. Missing asset on open: project loads with typed `AssetMissing` markers (state intact,
   render/export refuse gracefully) — projects are text; media is replaceable by hash.

## 6. Versioning & migration

1. `schema_version` gates the reader: engine N reads ≤ N; writer writes N.
2. Migration = explicit per-version serializer functions (Olive pattern, doc 19) with
   tests: migrate(v1 fixture) == expected v2 state (hash) — fixtures committed.
3. Replay across engine versions is a REOPEN condition for ADR-008 (whitepaper §6):
   old-log replay on new engine must be hash-equal or the migration layer must convert
   commands explicitly — never "best effort".

## 7. Acceptance suite (the directive's test, made concrete)

| ID | Test |
|---|---|
| P-1 | save → reopen → state_hash equal |
| P-2 | execute N commands → kill -9 (random point) → reopen → replay → hash == live hash |
| P-3 | snapshot compaction: pre/post compaction reopen hash equal (P equivalence) |
| P-4 | log corruption drill: truncated tail → quarantine + last-good load; explicit error |
| P-5 | float payload rejection; (num,den) round-trip exactness |
| P-6 | asset hash relink: move project folder → reopen works (hash addressing); corrupt asset → AssetMissing + render refusal |
| P-7 | undo markers replay: fresh engine replays log incl. undo history correctly (E-009 pattern) |
| P-8 | disposable dirs: rm -rf cache renders → hash unchanged |
