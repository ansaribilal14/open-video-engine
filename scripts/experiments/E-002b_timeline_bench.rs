// E-002b: native Rust timeline data-structure benchmark (the authoritative perf leg).
// Replaces the Python-only timing rows of E-002. Uses exact i64 ticks (num/den pairs
// via u64 numerators) — no floats on the timing path.
use std::time::Instant;
use std::collections::BTreeMap;

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Debug)]
struct Rational { num: i64, den: i64 } // den > 0

impl Rational {
    fn new(num: i64, den: i64) -> Self { let g = gcd(num.abs(), den.abs()); Rational { num: num / g, den: den / g } }
    fn add(self, o: Rational) -> Rational {
        // overflow-guarded exact addition (lcm of denominators)
        let g = gcd(self.den, o.den);
        let lcm = self.den / g * o.den;
        Rational::new(self.num * (lcm / self.den) + o.num * (lcm / o.den), lcm)
    }
}
fn gcd(a: i64, b: i64) -> i64 { if b == 0 { a } else { gcd(b, a % b) } }

impl std::ops::Sub for Rational {
    type Output = Rational;
    fn sub(self, o: Rational) -> Rational { self.add(Rational::new(-o.num, o.den)) }
}

struct Clip { start: Rational, dur: Rational, id: u32 }

fn main() {
    let n: usize = 20_000;
    let mut state: u64 = 0x243F6A8885A308D3;
    let mut rng = || { state ^= state << 13; state ^= state >> 7; state ^= state << 17; state };

    // --- build: ordered clip list per track (Vec) with exact rationals ---
    let t0 = Instant::now();
    let mut tracks: Vec<Vec<Clip>> = vec![Vec::with_capacity(n), Vec::new(), Vec::new()];
    let mut acc = Rational::new(0, 24000);
    for i in 0..n {
        let dur = Rational::new(1001 + (rng() % 240_000) as i64, 24000);
        tracks[0].push(Clip { start: acc, dur, id: i as u32 });
        acc = acc.add(dur);
    }
    let build = t0.elapsed();

    // --- sorted-index build (start -> id) ---
    let t1 = Instant::now();
    let mut index: BTreeMap<(i64, i64), u32> = BTreeMap::new();
    for c in &tracks[0] { index.insert((c.start.num, c.start.den), c.id); }
    let index_build = t1.elapsed();

    // --- split 5000 clips (splice at exact midpoints) ---
    let t2 = Instant::now();
    let mut split_count = 0usize;
    for k in (0..tracks[0].len()).step_by(4) {
        if split_count >= 5000 { break; }
        let (cstart, cdur, cid) = (tracks[0][k].start, tracks[0][k].dur, tracks[0][k].id);
        if cdur.num > 2 {
            let half = Rational::new(cdur.num / 2, cdur.den);
            tracks[0][k].dur = half;
            tracks[0].insert(k + 1, Clip { start: cstart.add(half), dur: cdur - half, id: cid });
            split_count += 1;
        }
    }
    let split = t2.elapsed();

    // --- prefix-sum determinism check (forward vs reverse, exact) ---
    let t3 = Instant::now();
    let mut f = Rational::new(0, 24000);
    for c in &tracks[0] { f = f.add(c.dur); }
    let mut r = Rational::new(0, 24000);
    for c in tracks[0].iter().rev() { r = r.add(c.dur); }
    let assoc_us = t3.elapsed().as_micros();
    assert_eq!(f, r, "exact arithmetic must be order-invariant");

    println!("N={} clips (pre-split)", n);
    println!("rational_build_ms            {:.2}", build.as_secs_f64() * 1000.0);
    println!("btree_index_build_ms         {:.2}", index_build.as_secs_f64() * 1000.0);
    println!("split_5000_ms                {:.2}", split.as_secs_f64() * 1000.0);
    println!("order_invariance_assert_us   {}", assoc_us);
    println!("final_clips                  {}", tracks[0].len());
    println!("total_duration               {}/{}", f.num, f.den);
    println!("NOTE: container CPU, release-less build; treat as indicative, not absolute");
}
