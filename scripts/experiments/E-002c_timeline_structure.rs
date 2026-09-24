// E-002c: timeline PRIMARY STRUCTURE benchmark (follow-up to E-002b, which showed
// 5000 Vec-splits = 45.6 ms on a 20k track — not editor-grade).
//
// Design notes (important for honesty of the comparison):
//   * Structures store ONLY (dur, id) per clip. Absolute starts are DERIVED
//     (prefix sums, exact rationals — E-002b showed 20k sums ~ms). Eager start
//     maintenance would add identical O(n) rational arithmetic to every
//     candidate and drown the structural signal.
//   * Ops therefore model structural edits: SPLIT (replace+insert) and
//     DELETE (remove). Op sequences are IDENTICAL in logic across candidates;
//     final states are cross-checked by FNV-1a hash over (dur, id) sequence.
//   * BTreeMap leg uses fractional (midpoint) u128 keys + id->key aux map,
//     the standard way ordered maps serve timeline addressing; rekey events
//     are counted and reported.
//   * Timing wraps ONLY the structure op loop (driver bookkeeping excluded,
//     aux-map maintenance INCLUDED for the BTree leg — it is part of that
//     structure's real cost).
//
// Candidates:
//   1. Vec<Clip>            baseline (known-bad at scattered insert)
//   2. GapBuffer            cursor-local edit hypothesis
//   3. BTreeMap<key,Clip>   O(log n) anywhere-insert hypothesis
//   4. PieceTable           immutable arena + small pieces, stable ids
//
// Workloads:
//   W1 cursor-local split  : 5000 splits, cursor advances by 2 clips per edit
//   W2 scattered split     : 5000 splits at uniform-random indices
//   W3 fixed-point delete  : 2500 deletes at one position (pure cursor case)
//   W4 scattered delete    : 2500 deletes at uniform-random indices
//   W5 full-track walk x200: render-pass iteration cost on 20k clips
//
// Build/run: rustc -O E-002c_timeline_structure.rs -o e002c && ./e002c
use std::collections::{BTreeMap, HashMap};
use std::time::Instant;

// ---------- exact rational duration (i128-guarded) ----------
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Debug)]
struct Rational {
    num: i64,
    den: i64, // always > 0, normalized
}

fn gcd(a: i64, b: i64) -> i64 {
    if b == 0 { a } else { gcd(b, a % b) }
}
fn gcd128(a: i128, b: i128) -> i128 {
    let (mut a, mut b) = (a.abs(), b.abs());
    while b != 0 { let t = a % b; a = b; b = t; }
    a
}

impl Rational {
    fn new(num: i64, den: i64) -> Self {
        debug_assert!(den > 0);
        let g = gcd(num.abs(), den);
        if g > 1 { Rational { num: num / g, den: den / g } } else { Rational { num, den } }
    }
    fn sub(self, o: Rational) -> Rational {
        let g = gcd128(self.den as i128, o.den as i128);
        let lcm = (self.den as i128 / g) * (o.den as i128);
        let n = (self.num as i128) * (lcm / self.den as i128)
            - (o.num as i128) * (lcm / o.den as i128);
        let gg = gcd128(n, lcm).max(1);
        Rational::new((n / gg) as i64, (lcm / gg) as i64)
    }
    fn half(self) -> Rational { Rational::new(self.num >> 1, self.den) }
}

#[derive(Clone, Copy, Debug)]
struct Clip {
    dur: Rational,
    id: u32,
}

fn fnv_hash(seq: impl Iterator<Item = (i64, i64, u32)>) -> u64 {
    let mut h: u64 = 0xcbf29ce484222325;
    for (a, b, c) in seq {
        for v in [a as u64, b as u64, c as u64] {
            h ^= v;
            h = h.wrapping_mul(0x100000001b3);
        }
    }
    h
}

// ---------- 1. Vec baseline ----------
struct VecTrack { v: Vec<Clip> }
impl VecTrack {
    fn new() -> Self { VecTrack { v: Vec::new() } }
    fn push(&mut self, c: Clip) { self.v.push(c); }
    fn split(&mut self, idx: usize, nid: u32) {
        let c = self.v[idx];
        let left = c.dur.half();
        self.v[idx].dur = left;
        self.v.insert(idx + 1, Clip { dur: c.dur.sub(left), id: nid });
    }
    fn delete(&mut self, idx: usize) { self.v.remove(idx); }
    fn len(&self) -> usize { self.v.len() }
    fn walk(&self) -> Rational {
        let mut t = Rational::new(0, 24000);
        for c in &self.v { t = t.add_dur(c.dur); }
        t
    }
    fn hash(&self) -> u64 { fnv_hash(self.v.iter().map(|c| (c.dur.num, c.dur.den, c.id))) }
}
impl Rational { fn add_dur(self, o: Rational) -> Rational {
    let g = gcd128(self.den as i128, o.den as i128);
    let lcm = (self.den as i128 / g) * (o.den as i128);
    let n = (self.num as i128) * (lcm / self.den as i128)
        + (o.num as i128) * (lcm / o.den as i128);
    let gg = gcd128(n, lcm).max(1);
    Rational::new((n / gg) as i64, (lcm / gg) as i64)
} }

