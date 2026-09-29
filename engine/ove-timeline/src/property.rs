//! Clip property animation — keyframes with exact-rational interpolation
//! (WAVE 8, ADR-019; ENGINE_BUILD_PLAN wave 7b keyframe leg).
//!
//! Model decisions (all normative in ADR-019):
//!   * Keyframe times are LOCAL clip times (0 = clip start). Keys ride the
//!     clip: Move carries the animation untouched, Split slices it (see
//!     [`PropertyTrack::slice_split`]), Resize strands keys beyond the
//!     duration as inert data (evaluation domain is [0, dur); growing the
//!     clip back re-activates them — the exact inverse of Resize needs
//!     property data untouched).
//!   * Per-key out-interpolation: how the segment FROM this key TO the next
//!     is evaluated. v1: [`Interpolation::Linear`] (exact rational lerp) and
//!     [`Interpolation::Hold`] (value until the next key). The last key's
//!     out-interp is inert.
//!   * Evaluation boundaries (exact, never fp): t ≤ first key → first
//!     value; t ≥ last key → last value; t exactly on a key → that key's
//!     value; inside a segment per the segment's out-interp. Empty track →
//!     None (the caller falls back to the static property value).
//!   * Keys are strictly increasing in time and never negative — validated
//!     loudly at every construction boundary ([`PropertyError`]); the engine
//!     never silently sorts or dedupes caller data.
//!   * Split slicing inserts the computed boundary value as a key on BOTH
//!     halves (time `at` on the left — inert but load-bearing for the
//!     interpolation of the segment before it; local 0 on the right), so
//!     evaluation across the seam is byte-exact preserved. The inserted
//!     keys carry the out-interp of the CUT segment, preserving Hold vs
//!     Linear semantics on the right half.
//!
//! All arithmetic is exact `ove_time::Rational` (ADR-007); overflow is a
//! fail-fast panic per the crate-wide contract.

use ove_time::Rational;

/// How the segment from a key to the NEXT key is evaluated (ADR-019 v1).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[repr(u8)]
pub enum Interpolation {
    /// Exact rational lerp: v(t) = v0 + (v1 − v0)·(t − t0)/(t1 − t0).
    Linear = 0,
    /// v(t) = v0 for the whole segment (until the next key).
    Hold = 1,
}

/// One keyframe: LOCAL clip time, exact value, out-interpolation.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Keyframe {
    pub time: Rational,
    pub value: Rational,
    pub interp: Interpolation,
}

impl Keyframe {
    pub fn new(time: Rational, value: Rational, interp: Interpolation) -> Self {
        Keyframe {
            time,
            value,
            interp,
        }
    }
}

/// Construction/validation failures — loud, typed, never repaired silently.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum PropertyError {
    /// A keyframe at negative local time (the clip domain is [0, dur)).
    NegativeTime,
    /// Key times must be STRICTLY increasing (equal times are ambiguous).
    NonMonotonic,
}

impl std::fmt::Display for PropertyError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            PropertyError::NegativeTime => write!(f, "keyframe time is negative"),
            PropertyError::NonMonotonic => {
                write!(f, "keyframe times must be strictly increasing")
            }
        }
    }
}

impl std::error::Error for PropertyError {}

/// The animation track of ONE clip property: sorted, strictly-increasing
/// keys. Empty = the property is static (no animation data).
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct PropertyTrack {
    keys: Vec<Keyframe>,
}

impl PropertyTrack {
    pub fn new() -> Self {
        PropertyTrack { keys: Vec::new() }
    }

    /// Construct from keys with loud validation (order, strict monotonicity,
    /// non-negative times). Caller data is never silently reordered.
    pub fn from_keys(keys: Vec<Keyframe>) -> Result<Self, PropertyError> {
        let mut prev: Option<Rational> = None;
        for k in &keys {
            if k.time < Rational::zero(1) {
                return Err(PropertyError::NegativeTime);
            }
            if let Some(p) = prev {
                if k.time <= p {
                    return Err(PropertyError::NonMonotonic);
                }
            }
            prev = Some(k.time);
        }
        Ok(PropertyTrack { keys })
    }

