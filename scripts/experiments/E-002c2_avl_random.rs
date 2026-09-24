// E-002c: timeline PRIMARY STRUCTURE benchmark.
// Question (from 41_EXPERIMENTS): gap-buffer vs BTreeMap vs piece-table vs Vec vs
// augmented order-statistic tree — which primary per-track clip structure survives
// the real edit-verb mix (build / split / ripple-resize / move / render-walk /
// position-lookup) at N=20k clips with RANDOM edit positions?
//
// Design notes:
// - Time units are i64 TICKS (1/24000 s). Rational-math cost was measured in
//   E-002/E-002b (C-001). This experiment isolates CONTAINER cost only.
// - Semantics: a track is an ordered clip sequence. Absolute start is STORED (families
//   A/B/C) or DERIVED (families D/E):
//     A Vec<ClipAbs>          stores start          (E-002b baseline)
//     B GapBuffer<ClipAbs>    stores start          (localized-edit hypothesis)
//     C BTreeMap<start, ..>   start IS the key      (absolute-start family)
//     D PieceTable            immutable records + piece list; starts = prefix sums
//     E Augmented AVL tree    implicit order key + subtree count + subtree dur sum
// - Op sequence is IDENTICAL across candidates (pre-generated from one seeded RNG);
//   final content is cross-validated by hash. Single-run indicative timings on
//   container CPU — compare RELATIVE order, not absolute numbers.
use std::collections::BTreeMap;
use std::time::Instant;

const N: usize = 20_000;
const SPLITS: usize = 5_000;
const RESIZES: usize = 2_000;
const MOVES: usize = 2_000;
const LOOKUPS: usize = 2_000;

// ---------- shared xorshift ----------
struct Rng(u64);
impl Rng {
    fn new(seed: u64) -> Self { Rng(seed ^ 0x9E3779B97F4A7C15) }
    fn next(&mut self) -> u64 {
        self.0 ^= self.0 << 13; self.0 ^= self.0 >> 7; self.0 ^= self.0 << 17; self.0
    }
    fn below(&mut self, n: usize) -> usize { (self.next() % n as u64) as usize }
}

const MIN_DUR: i64 = 480; // 20 ms at 24 kHz ticks

// ---------- A: Vec<ClipAbs> ----------
#[derive(Clone, Copy)]
struct ClipAbs { start: i64, dur: i64, id: u32 }

struct TrackVec { v: Vec<ClipAbs> }

impl TrackVec {
    fn build(durs: &[i64]) -> Self {
        let mut v = Vec::with_capacity(durs.len()); let mut s = 0i64;
        for (i, &d) in durs.iter().enumerate() { v.push(ClipAbs { start: s, dur: d, id: i as u32 }); s += d; }
        TrackVec { v }
    }
    fn split(&mut self, i: usize) {
        let c = self.v[i]; let half = c.dur / 2;
        self.v[i].dur = half;
        self.v.insert(i + 1, ClipAbs { start: c.start + half, dur: c.dur - half, id: c.id });
    }
    fn resize(&mut self, i: usize, new_dur: i64) {
        let delta = new_dur - self.v[i].dur; self.v[i].dur = new_dur;
        for c in self.v[i + 1..].iter_mut() { c.start += delta; } // O(k) ripple
    }
    fn mv(&mut self, i: usize, j: usize) {
        if i == j { return; }
        let c = self.v.remove(i);
        let jj = if j > i { j - 1 } else { j };
        self.v.insert(jj, ClipAbs { start: 0, dur: c.dur, id: c.id });
        let (lo, hi) = (i.min(jj), i.max(jj)); // starts change only inside [lo..=hi]
        let mut s = if lo == 0 { 0 } else { self.v[lo - 1].start + self.v[lo - 1].dur };
        for c2 in self.v[lo..=hi].iter_mut() { c2.start = s; s += c2.dur; }
    }
    fn walk(&self, out: &mut u64) { for c in &self.v { *out ^= c.start as u64; } }
    fn lookup(&self, p: i64) -> usize { self.v.partition_point(|c| c.start <= p).saturating_sub(1) }
}

// ---------- B: GapBuffer<ClipAbs> ----------
const ZERO_CLIP: ClipAbs = ClipAbs { start: 0, dur: 0, id: u32::MAX };

