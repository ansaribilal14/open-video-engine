//! The append-only command log (PROJECT_FORMAT_SPEC §3): one JSON object per
//! line, schema-validated on load, first invalid line = typed error (hard
//! stop, explicit repair path — never auto-edit).
//!
//! Entry grammar (v1, ADR-016 refinement of the spec sketch):
//!   * exec: {seq, owner, kind: <command kind>, payload: {...}, undo: {...}}
//!     — `undo` is the EXACT inverse as computed at execution time
//!     (apply()'s return value).
//!   * undo: {seq, owner, kind: "undo", payload: {target, inverse}}
//!   * redo: {seq, owner, kind: "redo", payload: {target, cmd}}
//!
//!   The undo/redo entries EMBED the inverse/forward command so a
//!   compaction suffix is self-contained (replay never needs entries at or
//!   below the snapshot seq — the marker may target a compacted-away exec);
//!   `target` is the exec seq, informational.
//!
//! Durability: append + flush-to-OS per entry — kill -9 (process death,
//! page cache intact) cannot lose a flushed line; torn LAST lines are the
//! corruption-drill class (P-4) with an explicit repair path.

use std::io::{BufRead, BufReader, Write};
use std::path::Path;

use ove_timeline::{Command, PropertyName};
use serde::{Deserialize, Serialize};

use crate::state::{MirrorKeyframe, MirrorProperties, NumPair};
use crate::{Owner, ProjectError};

/// The v1 animatable property set on the wire (ADR-019) — the mirror of
/// `ove_timeline::PropertyName` (the timeline crate stays serde-free).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MirrorProperty {
    Opacity,
    X,
    Y,
}

impl MirrorProperty {
    fn from_timeline(p: PropertyName) -> Self {
        match p {
            PropertyName::Opacity => MirrorProperty::Opacity,
            PropertyName::X => MirrorProperty::X,
            PropertyName::Y => MirrorProperty::Y,
        }
    }
    fn to_timeline(self) -> PropertyName {
        match self {
            MirrorProperty::Opacity => PropertyName::Opacity,
            MirrorProperty::X => PropertyName::X,
            MirrorProperty::Y => PropertyName::Y,
        }
    }
}

/// Exact mirror of `ove_timeline::Command` with schema-forced exactness:
/// times are {num, den} pairs — floats are REJECTED at load (serde type
/// error), never repaired (PROJECT_FORMAT_SPEC §3.2).
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum LogPayload {
    Insert {
        track: u64,
        index: usize,
        clip: crate::state::MirrorClip,
        /// Per-clip asset binding (W6 vertical slice): the content hash of
        /// the media this clip consumes. Optional (v0.1 logs and asset-free
        /// clips). Recorded at the project layer — ove-timeline clips stay
        /// time-only (ADR-013 seam: the binding is media, not time).
        #[serde(default, skip_serializing_if = "Option::is_none")]
        asset: Option<String>,
    },
    Remove {
        track: u64,
        id: u64,
    },
    Split {
        track: u64,
        id: u64,
        at: NumPair,
        new_id: u64,
    },
    Resize {
        track: u64,
        id: u64,
        duration: NumPair,
    },
    Move {
        id: u64,
        from_track: u64,
        to_track: u64,
        to_index: usize,
    },
    /// WAVE 8 (ADR-019): replace one property's keyframe animation. Keys
    /// are exact pairs; the embedded inverse (the previous key list) rides
    /// the entry via the standard `undo` field — no extra grammar.
    SetKeyframes {
        track: u64,
        id: u64,
        property: MirrorProperty,
        keys: Vec<MirrorKeyframe>,
    },
    Batch {
        cmds: Vec<LogPayload>,
    },
    Undo {
        target: u64,
        /// The exact inverse of the target exec — embedded (suffix
        /// self-containment).
        inverse: Box<LogPayload>,
    },
    Redo {
        target: u64,
        /// The exact forward command of the target exec — embedded.
        cmd: Box<LogPayload>,
    },
}

