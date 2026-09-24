//! ove-time property suite — the permanent Rust counterpart of experiment E-002.
//!
//! Deterministic (seeded xorshift, no external deps). Each property runs over
//! thousands of generated cases. These tests ARE the acceptance evidence for
//! ADR-007 ("property tests land in E-002"), so they must never be weakened.

use ove_time::Rational;

struct Rng(u64);
impl Rng {
    fn next(&mut self) -> u64 {
        self.0 ^= self.0 << 13;
        self.0 ^= self.0 >> 7;
        self.0 ^= self.0 << 17;
        self.0
    }
    fn gen_rational(&mut self, mag: i64) -> Rational {
        // random num in [-mag, mag], random den in [1, 24000], den > 0
        let num = ((self.next() % (2 * mag as u64 + 1)) as i64) - mag;
        let den = 1 + (self.next() % 24_000) as i64;
        Rational::new(num, den)
    }
}

const CASES: u64 = 20_000;

/// P1: addition is associative over normalized exact rationals.
#[test]
fn p1_add_associativity() {
    let mut rng = Rng(0xA1);
    for _ in 0..CASES {
        let (a, b, c) = (rng.gen_rational(1_000_000), rng.gen_rational(1_000_000), rng.gen_rational(1_000_000));
        assert_eq!(a.add(b.add(c)), a.add(b).add(c), "associativity: {a} {b} {c}");
    }
}

/// P2: normalization invariants hold after every operation.
#[test]
fn p2_normalization_invariants() {
    let mut rng = Rng(0xA2);
    for _ in 0..CASES {
        let (a, b) = (rng.gen_rational(1_000_000), rng.gen_rational(1_000_000));
        for r in [a.add(b), a.sub(b), a.mul(b)] {
            assert!(r.den() > 0, "den positive");
            let g = gcd(r.num().abs(), r.den());
            assert!(g == 1 || r.num() == 0, "normalized: {r} gcd={g}");
        }
    }
}
fn gcd(mut a: i64, mut b: i64) -> i64 {
    while b != 0 { let t = a % b; a = b; b = t; }
    a
}

/// P3: sums are order-invariant on a FIXED TICK AXIS (all values are exact
/// multiples of 1/24000, so every normalized denominator divides 24000 and
/// lcm stays bounded). This is the production representation: aggregates
/// accumulate on a common rational axis.
#[test]
fn p3_order_invariant_sums() {
    let mut rng = Rng(0xA3);
    for case in 0..200 {
        let n = 50 + (rng.next() % 500) as usize;
        let mut clips: Vec<Rational> =
            (0..n).map(|_| Rational::new(1 + (rng.next() % 240_000) as i64, 24_000)).collect();
        let f = clips.iter().copied().fold(Rational::zero(24000), |a, x| a.add(x));
        let r = clips.iter().rev().copied().fold(Rational::zero(24000), |a, x| a.add(x));
        for i in (1..clips.len()).rev() {
            let j = (rng.next() % (i as u64 + 1)) as usize;
            clips.swap(i, j);
        }
        let s = clips.iter().copied().fold(Rational::zero(24000), |a, x| a.add(x));
        assert_eq!(f, r, "case {case}: forward vs reverse");
        assert_eq!(f, s, "case {case}: forward vs shuffled");
    }
}

/// P3b (regression guard for a DISCOVERED failure mode): free-rational
/// accumulation over coprime denominators blows up the lcm chain — i64
/// overflow panic within a small number of terms. Python E-002 could not see
/// this (bigint). Consequence recorded in ADR-007: accumulators MUST run on
/// a fixed tick axis; free rationals are boundary/ingest values only.
#[test]
fn p3b_free_rational_accumulator_overflows() {
    // denominators deliberately from primes near 24000 => pairwise coprime
    const PRIMES: [i64; 7] = [23_987, 23_989, 23_993, 23_999, 24_007, 24_011, 24_017];
    let result = std::panic::catch_unwind(|| {
        let mut acc = Rational::zero(PRIMES[0]);
        for i in 0..64u32 {
            acc = acc.add(Rational::new(1, PRIMES[(i as usize) % PRIMES.len()]));
        }
        acc
    });
    assert!(result.is_err(), "free-rational accumulation must overflow the i64 path quickly");
    // the fixed-axis equivalent must NOT panic and stays bounded on the axis
    let mut acc2 = Rational::zero(24000);
    for _ in 0..10_000 { acc2 = acc2.add(Rational::new(1001, 24000)); }
    assert!(acc2.den() > 0 && 24000 % acc2.den() == 0, "fixed-axis den must divide the axis");
}

