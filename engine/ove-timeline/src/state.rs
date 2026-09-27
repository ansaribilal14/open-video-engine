//! Timeline state — per-track ordered clip sequences + the clip table.
//!
//! Structure (ADR-011 as realized here): one [`CursorBuffer`] per track, kept
//! sorted by clip start (starts are unique per track: overlap is rejected, and
//! zero-duration clips are rejected, so equal starts cannot occur). The sorted
//! buffer with O(1) random access serves binary-search queries directly — hit
//! testing, neighbor checks, insertion positions — with no second index structure
//! to keep in sync (E-002c2's rekey-collision bug class avoided by construction).
//!
//! All mutation goes through `apply_command` (engine.rs), which validates fully
//! before touching state. The helpers here are the validated primitives + queries.

use crate::clip::{ClipEntry, ClipId, TrackId, TrackKind};
use crate::gap::{binary_search_by_key, CursorBuffer};
use ove_time::Rational;

/// Default project tick axis for aggregates (ADR-007 P3b rule; E-002/E-003 axis).
pub const TICK_AXIS_DEFAULT: (i64, i64) = (24000, 1);

/// The timeline state. Tracks are fixed at construction (v1: no structural track
/// commands — anti-overbuild until an acceptance test asks for them).
#[derive(Clone, Debug)]
pub struct TimelineState {
    tracks: Vec<Track>,
    clips: std::collections::HashMap<ClipId, ClipEntry>,
    next_id: u64,
}

#[derive(Clone, Debug)]
struct Track {
    kind: TrackKind,
    seq: CursorBuffer<ClipId>,
}

impl TimelineState {
    /// Construct with a fixed track table (order = compositing bottom-up order).
    pub fn new(track_kinds: &[TrackKind]) -> Self {
        TimelineState {
            tracks: track_kinds
                .iter()
                .map(|k| Track {
                    kind: *k,
                    seq: CursorBuffer::new(),
                })
                .collect(),
            clips: std::collections::HashMap::new(),
            next_id: 1,
        }
    }

    pub fn track_count(&self) -> usize {
        self.tracks.len()
    }

    pub fn track_kind(&self, t: TrackId) -> Option<TrackKind> {
        self.tracks.get(t.0 as usize).map(|t| t.kind)
    }

    /// Assign the next engine id (O(1) folded counter — E-003 pattern).
    ///
    /// Replay-safety rule: the counter is NOT a deterministic function of the
    /// clip set (failed batch prefixes advance it without journaling), so replay
    /// must never re-derive assignments — `apply_command` returns the RESOLVED
    /// command (concrete ids) and the journal records that resolved form. Replay
    /// therefore assigns nothing; the counter only drives live assignment.
    pub(crate) fn assign_id(&mut self) -> ClipId {
        let id = ClipId(self.next_id);
        self.next_id += 1;
        id
    }

    /// Whether an explicit id is free (not currently in use).
    pub(crate) fn id_free(&self, id: ClipId) -> bool {
        !self.clips.contains_key(&id)
    }

    pub fn clip(&self, id: ClipId) -> Option<&ClipEntry> {
        self.clips.get(&id)
    }

    pub fn clip_count(&self) -> usize {
        self.clips.len()
    }

    /// Clips on a track in start order (the sorted sequence itself).
    pub fn clips_on_track(&self, t: TrackId) -> impl Iterator<Item = &ClipEntry> {
        self.tracks[t.0 as usize]
            .seq
            .iter()
            .filter_map(move |cid| self.clips.get(cid))
    }

    /// Total timeline duration: max clip end over all tracks (exact).
    pub fn duration(&self) -> Rational {
        let mut max = Rational::zero(1);
        for t in 0..self.tracks.len() {
            for c in self.clips_on_track(TrackId(t as u32)) {
                let e = c.end();
                if e > max {
                    max = e;
                }
            }
        }
        max
    }

    /// Duration of one track (exact fold of clip ends; empty track = 0).
    pub fn track_duration(&self, t: TrackId) -> Rational {
        let mut max = Rational::zero(1);
        for c in self.clips_on_track(t) {
            let e = c.end();
            if e > max {
                max = e;
            }
        }
        max
    }

    /// Hit-test: the clip on `t` whose `[start, start+dur)` contains `at`.
    pub fn hit_test(&self, t: TrackId, at: Rational) -> Option<ClipId> {
        let seq = &self.tracks[t.0 as usize].seq;
        match binary_search_by_key(seq, &at, |cid| self.clips[cid].start) {
            Ok(i) => Some(*seq.get(i)),
            Err(insert) => {
                if insert == 0 {
                    return None; // before the first clip starts
                }
                let pred = *seq.get(insert - 1);
                let c = &self.clips[&pred];
                if at < c.end() {
                    Some(pred)
                } else {
                    None // in a gap
                }
            }
        }
    }

    // ---- validated-mutation primitives (pub(crate); called by engine::apply_command
    // AFTER validation — they assume preconditions hold) ----

    /// Insert position for `start` on track `t` (buffer is sorted by start).
    pub(crate) fn insertion_pos(&self, t: TrackId, start: Rational) -> usize {
        let seq = &self.tracks[t.0 as usize].seq;
        match binary_search_by_key(seq, &start, |cid| self.clips[cid].start) {
            Ok(i) => i, // equal start cannot occur for valid adds (overlap), but stay total
            Err(p) => p,
        }
    }