impl LogPayload {
    pub fn of_command(cmd: &Command) -> Self {
        match cmd {
            Command::Insert { track, index, clip } => LogPayload::Insert {
                track: *track,
                index: *index,
                clip: crate::state::MirrorClip {
                    id: clip.id,
                    duration: NumPair::of(clip.duration),
                    source_in: NumPair::of(clip.source_in),
                    properties: MirrorProperties::from_timeline(&clip.properties),
                },
                asset: None,
            },
            Command::Remove { track, id } => LogPayload::Remove {
                track: *track,
                id: *id,
            },
            Command::Split {
                track,
                id,
                at,
                new_id,
            } => LogPayload::Split {
                track: *track,
                id: *id,
                at: NumPair::of(*at),
                new_id: *new_id,
            },
            Command::Resize {
                track,
                id,
                duration,
            } => LogPayload::Resize {
                track: *track,
                id: *id,
                duration: NumPair::of(*duration),
            },
            Command::Move {
                id,
                from_track,
                to_track,
                to_index,
            } => LogPayload::Move {
                id: *id,
                from_track: *from_track,
                to_track: *to_track,
                to_index: *to_index,
            },
            Command::SetKeyframes {
                track,
                id,
                property,
                keys,
            } => LogPayload::SetKeyframes {
                track: *track,
                id: *id,
                property: MirrorProperty::from_timeline(*property),
                keys: keys.iter().map(MirrorKeyframe::from_timeline).collect(),
            },
            Command::Batch { cmds } => LogPayload::Batch {
                cmds: cmds.iter().map(LogPayload::of_command).collect(),
            },
        }
    }

    pub fn to_command(&self) -> Result<Command, String> {
        Ok(match self {
            LogPayload::Insert {
                track, index, clip, ..
            } => Command::Insert {
                track: *track,
                index: *index,
                clip: ove_timeline::Clip::new(
                    clip.id,
                    clip.duration.to_rational(),
                    clip.source_in.to_rational(),
                )
                .with_properties(clip.properties.to_timeline()?),
            },
            LogPayload::Remove { track, id } => Command::Remove {
                track: *track,
                id: *id,
            },
            LogPayload::Split {
                track,
                id,
                at,
                new_id,
            } => Command::Split {
                track: *track,
                id: *id,
                at: at.to_rational(),
                new_id: *new_id,
            },
            LogPayload::Resize {
                track,
                id,
                duration,
            } => Command::Resize {
                track: *track,
                id: *id,
                duration: duration.to_rational(),
            },
            LogPayload::Move {
                id,
                from_track,
                to_track,
                to_index,
            } => Command::Move {
                id: *id,
                from_track: *from_track,
                to_track: *to_track,
                to_index: *to_index,
            },
            LogPayload::SetKeyframes {
                track,
                id,
                property,
                keys,
            } => Command::SetKeyframes {
                track: *track,
                id: *id,
                property: property.to_timeline(),
                keys: keys
                    .iter()
                    .map(|k| k.to_timeline())
                    .collect::<Result<Vec<_>, String>>()?,
            },
            LogPayload::Batch { cmds } => Command::Batch {
                cmds: cmds
                    .iter()
                    .map(LogPayload::to_command)
                    .collect::<Result<Vec<_>, _>>()?,
            },
            LogPayload::Undo { .. } | LogPayload::Redo { .. } => {
                return Err("undo/redo are log markers, not timeline commands".into());
            }
        })
    }
}

/// One log line. `seq` is the entry identifier (1-based, gap-free on a
/// healthy log).
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct LogEntry {
    pub seq: u64,
    pub owner: Owner,
    #[serde(flatten)]
    pub payload: LogPayload,
    /// Only on exec entries: the exact inverse command.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub undo: Option<LogPayload>,
    /// The session's id-allocation cursor AFTER this entry executed (the
    /// E-012 state rides the log: replay restores the exact cursor; 0 =
    /// absent in legacy v1 logs, skip the restore).
    #[serde(default)]
    pub nid: u64,
}

/// Append-only handle (single writer). Flushes to the OS after every entry:
/// kill -9 safe (process death preserves flushed page-cache writes);
/// fsync-to-disk happens on snapshot/close (power-loss durability beyond
/// the P-2 acceptance model is not claimed by v1 — documented in ADR-016).
pub struct LogWriter {
    file: std::fs::File,
    next_seq: u64,
}

