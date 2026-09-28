//! ove-project — persistence for the open-video-engine (WAVE 4).
//!
//! Design authority: ADR-008 (project format, single-writer), ADR-009/010
//! (undo/command discipline), PROJECT_FORMAT_SPEC v1 (docs/specs), ADR-016
//! (v1 format decisions recorded this wave).
//!
//! Authority order (ADR-016 — crash-safety argument of event sourcing):
//!   1. commands.jsonl     — the RECORD (append-only, write-through)
//!   2. snapshot/meta.json — the snapshot authority (written atomically
//!      AFTER state-<seq>.json is complete; torn compactions discarded)
//!   3. manifest.json      — a HINT (identity, assets, last-known state);
//!      stale manifests are reconciled on open, never trusted blindly
//!
//! State hash: BLAKE3-256 over the canonical document mirror (PROJECT_
//! FORMAT_SPEC §2 hash policy). The undo stack is session state (ADR-008)
//! rebuilt by replay — not hashed, not persisted.
//!
//! CORE CRATE: no libav linkage, ever (confinement gate). Asset import
//! accepts an already-computed probe (produced by an adapter layer, e.g.
//! ove-decode's FfmpegProbe at the CLI boundary); probe-less imports are
//! valid and simply unmarked.
//!
//! Crash model (P-2 acceptance): log entries are flushed to the OS before
//! `execute` returns, so the on-disk state after each call IS the kill-9
//! state. The acceptance test models this with a real subprocess abort plus
//! drop-without-close equivalence.

pub mod log;
pub mod manifest;
pub mod state;

use std::path::{Path, PathBuf};

use ove_media::ContentHash;
use ove_timeline::{Command, Timeline, TrackId, TrackKind};
use serde::{Deserialize, Serialize};

use log::{LogEntry, LogPayload};
use manifest::{Manifest, SnapshotMeta};
use state::NumPair;

// ---------------------------------------------------------------------------
// Errors (typed; the log is the project — corruption is a hard stop)
// ---------------------------------------------------------------------------

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ProjectError {
    Io(String),
    /// Log line invalid (schema/seq): load aborts, explicit repair required.
    LogCorruption {
        line_no: usize,
        reason: String,
    },
    /// Snapshot/state file inconsistent with its meta.
    SnapshotInvalid(String),
    ManifestInvalid(String),
    /// Engine N reads ≤ N (PROJECT_FORMAT_SPEC §6.1).
    SchemaVersion {
        found: u32,
        supported: u32,
    },
    Timeline(ove_timeline::TimelineError),
    /// Repair path precondition; context is mandatory (review reject if empty).
    Internal(String),
}

impl std::fmt::Display for ProjectError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ProjectError::Io(d) => write!(f, "io: {d}"),
            ProjectError::LogCorruption { line_no, reason } => {
                write!(f, "log corruption at line {line_no}: {reason}")
            }
            ProjectError::SnapshotInvalid(d) => write!(f, "snapshot invalid: {d}"),
            ProjectError::ManifestInvalid(d) => write!(f, "manifest invalid: {d}"),
            ProjectError::SchemaVersion { found, supported } => {
                write!(f, "schema version {found} > supported {supported}")
            }
            ProjectError::Timeline(e) => write!(f, "timeline: {e:?}"),
            ProjectError::Internal(d) => write!(f, "internal: {d}"),
        }
    }
}

impl std::error::Error for ProjectError {}

/// Who issued a command (E-009 owner-agnostic replay).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Owner {
    Human,
    Agent,
    Script,
}

/// Asset availability (PROJECT_FORMAT_SPEC §5.3: projects are text; media
/// is replaceable by hash — state stays intact, render/export refuse).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AssetStatus {
    /// Bytes on disk hash-match the registry identity.
    Present,
    /// File absent.
    Missing,
    /// File present but bytes hash differently — relink-by-path forbidden
    /// (R-13); treated as missing by render/export.
    Corrupt,
}

/// One session undo step: (inverse command, original command, exec seq).
/// Both commands are kept because the undo marker embeds the inverse and
/// the redo marker embeds the original (self-contained suffixes).
struct UndoStep {
    inverse: Command,
    original: Command,
    exec_seq: u64,
}

