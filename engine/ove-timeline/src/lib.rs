//! ove-timeline — per-track clip sequences, edit verbs with exact inverses.
//!
//! Design authority:
//!   * ADR-007 — authoritative time is exact rational (ove-time crate). All
//!     durations/positions in this crate are `Rational`; benchmarks and tests
//!     keep them on the 24 kHz tick grid (den divides 24000) per the tick-axis
//!     rule from E-002c P3b.
//!   * ADR-009 — every command ships an exact inverse; batch = composite
//!     inverse (reversed sub-inverses); undo is LIFO over one stack.
//!   * E-003   — explicit ids in every command; floats forbidden; replay of a
//!     command log must be hash-deterministic.
//!   * ADR-011 — primary per-track structure is under integration test (E-012):
//!     this crate ships two interchangeable implementations (gap buffer with a
//!     lazily rebuilt prefix index vs augmented order-statistic AVL) plus a
//!     naive oracle used by property tests, all behind one verb surface.

pub mod avl;
pub mod gap;
pub mod oracle;

use std::collections::{BTreeMap, HashSet};

use ove_time::Rational;

pub use avl::AvlTrack;
pub use gap::GapTrack;
pub use oracle::OracleTrack;

pub type ClipId = u64;
pub type TrackId = u64;

/// A clip as the timeline sees it. `duration` is the timeline-duration
/// (exact); `source_in` is the offset into the source media (exact). No
/// floats, ever (E-003).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Clip {
    pub id: ClipId,
    pub duration: Rational,
    pub source_in: Rational,
}

