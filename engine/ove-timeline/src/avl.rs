//! AvlTrack — augmented order-statistic AVL over sequence order with exact
//! Rational subtree duration sums (the "derived-index shape" from E-002c2).
//!
//! Why it is in E-012: ADR-011 provisionally chose gap buffer + lazy index;
//! the E-002c2 addendum showed this tree answers TIME queries structurally
//! (weighted descent, no rebuild ever). E-012 benches both shapes under
//! identical scripts; ADR-011 flips or is revised on the numbers.
//!
//! Adapted from the committed E-002c2 benchmark implementation
//! (scripts/experiments/E-002c2_avl_random.rs), generalized from i64 ticks
//! to exact `Rational` sums per ADR-007.

use ove_time::Rational;

use crate::{Clip, ClipId, TimelineError, TrackOps, TICK_DEN};

type Link = Option<Box<Node>>;

struct Node {
    clip: Clip,
    /// Exact sum of durations of this node's subtree (incl. itself).
    sum: Rational,
    cnt: u32,
    h: i32,
    l: Link,
    r: Link,
}

fn h(n: &Link) -> i32 {
    n.as_ref().map_or(0, |n| n.h)
}
fn sum(n: &Link) -> Rational {
    n.as_ref()
        .map_or_else(|| Rational::new(0, TICK_DEN), |n| n.sum)
}
fn cnt(n: &Link) -> u32 {
    n.as_ref().map_or(0, |n| n.cnt)
}
/// Number of clips in the left subtree (for rank-based addressing).
fn lc(n: &Node) -> usize {
    n.l.as_ref().map_or(0, |x| x.cnt) as usize
}

fn upd(n: &mut Node) {
    n.sum = n.clip.duration.add(sum(&n.l)).add(sum(&n.r));
    n.cnt = 1 + cnt(&n.l) + cnt(&n.r);
    n.h = 1 + h(&n.l).max(h(&n.r));
}

fn rot_right(mut a: Box<Node>) -> Box<Node> {
    let mut b = a.l.take().unwrap();
    a.l = b.r.take();
    upd(&mut a);
    b.r = Some(a);
    upd(&mut b);
    b
}

fn rot_left(mut a: Box<Node>) -> Box<Node> {
    let mut b = a.r.take().unwrap();
    a.r = b.l.take();
    upd(&mut a);
    b.l = Some(a);
    upd(&mut b);
    b
}

fn balance(mut n: Box<Node>) -> Box<Node> {
    upd(&mut n);
    let b = h(&n.l) - h(&n.r);
    if b > 1 {
        let l = n.l.as_ref().unwrap();
        if h(&l.l) < h(&l.r) {
            let ll = n.l.take().unwrap();
            n.l = Some(rot_left(ll));
        }
        return rot_right(n);
    }
    if b < -1 {
        let r = n.r.as_ref().unwrap();
        if h(&r.r) < h(&r.l) {
            let rr = n.r.take().unwrap();
            n.r = Some(rot_right(rr));
        }
        return rot_left(n);
    }
    n
}

fn insert_at(n: Link, idx: usize, clip: Clip) -> Box<Node> {
    match n {
        None => {
            let sum = clip.duration;
            Box::new(Node {
                clip,
                sum,
                cnt: 1,
                h: 1,
                l: None,
                r: None,
            })
        }
        Some(mut node) => {
            if idx <= lc(&node) {
                node.l = Some(insert_at(node.l.take(), idx, clip));
            } else {
                node.r = Some(insert_at(node.r.take(), idx - lc(&node) - 1, clip));
            }
            balance(node)
        }
    }
}

fn remove_min(mut n: Box<Node>) -> (Link, Box<Node>) {
    if n.l.is_none() {
        let r = n.r.take();
        (r, n)
    } else {
        let (l, m) = remove_min(n.l.take().unwrap());
        n.l = l;
        (Some(balance(n)), m)
    }
}

fn remove_at(n: Link, idx: usize) -> (Link, Box<Node>) {
    match n {
        None => panic!("avl remove out of bounds"),
        Some(mut node) => {
            if idx < lc(&node) {
                let (l, rem) = remove_at(node.l.take(), idx);
                node.l = l;
                (Some(balance(node)), rem)
            } else if idx == lc(&node) {
                match (node.l.take(), node.r.take()) {
                    (None, r) => (r, node),
                    (l, None) => (l, node),
                    (l, Some(r)) => {
                        let (rr, mut min) = remove_min(r);
                        min.l = l;
                        min.r = rr;
                        (Some(balance(min)), node)
                    }
                }
            } else {
                let (r, rem) = remove_at(node.r.take(), idx - lc(&node) - 1);
                node.r = r;
                (Some(balance(node)), rem)
            }
        }
    }
}

fn set_dur_at(n: Link, idx: usize, d: Rational) -> (Link, Rational) {
    match n {
        None => panic!("avl set_duration out of bounds"),
        Some(mut node) => {
            if idx < lc(&node) {
                let (l, old) = set_dur_at(node.l.take(), idx, d);
                node.l = l;
                (Some(balance(node)), old)
            } else if idx == lc(&node) {
                let old = node.clip.duration;
                node.clip.duration = d;
                (Some(balance(node)), old)
            } else {
                let (r, old) = set_dur_at(node.r.take(), idx - lc(&node) - 1, d);
                node.r = r;
                (Some(balance(node)), old)
            }
        }
    }
}