// ---------------------------------------------------------------------------
// Project
// ---------------------------------------------------------------------------

pub struct Project {
    dir: PathBuf,
    manifest: Manifest,
    timeline: Timeline,
    writer: log::LogWriter,
    /// Session undo stack — rebuilt by replay, never persisted (ADR-008).
    undo_stack: Vec<UndoStep>,
    /// Session redo branch (cleared by every new execute).
    redo_stack: Vec<Command>,
    /// Entries loaded from disk at open (reconciliation bookkeeping).
    loaded_entries: u64,
}

impl Project {
    // -- lifecycle ----------------------------------------------------------

    /// Create a fresh project folder (errors if it already exists).
    pub fn create(dir: &Path, tick_axis: (i64, i64)) -> Result<Self, ProjectError> {
        if dir.exists() {
            return Err(ProjectError::Internal(format!(
                "project folder already exists: {}",
                dir.display()
            )));
        }
        for sub in ["snapshot", "assets", "renders", "cache"] {
            std::fs::create_dir_all(dir.join(sub))
                .map_err(|e| ProjectError::Io(format!("mkdir {sub}: {e}")))?;
        }
        let manifest = Manifest::initial(NumPair {
            num: tick_axis.0,
            den: tick_axis.1,
        });
        manifest.store(&dir.join("manifest.json"))?;
        let writer = log::LogWriter::open(&dir.join("commands.jsonl"), 1)?;
        Ok(Project {
            dir: dir.to_path_buf(),
            manifest,
            timeline: Timeline::new(),
            writer,
            undo_stack: Vec::new(),
            redo_stack: Vec::new(),
            loaded_entries: 0,
        })
    }

    /// Full open: manifest + snapshot (if any) + log replay + reconciliation.
    pub fn open(dir: &Path) -> Result<Self, ProjectError> {
        let mut manifest = Manifest::load(&dir.join("manifest.json"))?;
        // snapshot authority (ADR-016): prefer meta.json over the manifest hint
        let meta = SnapshotMeta::load(&dir.join("snapshot"))?;
        let snapshot_seq = meta.as_ref().map(|m| m.snapshot_seq).unwrap_or(0);

        let mut timeline = match (&meta, snapshot_seq) {
            (Some(m), seq) if seq > 0 => {
                let state_path = dir.join("snapshot").join(format!("state-{seq}.json"));
                let raw = std::fs::read_to_string(&state_path).map_err(|e| {
                    ProjectError::SnapshotInvalid(format!("read state-{seq}.json: {e}"))
                })?;
                let mirror: state::StateMirror = serde_json::from_str(&raw)
                    .map_err(|e| ProjectError::SnapshotInvalid(format!("schema: {e}")))?;
                let hash = mirror.state_hash();
                if hash != m.state_hash {
                    return Err(ProjectError::SnapshotInvalid(format!(
                        "state hash {hash} != meta {}",
                        m.state_hash
                    )));
                }
                mirror
                    .to_timeline()
                    .map_err(ProjectError::SnapshotInvalid)?
            }
            _ => {
                // from-empty: rebuild the track registry (project setup)
                // before replaying the log onto it
                let mut tl = Timeline::new();
                for t in &manifest.tracks {
                    let kind = match t.kind.as_str() {
                        "avl" => TrackKind::Avl(ove_timeline::AvlTrack::new()),
                        "oracle" => TrackKind::Oracle(ove_timeline::OracleTrack::new()),
                        _ => TrackKind::Gap(ove_timeline::GapTrack::new()),
                    };
                    tl.add_track(t.id, kind).map_err(|e| {
                        ProjectError::ManifestInvalid(format!("track registry: {e:?}"))
                    })?;
                }
                tl
            }
        };

        // refresh the manifest registry from the snapshot mirror when a
        // snapshot was loaded (mirror is the state authority)
        if meta.is_some() {
            let mut tracks = Vec::new();
            for tid in timeline.track_ids() {
                let kind_tag = match timeline.track(tid) {
                    Some(TrackKind::Gap(_)) => "gap",
                    Some(TrackKind::Avl(_)) => "avl",
                    Some(TrackKind::Oracle(_)) => "oracle",
                    None => "gap",
                };
                tracks.push(manifest::TrackEntry {
                    id: tid,
                    kind: kind_tag.to_string(),
                });
            }
            manifest.tracks = tracks;
        }

        // replay the suffix — the RECORD is truth; a stale manifest (crash
        // between append and manifest rewrite) reconciles here.
        let entries = log::load(&dir.join("commands.jsonl"), snapshot_seq + 1)?;
        let mut undo_stack: Vec<UndoStep> = Vec::new();
        let mut redo_stack: Vec<Command> = Vec::new();
        for entry in &entries {
            if entry.seq <= snapshot_seq {
                return Err(ProjectError::LogCorruption {
                    line_no: entry.seq as usize,
                    reason: format!(
                        "entry seq {} predates snapshot_seq {snapshot_seq} (compaction leaked)",
                        entry.seq
                    ),
                });
            }
            replay_entry(&mut timeline, &mut undo_stack, &mut redo_stack, entry)?;
        }
        let loaded_entries = entries.len() as u64;

        // reconcile the hint: if the log outran the manifest (crash between
        // append and manifest rewrite), the TRUE state is the replayed one.
        let state_hash = state::StateMirror::from_timeline(&timeline).state_hash();
        let log_len = snapshot_seq + loaded_entries;
        if manifest.state.log_len != log_len || manifest.state.state_hash != state_hash {
            manifest.state.snapshot_seq = snapshot_seq;
            manifest.state.log_len = log_len;
            manifest.state.state_hash = state_hash;
            manifest.store(&dir.join("manifest.json"))?;
        }

        let writer = log::LogWriter::open(&dir.join("commands.jsonl"), log_len + 1)?;

        Ok(Project {
            dir: dir.to_path_buf(),
            manifest,
            timeline,
            writer,
            undo_stack,
            redo_stack,
            loaded_entries,
        })
    }

