# 19 — PROJECT FORMATS: SURVEY AND SERIALIZATION STRATEGIES

> Status: PARTIAL (v0.1).
> Owner: agent 3-b. Track: P (project format).
> This is a survey and candidate analysis, NOT a final format decision. The
> decision belongs to an ADR gated on experiment E-003 (per the claim ledger
> rule: no decision before confidence >= MEDIUM + implementation reference or
> an experiment).
> Sources cited as (S-2bx); see research/sources/SOURCE_LEDGER.md.

## 1. Survey of real project formats

### 1.1 OpenTimelineIO (.otio) — JSON, schema-per-type versioning

Fetched from the official file format specification (S-2b0):

- Plain JSON; numbers are int64 or IEEE double.
- Each object carries `"OTIO_SCHEMA": "Timeline.1"` / `"Clip.5"` etc. —
  **version is per data type, not per file**: "there is no top level file
  format version in OTIO. Each data type has a version instead to allow for
  more granular versioning."
- No instancing: "there cannot be references to the same object multiple times
  in the tree structure. If the same clip or media appears multiple times in a
  timeline, it will appear as identical copies."
- Top level may be any OTIO type (usually Timeline); readers must "guard
  against unexpected top level types".
- Files are written indented and with deterministic key order "to help with
  change tracking, comparisons, etc."; gzip preferred over minification.

Lessons: per-type schema versioning is a strong migration primitive; no
instancing is acceptable for interchange but wasteful for a live project file
(a 1000-clip project re-referencing one asset 1000 times duplicates the ref);
deterministic ordering is chosen explicitly for diffability.

### 1.2 FCPXML — rational seconds, asset references (vendor spec)

- Apple defines time as "a rational number of seconds with a 64-bit numerator
  and a 32-bit denominator"; NTSC media uses frame durations like
  1001/30000s or 1001/60000s (Apple doc text; via fcp.cafe FCPXML reference
  and macscripter quote of the official documentation, S-2b8).
- Media is referenced, not embedded (asset-clip ref= pattern; the Apple
  discussion example shows offsets like 52505124717/1000000000s — rationals
  with arbitrary denominators appear in real exports, S-2b8).
- Lesson: a *seconds-based* rational (num/den of seconds) can carry frame
  accuracy without a global frame rate — denominators absorb rate mixing.

### 1.3 AAF — the complexity cautionary tale

- AAF (Advanced Authoring Format) is the broadcast interchange format from
  the Avid era; comprehensive but notoriously complex to implement. Evidence
  of scale: OTIO ships a dedicated, long-evolving AAF adapter and its
  documentation explicitly treats AAF import as lossy/partial (UNVERIFIED
  details this pass; S-2b0 adapters docs list AAF among adapters). Lesson:
   an interchange format that tries to carry everything (full effect graphs,
   plugin binary references) becomes a tax on every implementer. TODO v0.2:
   fetch AAF spec overview + OTIO AAF adapter README for precise claims.

### 1.4 EDL (CMX 3600) — minimalism that still runs the world

- A CMX3600 EDL is a plain-text event list (event number, reel, track codes,
  transition type, source in/out, record in/out) with comments. It has
  survived ~5 decades because it is human-readable and trivially parseable.
  (Standard knowledge; OTIO's CMX 3600 adapter is listed among built-in
  adapters, S-2b0.) Lessons: a minimal *text* format enables diff/merge and
  generative tooling; but it cannot carry effects, color, or modern media
  references — the reason OTIO itself exists.

### 1.5 Kdenlive (.kdenlive) — MLT XML as the document

From dev-docs/fileformat.md (S-2b4): "Kdenlive's project files use an XML
format, based on MLT's format ... MLT is able to directly render Kdenlive
project files. MLT simply ignores all the additional Kdenlive-specific project
data."

- Generations: gen-1 (duplication of project data between inner Kdenlive data
  and outer MLT XML, "out of sync" corruption), gen-2 (single source of truth
  in MLT XML), gen-3 (timeline2 engine), gen-4 (comma/point decimal separator
  refactor, "fixing a long standing issue ... causing many crashes",
  not-backwards-compatible), gen-5 (23.04+, multiple sequences, main_bin).