pub struct AvlTrack {
    root: Link,
    n: usize,
}

impl AvlTrack {
    pub fn new() -> Self {
        AvlTrack { root: None, n: 0 }
    }

    pub fn from_clips(clips: Vec<Clip>) -> Self {
        let mut t = AvlTrack::new();
        for (i, c) in clips.into_iter().enumerate() {
            t.root = Some(insert_at(t.root.take(), i, c));
            t.n += 1;
        }
        t
    }

    fn get_at(&self, mut idx: usize) -> &Clip {
        let mut node = self.root.as_deref().expect("avl get in bounds");
        loop {
            let l = lc(node);
            if idx < l {
                node = node.l.as_deref().unwrap();
            } else if idx == l {
                return &node.clip;
            } else {
                idx -= l + 1;
                node = node.r.as_deref().unwrap();
            }
        }
    }

    /// Weighted descent — O(log n) exact rational comparisons and adds on
    /// cached subtree sums. NO rebuild after edits (the gap design's tax).
    fn hit_test_impl(&self, t: Rational) -> Option<usize> {
        let mut node = self.root.as_deref()?;
        let mut acc = Rational::new(0, TICK_DEN);
        let mut idx = 0usize;
        loop {
            let ls = sum(&node.l);
            let start = acc.add(ls);
            if t < start {
                node = node.l.as_deref()?;
            } else {
                let end = start.add(node.clip.duration);
                if t < end {
                    return Some(idx + lc(node));
                }
                acc = end;
                idx += lc(node) + 1;
                node = node.r.as_deref()?;
            }
        }
    }
}

impl Default for AvlTrack {
    fn default() -> Self {
        Self::new()
    }
}

impl TrackOps for AvlTrack {
    fn len(&self) -> usize {
        self.n
    }

    fn clip_at(&self, pos: usize) -> Option<&Clip> {
        if pos >= self.n {
            None
        } else {
            Some(self.get_at(pos))
        }
    }

    /// O(n) in-order id scan (documented TrackOps policy; keeps the
    /// comparison with GapTrack honest — both pay the same).
    fn index_of(&self, id: ClipId) -> Option<usize> {
        let mut found = None;
        let mut stack: Vec<&Node> = Vec::new();
        let mut cur = self.root.as_deref();
        let mut pos = 0usize;
        while cur.is_some() || !stack.is_empty() {
            while let Some(nd) = cur {
                stack.push(nd);
                cur = nd.l.as_deref();
            }
            let nd = stack.pop().unwrap();
            if found.is_none() && nd.clip.id == id {
                found = Some(pos);
            }
            pos += 1;
            cur = nd.r.as_deref();
        }
        found
    }

    fn hit_test(&self, t: Rational) -> Option<usize> {
        self.hit_test_impl(t)
    }

    fn insert_at(&mut self, pos: usize, clip: Clip) -> Result<(), TimelineError> {
        if pos > self.n {
            return Err(TimelineError::IndexOutOfBounds {
                index: pos,
                len: self.n,
            });
        }
        self.root = Some(insert_at(self.root.take(), pos, clip));
        self.n += 1;
        Ok(())
    }

    fn remove_at(&mut self, pos: usize) -> Result<Clip, TimelineError> {
        if pos >= self.n {
            return Err(TimelineError::IndexOutOfBounds {
                index: pos,
                len: self.n,
            });
        }
        let (root, node) = remove_at(self.root.take(), pos);
        self.root = root;
        self.n -= 1;
        Ok(node.clip)
    }

    fn set_duration_at(&mut self, pos: usize, d: Rational) -> Result<Rational, TimelineError> {
        if pos >= self.n {
            return Err(TimelineError::IndexOutOfBounds {
                index: pos,
                len: self.n,
            });
        }
        let (root, old) = set_dur_at(self.root.take(), pos, d);
        self.root = root;
        // Duration change alters sums but not positions: id map stays valid.
        Ok(old)
    }

    fn walk(&self, f: &mut dyn FnMut(usize, Rational, &Clip)) {
        let mut stack: Vec<&Node> = Vec::new();
        let mut cur = self.root.as_deref();
        let mut acc = Rational::new(0, TICK_DEN);
        let mut pos = 0usize;
        while cur.is_some() || !stack.is_empty() {
            while let Some(nd) = cur {
                stack.push(nd);
                cur = nd.l.as_deref();
            }
            let nd = stack.pop().unwrap();
            f(pos, acc, &nd.clip);
            pos += 1;
            acc = acc.add(nd.clip.duration);
            cur = nd.r.as_deref();
        }
    }
}

impl Clone for AvlTrack {
    fn clone(&self) -> Self {
        let mut clips = Vec::with_capacity(self.n);
        self.walk(&mut |_pos, _start, c| clips.push(c.clone()));
        AvlTrack::from_clips(clips)
    }
}