    pub fn dir(&self) -> &Path {
        &self.dir
    }

    pub fn tick_axis(&self) -> (i64, i64) {
        (self.manifest.tick_axis.num, self.manifest.tick_axis.den)
    }

    /// Add a track (project SETUP — not a command; see ADR-016). Persisted
    /// in the manifest registry so a from-empty replay rebuilds the same
    /// scaffolding before applying the log.
    pub fn add_track(&mut self, id: TrackId, kind: TrackKind) -> Result<(), ProjectError> {
        self.timeline
            .add_track(id, kind)
            .map_err(ProjectError::Timeline)?;
        let kind_tag = match self.timeline.track(id) {
            Some(TrackKind::Gap(_)) => "gap",
            Some(TrackKind::Avl(_)) => "avl",
            Some(TrackKind::Oracle(_)) => "oracle",
            _ => "gap",
        };
        self.manifest.tracks.push(manifest::TrackEntry {
            id,
            kind: kind_tag.to_string(),
        });
        self.manifest
            .store(&self.dir.join("manifest.json"))
            .map_err(|e| ProjectError::Internal(format!("manifest after add_track: {e}")))?;
        Ok(())
    }

    pub fn uuid(&self) -> &str {
        &self.manifest.uuid
    }

    /// BLAKE3-256 hex of the document state (canonical mirror) — what the
    /// P-1/P-2/P-3 acceptance tests compare.
    pub fn state_hash(&self) -> String {
        state::StateMirror::from_timeline(&self.timeline).state_hash()
    }

    pub fn timeline(&self) -> &Timeline {
        &self.timeline
    }

    /// Mutable timeline access for the session layer (single writer): id
    /// allocation before constructing Insert commands (E-012 discipline).
    pub fn timeline_mut(&mut self) -> &mut Timeline {
        &mut self.timeline
    }

    pub fn undo_depth(&self) -> usize {
        self.undo_stack.len()
    }

    pub fn loaded_entries(&self) -> u64 {
        self.loaded_entries
    }

    // -- command execution (write-through) ----------------------------------