impl Clip {
    pub fn new(id: ClipId, duration: Rational, source_in: Rational) -> Self {
        Clip {
            id,
            duration,
            source_in,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum TimelineError {
    TrackNotFound(TrackId),
    ClipNotFound(ClipId),
    DuplicateClipId(ClipId),
    IndexOutOfBounds {
        index: usize,
        len: usize,
    },
    /// Split point must be strictly inside the clip: 0 < at < duration.
    InvalidSplitPoint,
    /// Durations must be > 0.
    InvalidDuration,
}

/// The per-track container contract. Implementations must preserve exact
/// semantics (verified by property tests against `OracleTrack`):
/// positions are 0-based sequence order; absolute start of clip i is the
/// exact prefix sum of durations [0..i) — DERIVED, never stored.
pub trait TrackOps {
    fn len(&self) -> usize;
    fn is_empty(&self) -> bool {
        self.len() == 0
    }
    fn clip_at(&self, pos: usize) -> Option<&Clip>;
    /// Position of the clip with this id. NOTE (E-012 finding): O(n) linear
    /// scan by design — no id→position map is maintained because every
    /// structural edit shifts downstream positions (the E-002c2 rekey trap).
    /// At 20k clips the scan is tens of microseconds; the ordered index is
    /// reserved for TIME queries (hit_test), where it cannot be avoided.
    fn index_of(&self, id: ClipId) -> Option<usize> {
        let mut found = None;
        self.walk(&mut |pos, _start, c| {
            if found.is_none() && c.id == id {
                found = Some(pos);
            }
        });
        found
    }
    /// Clip whose derived [start, start+dur) contains `t`.
    /// Boundary: t == start of clip i resolves to clip i; t < 0 or
    /// t >= total → None. Exact rational comparisons only.
    fn hit_test(&self, t: Rational) -> Option<usize>;
    /// Scrub-path variant (single-writer engine: the scrub caller holds
    /// &mut). Default delegates to `hit_test`; GapTrack overrides it to
    /// rebuild the derived index once when dirty, then binary-search.
    fn hit_test_mut(&mut self, t: Rational) -> Option<usize> {
        self.hit_test(t)
    }
    fn insert_at(&mut self, pos: usize, clip: Clip) -> Result<(), TimelineError>;
    fn remove_at(&mut self, pos: usize) -> Result<Clip, TimelineError>;
    /// Set duration; returns the previous duration (for exact inverses).
    fn set_duration_at(&mut self, pos: usize, d: Rational) -> Result<Rational, TimelineError>;
    /// In-order walk with derived absolute starts (the render-walk path).
    fn walk(&self, f: &mut dyn FnMut(usize, Rational, &Clip));
    /// Deterministic FNV-1a over (positions, ids, durations, source offsets).
    fn state_hash(&self) -> u64 {
        let mut h: u64 = 0xcbf2_9ce4_8422_2325;
        let mut mix = |bytes: &[u8]| {
            for &b in bytes {
                h ^= b as u64;
                h = h.wrapping_mul(0x0000_0100_0000_01b3);
            }
        };
        self.walk(&mut |pos, start, c| {
            mix(&pos.to_le_bytes());
            mix(&c.id.to_le_bytes());
            mix(&c.duration.num().to_le_bytes());
            mix(&c.duration.den().to_le_bytes());
            mix(&c.source_in.num().to_le_bytes());
            mix(&c.source_in.den().to_le_bytes());
            mix(&start.num().to_le_bytes());
            mix(&start.den().to_le_bytes());
        });
        h
    }
}

/// Which container backs a track. `Oracle` exists for property tests and
/// cross-validation (doc 45 rule 4: every structure faces a naive oracle).
#[derive(Clone)]
pub enum TrackKind {
    Gap(GapTrack),
    Avl(AvlTrack),
    Oracle(OracleTrack),
}

impl TrackOps for TrackKind {
    fn len(&self) -> usize {
        match self {
            TrackKind::Gap(t) => t.len(),
            TrackKind::Avl(t) => t.len(),
            TrackKind::Oracle(t) => t.len(),
        }
    }
    fn clip_at(&self, pos: usize) -> Option<&Clip> {
        match self {
            TrackKind::Gap(t) => t.clip_at(pos),
            TrackKind::Avl(t) => t.clip_at(pos),
            TrackKind::Oracle(t) => t.clip_at(pos),
        }
    }
    fn hit_test(&self, t: Rational) -> Option<usize> {
        match self {
            TrackKind::Gap(g) => g.hit_test(t),
            TrackKind::Avl(a) => a.hit_test(t),
            TrackKind::Oracle(o) => o.hit_test(t),
        }
    }
    fn hit_test_mut(&mut self, t: Rational) -> Option<usize> {
        match self {
            TrackKind::Gap(g) => g.hit_test_mut(t),
            TrackKind::Avl(a) => a.hit_test_mut(t),
            TrackKind::Oracle(o) => o.hit_test_mut(t),
        }
    }
    fn insert_at(&mut self, pos: usize, clip: Clip) -> Result<(), TimelineError> {
        match self {
            TrackKind::Gap(t) => t.insert_at(pos, clip),
            TrackKind::Avl(t) => t.insert_at(pos, clip),
            TrackKind::Oracle(t) => t.insert_at(pos, clip),
        }
    }
    fn remove_at(&mut self, pos: usize) -> Result<Clip, TimelineError> {
        match self {
            TrackKind::Gap(t) => t.remove_at(pos),
            TrackKind::Avl(t) => t.remove_at(pos),
            TrackKind::Oracle(t) => t.remove_at(pos),
        }
    }
    fn set_duration_at(&mut self, pos: usize, d: Rational) -> Result<Rational, TimelineError> {
        match self {
            TrackKind::Gap(t) => t.set_duration_at(pos, d),
            TrackKind::Avl(t) => t.set_duration_at(pos, d),
            TrackKind::Oracle(t) => t.set_duration_at(pos, d),
        }
    }
    fn walk(&self, f: &mut dyn FnMut(usize, Rational, &Clip)) {
        match self {
            TrackKind::Gap(t) => t.walk(f),
            TrackKind::Avl(t) => t.walk(f),
            TrackKind::Oracle(t) => t.walk(f),
        }
    }
}

/// Edit verbs. Every command's exact inverse is computed by
/// `Timeline::apply` from the pre-state at apply time (ADR-009 option B);
/// the undo stack stores inverses only. Commands are pure data (E-003 log
/// rules: explicit ids, exact rationals, no floats).
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Command {
    /// Insert clip at `index` (pre-insert index) of `track`.
    Insert {
        track: TrackId,
        index: usize,
        clip: Clip,
    },
    /// Remove the clip with this id.
    Remove { track: TrackId, id: ClipId },
    /// Split clip `id` at offset `at` into its own duration
    /// (0 < at < duration). Left half keeps the id and source_in; the right
    /// half gets `new_id` and source_in + at. ALLOCATION DISCIPLINE (E-012
    /// finding, extends E-003): `new_id` is allocated by the CALLER via
    /// `Timeline::alloc_id()` at command-construction time and rides the log
    /// explicitly — the engine never allocates during apply. A log with
    /// implicit allocation diverges on replay once removed ids leave holes
    /// in the used-id set (found by the E-012 oracle-equivalence property).
    Split {
        track: TrackId,
        id: ClipId,
        at: Rational,
        new_id: ClipId,
    },
    /// Resize clip to an exact new duration (> 0).
    Resize {
        track: TrackId,
        id: ClipId,
        duration: Rational,
    },
    /// Move clip to `to_track` at `to_index`, where to_index is the index
    /// in the destination AFTER removal of the clip (post-state index).
    Move {
        id: ClipId,
        from_track: TrackId,
        to_track: TrackId,
        to_index: usize,
    },
    /// Atomic batch: all-or-nothing; inverse = reversed sub-inverses
    /// (E-009 batch atomicity).
    Batch { cmds: Vec<Command> },
}

/// The timeline: ordered tracks, id allocation, command application with
/// exact inverses. Single-writer (ADR-008 v1 rule); per-track sharding is
/// exercised by the threaded property test.
#[derive(Clone)]
pub struct Timeline {
    tracks: BTreeMap<TrackId, TrackKind>,
    next_id: ClipId,
    used_ids: HashSet<ClipId>,
}

impl Timeline {
    pub fn new() -> Self {
        Timeline {
            tracks: BTreeMap::new(),
            next_id: 1,
            used_ids: HashSet::new(),
        }
    }

    pub fn add_track(&mut self, id: TrackId, kind: TrackKind) -> Result<(), TimelineError> {
        if self.tracks.contains_key(&id) {
            return Err(TimelineError::TrackNotFound(id)); // caller mistake: id taken
        }
        self.tracks.insert(id, kind);
        Ok(())
    }

    pub fn track(&self, id: TrackId) -> Option<&TrackKind> {
        self.tracks.get(&id)
    }

    pub fn track_ids(&self) -> impl Iterator<Item = TrackId> + '_ {
        self.tracks.keys().copied()
    }

    pub fn track_len(&self, id: TrackId) -> Result<usize, TimelineError> {
        Ok(self.track_ref(id)?.len())
    }

    /// Immutable track access (read paths: walk, hit-test on a clean index).
    pub fn track_ref(&self, id: TrackId) -> Result<&dyn TrackOps, TimelineError> {
        match self.tracks.get(&id) {
            Some(TrackKind::Gap(t)) => Ok(t),
            Some(TrackKind::Avl(t)) => Ok(t),
            Some(TrackKind::Oracle(t)) => Ok(t),
            None => Err(TimelineError::TrackNotFound(id)),
        }
    }

    /// Mutable track access — the single-writer edit/scrub path (ADR-008).
    pub fn track_mut(&mut self, id: TrackId) -> Result<&mut dyn TrackOps, TimelineError> {
        match self.tracks.get_mut(&id) {
            Some(TrackKind::Gap(t)) => Ok(t),
            Some(TrackKind::Avl(t)) => Ok(t),
            Some(TrackKind::Oracle(t)) => Ok(t),
            None => Err(TimelineError::TrackNotFound(id)),
        }
    }

    /// Deterministic fresh id (skips any externally-inserted id). The id is
    /// only marked used when a command actually inserts it — alloc + apply
    /// is the intended pairing (caught by E-012 property tests: reserving
    /// here made every following Insert a DuplicateClipId).
    pub fn alloc_id(&mut self) -> ClipId {
        while self.used_ids.contains(&self.next_id) {
            self.next_id += 1;
        }
        let id = self.next_id;
        self.next_id += 1;
        id
    }

    pub fn id_exists(&self, id: ClipId) -> bool {
        self.used_ids.contains(&id)
    }

    /// Apply a command; returns its exact inverse computed from pre-state.
    /// On any error the timeline is unchanged (validated before mutation;
    /// Batch rolls back via already-collected inverses).
    pub fn apply(&mut self, cmd: &Command) -> Result<Command, TimelineError> {
        match cmd {
            Command::Insert { track, index, clip } => {
                if self.used_ids.contains(&clip.id) {
                    return Err(TimelineError::DuplicateClipId(clip.id));
                }
                let t = self.track_mut(*track)?;
                if *index > t.len() {
                    return Err(TimelineError::IndexOutOfBounds {
                        index: *index,
                        len: t.len(),
                    });
                }
                let inv = Command::Remove {
                    track: *track,
                    id: clip.id,
                };
                t.insert_at(*index, clip.clone())?;
                self.used_ids.insert(clip.id);
                Ok(inv)
            }
            Command::Remove { track, id } => {
                let t = self.track_mut(*track)?;
                let pos = t.index_of(*id).ok_or(TimelineError::ClipNotFound(*id))?;
                let c = t.remove_at(pos)?;
                self.used_ids.remove(id);
                Ok(Command::Insert {
                    track: *track,
                    index: pos,
                    clip: c,
                })
            }
            Command::Split {
                track,
                id,
                at,
                new_id,
            } => {
                if at.num() <= 0 {
                    return Err(TimelineError::InvalidSplitPoint);
                }
                if *new_id == 0 || self.used_ids.contains(new_id) {
                    return Err(TimelineError::DuplicateClipId(*new_id));
                }
                let (pos, c) = {
                    let t = self.track_mut(*track)?;
                    let pos = t.index_of(*id).ok_or(TimelineError::ClipNotFound(*id))?;
                    (pos, t.clip_at(pos).unwrap().clone())
                };
                if *at >= c.duration {
                    return Err(TimelineError::InvalidSplitPoint);
                }
                let right = Clip {
                    id: *new_id,
                    duration: c.duration.sub(*at),
                    source_in: c.source_in.add(*at),
                };
                let t = self.track_mut(*track)?;
                t.set_duration_at(pos, *at)?;
                t.insert_at(pos + 1, right)?;
                self.used_ids.insert(*new_id);
                Ok(Command::Batch {
                    cmds: vec![
                        Command::Remove {
                            track: *track,
                            id: *new_id,
                        },
                        Command::Resize {
                            track: *track,
                            id: *id,
                            duration: c.duration,
                        },
                    ],
                })
            }
            Command::Resize {
                track,
                id,
                duration,
            } => {
                if duration.num() <= 0 {
                    return Err(TimelineError::InvalidDuration);
                }
                let t = self.track_mut(*track)?;
                let pos = t.index_of(*id).ok_or(TimelineError::ClipNotFound(*id))?;
                let old = t.set_duration_at(pos, *duration)?;
                Ok(Command::Resize {
                    track: *track,
                    id: *id,
                    duration: old,
                })
            }
            Command::Move {
                id,
                from_track,
                to_track,
                to_index,
            } => {
                let (src_pos, _clip) = {
                    let t = self.track_mut(*from_track)?;
                    let p = t.index_of(*id).ok_or(TimelineError::ClipNotFound(*id))?;
                    (p, t.clip_at(p).unwrap().clone())
                };
                // Pre-validate destination index (post-removal length of dest).
                let dest_len_after = {
                    let d = self.track_ref(*to_track)?;
                    d.len() - if from_track == to_track { 1 } else { 0 }
                };
                if *to_index > dest_len_after {
                    return Err(TimelineError::IndexOutOfBounds {
                        index: *to_index,
                        len: dest_len_after,
                    });
                }
                let c = self.track_mut(*from_track)?.remove_at(src_pos)?;
                self.track_mut(*to_track)?.insert_at(*to_index, c)?;
                // Inverse from post-state: the clip now sits at index_of(id)
                // in to_track; moving it back to src_pos (post-removal
                // semantics of the inverse) restores the exact original state.
                let cur = self.track_ref(*to_track)?.index_of(*id).unwrap();
                debug_assert_eq!(cur, *to_index);
                Ok(Command::Move {
                    id: *id,
                    from_track: *to_track,
                    to_track: *from_track,
                    to_index: src_pos,
                })
            }
            Command::Batch { cmds } => {
                let mut inverses: Vec<Command> = Vec::with_capacity(cmds.len());
                for c in cmds {
                    match self.apply(c) {
                        Ok(inv) => inverses.push(inv),
                        Err(e) => {
                            for inv in inverses.iter().rev() {
                                // Rollback must succeed by construction; a
                                // panic here is an engine bug, not a user error.
                                self.apply(inv).expect("batch rollback must succeed");
                            }
                            return Err(e);
                        }
                    }
                }
                inverses.reverse();
                Ok(Command::Batch { cmds: inverses })
            }
        }
    }

    /// Total timeline state hash: tracks in TrackId order.
    pub fn state_hash(&self) -> u64 {
        let mut h: u64 = 0xcbf2_9ce4_8422_2325;
        let mut mix = |bytes: &[u8]| {
            for &b in bytes {
                h ^= b as u64;
                h = h.wrapping_mul(0x0000_0100_0000_01b3);
            }
        };
        for (tid, kind) in &self.tracks {
            mix(&tid.to_le_bytes());
            mix(&kind.state_hash().to_le_bytes());
        }
        h
    }
}

impl Default for Timeline {
    fn default() -> Self {
        Self::new()
    }
}

/// LIFO undo over exact inverses (ADR-009). `record` clears the redo branch.
/// The undo stack is per-session state and is never persisted — the command
/// log is the record (ADR-008).
#[derive(Default)]
pub struct UndoStack {
    undo: Vec<Command>,
    redo: Vec<Command>,
}

impl UndoStack {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn record(&mut self, inverse: Command) {
        self.undo.push(inverse);
        self.redo.clear();
    }

    /// Undo the top step. Returns Ok(false) when the stack is empty.
    pub fn undo(&mut self, tl: &mut Timeline) -> Result<bool, TimelineError> {
        match self.undo.pop() {
            None => Ok(false),
            Some(inv) => {
                let fwd = tl.apply(&inv)?;
                self.redo.push(fwd);
                Ok(true)
            }
        }
    }

    /// Redo the top undone step. Returns Ok(false) when nothing to redo.
    pub fn redo(&mut self, tl: &mut Timeline) -> Result<bool, TimelineError> {
        match self.redo.pop() {
            None => Ok(false),
            Some(fwd) => {
                let inv = tl.apply(&fwd)?;
                self.undo.push(inv);
                Ok(true)
            }
        }
    }

    pub fn depth(&self) -> usize {
        self.undo.len()
    }
}

/// Build helpers shared by tests and the E-012 bench: clips on the 24 kHz
/// tick grid (den divides 24000 — the tick-axis rule).
pub const TICK_DEN: i64 = 24_000;

pub fn ticks(n: i64) -> Rational {
    Rational::new(n, TICK_DEN)
}
