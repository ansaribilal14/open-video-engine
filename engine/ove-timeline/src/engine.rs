//! Command application — validation-before-mutation, exact inverses, undo/redo,
//! and journal replay (ADR-009 + E-003/E-009 invariants).

use crate::clip::{ClipEntry, ClipId, TrackId};
use crate::command::Command;
use crate::state::TimelineState;
use ove_time::Rational;

#[derive(Clone, PartialEq, Debug)]
pub enum TimelineError {
    UnknownClip(ClipId),
    UnknownTrack(TrackId),
    /// A clip already occupies the requested range on the track.
    Overlap {
        track: TrackId,
        existing: ClipId,
        at: Rational,
    },
    /// Duration must be > 0.
    InvalidDuration(Rational),
    /// Speed must be > 0.
    InvalidSpeed(Rational),
    /// Explicit id already in use.
    IdInUse(ClipId),
    /// Split offset must satisfy 0 < offset < clip duration.
    InvalidSplit {
        id: ClipId,
        offset: Rational,
    },
    /// A batch failed; the applied prefix was rolled back (all-or-nothing held).
    BatchRolledBack {
        applied: usize,
        label: String,
        error: Box<TimelineError>,
    },
    UndoUnavailable,
    RedoUnavailable,
}

impl std::fmt::Display for TimelineError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            TimelineError::UnknownClip(id) => write!(f, "unknown clip {:?}", id),
            TimelineError::UnknownTrack(t) => write!(f, "unknown track {:?}", t),
            TimelineError::Overlap {
                track,
                existing,
                at,
            } => {
                write!(
                    f,
                    "overlap on track {:?} at {} (clip {:?})",
                    track, at, existing
                )
            }
            TimelineError::InvalidDuration(r) => write!(f, "invalid duration {} (must be > 0)", r),
            TimelineError::InvalidSpeed(r) => write!(f, "invalid speed {} (must be > 0)", r),
            TimelineError::IdInUse(id) => write!(f, "clip id {:?} already in use", id),
            TimelineError::InvalidSplit { id, offset } => {
                write!(f, "split offset {} out of range for clip {:?}", offset, id)
            }
            TimelineError::BatchRolledBack {
                applied,
                label,
                error,
            } => {
                write!(
                    f,
                    "batch {:?} failed after {} applied (rolled back): {}",
                    label, applied, error
                )
            }
            TimelineError::UndoUnavailable => write!(f, "nothing to undo"),
            TimelineError::RedoUnavailable => write!(f, "nothing to redo"),
        }
    }
}

impl std::error::Error for TimelineError {}

/// Outcome of one successful atomic application.
#[derive(Clone, PartialEq, Debug)]
pub struct Applied {
    /// Engine-assigned ids in assignment order (empty if none were assigned).
    pub assigned: Vec<ClipId>,
    /// Exact inverse computed against the pre-state (ADR-009).
    pub inverse: Command,
    /// The RESOLVED command: engine-assigned ids replaced by their concrete ids
    /// (AddClip id: Some(n), SplitClip new_id: Some(n)). The journal records this
    /// form; replay applies it verbatim and assigns nothing (see assign_id docs).
    pub resolved: Command,
}

/// Result reported to the caller (the ONLY feedback channel — MEDIA_ENGINE_SPEC §2).
/// `resolved` is the durable form of what was applied (journal/replay use it).
/// Note: the receipt deliberately does NOT carry a state hash — full-state hashing
/// is O(n) and per-command hashing would make command streams O(n²) (caught by the
/// 20k perf smoke). Hashes are explicit O(n) queries at checkpoints; ove-engine's
/// façade layer adds them where the spec asks for them.
#[derive(Clone, PartialEq, Debug)]
pub struct Receipt {
    pub verb: &'static str,
    pub assigned: Vec<ClipId>,
    pub inverse: Command,
    pub resolved: Command,
}