    /// Apply + log. The entry is flushed to the OS before execute returns:
    /// the on-disk state after each call IS the kill-9 crash state.
    pub fn execute(&mut self, cmd: Command, owner: Owner) -> Result<(), ProjectError> {
        let inverse = self.timeline.apply(&cmd).map_err(ProjectError::Timeline)?;
        let entry = LogEntry {
            seq: 0, // assigned by the writer
            owner,
            payload: LogPayload::of_command(&cmd),
            undo: Some(LogPayload::of_command(&inverse)),
            nid: self.timeline.next_id_value(),
        };
        let seq = self.writer.append(entry)?;
        self.undo_stack.push(UndoStep {
            inverse,
            original: cmd,
            exec_seq: seq,
        });
        self.redo_stack.clear();
        self.touch_manifest()?;
        Ok(())
    }

    /// Undo the last step; logged as an undo marker with the inverse
    /// embedded (self-contained suffix). Ok(false) when nothing to undo.
    pub fn undo(&mut self, owner: Owner) -> Result<bool, ProjectError> {
        let Some(step) = self.undo_stack.pop() else {
            return Ok(false);
        };
        // apply the inverse; the forward command it yields back is the
        // deterministic original (exact inverses, ADR-009)
        let _fwd = self
            .timeline
            .apply(&step.inverse)
            .map_err(ProjectError::Timeline)?;
        self.redo_stack.push(step.original.clone());
        let entry = LogEntry {
            seq: 0,
            owner,
            payload: LogPayload::Undo {
                target: step.exec_seq,
                inverse: Box::new(LogPayload::of_command(&step.inverse)),
            },
            undo: None,
            nid: self.timeline.next_id_value(),
        };
        self.writer.append(entry)?;
        self.touch_manifest()?;
        Ok(true)
    }

    /// Redo the last undone step; logged as a redo marker with the forward
    /// command embedded. Ok(false) when nothing to redo.
    pub fn redo(&mut self, owner: Owner) -> Result<bool, ProjectError> {
        let Some(original) = self.redo_stack.pop() else {
            return Ok(false);
        };
        let inverse = self
            .timeline
            .apply(&original)
            .map_err(ProjectError::Timeline)?;
        let exec_seq = self.writer.next_seq(); // the redo marker's own seq
        self.undo_stack.push(UndoStep {
            inverse,
            original: original.clone(),
            exec_seq,
        });
        let entry = LogEntry {
            seq: 0,
            owner,
            payload: LogPayload::Redo {
                target: exec_seq,
                cmd: Box::new(LogPayload::of_command(&original)),
            },
            undo: None,
            nid: self.timeline.next_id_value(),
        };
        self.writer.append(entry)?;
        self.touch_manifest()?;
        Ok(true)
    }

    // -- snapshot compaction (PROJECT_FORMAT_SPEC §4) ------------------------

    /// Compact: write state-<N>.json, meta.json, rewrite the log to the
    /// suffix, update the manifest. Order = the crash-safety argument
    /// (ADR-016): state file → meta (snapshot authority) → log rewrite →
    /// manifest hint. A crash at ANY point leaves a loadable project.
    pub fn snapshot(&mut self) -> Result<u64, ProjectError> {
        let seq = self.writer.next_seq() - 1; // everything ≤ seq is folded
        let mirror = state::StateMirror::from_timeline(&self.timeline);
        let hash = mirror.state_hash();

        let snap_dir = self.dir.join("snapshot");
        std::fs::create_dir_all(&snap_dir)
            .map_err(|e| ProjectError::Io(format!("mkdir snapshot: {e}")))?;
        let state_tmp = snap_dir.join(format!("state-{seq}.json.tmp"));
        let state_fin = snap_dir.join(format!("state-{seq}.json"));
        let raw = serde_json::to_vec_pretty(&mirror)
            .map_err(|e| ProjectError::Internal(format!("state serialization: {e}")))?;
        std::fs::write(&state_tmp, raw)
            .map_err(|e| ProjectError::Io(format!("write state: {e}")))?;
        std::fs::rename(&state_tmp, &state_fin)
            .map_err(|e| ProjectError::Io(format!("state rename: {e}")))?;

        SnapshotMeta {
            snapshot_seq: seq,
            state_hash: hash.clone(),
            engine_version: env!("CARGO_PKG_VERSION").to_string(),
        }
        .store(&snap_dir)?;

        // rewrite the log to the suffix (> seq) — atomic temp+rename
        let entries = log::load(
            &self.dir.join("commands.jsonl"),
            self.manifest.state.snapshot_seq + 1,
        )?;
        let suffix: Vec<&LogEntry> = entries.iter().filter(|e| e.seq > seq).collect();
        let tmp = self.dir.join("commands.jsonl.tmp");
        {
            use std::io::Write;
            let mut f = std::fs::File::create(&tmp)
                .map_err(|e| ProjectError::Io(format!("create log tmp: {e}")))?;
            for e in &suffix {
                let mut line = serde_json::to_string(e)
                    .map_err(|e| ProjectError::Internal(format!("entry serialization: {e}")))?;
                line.push('\n');
                f.write_all(line.as_bytes())
                    .and_then(|_| f.flush())
                    .map_err(|e| ProjectError::Io(format!("write suffix: {e}")))?;
            }
        }
        std::fs::rename(&tmp, self.dir.join("commands.jsonl"))
            .map_err(|e| ProjectError::Io(format!("log rename: {e}")))?;

        self.manifest.state.snapshot_seq = seq;
        self.manifest.state.log_len = suffix.len() as u64;
        self.manifest.state.state_hash = hash;
        self.manifest.store(&self.dir.join("manifest.json"))?;

        // the writer continues after the compacted log
        let next = seq + suffix.len() as u64 + 1;
        self.writer = log::LogWriter::open(&self.dir.join("commands.jsonl"), next)?;
        Ok(seq)
    }