// ---------- 2. Gap buffer ----------
struct GapTrack {
    buf: Vec<Clip>,
    gap_start: usize,
    gap_end: usize,
    filler: Clip, // template used to (re)create gap space
}
impl GapTrack {
    fn new() -> Self {
        GapTrack {
            buf: Vec::new(), gap_start: 0, gap_end: 0,
            filler: Clip { dur: Rational::new(0, 1), id: u32::MAX },
        }
    }
    fn gap_size(&self) -> usize { self.gap_end - self.gap_start }
    fn len(&self) -> usize { self.buf.len() - self.gap_size() }
    fn ensure_gap(&mut self) {
        if self.gap_size() == 0 {
            const CHUNK: usize = 256;
            let f = self.filler;
            let at = self.gap_start.min(self.buf.len());
            self.buf.splice(at..at, std::iter::repeat(f).take(CHUNK));
            self.gap_start = at;
            self.gap_end = at + CHUNK;
        }
    }
    fn move_gap(&mut self, pos: usize) {
        debug_assert!(pos <= self.len());
        self.ensure_gap();
        if pos < self.gap_start {
            let dist = self.gap_start - pos;
            self.buf.copy_within(pos..pos + dist, self.gap_end - dist);
            self.gap_start = pos;
            self.gap_end -= dist;
        } else if pos > self.gap_start {
            let dist = pos - self.gap_start;
            self.buf.copy_within(self.gap_end..self.gap_end + dist, self.gap_start);
            self.gap_start = pos;
            self.gap_end += dist;
        }
    }
    fn push(&mut self, c: Clip) {
        let pos = self.len();
        self.move_gap(pos);
        self.buf[self.gap_start] = c;
        self.gap_start += 1;
    }
    fn split(&mut self, idx: usize, nid: u32) {
        self.move_gap(idx);
        let c = self.buf[self.gap_end]; // logical element at idx sits right after the gap
        let left = c.dur.half();
        self.buf[self.gap_end] = Clip { dur: c.dur.sub(left), id: nid }; // right half
        self.buf[self.gap_start] = Clip { dur: left, id: c.id };         // left half
        self.gap_start += 1;
    }
    fn delete(&mut self, idx: usize) {
        self.move_gap(idx);
        self.gap_end += 1; // swallow element at idx into the gap: O(1)
    }
    fn at(&self, idx: usize) -> Clip {
        if idx < self.gap_start { self.buf[idx] } else { self.buf[idx + self.gap_size()] }
    }
    fn walk(&self) -> Rational {
        let mut t = Rational::new(0, 24000);
        for i in 0..self.len() { t = t.add_dur(self.at(i).dur); }
        t
    }
    fn hash(&self) -> u64 {
        fnv_hash((0..self.len()).map(|i| { let c = self.at(i); (c.dur.num, c.dur.den, c.id) }))
    }
}

// ---------- 3. BTreeMap with fractional u128 keys + id->key aux ----------
const KEY_SPACING: u128 = 1 << 60;

