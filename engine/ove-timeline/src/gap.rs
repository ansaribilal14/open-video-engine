//! GapTrack — ADR-011's provisional primary structure under E-012 test:
//! gap buffer as the per-track sequence (O(1) edits at the cursor) plus a
//! DERIVED, lazily rebuilt prefix-sum index for time queries (hit_test).
//!
//! Semantics contract (shared with AvlTrack/OracleTrack, see TrackOps):
//!   * absolute starts are never stored authoritatively — they are exact
//!     prefix sums of durations, rebuilt lazily for time queries;
//!   * every mutation marks the index dirty (any edit shifts all downstream
//!     starts — the E-002c2 rekey lesson forbids incremental time-rekeys).

use std::collections::HashMap;

use ove_time::Rational;

use crate::{Clip, ClipId, TimelineError, TrackOps, TICK_DEN};

/// Generic gap buffer over `Option<T>` slots (None = gap cells). Logical
/// index i maps to physical `i` if i < gap, else `i + gap_len`.
pub struct GapBuffer<T> {
    buf: Vec<Option<T>>,
    gap: usize,
    gap_len: usize,
}

impl<T> GapBuffer<T> {
    pub fn with_capacity(n: usize) -> Self {
        GapBuffer {
            buf: (0..n).map(|_| None).collect(),
            gap: 0,
            gap_len: n,
        }
    }

    pub fn len(&self) -> usize {
        self.buf.len() - self.gap_len
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    #[inline]
    fn phys(&self, i: usize) -> usize {
        if i < self.gap {
            i
        } else {
            i + self.gap_len
        }
    }

    /// Move the gap so logical index `to` sits at the gap start.
    /// Uses slice rotation — correct for non-Copy T.
    pub fn move_gap(&mut self, to: usize) {
        debug_assert!(to <= self.len());
        if to == self.gap {
            return;
        }
        if to < self.gap {
            // live segment [to, gap) must jump right past the gap.
            let w = self.gap + self.gap_len; // window end (exclusive)
            self.buf[to..w].rotate_left(self.gap - to);
        } else {
            // live segment [gap_end, to_end) must jump left to the gap start.
            let s = self.gap + self.gap_len;
            let e = to + self.gap_len;
            self.buf[self.gap..e].rotate_right(e - s);
        }
        self.gap = to;
    }

    fn grow(&mut self) {
        let n = self.buf.len();
        let suffix: Vec<Option<T>> = self.buf.drain(self.gap..n).collect();
        // Final layout: prefix [0..gap) | gap cells [gap..n+gap) | suffix
        // [n+gap..2n). gap_len becomes n (doubled capacity).
        self.buf.resize_with(n + self.gap, || None);
        self.buf.extend(suffix);
        self.gap_len = n;
    }

    /// Insert at logical index `i` (0..=len).
    pub fn insert(&mut self, i: usize, value: T) {
        if self.gap_len == 0 {
            self.grow();
        }
        self.move_gap(i);
        self.buf[self.gap] = Some(value);
        self.gap += 1;
        self.gap_len -= 1;
    }

    /// Remove and return the element at logical index `i`.
    pub fn remove(&mut self, i: usize) -> T {
        self.move_gap(i);
        // element i now sits immediately right of the gap
        let c = self.buf[self.gap + self.gap_len]
            .take()
            .expect("gap remove: live element");
        self.gap_len += 1;
        c
    }

    #[inline]
    pub fn get(&self, i: usize) -> &T {
        self.buf[self.phys(i)]
            .as_ref()
            .expect("gap get: live element")
    }

    #[inline]
    pub fn set(&mut self, i: usize, value: T) {
        let p = self.phys(i);
        self.buf[p] = Some(value);
    }
}

#[derive(Default)]
struct GapIndex {
    /// starts[i] = exact absolute start of logical clip i.
    starts: Vec<Rational>,
    by_id: HashMap<ClipId, usize>,
}

/// ADR-011 provisional design: gap buffer + lazily rebuilt derived index.
pub struct GapTrack {
    buf: GapBuffer<Clip>,
    index: GapIndex,
    dirty: bool,
}

impl GapTrack {
    pub fn new() -> Self {
        GapTrack {
            buf: GapBuffer::with_capacity(64),
            index: GapIndex::default(),
            dirty: true,
        }
    }

    pub fn from_clips(clips: Vec<Clip>) -> Self {
        let mut t = GapTrack::new();
        for c in clips {
            t.buf.insert(t.buf.len(), c);
        }
        t.dirty = true;
        t
    }

