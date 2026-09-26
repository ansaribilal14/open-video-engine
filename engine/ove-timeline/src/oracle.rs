//! OracleTrack — deliberately naive Vec-based reference implementation.
//!
//! Role (doc 45 rule 4): every production structure must hash-match this
//! oracle under identical randomized op scripts. Obviously correct, slow on
//! purpose: linear id search, linear hit-test, full re-derivation of starts
//! on every walk. Never use in production paths.

use ove_time::Rational;

use crate::{Clip, ClipId, TimelineError, TrackOps, TICK_DEN};

#[derive(Clone)]
pub struct OracleTrack {
    v: Vec<Clip>,
}

impl OracleTrack {
    pub fn new() -> Self {
        OracleTrack { v: Vec::new() }
    }

    pub fn from_clips(clips: Vec<Clip>) -> Self {
        OracleTrack { v: clips }
    }
}

impl Default for OracleTrack {
    fn default() -> Self {
        Self::new()
    }
}

impl TrackOps for OracleTrack {
    fn len(&self) -> usize {
        self.v.len()
    }

    fn clip_at(&self, pos: usize) -> Option<&Clip> {
        self.v.get(pos)
    }

    fn index_of(&self, id: ClipId) -> Option<usize> {
        self.v.iter().position(|c| c.id == id)
    }

    fn hit_test(&self, t: Rational) -> Option<usize> {
        let mut acc = Rational::new(0, TICK_DEN);
        for (i, c) in self.v.iter().enumerate() {
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

    fn insert_at(&mut self, pos: usize, clip: Clip) -> Result<(), TimelineError> {
        if pos > self.v.len() {
            return Err(TimelineError::IndexOutOfBounds {
                index: pos,
                len: self.v.len(),
            });
        }
        self.v.insert(pos, clip);
        Ok(())
    }

    fn remove_at(&mut self, pos: usize) -> Result<Clip, TimelineError> {
        if pos >= self.v.len() {
            return Err(TimelineError::IndexOutOfBounds {
                index: pos,
                len: self.v.len(),
            });
        }
        Ok(self.v.remove(pos))
    }

    fn set_duration_at(&mut self, pos: usize, d: Rational) -> Result<Rational, TimelineError> {
        if pos >= self.v.len() {
            return Err(TimelineError::IndexOutOfBounds {
                index: pos,
                len: self.v.len(),
            });
        }
        let old = self.v[pos].duration;
        self.v[pos].duration = d;
        Ok(old)
    }

    fn walk(&self, f: &mut dyn FnMut(usize, Rational, &Clip)) {
        let mut acc = Rational::new(0, TICK_DEN);
        for (i, c) in self.v.iter().enumerate() {
            f(i, acc, c);
            acc = acc.add(c.duration);
        }
    }
}