/// P4: floor-to-frame is exact at boundaries where fp was off-by-one
/// (E-002 T1). For every k, value k/rate must floor to exactly k.
#[test]
fn p4_boundary_floor_exact() {
    for rate in [(30, 1), (24000, 1001), (30000, 1001), (25, 1), (24, 1)] {
        let (rn, rd) = rate;
        for k in 0..5_000i64 {
            let v = Rational::new(k * rd, rn); // exactly k frames worth of time
            assert_eq!(v.floor_div_rate(rn, rd), k, "exact boundary k={k} rate={rn}/{rd}");
            // one tick below k frames must floor to k-1
            let below = v.sub(Rational::new(1, rn * rd));
            assert_eq!(below.floor_div_rate(rn, rd), k - 1, "just-below k={k}");
        }
    }
}

/// P5: 23.976-fps decimal alias drifts vs 24000/1001 (E-002 T2) — assert the
/// exact model does NOT drift: 2h of frame-accurate ticks keep counting exact.
#[test]
fn p5_no_drift_exact_rates() {
    // 2 hours at 24000/1001 = 172_262... frames; accumulate frame durations exactly
    let frame = Rational::new(1001, 24000);
    let mut t = Rational::zero(24000);
    let frames = 172_262u64; // floor(7200 * 24000/1001)
    for _ in 0..frames { t = t.add(frame); }
    // exact: t == frames * 1001/24000
    assert_eq!(t, Rational::new(frames as i64 * 1001, 24000));
    // frame index at that time must be exactly `frames` (boundary-adjacent, exact)
    assert_eq!(t.floor_div_rate(24000, 1001), frames as i64);
}

/// P6: sub/add roundtrip: (a + b) - b == a over random cases.
#[test]
fn p6_add_sub_roundtrip() {
    let mut rng = Rng(0xA6);
    for _ in 0..CASES {
        let (a, b) = (rng.gen_rational(1_000_000), rng.gen_rational(1_000_000));
        assert_eq!(a.add(b).sub(b), a, "roundtrip {a} + {b}");
    }
}

/// P7: split roundtrip: half + remainder == original (both parities), the
/// invariant timeline SPLIT depends on.
#[test]
fn p7_split_invariant() {
    let mut rng = Rng(0xA7);
    for _ in 0..CASES {
        let d = rng.gen_rational(240_000);
        let h = d.half();
        let rem = d.sub(h);
        assert_eq!(h.add(rem), d, "split {d}");
        // halves are valid rationals
        assert!(h.den() > 0 && rem.den() > 0);
    }
}

/// P8: comparison by cross-multiplication is a strict total order consistent
/// with i128 evaluation (no i64 overflow false orderings).
#[test]
fn p8_total_order() {
    let mut rng = Rng(0xA8);
    for _ in 0..CASES {
        let (a, b) = (rng.gen_rational(10_000_000), rng.gen_rational(10_000_000));
        let cmp = a.cmp(&b);
        // independent i128 evaluation
        let l = a.num() as i128 * b.den() as i128;
        let r = b.num() as i128 * a.den() as i128;
        let expected = l.cmp(&r);
        assert_eq!(cmp, expected, "order {a} vs {b}");
        // antisymmetry
        assert_eq!(b.cmp(&a), expected.reverse());
    }
}

/// P9: fp legacy rates drift vs exact rates — measured on CUMULATIVE frame
/// indices at integer-second boundaries (E-002 T2: ~7.2 ms / 2h). Totals over
/// exactly 2h can re-coincide; the drift shows at intermediate boundaries.
#[test]
fn p9_decimal_alias_drift_exists() {
    let mut diverged_at: Option<i64> = None;
    for s in 1..=7200i64 {
        let exact = (s as f64 * (24000.0 / 1001.0)).floor() as i64;
        let alias = (s as f64 * 23.976).floor() as i64;
        if exact != alias { diverged_at = Some(s); break; }
    }
    assert!(diverged_at.is_some(),
        "decimal 23.976 must diverge from 24000/1001 at some boundary within 2h (E-002 T2)");
    // and the divergence is systematic, growing without bound over 24h
    let d24 = (86400.0_f64 * (24000.0 / 1001.0)) - (86400.0 * 23.976);
    assert!(d24.abs() > 1.0, "drift must exceed one frame over 24h");
}

/// P10: large-magnitude arithmetic stays exact via i128 path (no wrap).
/// Verification is independent: s = a+b must satisfy
/// s * (a.den*b.den) == a.num*b.den + b.num*a.den, all in i64-safe ranges.
#[test]
fn p10_large_magnitude_exact() {
    let mut rng = Rng(0xAA);
    for _ in 0..CASES {
        let a = rng.gen_rational(50_000_000);
        let b = rng.gen_rational(50_000_000);
        let s = a.add(b);
        let lhs = s.mul(Rational::new(a.den() * b.den(), 1));
        let rhs = Rational::new(a.num() * b.den() + b.num() * a.den(), 1);
        assert_eq!(lhs, rhs, "large-magnitude exactness: {a} + {b} = {s}");
    }
}