/// Journal entries — fully explicit so replay is a trivial fold (E-009: undo
/// markers replay hash-exact in a fresh engine; no hidden stack state).
/// Replay VERIFICATION of state hashes is a checkpoint-time concern (tests,
/// project save) — see `state_hash()`.
#[derive(Clone, PartialEq, Debug)]
pub enum JournalEntry {
    Do { cmd: Command, inverse: Command },
    Undo { inverse: Command },
    Redo { cmd: Command },
}

fn positive(r: Rational) -> bool {
    r.num() > 0 // den > 0 by construction; num > 0 <=> value > 0
}

/// Successor (id, start) at buffer position pos+1 on a track, if any. O(1) —
/// direct buffer access, NOT an iterator walk (which would make random-access
/// validation O(n) and blow the 20k-clip edit budgets).
fn successor_of(state: &TimelineState, t: TrackId, pos: usize) -> Option<(ClipId, Rational)> {
    state
        .clip_at_pos(t, pos + 1)
        .and_then(|cid| state.clip(cid).map(|e| (cid, e.start)))
}

/// Apply one command atomically: validate fully, mutate, compute the exact
/// inverse. On failure the state is UNCHANGED (including inside batches — the
/// prefix is rolled back via the sub-inverses before returning the error).
pub fn apply_command(state: &mut TimelineState, cmd: &Command) -> Result<Applied, TimelineError> {
    match cmd {
        Command::AddClip {
            id,
            track,
            asset,
            start,
            dur,
            src_in,
            speed,
            label,
        } => {
            if track.0 as usize >= state.track_count() {
                return Err(TimelineError::UnknownTrack(*track));
            }
            if !positive(*dur) {
                return Err(TimelineError::InvalidDuration(*dur));
            }
            if !positive(*speed) {
                return Err(TimelineError::InvalidSpeed(*speed));
            }
            if let Some(id) = id {
                if !state.id_free(*id) {
                    return Err(TimelineError::IdInUse(*id));
                }
            }
            let pos = state.insertion_pos(*track, *start);
            let (pred, succ) = state.neighbors_at(*track, pos);
            if let Some(p) = pred {
                if p.end() > *start {
                    return Err(TimelineError::Overlap {
                        track: *track,
                        existing: p.id,
                        at: *start,
                    });
                }
            }
            if let Some(s) = succ {
                if start.add(*dur) > s.start {
                    return Err(TimelineError::Overlap {
                        track: *track,
                        existing: s.id,
                        at: *start,
                    });
                }
            }
            let cid = match id {
                Some(id) => *id,
                None => state.assign_id(),
            };
            state.insert_clip(*track, cid, pos);
            state.put_clip(ClipEntry {
                id: cid,
                track: *track,
                asset: *asset,
                start: *start,
                dur: *dur,
                src_in: *src_in,
                speed: *speed,
                label: label.clone(),
            });
            let mut resolved = cmd.clone();
            if let Command::AddClip { id: rid, .. } = &mut resolved {
                *rid = Some(cid);
            }
            Ok(Applied {
                assigned: vec![cid],
                inverse: Command::RemoveClip { id: cid },
                resolved,
            })
        }

        Command::RemoveClip { id } => {
            let entry = match state.clip(*id) {
                Some(e) => e.clone(),
                None => return Err(TimelineError::UnknownClip(*id)),
            };
            let pos = state.position_of(entry.track, *id);
            state.remove_clip_from_track(entry.track, pos);
            state.take_clip(*id);
            Ok(Applied {
                assigned: vec![],
                inverse: Command::AddClip {
                    id: Some(*id),
                    track: entry.track,
                    asset: entry.asset,
                    start: entry.start,
                    dur: entry.dur,
                    src_in: entry.src_in,
                    speed: entry.speed,
                    label: entry.label,
                },
                resolved: cmd.clone(),
            })
        }

        Command::MoveClip {
            id,
            track: new_track,
            start: new_start,
        } => {
            let entry = match state.clip(*id) {
                Some(e) => e.clone(),
                None => return Err(TimelineError::UnknownClip(*id)),
            };
            if new_track.0 as usize >= state.track_count() {
                return Err(TimelineError::UnknownTrack(*new_track));
            }
            let same_track = entry.track == *new_track;
            // validate against target-track neighbors excluding the clip itself
            let pos = state.insertion_pos(*new_track, *new_start);
            let (pred, succ) = if same_track {
                state.neighbors_excluding(*new_track, pos, *id)
            } else {
                state.neighbors_at(*new_track, pos)
            };
            if let Some(p) = pred {
                if p.end() > *new_start {
                    return Err(TimelineError::Overlap {
                        track: *new_track,
                        existing: p.id,
                        at: *new_start,
                    });
                }
            }
            if let Some(s) = succ {
                if new_start.add(entry.dur) > s.start {
                    return Err(TimelineError::Overlap {
                        track: *new_track,
                        existing: s.id,
                        at: *new_start,
                    });
                }
            }
            // mutate: remove from source sequence, update, insert at sorted position
            let old_pos = state.position_of(entry.track, *id);
            state.remove_clip_from_track(entry.track, old_pos);
            let mut updated = entry.clone();
            updated.track = *new_track;
            updated.start = *new_start;
            let ins = state.insertion_pos(*new_track, *new_start);
            state.insert_clip(*new_track, *id, ins);
            state.put_clip(updated);
            Ok(Applied {
                assigned: vec![],
                inverse: Command::MoveClip {
                    id: *id,
                    track: entry.track,
                    start: entry.start,
                },
                resolved: cmd.clone(),
            })
        }

        Command::ResizeClip { id, dur } => {
            let entry = match state.clip(*id) {
                Some(e) => e.clone(),
                None => return Err(TimelineError::UnknownClip(*id)),
            };
            if !positive(*dur) {
                return Err(TimelineError::InvalidDuration(*dur));
            }
            let pos = state.position_of(entry.track, *id);
            if let Some((succ_id, succ_start)) = successor_of(state, entry.track, pos) {
                if entry.start.add(*dur) > succ_start {
                    return Err(TimelineError::Overlap {
                        track: entry.track,
                        existing: succ_id,
                        at: entry.start,
                    });
                }
            }
            let mut updated = entry.clone();
            updated.dur = *dur;
            state.put_clip(updated);
            Ok(Applied {
                assigned: vec![],
                inverse: Command::ResizeClip {
                    id: *id,
                    dur: entry.dur,
                },
                resolved: cmd.clone(),
            })
        }

        Command::SplitClip { id, offset, new_id } => {
            let entry = match state.clip(*id) {
                Some(e) => e.clone(),
                None => return Err(TimelineError::UnknownClip(*id)),
            };
            if !(*offset > Rational::zero(1) && *offset < entry.dur) {
                return Err(TimelineError::InvalidSplit {
                    id: *id,
                    offset: *offset,
                });
            }
            if let Some(nid) = new_id {
                if !state.id_free(*nid) {
                    return Err(TimelineError::IdInUse(*nid));
                }
            }
            let right_id = match new_id {
                Some(nid) => *nid,
                None => state.assign_id(),
            };
            let right = ClipEntry {
                id: right_id,
                track: entry.track,
                asset: entry.asset,
                start: entry.start.add(*offset),
                dur: entry.dur.sub(*offset),
                src_in: entry.src_at(*offset),
                speed: entry.speed,
                label: format!("{}_b", entry.label),
            };
            let mut updated = entry.clone();
            updated.dur = *offset;
            state.put_clip(updated);
            state.put_clip(right);
            let pos = state.position_of(entry.track, *id);
            state.insert_clip(entry.track, right_id, pos + 1);
            let mut resolved = cmd.clone();
            if let Command::SplitClip { new_id: rid, .. } = &mut resolved {
                *rid = Some(right_id);
            }
            Ok(Applied {
                assigned: vec![right_id],
                inverse: Command::Batch {
                    cmds: vec![
                        Command::RemoveClip { id: right_id },
                        Command::ResizeClip {
                            id: *id,
                            dur: entry.dur,
                        },
                    ],
                    label: format!("unsplit:{}", entry.label),
                },
                resolved,
            })
        }

        Command::RetimeClip { id, speed } => {
            let entry = match state.clip(*id) {
                Some(e) => e.clone(),
                None => return Err(TimelineError::UnknownClip(*id)),
            };
            if !positive(*speed) {
                return Err(TimelineError::InvalidSpeed(*speed));
            }
            // exact: new_dur = src_dur / speed' = (dur * speed) * (speed'.den / speed'.num)
            let new_dur = entry.src_dur().mul(Rational::new(speed.den(), speed.num()));
            let pos = state.position_of(entry.track, *id);
            if let Some((succ_id, succ_start)) = successor_of(state, entry.track, pos) {
                if entry.start.add(new_dur) > succ_start {
                    return Err(TimelineError::Overlap {
                        track: entry.track,
                        existing: succ_id,
                        at: entry.start,
                    });
                }
            }
            let mut updated = entry.clone();
            updated.dur = new_dur;
            updated.speed = *speed;
            state.put_clip(updated);
            Ok(Applied {
                assigned: vec![],
                inverse: Command::RetimeClip {
                    id: *id,
                    speed: entry.speed,
                },
                resolved: cmd.clone(),
            })
        }

        Command::Batch { cmds, label } => {
            let mut inverses: Vec<Command> = Vec::with_capacity(cmds.len());
            let mut resolved_subs: Vec<Command> = Vec::with_capacity(cmds.len());
            let mut assigned = Vec::new();
            for (i, sub) in cmds.iter().enumerate() {
                match apply_command(state, sub) {
                    Ok(a) => {
                        inverses.push(a.inverse);
                        resolved_subs.push(a.resolved);
                        assigned.extend(a.assigned);
                    }
                    Err(e) => {
                        // roll back the applied prefix (exact inverses, reverse order)
                        for inv in inverses.iter().rev() {
                            apply_command(state, inv)
                                .expect("rollback inverse failed — engine invariant broken");
                        }
                        return Err(TimelineError::BatchRolledBack {
                            applied: i,
                            label: label.clone(),
                            error: Box::new(e),
                        });
                    }
                }
            }
            inverses.reverse(); // composite inverse = reversed sub-inverses (ADR-009)
            Ok(Applied {
                assigned,
                inverse: Command::Batch {
                    cmds: inverses,
                    label: format!("undo:{}", label),
                },
                resolved: Command::Batch {
                    cmds: resolved_subs,
                    label: label.clone(),
                },
            })
        }
    }
}