struct GapBuffer { buf: Vec<ClipAbs>, gap: usize, gap_len: usize }

impl GapBuffer {
    fn with_capacity(n: usize) -> Self {
        GapBuffer { buf: vec![ZERO_CLIP; n], gap: 0, gap_len: n }
    }
    fn len(&self) -> usize { self.buf.len() - self.gap_len }
    fn phys(&self, i: usize) -> usize { if i < self.gap { i } else { i + self.gap_len } }
    fn move_gap(&mut self, to: usize) {
        if to == self.gap { return; }
        if to < self.gap {
            self.buf.copy_within(to..self.gap, to + self.gap_len);
        } else {
            let src = self.gap + self.gap_len;
            self.buf.copy_within(src..src + (to - self.gap), self.gap);
        }
        self.gap = to;
    }
    fn grow(&mut self) { // gap_len == 0
        let n = self.buf.len();
        let mut nb = vec![ZERO_CLIP; n * 2];
        nb[..self.gap].copy_from_slice(&self.buf[..self.gap]);
        let suffix = n - self.gap;
        nb[n * 2 - suffix..].copy_from_slice(&self.buf[self.gap..n]);
        self.buf = nb; self.gap_len = n * 2 - n;
    }
    fn insert(&mut self, i: usize, c: ClipAbs) {
        if self.gap_len == 0 { self.grow(); }
        self.move_gap(i);
        self.buf[self.gap] = c; self.gap += 1; self.gap_len -= 1;
    }
    fn remove(&mut self, i: usize) -> ClipAbs {
        self.move_gap(i);
        let c = self.buf[self.gap + self.gap_len]; // element just right of the hole
        self.gap_len += 1;
        c
    }
    fn get(&self, i: usize) -> ClipAbs { self.buf[self.phys(i)] }
    fn set(&mut self, i: usize, c: ClipAbs) { let p = self.phys(i); self.buf[p] = c; }
}

struct TrackGap { g: GapBuffer }

impl TrackGap {
    fn build(durs: &[i64]) -> Self {
        let mut t = TrackGap { g: GapBuffer::with_capacity(durs.len() * 2 + 64) };
        let mut s = 0i64;
        for (i, &d) in durs.iter().enumerate() {
            t.g.insert(i, ClipAbs { start: s, dur: d, id: i as u32 }); s += d;
        }
        t
    }
    fn split(&mut self, i: usize) {
        let c = self.g.get(i); let half = c.dur / 2;
        self.g.set(i, ClipAbs { start: c.start, dur: half, id: c.id });
        self.g.insert(i + 1, ClipAbs { start: c.start + half, dur: c.dur - half, id: c.id });
    }
    fn resize(&mut self, i: usize, new_dur: i64) {
        let delta = new_dur - self.g.get(i).dur;
        let mut c = self.g.get(i); c.dur = new_dur; self.g.set(i, c);
        for j in i + 1..self.g.len() {
            let mut c = self.g.get(j); c.start += delta; self.g.set(j, c);
        }
    }
    fn mv(&mut self, i: usize, j: usize) {
        if i == j { return; }
        let c = self.g.remove(i);
        let jj = if j > i { j - 1 } else { j };
        self.g.insert(jj, ClipAbs { start: 0, dur: c.dur, id: c.id });
        let (lo, hi) = (i.min(jj), i.max(jj));
        let mut s = if lo == 0 { 0 } else { self.g.get(lo - 1).start + self.g.get(lo - 1).dur };
        for k in lo..=hi { let mut c2 = self.g.get(k); c2.start = s; s += c2.dur; self.g.set(k, c2); }
    }
    fn walk(&self, out: &mut u64) { for i in 0..self.g.len() { *out ^= self.g.get(i).start as u64; } }
    fn lookup(&self, p: i64) -> usize {
        let (mut lo, mut hi) = (0usize, self.g.len());
        while lo < hi {
            let mid = (lo + hi) / 2;
            if self.g.get(mid).start <= p { lo = mid + 1 } else { hi = mid }
        }
        lo.saturating_sub(1)
    }
}