    /// Insert one key at its sorted position. Rejects a duplicate time
    /// (strict monotonicity) and negative times.
    pub fn insert(&mut self, kf: Keyframe) -> Result<(), PropertyError> {
        if kf.time < Rational::zero(1) {
            return Err(PropertyError::NegativeTime);
        }
        let pos = self.keys.partition_point(|k| k.time < kf.time);
        if self
            .keys
            .get(pos)
            .map(|k| k.time == kf.time)
            .unwrap_or(false)
        {
            return Err(PropertyError::NonMonotonic);
        }
        self.keys.insert(pos, kf);
        Ok(())
    }

    pub fn len(&self) -> usize {
        self.keys.len()
    }

    pub fn is_empty(&self) -> bool {
        self.keys.is_empty()
    }

    /// The keys, strictly increasing in time (read-only).
    pub fn keys(&self) -> &[Keyframe] {
        &self.keys
    }

    /// Evaluate at local clip time `t`. `None` only when the track is empty.
    /// Exact at every boundary: on-key identity, hold-before-first,
    /// hold-after-last, exact rational lerp inside Linear segments.
    pub fn evaluate(&self, t: Rational) -> Option<Rational> {
        let first = *self.keys.first()?;
        let last = *self.keys.last().expect("first() succeeded");
        if t <= first.time {
            return Some(first.value);
        }
        if t >= last.time {
            return Some(last.value);
        }
        // first.time < t < last.time: locate the segment [k0, k1) with
        // k0.time <= t < k1.time. partition_point = first index with
        // time > t (>= 1 here because t > first.time).
        let hi = self.keys.partition_point(|k| k.time <= t);
        let k0 = self.keys[hi - 1];
        let k1 = self.keys[hi];
        match k0.interp {
            Interpolation::Hold => Some(k0.value),
            Interpolation::Linear => {
                // v = v0 + (v1 − v0)·(t − t0)/(t1 − t0) — all exact. t > t0
                // here, and t1 > t0 by the strict-monotonic invariant, so
                // recip() is division-by-zero-free. Overflow panics per the
                // ADR-007 fail-fast contract (caller bug, never silent).
                let dt = t.sub(k0.time);
                let span = k1.time.sub(k0.time);
                let dv = k1.value.sub(k0.value);
                Some(k0.value.add(dv.mul(dt).mul(span.recip())))
            }
        }
    }

    /// The out-interp of the key governing local time `t` (the segment
    /// containing t). Before the first key or after the last there is no
    /// governing segment — [`Interpolation::Linear`] is the documented
    /// neutral answer (it is value-irrelevant there: the boundary value
    /// equals the adjacent key's value either way).
    fn segment_interp_at(&self, t: Rational) -> Interpolation {
        match self.keys.partition_point(|k| k.time <= t) {
            0 => Interpolation::Linear, // t ≤ first key: no segment
            hi if hi >= self.keys.len() => {
                // t ≥ last key's segment end: the last key governs
                self.keys[hi - 1].interp
            }
            hi => self.keys[hi - 1].interp,
        }
    }

    /// Whether a key sits exactly at `at`.
    fn has_key_at(&self, at: Rational) -> bool {
        self.keys.binary_search_by(|k| k.time.cmp(&at)).is_ok()
    }