    // -- assets (PROJECT_FORMAT_SPEC §5) -------------------------------------

    /// Import = copy into assets/<content-hash>/ (dedupe by hash) + optional
    /// probe sidecar. NO libav here: probes come from an adapter layer.
    pub fn import_asset(
        &mut self,
        src: &Path,
        probe: Option<&serde_json::Value>,
    ) -> Result<ContentHash, ProjectError> {
        let hash = ContentHash::from_file(src)
            .map_err(|e| ProjectError::Io(format!("hash asset: {e}")))?;
        let hex = hash.hex();
        let hex_for_entry = hex.clone();
        let asset_dir = self.dir.join("assets").join(&hex);
        let ext = src
            .extension()
            .map(|e| e.to_string_lossy().to_string())
            .unwrap_or_else(|| "bin".to_string());
        let dest = asset_dir.join(format!("src.{ext}"));
        if !dest.exists() {
            std::fs::create_dir_all(&asset_dir)
                .map_err(|e| ProjectError::Io(format!("mkdir asset dir: {e}")))?;
            std::fs::copy(src, &dest).map_err(|e| ProjectError::Io(format!("copy asset: {e}")))?;
        }
        if let Some(p) = probe {
            std::fs::write(
                asset_dir.join("probe.json"),
                serde_json::to_vec_pretty(p).unwrap_or_default(),
            )
            .map_err(|e| ProjectError::Io(format!("write probe: {e}")))?;
        }
        // registry entry is idempotent by content hash
        if !self.manifest.assets.iter().any(|a| a.content_hash == hex) {
            let id = format!("a-{}", self.manifest.assets.len() + 1);
            self.manifest.assets.push(manifest::AssetEntry {
                id,
                content_hash: hex_for_entry,
                path: format!("assets/{hex}/src.{ext}"),
                probe: probe.is_some().then(|| format!("assets/{hex}/probe.json")),
            });
            self.touch_manifest()?;
        }
        Ok(hash)
    }

    /// Availability of a registered asset (P-6): Missing or Corrupt leave
    /// state intact; render/export must refuse (W6 wires the refusal).
    pub fn asset_status(&self, content_hash: &str) -> AssetStatus {
        let Some(entry) = self
            .manifest
            .assets
            .iter()
            .find(|a| a.content_hash == content_hash)
        else {
            return AssetStatus::Missing;
        };
        let p = self.dir.join(&entry.path);
        if !p.exists() {
            return AssetStatus::Missing;
        }
        match ContentHash::from_file(&p) {
            Ok(h) if h.hex() == entry.content_hash => AssetStatus::Present,
            _ => AssetStatus::Corrupt,
        }
    }

    pub fn assets(&self) -> &[manifest::AssetEntry] {
        &self.manifest.assets
    }

    // -- repair path (PROJECT_FORMAT_SPEC §3.3 / P-4) -------------------------