// ---------- C: BTreeMap<start, (dur, id)> ----------
struct TrackBTree { m: BTreeMap<i64, (i64, u32)> }

impl TrackBTree {
    fn build(durs: &[i64]) -> Self {
        let mut m = BTreeMap::new(); let mut s = 0i64;
        for (i, &d) in durs.iter().enumerate() { m.insert(s, (d, i as u32)); s += d; }
        TrackBTree { m }
    }
    fn rank_key(&self, i: usize) -> i64 { self.m.keys().nth(i).copied().unwrap() } // O(i): no order statistic
    fn split(&mut self, i: usize) {
        let key = self.rank_key(i);
        let (dur, id) = self.m[&key]; let half = dur / 2;
        self.m.remove(&key);
        self.m.insert(key, (half, id));
        self.m.insert(key + half, (dur - half, id));
    }
    fn resize(&mut self, i: usize, new_dur: i64) {
        let key = self.rank_key(i);
        let (dur, id) = self.m[&key];
        let delta = new_dur - dur;
        self.m.insert(key, (new_dur, id));
        if delta != 0 {
            // absolute-start family: every subsequent start must be REKEYED (O(k log n)).
            // LESSON (found by this experiment's own first run): single-pass
            // remove(k);insert(k+delta) collides when k+delta lands exactly on an
            // existing key (e.g. `key` itself) and silently OVERWRITES it — losing a
            // clip. Two-phase (remove all, then insert shifted) is mandatory.
            let entries: Vec<(i64, (i64, u32))> = self.m.range(key + 1..).map(|(k, v)| (*k, *v)).collect();
            for (k, _) in &entries { self.m.remove(k); }
            for (k, v) in entries { self.m.insert(k + delta, v); }
        }
    }
    fn mv(&mut self, i: usize, j: usize) {
        if i == j { return; }
        // Measured via full-order snapshot + rebuild. A window-limited rekey is at most
        // ~3x cheaper for i<j moves but is the same O(n) complexity class — the honest
        // conclusion (absolute-start pays O(n) per random move) is unchanged.
        let mut items: Vec<(i64, i64, u32)> = self.m.iter().map(|(k, v)| (*k, v.0, v.1)).collect();
        let removed = items.remove(i);
        let jj = if j > i { j - 1 } else { j };
        items.insert(jj, (0, removed.1, removed.2));
        let (lo, hi) = (i.min(jj), i.max(jj));
        let mut s = if lo == 0 { 0 } else { items[lo - 1].0 + items[lo - 1].1 };
        for it in items[lo..=hi].iter_mut() { it.0 = s; s += it.1; }
        self.m.clear();
        for (st, d, id) in items { self.m.insert(st, (d, id)); }
    }
    fn walk(&self, out: &mut u64) { for (k, _) in &self.m { *out ^= *k as u64; } }
    fn lookup(&self, p: i64) -> i64 { *self.m.range(..=p).next_back().map(|(k, _)| k).unwrap_or(&0) }
}

// ---------- D: PieceTable (immutable records + piece list) ----------
#[derive(Clone, Copy)]
struct Rec { dur: i64, id: u32 }
#[derive(Clone, Copy)]
struct Piece { table: u8, idx: u32 } // 0=base, 1=added

struct TrackPiece { base: Vec<Rec>, added: Vec<Rec>, pieces: Vec<Piece> }