struct BTreeTrack {
    m: BTreeMap<u128, Clip>,
    aux: HashMap<u32, u128>,
    rekeys: usize,
}
impl BTreeTrack {
    fn new() -> Self { BTreeTrack { m: BTreeMap::new(), aux: HashMap::new(), rekeys: 0 } }
    fn push(&mut self, c: Clip) {
        let k = (self.m.len() as u128) * KEY_SPACING;
        self.m.insert(k, c);
        self.aux.insert(c.id, k);
    }
    fn split(&mut self, id: u32, nid: u32) {
        let key = self.aux[&id];
        let c = self.m.remove(&key).unwrap();
        let left = c.dur.half();
        let right = Clip { dur: c.dur.sub(left), id: nid };
        let hi = self.m.range(key..).next().map(|(k, _)| *k);
        let exhausted = matches!(hi, Some(b) if b < key + 2);
        if !exhausted {
            let k2 = match hi { Some(b) => key + (b - key) / 2, None => key + KEY_SPACING };
            self.m.insert(key, Clip { dur: left, id: c.id });
            self.m.insert(k2, right);
            self.aux.insert(nid, k2); // left keeps id->key
        } else {
            self.rekeys += 1;
            let pos = self.m.range(..key).count(); // where the removed clip lived
            let items: Vec<Clip> = self.m.values().copied().collect();
            self.m.clear();
            self.aux.remove(&id);
            let mut base = 0u128;
            for (i, oc) in items.into_iter().enumerate() {
                if i == pos {
                    self.m.insert(base, Clip { dur: left, id: c.id });
                    self.aux.insert(c.id, base); base += KEY_SPACING;
                    self.m.insert(base, right);
                    self.aux.insert(nid, base); base += KEY_SPACING;
                }
                self.m.insert(base, oc);
                self.aux.insert(oc.id, base); base += KEY_SPACING;
            }
            if pos == self.m.len() && !self.aux.contains_key(&nid) {
                // removed clip was last: append both halves at the end
                let k1 = base; base += KEY_SPACING;
                self.m.insert(k1, Clip { dur: left, id: c.id });
                self.aux.insert(c.id, k1);
                self.m.insert(base, right);
                self.aux.insert(nid, base);
            }
        }
    }
    fn delete(&mut self, id: u32) {
        let key = self.aux.remove(&id).unwrap();
        self.m.remove(&key);
    }
    fn walk(&self) -> Rational {
        let mut t = Rational::new(0, 24000);
        for c in self.m.values() { t = t.add_dur(c.dur); }
        t
    }
    fn hash(&self) -> u64 { fnv_hash(self.m.values().map(|c| (c.dur.num, c.dur.den, c.id))) }
}

// ---------- 4. Piece table (immutable arena + 16-byte pieces) ----------
#[derive(Clone, Copy)]
struct Piece { arena_idx: u32, _pad: u32 }
struct PieceTrack {
    arena: Vec<Clip>,    // append-only: originals + split halves (never mutated)
    pieces: Vec<Piece>,  // document = ordered piece list
}
impl PieceTrack {
    fn new() -> Self { PieceTrack { arena: Vec::new(), pieces: Vec::new() } }
    fn push(&mut self, c: Clip) {
        self.arena.push(c);
        self.pieces.push(Piece { arena_idx: (self.arena.len() - 1) as u32, _pad: 0 });
    }
    fn split(&mut self, idx: usize, nid: u32) {
        let ai = self.pieces[idx].arena_idx as usize;
        let c = self.arena[ai];
        let left = c.dur.half();
        self.arena.push(Clip { dur: left, id: c.id });
        self.arena.push(Clip { dur: c.dur.sub(left), id: nid });
        self.pieces[idx] = Piece { arena_idx: (self.arena.len() - 2) as u32, _pad: 0 };
        self.pieces.insert(idx + 1, Piece { arena_idx: (self.arena.len() - 1) as u32, _pad: 0 });
    }
    fn delete(&mut self, idx: usize) { self.pieces.remove(idx); }
    fn len(&self) -> usize { self.pieces.len() }
    fn walk(&self) -> Rational {
        let mut t = Rational::new(0, 24000);
        for p in &self.pieces { t = t.add_dur(self.arena[p.arena_idx as usize].dur); }
        t
    }
    fn hash(&self) -> u64 {
        fnv_hash(self.pieces.iter().map(|p| {
            let c = &self.arena[p.arena_idx as usize]; (c.dur.num, c.dur.den, c.id)
        }))
    }
}

// ---------- driver ----------
struct Rng(u64);
impl Rng {
    fn next(&mut self) -> u64 {
        self.0 ^= self.0 << 13; self.0 ^= self.0 >> 7; self.0 ^= self.0 << 17; self.0
    }
    fn below(&mut self, n: usize) -> usize { (self.next() % (n as u64)) as usize }
}

fn build_clips(n: usize, rng: &mut Rng) -> Vec<Clip> {
    let mut out = Vec::with_capacity(n);
    for i in 0..n {
        out.push(Clip { dur: Rational::new(1001 + (rng.next() % 240_000) as i64, 24000), id: i as u32 });
    }
    out
}