impl LogWriter {
    /// Open (or create) the log for appending. NEVER truncates: the log is
    /// the record — truncation is a compaction-only operation and goes
    /// through the atomic temp+rename path, never through this handle.
    pub fn open(path: &Path, next_seq: u64) -> Result<Self, ProjectError> {
        let file = std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(path)
            .map_err(|e| ProjectError::Io(format!("open log: {e}")))?;
        Ok(LogWriter { file, next_seq })
    }

    pub fn append(&mut self, mut entry: LogEntry) -> Result<u64, ProjectError> {
        entry.seq = self.next_seq;
        let mut line = serde_json::to_string(&entry)
            .map_err(|e| ProjectError::Internal(format!("entry serialization: {e}")))?;
        line.push('\n');
        self.file
            .write_all(line.as_bytes())
            .and_then(|_| self.file.flush())
            .map_err(|e| ProjectError::Io(format!("log append: {e}")))?;
        self.next_seq += 1;
        Ok(entry.seq)
    }

    pub fn next_seq(&self) -> u64 {
        self.next_seq
    }
}

/// Read every entry; the FIRST invalid/torn line aborts with the typed
/// corruption error (never silent truncation).
///
/// `expected_first`: the seq the log MUST start at — 1 for a fresh log,
/// snapshot_seq+1 for a compacted one (a different value means commands
/// were LOST between snapshot and log: hard error, PROJECT_FORMAT_SPEC
/// §3.3 — gaps are corruption, not truncation).
pub fn load(path: &Path, expected_first: u64) -> Result<Vec<LogEntry>, ProjectError> {
    let f = std::fs::File::open(path).map_err(|e| ProjectError::Io(format!("open log: {e}")))?;
    let reader = BufReader::new(f);
    let mut out: Vec<LogEntry> = Vec::new();
    for (idx, line) in reader.lines().enumerate() {
        let line_no = idx + 1;
        let line = line.map_err(|e| ProjectError::Io(format!("read log line {line_no}: {e}")))?;
        if line.trim().is_empty() {
            continue;
        }
        let entry: LogEntry =
            serde_json::from_str(&line).map_err(|e| ProjectError::LogCorruption {
                line_no,
                reason: format!("schema violation: {e}"),
            })?;
        // CONTIGUITY: from `expected_first`, then each = previous + 1
        let expected = out.last().map(|f| f.seq + 1).unwrap_or(expected_first);
        if entry.seq != expected {
            return Err(ProjectError::LogCorruption {
                line_no,
                reason: format!("seq {} out of order (expected {expected})", entry.seq),
            });
        }
        out.push(entry);
    }
    Ok(out)
}

/// The explicit repair path (PROJECT_FORMAT_SPEC §3.3): quarantine the whole
/// damaged log verbatim to `commands.corrupt-<stamp>.jsonl` and return the
/// GOOD prefix (which the caller rewrites as the repaired log). The original
/// file is never edited in place.
pub fn quarantine_and_split(path: &Path) -> Result<(Vec<LogEntry>, String), ProjectError> {
    let f = std::fs::File::open(path).map_err(|e| ProjectError::Io(format!("open log: {e}")))?;
    let reader = BufReader::new(f);
    let mut good: Vec<LogEntry> = Vec::new();
    let mut torn: Vec<String> = Vec::new();
    for (idx, line) in reader.lines().enumerate() {
        let line_no = idx + 1;
        match line {
            Ok(l) if torn.is_empty() => match serde_json::from_str::<LogEntry>(&l) {
                Ok(e) => good.push(e), // anchor + contiguity checked by load()
                other => {
                    let reason = match other {
                        Err(e) => format!("schema violation: {e}"),
                        Ok(_) => "seq out of order".to_string(),
                    };
                    torn.push(format!("LINE {line_no}: {reason}: {l}"));
                }
            },
            Ok(l) => torn.push(format!("LINE {line_no}: quarantined tail: {l}")),
            Err(e) => torn.push(format!("LINE {line_no}: io: {e}")),
        }
    }
    let stamp = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);
    let quarantine_name = format!("commands.corrupt-{stamp}.jsonl");
    let quarantine_path = path
        .parent()
        .unwrap_or(Path::new("."))
        .join(&quarantine_name);
    std::fs::write(&quarantine_path, torn.join("\n"))
        .map_err(|e| ProjectError::Io(format!("write quarantine: {e}")))?;
    Ok((good, quarantine_name))
}