    /// Explicit repair: quarantine the damaged log verbatim
    /// (commands.corrupt-<stamp>.jsonl) and rewrite the GOOD prefix as the
    /// new log. Callers then re-open — load aborts are the gate, repair is
    /// never automatic.
    pub fn repair_log(dir: &Path) -> Result<(u64, String), ProjectError> {
        let log_path = dir.join("commands.jsonl");
        let (good, quarantine) = log::quarantine_and_split(&log_path)?;
        let tmp = dir.join("commands.jsonl.tmp");
        {
            use std::io::Write;
            let mut f = std::fs::File::create(&tmp)
                .map_err(|e| ProjectError::Io(format!("create repaired log: {e}")))?;
            for e in &good {
                let mut line = serde_json::to_string(e)
                    .map_err(|e| ProjectError::Internal(format!("entry serialization: {e}")))?;
                line.push('\n');
                f.write_all(line.as_bytes())
                    .and_then(|_| f.flush())
                    .map_err(|e| ProjectError::Io(format!("write repaired log: {e}")))?;
            }
        }
        std::fs::rename(&tmp, &log_path)
            .map_err(|e| ProjectError::Io(format!("log rename: {e}")))?;
        Ok((good.len() as u64, quarantine))
    }

    // -- internals ------------------------------------------------------------

    fn touch_manifest(&mut self) -> Result<(), ProjectError> {
        self.manifest.state.log_len = self.writer.next_seq() - 1;
        self.manifest.state.state_hash =
            state::StateMirror::from_timeline(&self.timeline).state_hash();
        self.manifest.store(&self.dir.join("manifest.json"))
    }
}

// ---------------------------------------------------------------------------
// Replay machinery (the log is executable; undo history rebuilds here)
// ---------------------------------------------------------------------------

type ReplayStacks = (Vec<UndoStep>, Vec<Command>);

fn replay_entry(
    tl: &mut Timeline,
    undo_stack: &mut Vec<UndoStep>,
    redo_stack: &mut Vec<Command>,
    entry: &LogEntry,
) -> Result<(), ProjectError> {
    // cursor restore (E-012): the entry carries the session cursor AFTER
    // execution; nid == 0 marks a legacy entry without cursor state.
    if entry.nid > 0 {
        tl.set_next_id(entry.nid).map_err(ProjectError::Timeline)?;
    }
    match &entry.payload {
        LogPayload::Undo { target, inverse } => {
            let inv = inverse
                .to_command()
                .map_err(ProjectError::SnapshotInvalid)?;
            // apply the embedded inverse; the returned forward command is
            // the deterministic original (exact inverses)
            let _fwd = tl.apply(&inv).map_err(ProjectError::Timeline)?;
            // the popped step should agree with the marker (validate when a
            // step exists; after compaction the stack may have been rebuilt
            // — the embedded inverse is the authority)
            if let Some(step) = undo_stack.pop() {
                let matches = matches_seqs(step.exec_seq, *target);
                if !matches && *target != 0 {
                    return Err(ProjectError::LogCorruption {
                        line_no: entry.seq as usize,
                        reason: format!(
                            "undo marker target {target} != stacked exec seq {}",
                            step.exec_seq
                        ),
                    });
                }
            }
            redo_stack.push(inv);
            Ok(())
        }
        LogPayload::Redo { target, cmd } => {
            let fwd = cmd.to_command().map_err(ProjectError::SnapshotInvalid)?;
            let inverse = tl.apply(&fwd).map_err(ProjectError::Timeline)?;
            redo_stack.pop(); // marker is the authority; keep stacks aligned
            undo_stack.push(UndoStep {
                inverse,
                original: fwd,
                exec_seq: *target,
            });
            Ok(())
        }
        other => {
            let cmd = other.to_command().map_err(ProjectError::SnapshotInvalid)?;
            let inverse = tl.apply(&cmd).map_err(ProjectError::Timeline)?;
            undo_stack.push(UndoStep {
                inverse,
                exec_seq: entry.seq,
                original: cmd,
            });
            redo_stack.clear();
            Ok(())
        }
    }
}

fn matches_seqs(a: u64, b: u64) -> bool {
    a == b
}

// ReplayStacks is used implicitly via the destructured parameters above.
#[allow(dead_code)]
type _ReplayStacksAlias = ReplayStacks;