    /// O(1) clip id at a sequence position on a track (buffer random access).
    pub(crate) fn clip_at_pos(&self, t: TrackId, pos: usize) -> Option<ClipId> {
        let seq = &self.tracks[t.0 as usize].seq;
        if pos < seq.len() {
            Some(*seq.get(pos))
        } else {
            None
        }
    }

    /// Neighbor entries around an insertion at `pos` on track `t`.
    pub(crate) fn neighbors_at(
        &self,
        t: TrackId,
        pos: usize,
    ) -> (Option<&ClipEntry>, Option<&ClipEntry>) {
        let seq = &self.tracks[t.0 as usize].seq;
        let pred = if pos == 0 {
            None
        } else {
            Some(*seq.get(pos - 1))
        };
        let succ = if pos >= seq.len() {
            None
        } else {
            Some(*seq.get(pos))
        };
        (
            pred.map(|cid| &self.clips[&cid]),
            succ.map(|cid| &self.clips[&cid]),
        )
    }

    /// Neighbor entries around an insertion that must skip `self_id` (same-track
    /// move validation — pure, no mutation: index arithmetic accounts for the
    /// clip's own position in the buffer).
    pub(crate) fn neighbors_excluding(
        &self,
        t: TrackId,
        pos: usize,
        self_id: ClipId,
    ) -> (Option<&ClipEntry>, Option<&ClipEntry>) {
        let seq = &self.tracks[t.0 as usize].seq;
        let self_pos = match binary_search_by_key(seq, &self.clips[&self_id].start, |cid| {
            self.clips[cid].start
        }) {
            Ok(i) => i,
            Err(_) => unreachable!("self clip missing from its track sequence"),
        };
        // effective position in the without-self sequence
        let eff = if pos > self_pos { pos - 1 } else { pos };
        // map back to buffer indices, skipping self
        let map = |w: usize| -> usize {
            if w >= self_pos {
                w + 1
            } else {
                w
            }
        };
        let pred = if eff == 0 {
            None
        } else {
            Some(*seq.get(map(eff - 1)))
        };
        let succ = if eff >= seq.len() - 1 {
            None
        } else {
            Some(*seq.get(map(eff)))
        };
        (
            pred.map(|cid| &self.clips[&cid]),
            succ.map(|cid| &self.clips[&cid]),
        )
    }

    pub(crate) fn position_of(&self, t: TrackId, id: ClipId) -> usize {
        let seq = &self.tracks[t.0 as usize].seq;
        match binary_search_by_key(seq, &self.clips[&id].start, |cid| self.clips[cid].start) {
            Ok(i) if *seq.get(i) == id => i,
            _ => unreachable!("clip id/start mismatch in track sequence"),
        }
    }

    pub(crate) fn insert_clip(&mut self, t: TrackId, id: ClipId, pos: usize) {
        let seq = &mut self.tracks[t.0 as usize].seq;
        seq.move_cursor_to(pos);
        seq.insert_at_cursor(id);
    }

    pub(crate) fn remove_clip_from_track(&mut self, t: TrackId, pos: usize) -> ClipId {
        let seq = &mut self.tracks[t.0 as usize].seq;
        seq.move_cursor_to(pos + 1);
        seq.remove_before_cursor()
    }

    pub(crate) fn put_clip(&mut self, entry: ClipEntry) {
        self.clips.insert(entry.id, entry);
    }

    pub(crate) fn take_clip(&mut self, id: ClipId) -> ClipEntry {
        self.clips.remove(&id).expect("clip exists")
    }

    // ---- canonical state hash ----

    /// FNV-1a 64 over the canonical serialization: per track (in table order),
    /// clips in start order; per clip all exact fields + label. Covers observable
    /// clip content; EXCLUDES the id-assignment counter (crate docs, hash rule).
    pub fn state_hash(&self) -> u64 {
        let mut h: u64 = 0xcbf2_9ce4_8422_2325;
        let mut w = |bytes: &[u8]| {
            for b in bytes {
                h ^= *b as u64;
                h = h.wrapping_mul(0x0000_0100_0000_01b3);
            }
        };
        w(&(self.tracks.len() as u32).to_le_bytes());
        for (ti, t) in self.tracks.iter().enumerate() {
            w(&[match t.kind {
                TrackKind::Video => 0u8,
                TrackKind::Audio => 1u8,
            }]);
            w(&(t.seq.len() as u32).to_le_bytes());
            for cid in t.seq.iter() {
                let c = &self.clips[cid];
                w(&(c.id.0).to_le_bytes());
                w(&c.start.num().to_le_bytes());
                w(&c.start.den().to_le_bytes());
                w(&c.dur.num().to_le_bytes());
                w(&c.dur.den().to_le_bytes());
                w(&c.src_in.num().to_le_bytes());
                w(&c.src_in.den().to_le_bytes());
                w(&c.speed.num().to_le_bytes());
                w(&c.speed.den().to_le_bytes());
                w(&(c.asset.0).to_le_bytes());
                w(c.label.as_bytes());
                w(&[0xff]); // label terminator (labels may be empty)
            }
            let _ = ti;
        }
        h
    }
}