/// Session engine: state + undo/redo stacks + journal (ADR-009: stacks are
/// session state; the journal is the durable record — ove-project persists it).
#[derive(Clone, Debug)]
pub struct TimelineEngine {
    pub state: TimelineState,
    undo_stack: Vec<(Command, Command)>, // (resolved original, resolved inverse)
    redo_stack: Vec<Command>,            // resolved originals, redo order
    journal: Vec<JournalEntry>,
}

impl TimelineEngine {
    pub fn new(state: TimelineState) -> Self {
        TimelineEngine {
            state,
            undo_stack: Vec::new(),
            redo_stack: Vec::new(),
            journal: Vec::new(),
        }
    }

    pub fn apply(&mut self, cmd: Command) -> Result<Receipt, TimelineError> {
        let verb = cmd.verb();
        let applied = apply_command(&mut self.state, &cmd)?;
        let inverse = applied.inverse.clone();
        self.journal.push(JournalEntry::Do {
            cmd: applied.resolved.clone(),
            inverse: inverse.clone(),
        });
        self.undo_stack
            .push((applied.resolved.clone(), inverse.clone()));
        self.redo_stack.clear();
        Ok(Receipt {
            verb,
            assigned: applied.assigned,
            inverse,
            resolved: applied.resolved,
        })
    }

