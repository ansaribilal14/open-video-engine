//! Commands — the ONE mutation vocabulary (ADR-010: human and agent share it).
//!
//! Verb set = ENGINE_BUILD_PLAN Wave 1: {add, remove, move, resize, split, retime}
//! + atomic batches.
//!
//! Every command's exact inverse is computed by the engine at apply time (ADR-009)
//! and returned in the receipt; clients never construct inverses. Ids inside
//! commands are always explicit; engine-ASSIGNED ids happen at apply time and are
//! returned in the receipt (E-003 pattern).

use crate::clip::{AssetId, ClipId, TrackId};
use ove_time::Rational;

#[derive(Clone, PartialEq, Debug)]
pub enum Command {
    /// Place `src_in..src_in+dur*speed` of `asset` at `start..start+dur` on `track`.
    /// `id`: None = engine-assigned (deterministic fold, receipt returns it);
    /// Some(id) = explicit id (used by engine-computed inverses and by clients
    /// that need stable ids; must be free). Overlap on `track` is rejected.
    AddClip {
        id: Option<ClipId>,
        track: TrackId,
        asset: AssetId,
        start: Rational,
        dur: Rational,
        src_in: Rational,
        speed: Rational,
        label: String,
    },
    RemoveClip {
        id: ClipId,
    },
    /// Move (and optionally re-track) a clip to a new start. Overlap on the target
    /// track is rejected.
    MoveClip {
        id: ClipId,
        track: TrackId,
        start: Rational,
    },
    /// Out-trim: change the timeline duration (source range shrinks/grows by speed).
    ResizeClip {
        id: ClipId,
        dur: Rational,
    },
    /// Split at a timeline offset within the clip: left keeps `id`, right gets
    /// `new_id` (None = engine-assigned at apply; Some = resolved/replayed form).
    SplitClip {
        id: ClipId,
        offset: Rational,
        new_id: Option<ClipId>,
    },
    /// Retime: change `speed`, preserve the SOURCE range; the timeline duration is
    /// derived exactly (`dur' = dur * speed / speed'`).
    RetimeClip {
        id: ClipId,
        speed: Rational,
    },
    /// Atomic batch: all-or-nothing (ADR-009 composite inverse). Applied
    /// sequentially; any failure rolls back the applied prefix via exact inverses.
    Batch {
        cmds: Vec<Command>,
        label: String,
    },
}

impl Command {
    /// Short verb name (for logs, receipts, UX grouping per doc 34).
    pub fn verb(&self) -> &'static str {
        match self {
            Command::AddClip { .. } => "add",
            Command::RemoveClip { .. } => "remove",
            Command::MoveClip { .. } => "move",
            Command::ResizeClip { .. } => "resize",
            Command::SplitClip { .. } => "split",
            Command::RetimeClip { .. } => "retime",
            Command::Batch { .. } => "batch",
        }
    }

    /// Batch label if this is a batch (doc 34 transaction labels).
    pub fn label(&self) -> Option<&str> {
        match self {
            Command::Batch { label, .. } => Some(label),
            _ => None,
        }
    }
}