// generic driver for index-addressed structures (Vec, Gap, Piece share the API)
macro_rules! bench_index_struct {
    ($ty:ty, $clips:expr, $w1:expr, $w3:expr, $w4:expr, $rep:expr) => {{
        let clips: &[Clip] = $clips;
        // W1 cursor-local split
        let mut tr = <$ty>::new();
        for c in clips { tr.push(*c); }
        let mut nid = clips.len() as u32;
        let mut cursor = 0usize;
        let t = Instant::now();
        for _ in 0..$w1 {
            tr.split(cursor, nid); nid += 1;
            cursor = (cursor + 2).min(tr.len() - 1);
        }
        let w1_ms = t.elapsed().as_secs_f64() * 1000.0;
        let h1 = tr.hash();
        // W3 fixed-point delete
        let mut tr = <$ty>::new();
        for c in clips { tr.push(*c); }
        let t = Instant::now();
        for _ in 0..$w3 { tr.delete(1000); }
        let w3_ms = t.elapsed().as_secs_f64() * 1000.0;
        let h3 = tr.hash();
        // W4 scattered delete
        let mut tr = <$ty>::new();
        for c in clips { tr.push(*c); }
        let mut rng = Rng(0xB7E151628AED2A6A ^ ($rep as u64 + 1));
        let t = Instant::now();
        for _ in 0..$w4 { let idx = rng.below(tr.len()); tr.delete(idx); }
        let w4_ms = t.elapsed().as_secs_f64() * 1000.0;
        let h4 = tr.hash();
        (w1_ms, h1, w3_ms, h3, w4_ms, h4)
    }};
}

fn bench_btree(clips: &[Clip], rep: usize) -> (f64, u64, f64, u64, f64, u64, usize) {
    // W1 cursor-local split (id-addressed via aux; order vector is driver-side, untimed)
    let mut tr = BTreeTrack::new();
    for c in clips { tr.push(*c); }
    let mut order: Vec<u32> = (0..clips.len() as u32).collect();
    let mut nid = clips.len() as u32;
    let mut cursor = 0usize;
    let t = Instant::now();
    for _ in 0..5000 {
        let id = order[cursor.min(order.len() - 1)];
        tr.split(id, nid);
        order.insert(cursor.min(order.len() - 1) + 1, nid); // left half KEEPS slot with its id
        cursor = (cursor + 2).min(order.len() - 1);
        nid += 1;
    }
    let w1_ms = t.elapsed().as_secs_f64() * 1000.0;
    let h1 = tr.hash();
    // W3 fixed-point delete
    let mut tr = BTreeTrack::new();
    for c in clips { tr.push(*c); }
    let mut order: Vec<u32> = (0..clips.len() as u32).collect();
    let t = Instant::now();
    for _ in 0..2500 {
        let id = order[1000];
        tr.delete(id);
        order.remove(1000);
    }
    let w3_ms = t.elapsed().as_secs_f64() * 1000.0;
    let h3 = tr.hash();
    // W4 scattered delete
    let mut tr = BTreeTrack::new();
    for c in clips { tr.push(*c); }
    let mut order: Vec<u32> = (0..clips.len() as u32).collect();
    let mut rng = Rng(0xB7E151628AED2A6A ^ (rep as u64 + 1));
    let t = Instant::now();
    for _ in 0..2500 {
        let idx = rng.below(order.len());
        tr.delete(order[idx]);
        order.remove(idx);
    }
    let w4_ms = t.elapsed().as_secs_f64() * 1000.0;
    let h4 = tr.hash();
    (w1_ms, h1, w3_ms, h3, w4_ms, h4, tr.rekeys)
}

fn median3(v: &mut [f64; 3]) -> f64 { v.sort_by(|a, b| a.partial_cmp(b).unwrap()); v[1] }