    /// Undo the last command by applying its exact inverse (ADR-009). The undo
    /// marker goes to the journal so a fresh replay reproduces everything.
    pub fn undo(&mut self) -> Result<Receipt, TimelineError> {
        let (orig, inverse) = self
            .undo_stack
            .pop()
            .ok_or(TimelineError::UndoUnavailable)?;
        let verb = inverse.verb();
        let applied = apply_command(&mut self.state, &inverse)?;
        self.journal.push(JournalEntry::Undo {
            inverse: inverse.clone(),
        });
        self.redo_stack.push(orig.clone());
        Ok(Receipt {
            verb,
            assigned: applied.assigned,
            inverse: orig,
            resolved: inverse,
        })
    }

    /// Redo the last undone command by re-applying the original. Id assignment is
    /// deterministic (same counter state as the original application), so redo is
    /// hash-exact (E-003/E-009 replay rule).
    pub fn redo(&mut self) -> Result<Receipt, TimelineError> {
        let orig = self
            .redo_stack
            .pop()
            .ok_or(TimelineError::RedoUnavailable)?;
        let verb = orig.verb();
        let applied = apply_command(&mut self.state, &orig)?;
        let inverse = applied.inverse.clone();
        self.journal.push(JournalEntry::Redo { cmd: orig.clone() });
        self.undo_stack.push((orig, inverse.clone()));
        Ok(Receipt {
            verb,
            assigned: applied.assigned,
            inverse,
            resolved: applied.resolved,
        })
    }

