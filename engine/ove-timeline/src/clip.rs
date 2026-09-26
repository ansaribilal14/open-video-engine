//! Clip model — explicit identities and exact times (ADR-006, E-003 schema rules).

use ove_time::Rational;
use std::fmt;

/// Explicit clip identity. Assigned by the engine from a folded counter; ids are
/// never positional and never reused by clients (E-003/E-009 schema rule).
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Debug)]
pub struct ClipId(pub u64);

/// Explicit track identity (index into the track table, fixed at construction).
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Debug)]
pub struct TrackId(pub u32);

/// Explicit asset identity (content-addressed in ove-project; opaque here).
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Debug)]
pub struct AssetId(pub u64);

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum TrackKind {
    Video,
    Audio,
}

impl fmt::Display for TrackKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            TrackKind::Video => write!(f, "video"),
            TrackKind::Audio => write!(f, "audio"),
        }
    }
}

/// One clip on the timeline. All times are exact rationals on the project axis.
///
/// Source mapping: the clip consumes source range
/// `[src_in, src_in + dur * speed)` (exact rational arithmetic) and places it at
/// `[start, start + dur)`. `speed` defaults to 1/1; retiming preserves the source
/// range and derives the new timeline duration (RetimeClip semantics).
#[derive(Clone, PartialEq, Debug)]
pub struct ClipEntry {
    pub id: ClipId,
    pub track: TrackId,
    pub asset: AssetId,
    pub start: Rational,
    pub dur: Rational,
    pub src_in: Rational,
    pub speed: Rational,
    pub label: String,
}

impl ClipEntry {
    /// Exact source-range duration consumed by this clip (`dur * speed`).
    pub fn src_dur(&self) -> Rational {
        self.dur.mul(self.speed)
    }

    /// Exact source position for a timeline offset within the clip
    /// (`src_in + offset * speed`); `offset` must be within `[0, dur]`.
    pub fn src_at(&self, offset: Rational) -> Rational {
        self.src_in.add(offset.mul(self.speed))
    }

    /// End time on the timeline (`start + dur`).
    pub fn end(&self) -> Rational {
        self.start.add(self.dur)
    }
}