    /// Rebuild the derived index: one exact prefix walk. O(n) rational adds.
    fn ensure_index(&mut self) {
        if !self.dirty {
            return;
        }
        let mut starts = Vec::with_capacity(self.buf.len());
        let mut by_id = HashMap::with_capacity(self.buf.len());
        let mut acc = Rational::new(0, TICK_DEN);
        for i in 0..self.buf.len() {
            let c = self.buf.get(i);
            starts.push(acc);
            by_id.insert(c.id, i);
            acc = acc.add(c.duration);
        }
        self.index.starts = starts;
        self.index.by_id = by_id;
        self.dirty = false;
    }
}

impl Default for GapTrack {
    fn default() -> Self {
        Self::new()
    }
}

impl TrackOps for GapTrack {
    fn len(&self) -> usize {
        self.buf.len()
    }

    fn clip_at(&self, pos: usize) -> Option<&Clip> {
        if pos >= self.buf.len() {
            None
        } else {
            Some(self.buf.get(pos))
        }
    }

    /// O(n) linear scan (see TrackOps::index_of rationale). Does NOT touch
    /// the derived time index.
    fn index_of(&self, id: ClipId) -> Option<usize> {
        (0..self.buf.len()).find(|&i| self.buf.get(i).id == id)
    }

    fn hit_test(&self, t: Rational) -> Option<usize> {
        // Read-only fallback: if the index is clean, binary-search it;
        // otherwise an allocation-free exact linear walk (O(n)). The scrub
        // path uses `hit_test_mut`, which rebuilds once and binary-searches.
        if !self.dirty {
            let starts = &self.index.starts;
            let pp = starts.partition_point(|s| *s <= t);
            if pp == 0 {
                return None;
            }
            let i = pp - 1;
            // t >= total (beyond last clip's end) must resolve to None.
            let end = starts[i].add(self.buf.get(i).duration);
            return if t < end { Some(i) } else { None };
        }
        let mut acc = Rational::new(0, TICK_DEN);
        for i in 0..self.buf.len() {
            let c = self.buf.get(i);
            let end = acc.add(c.duration);
            if t < acc {
                return None;
            }
            if t < end {
                return Some(i);
            }
            acc = end;
        }
        None
    }

    /// Scrub path: rebuild the derived index once if dirty (O(n) exact adds),
    /// then binary-search — O(log n) per query. This asymmetry (rebuild on
    /// first query after ANY edit) is exactly what E-012 W4 measures against
    /// the 1 ms scrub-burst budget.
    fn hit_test_mut(&mut self, t: Rational) -> Option<usize> {
        self.ensure_index();
        let starts = &self.index.starts;
        let pp = starts.partition_point(|s| *s <= t);
        if pp == 0 {
            return None;
        }
        let i = pp - 1;
        let end = starts[i].add(self.buf.get(i).duration);
        if t < end {
            Some(i)
        } else {
            None
        }
    }

    fn insert_at(&mut self, pos: usize, clip: Clip) -> Result<(), TimelineError> {
        if pos > self.buf.len() {
            return Err(TimelineError::IndexOutOfBounds {
                index: pos,
                len: self.buf.len(),
            });
        }
        self.buf.insert(pos, clip);
        self.dirty = true;
        Ok(())
    }

    fn remove_at(&mut self, pos: usize) -> Result<Clip, TimelineError> {
        if pos >= self.buf.len() {
            return Err(TimelineError::IndexOutOfBounds {
                index: pos,
                len: self.buf.len(),
            });
        }
        let c = self.buf.remove(pos);
        self.dirty = true;
        Ok(c)
    }

    fn set_duration_at(&mut self, pos: usize, d: Rational) -> Result<Rational, TimelineError> {
        if pos >= self.buf.len() {
            return Err(TimelineError::IndexOutOfBounds {
                index: pos,
                len: self.buf.len(),
            });
        }
        let mut c = self.buf.get(pos).clone();
        let old = c.duration;
        c.duration = d;
        self.buf.set(pos, c);
        self.dirty = true;
        Ok(old)
    }

    fn walk(&self, f: &mut dyn FnMut(usize, Rational, &Clip)) {
        let mut acc = Rational::new(0, TICK_DEN);
        for i in 0..self.buf.len() {
            let c = self.buf.get(i);
            f(i, acc, c);
            acc = acc.add(c.duration);
        }
    }
}

impl Clone for GapTrack {
    fn clone(&self) -> Self {
        let mut clips = Vec::with_capacity(self.buf.len());
        for i in 0..self.buf.len() {
            clips.push(self.buf.get(i).clone());
        }
        GapTrack::from_clips(clips)
    }
}