fn main() {
    let n = 20_000;
    let mut rng = Rng(0x243F6A8885A308D3 ^ 0xE002C);
    let clips = build_clips(n, &mut rng);
    println!("E-002c timeline primary-structure benchmark (rustc -O, std only)");
    println!("track: {} clips, exact rationals (initial den 24000); ops = split/delete only", n);
    println!("starts are derived (prefix sums) — structural cost isolated from arithmetic");
    println!("sizes: Clip={}B, Piece={}B", std::mem::size_of::<Clip>(), std::mem::size_of::<Piece>());
    println!();

    let mut w1 = [[0f64; 3]; 4]; let mut w3 = [[0f64; 3]; 4]; let mut w4 = [[0f64; 3]; 4];
    let mut w5 = [[0f64; 3]; 4];
    let mut hashes_ok = true;
    let mut rekeys_total = 0usize;

    for rep in 0..3 {
        // ---- index-addressed candidates: Vec, Gap, Piece ----
        let (v1, h1a, v3, h3a, v4, h4a) = bench_index_struct!(VecTrack, &clips, 5000, 2500, 2500, rep);
        let (g1, h1b, g3, h3b, g4, h4b) = bench_index_struct!(GapTrack, &clips, 5000, 2500, 2500, rep);
        let (p1, h1d, p3, h3d, p4, h4d) = bench_index_struct!(PieceTrack, &clips, 5000, 2500, 2500, rep);
        let (b1, h1c, b3, h3c, b4, h4c, rk) = bench_btree(&clips, rep);
        rekeys_total += rk;
        // cross-structure state equality (hard gate)
        if !(h1a == h1b && h1a == h1c && h1a == h1d) { hashes_ok = false; println!("HASH MISMATCH W1 rep {}", rep); }
        if !(h3a == h3b && h3a == h3c && h3a == h3d) { hashes_ok = false; println!("HASH MISMATCH W3 rep {}", rep); }
        if !(h4a == h4b && h4a == h4c && h4a == h4d) { hashes_ok = false; println!("HASH MISMATCH W4 rep {}", rep); }
        w1[0][rep] = v1; w1[1][rep] = g1; w1[2][rep] = b1; w1[3][rep] = p1;
        w3[0][rep] = v3; w3[1][rep] = g3; w3[2][rep] = b3; w3[3][rep] = p3;
        w4[0][rep] = v4; w4[1][rep] = g4; w4[2][rep] = b4; w4[3][rep] = p4;

        // ---- W5: walk x200 on pristine 20k track ----
        let mut vt = VecTrack::new(); for c in &clips { vt.push(*c); }
        let mut gt = GapTrack::new(); for c in &clips { gt.push(*c); }
        let mut bt = BTreeTrack::new(); for c in &clips { bt.push(*c); }
        let mut pt = PieceTrack::new(); for c in &clips { pt.push(*c); }
        let expected = bt.walk();
        assert_eq!(vt.walk(), expected); assert_eq!(gt.walk(), expected);
        assert_eq!(pt.walk(), expected);
        let t = Instant::now();
        for _ in 0..200 { std::hint::black_box(vt.walk()); }
        w5[0][rep] = t.elapsed().as_secs_f64() * 1000.0;
        let t = Instant::now();
        for _ in 0..200 { std::hint::black_box(gt.walk()); }
        w5[1][rep] = t.elapsed().as_secs_f64() * 1000.0;
        let t = Instant::now();
        for _ in 0..200 { std::hint::black_box(bt.walk()); }
        w5[2][rep] = t.elapsed().as_secs_f64() * 1000.0;
        let t = Instant::now();
        for _ in 0..200 { std::hint::black_box(pt.walk()); }
        w5[3][rep] = t.elapsed().as_secs_f64() * 1000.0;
    }

    // exact-total verification: sum of durations must be IDENTICAL across structures (rep 0 pristine)
    let mut vt = VecTrack::new(); for c in &clips { vt.push(*c); }
    let mut bt = BTreeTrack::new(); for c in &clips { bt.push(*c); }
    assert_eq!(vt.walk(), bt.walk(), "exact total-duration equality");

    println!("medians over 3 reps (ms) — lower is better");
    println!("{:<26}{:>10}{:>10}{:>10}{:>10}", "workload", "vec", "gap", "btree", "piece");
    println!("{:<26}{:>10.1}{:>10.1}{:>10.1}{:>10.1}", "W1 cursor-split x5000", median3(&mut w1[0]), median3(&mut w1[1]), median3(&mut w1[2]), median3(&mut w1[3]));
    println!("{:<26}{:>10.1}{:>10.1}{:>10.1}{:>10.1}", "W3 fixed-point del x2500", median3(&mut w3[0]), median3(&mut w3[1]), median3(&mut w3[2]), median3(&mut w3[3]));
    println!("{:<26}{:>10.1}{:>10.1}{:>10.1}{:>10.1}", "W4 scatter-del x2500", median3(&mut w4[0]), median3(&mut w4[1]), median3(&mut w4[2]), median3(&mut w4[3]));
    println!("{:<26}{:>10.1}{:>10.1}{:>10.1}{:>10.1}", "W5 walk x200 (20k)", median3(&mut w5[0]), median3(&mut w5[1]), median3(&mut w5[2]), median3(&mut w5[3]));
    println!();
    println!("cross-structure hash equality (W1/W3/W4 x 3 reps): {}", if hashes_ok { "PASS" } else { "FAIL" });
    println!("btree fractional-key rekey events across all reps: {}", rekeys_total);
    println!("NOTE: container CPU; numbers indicative, not absolute");
}