impl TrackPiece {
    fn build(durs: &[i64]) -> Self {
        let base: Vec<Rec> = durs.iter().enumerate().map(|(i, &d)| Rec { dur: d, id: i as u32 }).collect();
        let pieces: Vec<Piece> = (0..base.len()).map(|i| Piece { table: 0, idx: i as u32 }).collect();
        TrackPiece { base, added: Vec::new(), pieces }
    }
    fn rec(&self, p: Piece) -> Rec {
        if p.table == 0 { self.base[p.idx as usize] } else { self.added[p.idx as usize] }
    }
    fn split(&mut self, i: usize) {
        let r = self.rec(self.pieces[i]); let half = r.dur / 2;
        self.added.push(Rec { dur: half, id: r.id });
        self.added.push(Rec { dur: r.dur - half, id: r.id });
        let a = self.added.len() as u32;
        self.pieces[i] = Piece { table: 1, idx: a - 2 };
        self.pieces.insert(i + 1, Piece { table: 1, idx: a - 1 });
    }
    fn resize(&mut self, i: usize, new_dur: i64) { // O(1): old record kept (free undo!)
        let id = self.rec(self.pieces[i]).id;
        self.added.push(Rec { dur: new_dur, id });
        let a = self.added.len() as u32;
        self.pieces[i] = Piece { table: 1, idx: a - 1 };
    }
    fn mv(&mut self, i: usize, j: usize) {
        if i == j { return; }
        let p = self.pieces.remove(i);
        let jj = if j > i { j - 1 } else { j };
        self.pieces.insert(jj, p);
    }
    fn walk(&self, out: &mut u64) {
        let mut s = 0i64;
        for p in &self.pieces { let r = self.rec(*p); *out ^= s as u64; s += r.dur; }
    }
    fn lookup(&self, p: i64) -> usize { // O(n) prefix walk — the piece-table tax
        let mut s = 0i64;
        for (i, pi) in self.pieces.iter().enumerate() {
            let r = self.rec(*pi);
            if s + r.dur > p { return i; }
            s += r.dur;
        }
        self.pieces.len().saturating_sub(1)
    }
}

// ---------- E: augmented order-statistic AVL (cnt + dur-sum) ----------
#[derive(Clone)]
struct Node { dur: i64, id: u32, sum: i64, cnt: u32, h: i32, l: Link, r: Link }
type Link = Option<Box<Node>>;

fn h(n: &Link) -> i32 { n.as_ref().map_or(0, |n| n.h) }
fn sum(n: &Link) -> i64 { n.as_ref().map_or(0, |n| n.sum) }
fn cnt(n: &Link) -> u32 { n.as_ref().map_or(0, |n| n.cnt) }
fn lc(n: &Box<Node>) -> usize { n.l.as_ref().map_or(0, |x| x.cnt) as usize }

fn new_node(dur: i64, id: u32) -> Box<Node> {
    Box::new(Node { dur, id, sum: dur, cnt: 1, h: 1, l: None, r: None })
}
fn upd(n: &mut Box<Node>) {
    n.sum = n.dur + sum(&n.l) + sum(&n.r);
    n.cnt = 1 + cnt(&n.l) + cnt(&n.r);
    n.h = 1 + h(&n.l).max(h(&n.r));
}
fn rot_right(mut a: Box<Node>) -> Box<Node> { // left child moves up
    let mut b = a.l.take().unwrap();
    a.l = b.r.take(); upd(&mut a);
    b.r = Some(a); upd(&mut b);
    b
}
fn rot_left(mut a: Box<Node>) -> Box<Node> { // right child moves up
    let mut b = a.r.take().unwrap();
    a.r = b.l.take(); upd(&mut a);
    b.l = Some(a); upd(&mut b);
    b
}
fn balance(mut n: Box<Node>) -> Box<Node> {
    upd(&mut n);
    let b = h(&n.l) - h(&n.r);
    if b > 1 {
        let l = n.l.as_ref().unwrap();
        if h(&l.l) < h(&l.r) { let ll = n.l.take().unwrap(); n.l = Some(rot_left(ll)); }
        return rot_right(n);
    }
    if b < -1 {
        let r = n.r.as_ref().unwrap();
        if h(&r.r) < h(&r.l) { let rr = n.r.take().unwrap(); n.r = Some(rot_right(rr)); }
        return rot_left(n);
    }
    n
}
fn insert_at(n: Link, idx: usize, dur: i64, id: u32) -> Box<Node> {
    match n {
        None => new_node(dur, id),
        Some(mut node) => {
            if idx <= lc(&node) { node.l = Some(insert_at(node.l.take(), idx, dur, id)); }
            else { node.r = Some(insert_at(node.r.take(), idx - lc(&node) - 1, dur, id)); }
            balance(node)
        }
    }
}
fn remove_min(mut n: Box<Node>) -> (Link, Box<Node>) {
    if n.l.is_none() { let r = n.r.take(); (r, n) }
    else { let (l, m) = remove_min(n.l.take().unwrap()); n.l = l; (Some(balance(n)), m) }
}
fn remove_at(n: Link, idx: usize) -> (Link, Box<Node>) {
    match n {
        None => panic!("remove oob"),
        Some(mut node) => {
            if idx < lc(&node) {
                let (l, rem) = remove_at(node.l.take(), idx);
                node.l = l; (Some(balance(node)), rem)
            } else if idx == lc(&node) {
                match (node.l.take(), node.r.take()) {
                    (None, r) => (r, node),
                    (l, None) => (l, node),
                    (l, Some(r)) => {
                        let (rr, mut min) = remove_min(r);
                        min.l = l; min.r = rr;
                        (Some(balance(min)), node)
                    }
                }
            } else {
                let (r, rem) = remove_at(node.r.take(), idx - lc(&node) - 1);
                node.r = r; (Some(balance(node)), rem)
            }
        }
    }
}
fn update_dur_at(n: Link, idx: usize, new_dur: i64) -> Box<Node> {
    match n {
        None => panic!("update oob"),
        Some(mut node) => {
            if idx < lc(&node) { node.l = Some(update_dur_at(node.l.take(), idx, new_dur)); }
            else if idx == lc(&node) { node.dur = new_dur; }
            else { node.r = Some(update_dur_at(node.r.take(), idx - lc(&node) - 1, new_dur)); }
            balance(node)
        }
    }
}

