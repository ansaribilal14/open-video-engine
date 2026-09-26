//! Cursor buffer — the safe-Rust realization of ADR-011's gap-buffer decision.
//!
//! Two stacks facing each other at a cursor: `front` holds elements left of the
//! cursor in order; `back` holds elements right of the cursor REVERSED. Cursor-local
//! insert/remove are O(1); moving the cursor by k is O(k) (the gap-buffer move);
//! iteration is a contiguous front-to-back walk; random access by index is O(1).
//!
//! The timeline keeps each track's buffer SORTED by clip start, which lets the same
//! structure serve binary-search queries (the "derived ordered index" role of
//! ADR-011) without a second structure — see lib.rs crate docs.

/// Cursor-separated element buffer (gap-buffer class, ADR-011).
#[derive(Clone, Debug)]
pub struct CursorBuffer<T> {
    front: Vec<T>,
    back_rev: Vec<T>, // reversed right side: last element of the sequence = back_rev[0]
}

impl<T> CursorBuffer<T> {
    pub fn new() -> Self {
        CursorBuffer {
            front: Vec::new(),
            back_rev: Vec::new(),
        }
    }

    pub fn len(&self) -> usize {
        self.front.len() + self.back_rev.len()
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// Cursor position (index of the next insertion).
    pub fn cursor(&self) -> usize {
        self.front.len()
    }

    /// O(1) random access by sequence index.
    pub fn get(&self, i: usize) -> &T {
        if i < self.front.len() {
            &self.front[i]
        } else {
            &self.back_rev[self.back_rev.len() - 1 - (i - self.front.len())]
        }
    }

    /// Move the cursor to an absolute position (O(distance), the gap move).
    pub fn move_cursor_to(&mut self, pos: usize) {
        assert!(pos <= self.len(), "cursor position out of range");
        while self.front.len() < pos {
            let v = self
                .back_rev
                .pop()
                .expect("back nonempty when growing front");
            self.front.push(v);
        }
        while self.front.len() > pos {
            let v = self.front.pop().expect("front nonempty when shrinking");
            self.back_rev.push(v);
        }
    }

    /// Insert at the cursor (O(1) after positioning).
    pub fn insert_at_cursor(&mut self, v: T) {
        self.front.push(v);
    }

    /// Remove the element just left of the cursor (O(1) after positioning).
    /// Caller must guarantee the cursor is not at 0.
    pub fn remove_before_cursor(&mut self) -> T {
        self.front.pop().expect("cursor at 0: nothing to remove")
    }

    pub fn iter(&self) -> impl Iterator<Item = &T> {
        self.front.iter().chain(self.back_rev.iter().rev())
    }
}

impl<T> Default for CursorBuffer<T> {
    fn default() -> Self {
        Self::new()
    }
}

/// Binary search over the buffer by an ordering key, given a key extractor.
/// Mirrors `slice::binary_search_by` semantics but works through the cursor
/// structure with O(1) random access. The buffer MUST be sorted by the key.
pub fn binary_search_by_key<T, K: Ord, F: Fn(&T) -> K>(
    buf: &CursorBuffer<T>,
    key: &K,
    extract: F,
) -> Result<usize, usize> {
    let mut lo = 0usize;
    let mut hi = buf.len();
    while lo < hi {
        let mid = lo + (hi - lo) / 2;
        let mid_key = extract(buf.get(mid));
        match mid_key.cmp(key) {
            std::cmp::Ordering::Less => lo = mid + 1,
            std::cmp::Ordering::Equal => return Ok(mid),
            std::cmp::Ordering::Greater => hi = mid,
        }
    }
    Err(lo)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cursor_ops_and_iteration() {
        let mut b = CursorBuffer::new();
        for v in [1, 2, 3, 4, 5] {
            b.move_cursor_to(b.len());
            b.insert_at_cursor(v);
        }
        assert_eq!(b.iter().copied().collect::<Vec<_>>(), vec![1, 2, 3, 4, 5]);

        // remove middle (index 2, value 3): cursor to 3, remove_before
        b.move_cursor_to(3);
        assert_eq!(b.remove_before_cursor(), 3);
        assert_eq!(b.iter().copied().collect::<Vec<_>>(), vec![1, 2, 4, 5]);

        // insert at front
        b.move_cursor_to(0);
        b.insert_at_cursor(0);
        assert_eq!(b.iter().copied().collect::<Vec<_>>(), vec![0, 1, 2, 4, 5]);

        // O(1) random access across the cursor
        b.move_cursor_to(2);
        assert_eq!(b.get(0), &0);
        assert_eq!(b.get(4), &5);
        assert_eq!(b.get(2), &2);
    }

    #[test]
    fn binary_search_hits_and_gaps() {
        let mut b = CursorBuffer::new();
        for v in [10, 20, 30, 40] {
            b.move_cursor_to(b.len());
            b.insert_at_cursor(v);
        }
        assert_eq!(binary_search_by_key(&b, &30, |v| *v), Ok(2));
        assert_eq!(binary_search_by_key(&b, &25, |v| *v), Err(2));
        assert_eq!(binary_search_by_key(&b, &5, |v| *v), Err(0));
        assert_eq!(binary_search_by_key(&b, &50, |v| *v), Err(4));
    }
}
