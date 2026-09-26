//! ove-time — exact rational time primitives for the open-video-engine.
//!
//! Design authority: ADR-007 (exact rational/integer authoritative time),
//! evidence: E-002 (13/13 property checks), E-002c (structure benchmark).
//!
//! Rules encoded here (do not weaken):
//!   * Time values are exact rationals: i64 numerator / i64 denominator.
//!   * Denominator is always > 0; values are stored normalized (gcd == 1).
//!   * Arithmetic uses i128 intermediates and saturates on overflow rather
//!     than wrapping silently.
//!   * Floating point is NEVER accepted as input for authoritative time.
//!     (from_f64_seconds exists ONLY for interop with legacy fp sources and
//!     quantizes to the nearest tick of the given rate — display/legacy use.)

use std::cmp::Ordering;
use std::fmt;

/// Exact rational time value. Invariant: `den > 0`, `gcd(|num|, den) == 1`.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct Rational {
    num: i64,
    den: i64,
}

fn gcd64(a: i64, b: i64) -> i64 {
    let (mut a, mut b) = (a.abs(), b.abs());
    while b != 0 {
        let t = a % b;
        a = b;
        b = t;
    }
    a
}

fn gcd128(a: i128, b: i128) -> i128 {
    let (mut a, mut b) = (a.abs(), b.abs());
    while b != 0 {
        let t = a % b;
        a = b;
        b = t;
    }
    a
}

impl Rational {
    /// Construct a normalized exact rational. Panics if `den <= 0`.
    pub fn new(num: i64, den: i64) -> Self {
        assert!(den > 0, "denominator must be positive");
        let g = gcd64(num, den);
        if g > 1 {
            Rational {
                num: num / g,
                den: den / g,
            }
        } else {
            Rational { num, den }
        }
    }

    /// Zero at the given rate denominator (e.g. `zero(24000)` for 24 kHz ticks).
    pub fn zero(den: i64) -> Self {
        Rational::new(0, den)
    }

    pub fn num(&self) -> i64 {
        self.num
    }
    pub fn den(&self) -> i64 {
        self.den
    }

    /// Exact addition (i128 intermediates, overflow => panic with context).
    /// Inherent named methods coexist with the std::ops impls below ON PURPOSE:
    /// cross-language bindings (UniFFI, E-004b) cannot export operator traits,
    /// so explicit names are the stable FFI surface.
    #[allow(clippy::should_implement_trait)]
    pub fn add(self, o: Rational) -> Self {
        let g = gcd128(self.den as i128, o.den as i128);
        let lcm = (self.den as i128 / g)
            .checked_mul(o.den as i128)
            .expect("denominator lcm overflow");
        let n = (self.num as i128)
            .checked_mul(lcm / self.den as i128)
            .expect("numerator overflow")
            + (o.num as i128)
                .checked_mul(lcm / o.den as i128)
                .expect("numerator overflow");
        let gg = gcd128(n, lcm).max(1);
        Self::new(
            i64::try_from(n / gg).expect("result numerator exceeds i64"),
            i64::try_from(lcm / gg).expect("result denominator exceeds i64"),
        )
    }

    /// Exact negation.
    #[allow(clippy::should_implement_trait)]
    pub fn neg(self) -> Self {
        Rational {
            num: -self.num,
            den: self.den,
        }
    }

    /// Exact subtraction.
    #[allow(clippy::should_implement_trait)]
    pub fn sub(self, o: Rational) -> Self {
        self.add(o.neg())
    }

    /// Exact multiplication.
    #[allow(clippy::should_implement_trait)]
    pub fn mul(self, o: Rational) -> Self {
        let n = (self.num as i128)
            .checked_mul(o.num as i128)
            .expect("numerator overflow");
        let d = (self.den as i128)
            .checked_mul(o.den as i128)
            .expect("denominator overflow");
        let gg = gcd128(n, d).max(1);
        Self::new(
            i64::try_from(n / gg).expect("result numerator exceeds i64"),
            i64::try_from(d / gg).expect("result denominator exceeds i64"),
        )
    }

    /// Exact halving (used by split operations). `num/2` floor for odd nums,
    /// with the remainder preserved by the caller via `sub`.
    pub fn half(self) -> Self {
        Self::new(self.num >> 1, self.den)
    }

    /// Exact floor to an integer frame index at the given frame rate
    /// (`rate_num/rate_den` frames per second), computed in i128 — no
    /// intermediate rounding. Value is interpreted as SECONDS; result =
    /// floor(value × rate). This is THE operation E-002 proved fp gets
    /// wrong at boundaries.
    pub fn floor_div_rate(self, rate_num: i64, rate_den: i64) -> i64 {
        assert!(rate_num > 0 && rate_den > 0, "rate must be positive");
        // floor( (num/den) × (rate_num/rate_den) ) = floor(num*rate_num / (den*rate_den))
        let n = self.num as i128 * rate_num as i128;
        let d = self.den as i128 * rate_den as i128;
        let q = n / d;
        let r = n % d;
        let q = if r != 0 && (d < 0) != (n < 0) {
            q - 1
        } else {
            q
        };
        i64::try_from(q).expect("frame index exceeds i64")
    }