struct TrackTree { root: Link }

impl TrackTree {
    fn build(durs: &[i64]) -> Self {
        let mut root: Link = None;
        for (i, &d) in durs.iter().enumerate() { root = Some(insert_at(root, i, d, i as u32)); }
        TrackTree { root }
    }
    fn get_at(&self, idx: usize) -> (i64, u32) {
        let mut node = self.root.as_deref().unwrap();
        let mut idx = idx;
        loop {
            let l = node.l.as_ref().map_or(0, |x| x.cnt) as usize;
            if idx < l { node = node.l.as_deref().unwrap(); }
            else if idx == l { return (node.dur, node.id); }
            else { idx -= l + 1; node = node.r.as_deref().unwrap(); }
        }
    }
    fn split(&mut self, i: usize) {
        let (dur, id) = self.get_at(i);
        self.root = Some(update_dur_at(self.root.take(), i, dur / 2));
        self.root = Some(insert_at(self.root.take(), i + 1, dur - dur / 2, id));
    }
    fn take_node(&mut self, i: usize) -> (i64, u32) {
        let (root, rem) = remove_at(self.root.take(), i);
        self.root = root; (rem.dur, rem.id)
    }
    fn resize(&mut self, i: usize, new_dur: i64) { // O(log n): no ripple, starts derived
        self.root = Some(update_dur_at(self.root.take(), i, new_dur));
    }
    fn mv(&mut self, i: usize, j: usize) {
        if i == j { return; }
        let (dur, id) = self.take_node(i);
        let jj = if j > i { j - 1 } else { j };
        self.root = Some(insert_at(self.root.take(), jj, dur, id));
    }
    fn walk(&self, out: &mut u64) {
        let mut stack: Vec<&Node> = Vec::new();
        let mut cur = self.root.as_deref();
        let mut s = 0i64;
        while cur.is_some() || !stack.is_empty() {
            while let Some(nd) = cur { stack.push(nd); cur = nd.l.as_deref(); }
            let nd = stack.pop().unwrap();
            *out ^= s as u64; s += nd.dur;
            cur = nd.r.as_deref();
        }
    }
    fn lookup(&self, p: i64) -> usize { // weighted descend, O(log n)
        let mut node = self.root.as_deref();
        let mut acc = 0i64; let mut idx = 0usize;
        while let Some(nd) = node {
            let ls = sum(&nd.l);
            let start = acc + ls;
            if p < start { node = nd.l.as_deref(); }
            else if p < start + nd.dur { return idx + nd.l.as_ref().map_or(0, |n| n.cnt) as usize; }
            else { acc = start + nd.dur; idx += nd.l.as_ref().map_or(0, |n| n.cnt) as usize + 1; node = nd.r.as_deref(); }
        }
        panic!("lookup oob")
    }
}

