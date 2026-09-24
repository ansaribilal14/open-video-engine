// E-004b: UniFFI proc-macro surface over ove-time.
//
// Questions answered (see research/experiments/E-004b_uniffi_kotlin.md):
//   Q1 Does proc-macro codegen handle exact-time records/enums cleanly?
//   Q2 Does the generated Kotlin compile against JDK/kotlinc without hand edits?
//   Q3 What do the ergonomics look like from Kotlin (nullability, types)?

use ove_time::Rational;

/// Exact rational time exported to Kotlin as an immutable data class.
#[derive(uniffi::Record, Clone, Copy, Debug)]
pub struct RationalTime {
    pub num: i64,
    pub den: i64, // always > 0, normalized
}

/// The two authoritative representations considered by ADR-007.
#[derive(uniffi::Enum, Clone, Copy, Debug)]
pub enum TimeValue {
    /// Integer ticks on a fixed rational axis (production accumulator form).
    TickAxis { ticks: i64, rate_num: i64, rate_den: i64 },
    /// Free rational (ingest/boundary form — see E-002c P3b: never accumulate).
    FreeRational { num: i64, den: i64 },
}

fn from_rational(r: Rational) -> RationalTime {
    RationalTime { num: r.num(), den: r.den() }
}

fn to_rational(t: RationalTime) -> Rational {
    Rational::new(t.num, t.den)
}

#[uniffi::export]
fn ove_time_add(a: RationalTime, b: RationalTime) -> RationalTime {
    from_rational(to_rational(a).add(to_rational(b)))
}

#[uniffi::export]
fn ove_time_sub(a: RationalTime, b: RationalTime) -> RationalTime {
    from_rational(to_rational(a).sub(to_rational(b)))
}

/// Split helper: left half (right = value - left, exact).
#[uniffi::export]
fn ove_time_half(a: RationalTime) -> RationalTime {
    from_rational(to_rational(a).half())
}

/// Exact frame index: floor(value * rate_num / rate_den).
#[uniffi::export]
fn ove_time_floor_frame(a: RationalTime, rate_num: i64, rate_den: i64) -> i64 {
    to_rational(a).floor_div_rate(rate_num, rate_den)
}

/// Convert any TimeValue to ticks on its own axis (TickAxis) or to the given
/// rate (FreeRational). Returns exact integer ticks via floor.
#[uniffi::export]
fn ove_time_to_ticks(v: TimeValue, rate_num: i64, rate_den: i64) -> i64 {
    match v {
        TimeValue::TickAxis { ticks, rate_num: rn, rate_den: rd } => {
            // exact rescale: floor(ticks * (rn/rd) / (rate_num/rate_den))
            Rational::new(ticks * rd, rn).floor_div_rate(rate_num, rate_den)
        }
        TimeValue::FreeRational { num, den } => {
            Rational::new(num, den).floor_div_rate(rate_num, rate_den)
        }
    }
}

/// Round-trip sanity used by the Kotlin-side test.
#[uniffi::export]
fn ove_time_split_roundtrip(a: RationalTime) -> Vec<RationalTime> {
    let r = to_rational(a);
    let h = r.half();
    vec![from_rational(h), from_rational(r.sub(h))]
}

uniffi::setup_scaffolding!();