    pub fn journal(&self) -> &[JournalEntry] {
        &self.journal
    }

    pub fn undo_depth(&self) -> usize {
        self.undo_stack.len()
    }

    pub fn redo_depth(&self) -> usize {
        self.redo_stack.len()
    }

    /// Rebuild an engine by folding a journal from empty state (E-003 P1/P3
    /// invariants: apply == replay). Hash verification against checkpoints is the
    /// CALLER's job at checkpoint granularity (properties P3/P5 here; project
    /// save/load in ove-project) — folding itself must stay O(entries × log n).
    pub fn replay(
        track_kinds: &[crate::clip::TrackKind],
        journal: &[JournalEntry],
    ) -> Result<TimelineEngine, TimelineError> {
        let mut eng = TimelineEngine::new(TimelineState::new(track_kinds));
        for entry in journal {
            eng.journal.push(entry.clone()); // replayed engine inherits the journal
            match entry {
                JournalEntry::Do { cmd, inverse } => {
                    let applied = apply_command(&mut eng.state, cmd)?;
                    debug_assert_eq!(applied.inverse, *inverse, "replay inverse mismatch");
                    eng.undo_stack.push((cmd.clone(), inverse.clone()));
                    eng.redo_stack.clear();
                }
                JournalEntry::Undo { inverse } => {
                    let (orig, inv) = eng
                        .undo_stack
                        .last()
                        .cloned()
                        .ok_or(TimelineError::UndoUnavailable)?;
                    debug_assert_eq!(&inv, inverse, "replay undo marker mismatch");
                    apply_command(&mut eng.state, inverse)?;
                    eng.undo_stack.pop();
                    eng.redo_stack.push(orig);
                }
                JournalEntry::Redo { cmd } => {
                    let orig = eng.redo_stack.pop().ok_or(TimelineError::RedoUnavailable)?;
                    debug_assert_eq!(&orig, cmd, "replay redo marker mismatch");
                    let applied = apply_command(&mut eng.state, cmd)?;
                    eng.undo_stack.push((cmd.clone(), applied.inverse));
                }
            }
        }
        Ok(eng)
    }
}