// ---------- driver ----------
struct Timing { name: &'static str, build: f64, split: f64, resize: f64, mv: f64, walk: f64, lookup: f64, per_clip: usize, hash: u64, total: i64, clips: usize }

fn ms(t: Instant) -> f64 { t.elapsed().as_secs_f64() * 1000.0 }

fn main() {
    // shared op script (identical across candidates)
    let mut rng = Rng::new(0x5EED_C0DE_2026_0923);
    let durs: Vec<i64> = (0..N).map(|_| (1001 + rng.next() % 240_000) as i64).collect();
    let mut shadow = durs.clone();
    let splits: Vec<usize> = (0..SPLITS).map(|_| { let i = rng.below(shadow.len()); let d = shadow[i]; shadow[i] = d / 2; shadow.insert(i + 1, d - d / 2); i }).collect();
    let resizes: Vec<(usize, i64)> = (0..RESIZES).map(|_| {
        let i = rng.below(shadow.len());
        let nd = MIN_DUR + (rng.next() % 240_000) as i64; shadow[i] = nd; (i, nd)
    }).collect();
    let moves: Vec<(usize, usize)> = (0..MOVES).map(|_| {
        let i = rng.below(shadow.len()); let j = rng.below(shadow.len());
        let c = shadow.remove(i); let jj = if j > i { j - 1 } else { j }; shadow.insert(jj, c); (i, j)
    }).collect();
    let final_total: i64 = shadow.iter().sum();
    let lookups: Vec<i64> = (0..LOOKUPS).map(|_| (rng.next() % (final_total as u64)) as i64).collect();
    println!("ops: N={} splits={} resizes={} moves={} lookups={} (RANDOM positions, shared script)", N, SPLITS, RESIZES, MOVES, LOOKUPS);
    println!("final_total={} ticks ({:.3} s)\n", final_total, final_total as f64 / 24_000.0);

    let mut results: Vec<Timing> = Vec::new();

    // --- A: Vec ---
    {
        let t0 = Instant::now(); let mut tr = TrackVec::build(&durs); let build = ms(t0);
        let t1 = Instant::now(); for &i in &splits { tr.split(i); } let split = ms(t1);
        let t2 = Instant::now(); for &(i, nd) in &resizes { tr.resize(i, nd); } let resize = ms(t2);
        let t3 = Instant::now(); for &(i, j) in &moves { tr.mv(i, j); } let mvv = ms(t3);
        let t4 = Instant::now(); let mut hash = 0u64; tr.walk(&mut hash); let walk = ms(t4);
        let t5 = Instant::now(); let mut lh = 0u64; for &p in &lookups { lh ^= tr.lookup(p) as u64; } let lookup = ms(t5);
        let total: i64 = tr.v.iter().map(|c| c.dur).sum();
        results.push(Timing { name: "A Vec<Clip>", build, split, resize, mv: mvv, walk, lookup, per_clip: 24, hash, total, clips: tr.v.len(), });
        let _ = lh;
    }
    // --- B: GapBuffer ---
    {
        let t0 = Instant::now(); let mut tr = TrackGap::build(&durs); let build = ms(t0);
        let t1 = Instant::now(); for &i in &splits { tr.split(i); } let split = ms(t1);
        let t2 = Instant::now(); for &(i, nd) in &resizes { tr.resize(i, nd); } let resize = ms(t2);
        let t3 = Instant::now(); for &(i, j) in &moves { tr.mv(i, j); } let mvv = ms(t3);
        let t4 = Instant::now(); let mut hash = 0u64; tr.walk(&mut hash); let walk = ms(t4);
        let t5 = Instant::now(); let mut lh = 0u64; for &p in &lookups { lh ^= tr.lookup(p) as u64; } let lookup = ms(t5);
        let mut total = 0i64; for i in 0..tr.g.len() { total += tr.g.get(i).dur; }
        results.push(Timing { name: "B GapBuffer", build, split, resize, mv: mvv, walk, lookup, per_clip: 48, hash, total, clips: tr.g.len() });
        let _ = lh;
    }
    // --- C: BTreeMap ---
    {
        let t0 = Instant::now(); let mut tr = TrackBTree::build(&durs); let build = ms(t0);
        let t1 = Instant::now(); for &i in &splits { tr.split(i); } let split = ms(t1);
        let t2 = Instant::now(); for &(i, nd) in &resizes { tr.resize(i, nd); } let resize = ms(t2);
        let t3 = Instant::now(); for &(i, j) in &moves { tr.mv(i, j); } let mvv = ms(t3);
        let t4 = Instant::now(); let mut hash = 0u64; tr.walk(&mut hash); let walk = ms(t4);
        let t5 = Instant::now(); let mut lh = 0u64; for &p in &lookups { lh ^= tr.lookup(p) as u64; } let lookup = ms(t5);
        let total: i64 = tr.m.values().map(|v| v.0).sum();
        results.push(Timing { name: "C BTreeMap", build, split, resize, mv: mvv, walk, lookup, per_clip: 64, hash, total, clips: tr.m.len() });
        let _ = lh;
    }
    // --- D: PieceTable ---
    {
        let t0 = Instant::now(); let mut tr = TrackPiece::build(&durs); let build = ms(t0);
        let t1 = Instant::now(); for &i in &splits { tr.split(i); } let split = ms(t1);
        let t2 = Instant::now(); for &(i, nd) in &resizes { tr.resize(i, nd); } let resize = ms(t2);
        let t3 = Instant::now(); for &(i, j) in &moves { tr.mv(i, j); } let mvv = ms(t3);
        let t4 = Instant::now(); let mut hash = 0u64; tr.walk(&mut hash); let walk = ms(t4);
        let t5 = Instant::now(); let mut lh = 0u64; for &p in &lookups { lh ^= tr.lookup(p) as u64; } let lookup = ms(t5);
        let mut total = 0i64; for p in &tr.pieces { total += tr.rec(*p).dur; }
        results.push(Timing { name: "D PieceTable", build, split, resize, mv: mvv, walk, lookup, per_clip: 24, hash, total, clips: tr.pieces.len() });
        let _ = lh;
    }
    // --- E: AVL tree ---
    {
        let t0 = Instant::now(); let mut tr = TrackTree::build(&durs); let build = ms(t0);
        let t1 = Instant::now(); for &i in &splits { tr.split(i); } let split = ms(t1);
        let t2 = Instant::now(); for &(i, nd) in &resizes { tr.resize(i, nd); } let resize = ms(t2);
        let t3 = Instant::now(); for &(i, j) in &moves { tr.mv(i, j); } let mvv = ms(t3);
        let t4 = Instant::now(); let mut hash = 0u64; tr.walk(&mut hash); let walk = ms(t4);
        let t5 = Instant::now(); let mut lh = 0u64; for &p in &lookups { lh ^= tr.lookup(p) as u64; } let lookup = ms(t5);
        let total = sum(&tr.root);
        results.push(Timing { name: "E AugAVL", build, split, resize, mv: mvv, walk, lookup, per_clip: 56, hash, total, clips: cnt(&tr.root) as usize });
        let _ = lh;
    }

    // cross-validation
    let (h0, t0v, c0) = (results[0].hash, results[0].total, results[0].clips);
    for r in &results {
        assert_eq!(r.hash, h0, "{} start-prefix hash mismatch", r.name);
        assert_eq!(r.total, t0v, "{} total-duration mismatch", r.name);
        assert_eq!(r.clips, c0, "{} clip-count mismatch", r.name);
    }
    println!("CROSS-VALIDATION: all 5 structures identical (hash/total/count)  OK\n");

    println!("{:<14} {:>9} {:>9} {:>9} {:>9} {:>9} {:>9} {:>8}", "structure", "build", "split", "resize", "move", "walk", "lookup", "B/clip");
    println!("{}", "-".repeat(84));
    for r in &results {
        println!("{:<14} {:>8.1}ms {:>8.1}ms {:>8.1}ms {:>8.1}ms {:>8.2}ms {:>8.2}ms {:>8}", r.name, r.build, r.split, r.resize, r.mv, r.walk, r.lookup, r.per_clip);
    }
    println!("\nall values ms, single run, container CPU — indicative relative order only");
    println!("NOTE: E-002b's split=45.6ms used front-clustered split positions (step_by(4)); this run");
    println!("uses RANDOM positions — the honest worst case for absolute-start families.");
}