- Autosave: Kdenlive writes autosave copies of the project (standard behavior;
  exact file naming UNVERIFIED this pass).
- Lessons: (1) the document format doubles as a *render input* — a huge win
  for headless rendering and debugging (melt can play a .kdenlive file);
  (2) format generations are unavoidable; design for them from day one;
  (3) locale-sensitive number serialization (comma decimal separator) is a
  crash-grade bug class.

### 1.6 Shotcut (.mlt) — the project file IS MLT XML

Shotcut has no format of its own; it annotates MLT XML with `shotcut:`
properties (shotcut.org "MLT XML Annotations", S-2b9). Lessons: zero
duplication; any MLT tool can read Shotcut projects; but Shotcut cannot
represent application state MLT lacks (e.g., its undo history) — acceptable
for Shotcut, a constraint for us if we want AI-agent transactionality.

### 1.7 Olive (.ove) — versioned XML with per-version serializers

From app/node/project/serializer (S-2b6): the project file is XML with a
version attribute; the serializer module contains one class per released
format (serializer190219, 210528, 210907, 211228, 220403, 230220) and loads
old projects by dispatching on the sniffed version ("Allows easy integer math
for checking project versions"). Lessons: the simplest possible migration
mechanism is "keep the old reader around"; costs are code growth and load-time
conversion chains.

### 1.8 OpenShot (.osp) — JSON

OpenShot project files are JSON ("libopenshot-AV/OpenShot .osp"; libopenshot
Timeline "Loads a JSON structure from a file path" — openshot.org API docs and
justsolve.archiveteam.org catalog entry, S-2b9). Lesson: single-file JSON is
the fastest format to ship; OpenShot's issue tracker shows corruption cases
("invalid load key, '{'") when files are truncated — a hint of the
crash-recovery weakness of whole-file rewrite (community evidence, quality 9).

### 1.9 OpenCut — no portable format (counter-example)

OpenCut (browser editor) stores "all project data ... locally in your browser
using IndexedDB" (opencut.app privacy policy) and a March 2026 feature issue
requests import/export: "OpenCut has no portable project file format. Projects
are locked in IndexedDB/OPFS — users can't save, share, or move projects
between devices" (github.com issue, S-2b9). Lesson: storage-layer
convenience (IndexedDB) is not a project format; portability must be a
designed-in serialization, or the ecosystem around your editor is poisoned.

### 1.10 Databases in NLEs — Resolve (SQLite/PostgreSQL), Lightworks (UNVERIFIED)

- DaVinci Resolve stores projects in "either SQLite databases or PostgreSQL
  databases" ("Disk" libraries vs Project Server) — Blackmagic forum
  statement + ecosystem tooling (Docker-Davinci-Resolve-Project-Server runs
  the PostgreSQL variant), S-2b9. Community-level evidence (quality 9);
  consistent and widely corroborated, but no official doc fetched.
- Lightworks: project storage is a proprietary media/database bundle;
  claims that it uses SQLite specifically are UNVERIFIED — do not rely on it.
- Blender: .blend is a custom binary format (standard knowledge, UNVERIFIED
  in this pass) — an example of binary-tree serialization, not a database.

## 2. Serialization strategies compared

| Strategy | Autosave cost | Crash recovery | Migrations | Undo integration | Diff/merge | Large-project perf | Verified examples |
|---|---|---|---|---|---|---|---|
| Single JSON/XML document, whole-file rewrite | O(file) every save; big files stall | truncation risk (OpenShot .osp cases) | one version field (Olive) or per-type schemas (OTIO) | none built in; save = snapshot only | good (text) | degrades with file size | OpenShot, OTIO files, Shotcut, Kdenlive, Olive |
| Database (SQLite) rows | O(row) incremental | transactional, WAL crash-safe | schema migrations mature | none built in; undo is app-level | poor (opaque rows) | good at scale; random access | Resolve disk DB (community evidence) |
| Event-sourced command log + periodic snapshots | O(1) append; snapshot debounced | replay log after snapshot; partial loss bounded | snapshot + log schema version | *native* — undo log is the project log | excellent (log is the diff) | snapshot size constant; replay cost bounded | no NLE ships this as its file format (Kdenlive/Olive undo logs are session-local); document-tracking systems (e.g. CRDT editors) UNVERIFIED as NLE precedent |
| Hybrid package (zip: snapshot + command log + media refs + thumbs) | append to log; periodic snapshot swap | replay or fall back to snapshot | per-section schema versions (OTIO-style) | native for in-session; log persisted for cross-session undo | log is text-diffable | bounded | none found (candidate) |

Notes:

- The command-log strategy and the undo systems we verified (Kdenlive lambda
  pairs, Shotcut QUndoCommand verbs, Olive node undo, S-2b4/S-2b5/S-2b6) are
  the same data structure with different lifetimes. NLEs today throw the log
  away at exit; the log IS a valid document if commands are (a) serializable,
  (b) replayable against a snapshot deterministically, (c) schema-versioned.
- Every strategy still needs media: project files reference media by URI +
  probe metadata + hashes; nothing embeds large media (verified across OTIO
  README "not a container for media", kdenlive fileformat.md "stores only a
  reference", FCPXML asset refs).

## 3. Criteria ranking for the Open Video Engine

Driving requirements from the mission: crash recovery, autosave, AI-agent
transactions (multi-command atomic batches), branching/experimentation, and
human+AI shared command API (claims C-003/C-007).

| Criterion | Winner |
|---|---|
| AI transactionality (all-or-nothing multi-step edit) | command log (transactions are log sections) / DB transactions |
| Crash recovery granularity | DB ≈ command log > single file |
| Undo across sessions (re-open and undo last session's edit) | command log |
| Branching (AI explores 3 alternate cuts) | command log (fork the log) ; DB possible via copy; single file painful |
| Human diff/audit of an AI session | command log (readable verbs) > JSON > DB |
| Migrations | OTIO-style per-type versions on snapshots + versioned command verbs |
| Implementation cost | single JSON cheapest; command log medium (needs deterministic replay discipline) |
| Risk | command log: replay determinism across engine versions is the hard invariant; bugs poison old files |

## 4. Recommendation candidates (NOT a decision)

- **Candidate A (leading, matches C-007):** document = snapshot + serializable
  command log in a container (zip or directory): `project.json` snapshot with
  OTIO-style per-type schema versions + `commands.jsonl` log + `media/` refs.
  Autosave = log append; snapshot every N ops or T seconds; crash recovery =
  load snapshot + replay tail with per-command schema version; undo = log
  position pointer; AI branch = log fork with copy-on-write snapshot.
- **Candidate B:** SQLite database (Resolve-style) — strongest for very large
  projects and multi-user, weakest for diff/audit and cross-platform embedding
  (browser cannot embed SQLite natively without WASM sql.js; relevant to C-002).
- **Candidate C:** single OTIO-superset JSON (max interchange alignment,
  simplest tooling; weakest crash story; acceptable fallback export format
  regardless of the primary choice).
- Regardless of the primary choice: **OTIO import/export adapters are
  mandatory** (interchange with the pro ecosystem), and the internal model
  must remain OTIO-mappable (03_NLE_ARCHITECTURE.md section 3).

Gates before decision: E-003 benchmark (replay cost vs snapshot frequency at
10^4-10^5 clips; log size growth), plus a spike implementing deterministic
replay across a version bump.

## 5. Open questions (v0.2)

1. Command schema versioning discipline (can commands reference objects that
   later schema versions rename? snapshot+log interaction).
2. Where thumbnails/peaks/waveform caches live (sidecar? content-addressed
   cache dir? OTIO has markers but no cache concept).
3. AAF and Resolve .drp (project archive) as additional import targets.
4. Verify Lightworks storage and Blender .blend specifics; fetch AAF spec
   overview; fetch OTIO AAF adapter README.