    /// Split-slice at local time `at` (0 < at; the caller's Split command
    /// guarantees at < duration). Returns (left, right) property tracks
    /// such that evaluating LEFT on [0, at) and RIGHT on [at, dur) (local
    /// time = original − at) reproduces the original evaluation EXACTLY at
    /// every instant — the split-preserve property (S8).
    ///
    /// Mechanics: left = keys < at, plus a computed key at `at` (value =
    /// evaluate(at)) — inert for display (the left domain is half-open) but
    /// load-bearing for the interpolation of the segment before it. Right =
    /// keys ≥ at shifted by −at, plus a computed key at 0 with the cut
    /// segment's out-interp when no key existed exactly at `at` (a key at
    /// `at` shifts to 0 carrying its own interp). Empty tracks slice to
    /// empty.
    pub fn slice_split(&self, at: Rational) -> (PropertyTrack, PropertyTrack) {
        if self.keys.is_empty() {
            return (PropertyTrack::new(), PropertyTrack::new());
        }
        let v_at = self
            .evaluate(at)
            .expect("non-empty track evaluates at every instant");
        let seg_interp = self.segment_interp_at(at);
        let boundary_existed = self.has_key_at(at);

        let mut left_keys: Vec<Keyframe> = self
            .keys
            .iter()
            .take_while(|k| k.time < at)
            .copied()
            .collect();
        left_keys.push(Keyframe {
            time: at,
            value: v_at,
            interp: seg_interp,
        });

        let mut right_keys: Vec<Keyframe> = self
            .keys
            .iter()
            .skip_while(|k| k.time < at)
            .map(|k| Keyframe {
                time: k.time.sub(at),
                value: k.value,
                interp: k.interp,
            })
            .collect();
        if !boundary_existed {
            right_keys.insert(
                0,
                Keyframe {
                    time: Rational::zero(1),
                    value: v_at,
                    interp: seg_interp,
                },
            );
        }
        (
            PropertyTrack { keys: left_keys },
            PropertyTrack { keys: right_keys },
        )
    }

    /// Deterministic byte contribution for state hashing (TrackOps::state_
    /// hash): count + each key's exact (time, value, interp) fields.
    pub fn mix_into_hash(&self, mix: &mut dyn FnMut(&[u8])) {
        mix(&(self.keys.len() as u32).to_le_bytes());
        for k in &self.keys {
            mix(&k.time.num().to_le_bytes());
            mix(&k.time.den().to_le_bytes());
            mix(&k.value.num().to_le_bytes());
            mix(&k.value.den().to_le_bytes());
            mix(&[k.interp as u8]);
        }
    }
}

/// The v1 animatable property set (ADR-019): exactly the software
/// renderer's per-placement animated inputs (RENDER_GRAPH_SPEC — alpha and
/// integer translation). Declaration order is the hash/serialization order.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PropertyName {
    Opacity,
    X,
    Y,
}

/// The per-clip animation data: one track per v1 property. Fixed fields —
/// no maps, so iteration order is structural and hashing is canonical.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct ClipProperties {
    pub opacity: PropertyTrack,
    pub x: PropertyTrack,
    pub y: PropertyTrack,
}

impl ClipProperties {
    pub fn track(&self, p: PropertyName) -> &PropertyTrack {
        match p {
            PropertyName::Opacity => &self.opacity,
            PropertyName::X => &self.x,
            PropertyName::Y => &self.y,
        }
    }

    pub fn track_mut(&mut self, p: PropertyName) -> &mut PropertyTrack {
        match p {
            PropertyName::Opacity => &mut self.opacity,
            PropertyName::X => &mut self.x,
            PropertyName::Y => &mut self.y,
        }
    }

    pub fn is_empty(&self) -> bool {
        self.opacity.is_empty() && self.x.is_empty() && self.y.is_empty()
    }

    /// Hash all three tracks in declaration order (tagged).
    pub fn mix_into_hash(&self, mix: &mut dyn FnMut(&[u8])) {
        for (tag, track) in [(0u8, &self.opacity), (1, &self.x), (2, &self.y)] {
            mix(&[tag]);
            track.mix_into_hash(mix);
        }
    }

    /// Split-slice EVERY property track at local time `at` (ADR-019): the
    /// clip-level operation the Split command performs, exact across the
    /// seam (see [`PropertyTrack::slice_split`]).
    pub fn slice_split(&self, at: Rational) -> (ClipProperties, ClipProperties) {
        let (lo, ro) = self.opacity.slice_split(at);
        let (lx, rx) = self.x.slice_split(at);
        let (ly, ry) = self.y.slice_split(at);
        (
            ClipProperties {
                opacity: lo,
                x: lx,
                y: ly,
            },
            ClipProperties {
                opacity: ro,
                x: rx,
                y: ry,
            },
        )
    }
}