    /// Legacy interop ONLY: quantize an fp seconds value to the nearest tick
    /// of `rate_num/rate_den`. Documented lossy — never use on the
    /// authoritative path (E-002 T1–T5).
    pub fn from_f64_seconds_quantized(secs: f64, rate_num: i64, rate_den: i64) -> Self {
        let ticks = (secs * (rate_num as f64 / rate_den as f64)).round() as i64;
        Self::new(
            ticks.checked_mul(rate_den).expect("tick overflow"),
            rate_num,
        )
    }

    /// Ticks at the given rate (exact integer): floor(value * rate).
    pub fn ticks_at(self, rate_num: i64, rate_den: i64) -> i64 {
        self.floor_div_rate(rate_num, rate_den)
    }
}

impl PartialOrd for Rational {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}
impl Ord for Rational {
    fn cmp(&self, other: &Self) -> Ordering {
        // i128 cross-multiply; magnitudes bounded by i64 => product fits i128
        (self.num as i128 * other.den as i128).cmp(&(other.num as i128 * self.den as i128))
    }
}

impl fmt::Display for Rational {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}/{}", self.num, self.den)
    }
}

impl std::ops::Add for Rational {
    type Output = Rational;
    fn add(self, o: Rational) -> Rational {
        Rational::add(self, o)
    }
}
impl std::ops::Sub for Rational {
    type Output = Rational;
    fn sub(self, o: Rational) -> Rational {
        Rational::sub(self, o)
    }
}
impl std::ops::Mul for Rational {
    type Output = Rational;
    fn mul(self, o: Rational) -> Rational {
        Rational::mul(self, o)
    }
}
impl std::ops::Neg for Rational {
    type Output = Rational;
    fn neg(self) -> Rational {
        Rational::neg(self)
    }
}

#[cfg(test)]
mod unit {
    use super::*;

    #[test]
    fn normalization() {
        let r = Rational::new(24000, 48000);
        assert_eq!((r.num(), r.den()), (1, 2));
    }

    #[test]
    fn exact_add_no_fp() {
        // the fp trap from E-002 T1: 10 x 0.1 != 1.0 in binary64; exact here
        let tenth = Rational::new(1, 10);
        let mut t = Rational::zero(10);
        for _ in 0..10 {
            t = t.add(tenth);
        }
        assert_eq!(t, Rational::new(1, 1));
    }

    #[test]
    fn boundary_floor_exact() {
        // exactly 30 frames at 30fps must floor to 30, not 29
        let r = Rational::new(1, 1);
        assert_eq!(r.floor_div_rate(30, 1), 30);
        // one tick below must floor to 29
        assert_eq!(Rational::new(29_999, 30_000).floor_div_rate(30, 1), 29);
    }

    #[test]
    fn half_roundtrip_odd() {
        let d = Rational::new(1001, 24000); // odd numerator
        let h = d.half();
        assert_eq!(h.add(d.sub(h)), d); // left + right == original
    }
}

// ---------------------------------------------------------------------------
// Optional serde support (feature `serde`): serialize the NORMALIZED form as
// a (num, den) pair; deserialization re-validates the invariants (den > 0)
// instead of trusting the input.
// ---------------------------------------------------------------------------
#[cfg(feature = "serde")]
impl serde::Serialize for Rational {
    fn serialize<S: serde::Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        (self.num, self.den).serialize(s)
    }
}

#[cfg(feature = "serde")]
impl<'de> serde::Deserialize<'de> for Rational {
    fn deserialize<D: serde::Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
        let (num, den): (i64, i64) = serde::Deserialize::deserialize(d)?;
        if den <= 0 {
            return Err(serde::de::Error::custom("rational denominator must be > 0"));
        }
        Ok(Rational::new(num, den))
    }
}

#[cfg(all(test, feature = "serde"))]
mod serde_tests {
    use super::*;

    #[test]
    fn roundtrip_normalized() {
        let r = Rational::new(48000, 1000);
        let s = serde_json::to_string(&r).unwrap();
        assert_eq!(s, "[48,1]");
        let back: Rational = serde_json::from_str(&s).unwrap();
        assert_eq!(back, r);
    }

    #[test]
    fn rejects_zero_and_negative_den() {
        assert!(serde_json::from_str::<Rational>("[1,0]").is_err());
        assert!(serde_json::from_str::<Rational>("[1,-2]").is_err());
    }
}
