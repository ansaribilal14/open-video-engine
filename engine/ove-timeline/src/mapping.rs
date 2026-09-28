//! Timeline↔media seam — exact mapping between project-axis clip times and
//! source-rate media times (MEDIA_ENGINE_SPEC §3; ADR-013).
//!
//! Layering rule (ADR-002): this crate must NOT depend on ove-media. The
//! source side is parameterized by [`SourceClock`], a plain exact-time struct
//! (duration + keyframe list) that the project/engine layer (waves 4–5) fills
//! from `ove-media` probe records (`ProbeInfo.duration`, `KeyframeIndex`
//! floors). Mapping happens AT QUERY TIME on source-pts provenance — the v1
//! resolution of Q-06 (map-in-format vs normalize): no converted copies are
//! stored; every mapping is exact rational arithmetic, never fp.
//!
//! [`ClipWindow`] is the seam's view of one clip placement: timeline start,
//! duration, source in-point, and speed. `From<&Clip>` lifts a live timeline
//! clip (speed 1/1 — the current Command verb set has no retime verb yet;
//! retiming constructs windows with speed ≠ 1 and the S-suite pins that math
//! now, so the future retime verb inherits a tested seam).
//!
//! The seek planner encodes the E-007 lesson positively (DECODER_SPEC D-5):
//! mid-GOP seeks land on the ≤ target keyframe; "Exact" mode means decode-
//! forward-and-drop from that keyframe, never a byte-precise guess.
//!
//! Overflow semantics follow the crate-wide fail-fast contract: every
//! arithmetic step is exact i128-backed ove-time arithmetic; unrepresentable
//! results panic (P12) — the seam adds typed range validation BEFORE
//! arithmetic so caller bugs surface as errors, not panics.

use crate::Clip;
use ove_time::Rational;

/// One clip placement, seen by the seam (exact rationals throughout).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ClipWindow {
    /// Where the clip sits on the timeline (project axis).
    pub timeline_start: Rational,
    /// Timeline duration of the clip.
    pub dur: Rational,
    /// Source pts where the clip starts consuming the asset (source axis).
    pub src_in: Rational,
    /// Playback rate: the clip consumes `dur * speed` of source. 1/1 today;
    /// ≠ 1 when a retime verb lands (the math is pinned by S1 regardless).
    pub speed: Rational,
}

impl ClipWindow {
    /// Placement at natural speed (the only speed the live verb set produces).
    pub fn new(timeline_start: Rational, dur: Rational, src_in: Rational) -> Self {
        ClipWindow {
            timeline_start,
            dur,
            src_in,
            speed: Rational::new(1, 1),
        }
    }

    /// Placement with an explicit playback rate (`speed > 0`; v1 has no
    /// reverse playback — validated by [`validate_ingest`]).
    pub fn retimed(
        timeline_start: Rational,
        dur: Rational,
        src_in: Rational,
        speed: Rational,
    ) -> Self {
        ClipWindow {
            timeline_start,
            dur,
            src_in,
            speed,
        }
    }

    /// Lift a live timeline clip (positional model: `start` is derived by the
    /// caller from the track prefix — the seam takes it explicitly).
    pub fn from_clip(c: &Clip, timeline_start: Rational) -> Self {
        ClipWindow {
            timeline_start,
            dur: c.duration,
            src_in: c.source_in,
            speed: Rational::new(1, 1),
        }
    }

    /// End time on the timeline (`start + dur`, exact).
    pub fn end(&self) -> Rational {
        self.timeline_start.add(self.dur)
    }

    /// Exact source range consumed (`dur * speed`).
    pub fn src_dur(&self) -> Rational {
        self.dur.mul(self.speed)
    }

    /// Source pts at a timeline offset within the clip (`src_in + offset*speed`).
    pub fn src_at(&self, offset: Rational) -> Rational {
        self.src_in.add(offset.mul(self.speed))
    }
}

/// The media-side clock of one asset stream, in exact seconds.
///
/// Constructed by the layer above from probe data; ove-timeline never sees
/// file paths, codecs, or ove-media types.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SourceClock {
    /// Total source duration in seconds (exact; from probe).
    pub duration: Rational,
    /// Keyframe presentation timestamps in seconds, sorted ascending, exact.
    /// Empty for streams without a keyframe index (audio, some stills).
    pub keyframes: Vec<Rational>,
}

impl SourceClock {
    /// Build a clock from raw keyframe pts, enforcing sortedness + range.
    /// (ove-media guarantees sorted `KeyframeIndex`; this is the boundary
    /// re-validation — data crossing a crate boundary is checked, not trusted.)
    pub fn new(duration: Rational, keyframes: Vec<Rational>) -> Result<Self, SeamError> {
        for pair in keyframes.windows(2) {
            if pair[0] >= pair[1] {
                return Err(SeamError::KeyframesNotSorted);
            }
        }
        if let Some(&last) = keyframes.last() {
            if last >= duration {
                return Err(SeamError::KeyframeBeyondDuration {
                    pts: last,
                    duration,
                });
            }
        }
        Ok(SourceClock {
            duration,
            keyframes,
        })
    }

    /// Last keyframe with pts <= t (the D-5 Snap landing point).
    /// None if t is before the first keyframe.
    pub fn keyframe_floor(&self, t: Rational) -> Option<Rational> {
        let idx = self.keyframes.partition_point(|k| *k <= t);
        idx.checked_sub(1).map(|i| self.keyframes[i])
    }
}

/// Typed seam errors — validation happens BEFORE arithmetic; nothing panics
/// for caller-range bugs (overflow panics remain the fail-fast contract for
/// genuinely unrepresentable values, per ADR-007 amendment 2026-09-28).
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum SeamError {
    /// Timeline time not within `[start, end]`.
    TimeOutsideClip { at: Rational },
    /// Source time not within the clip's consumed source range.
    SourceOutsideClip { at: Rational },
    /// Ingest rejection: the clip consumes source before the stream start.
    SourceNegative { src_in: Rational },
    /// Ingest rejection: the clip consumes source beyond the stream duration.
    SourceBeyondDuration {
        requested_end: Rational,
        duration: Rational,
    },
    /// Ingest rejection: zero/negative speed (v1 has no reverse playback).
    SpeedNotPositive { speed: Rational },
    /// Ingest rejection: non-positive clip duration.
    DurationNotPositive { dur: Rational },
    /// frame_span: frame rate must be > 0.
    RateNotPositive { rate_num: i64, rate_den: i64 },
    /// Clock construction: keyframe list not strictly ascending.
    KeyframesNotSorted,
    /// Clock construction: a keyframe at/after the stream duration.
    KeyframeBeyondDuration { pts: Rational, duration: Rational },
    /// Seek planning: target precedes the first keyframe of an indexed stream.
    NoKeyframeAtOrBefore { at: Rational },
}

impl std::fmt::Display for SeamError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SeamError::TimeOutsideClip { at } => write!(f, "time {at} outside clip"),
            SeamError::SourceOutsideClip { at } => write!(f, "source {at} outside clip range"),
            SeamError::SourceNegative { src_in } => write!(f, "source in {src_in} negative"),
            SeamError::SourceBeyondDuration {
                requested_end,
                duration,
            } => write!(
                f,
                "clip consumes up to {requested_end} but source duration is {duration}"
            ),
            SeamError::SpeedNotPositive { speed } => write!(f, "speed {speed} not > 0"),
            SeamError::DurationNotPositive { dur } => write!(f, "duration {dur} not > 0"),
            SeamError::RateNotPositive { rate_num, rate_den } => {
                write!(f, "frame rate {rate_num}/{rate_den} not > 0")
            }
            SeamError::KeyframesNotSorted => write!(f, "keyframes not strictly ascending"),
            SeamError::KeyframeBeyondDuration { pts, duration } => {
                write!(f, "keyframe {pts} at/after duration {duration}")
            }
            SeamError::NoKeyframeAtOrBefore { at } => {
                write!(f, "no keyframe at or before {at}")
            }
        }
    }
}

impl std::error::Error for SeamError {}

/// Validate a window against its source clock AT INGEST (before any placement
/// lands in the timeline). Exact checks only:
/// `speed > 0`, `dur > 0`, `src_in >= 0`, `src_in + dur*speed <= duration`.
pub fn validate_ingest(window: &ClipWindow, clock: &SourceClock) -> Result<(), SeamError> {
    if window.speed.num() <= 0 {
        return Err(SeamError::SpeedNotPositive {
            speed: window.speed,
        });
    }
    if window.dur.num() <= 0 {
        return Err(SeamError::DurationNotPositive { dur: window.dur });
    }
    if window.src_in < Rational::zero(1) {
        return Err(SeamError::SourceNegative {
            src_in: window.src_in,
        });
    }
    let requested_end = window.src_in.add(window.src_dur()); // exact
    if requested_end > clock.duration {
        return Err(SeamError::SourceBeyondDuration {
            requested_end,
            duration: clock.duration,
        });
    }
    Ok(())
}

/// Map a TIMELINE time to the exact SOURCE pts it shows (MEDIA_ENGINE_SPEC §3
/// forward direction). `t` must lie in `[start, end]` (closed at both ends:
/// the last instant of a clip shows the last source frame).
pub fn timeline_to_source(window: &ClipWindow, t: Rational) -> Result<Rational, SeamError> {
    if t < window.timeline_start || t > window.end() {
        return Err(SeamError::TimeOutsideClip { at: t });
    }
    let offset = t.sub(window.timeline_start);
    Ok(window.src_at(offset)) // src_in + offset * speed — exact
}

/// Map a SOURCE pts back to the exact TIMELINE time where it appears (the
/// reverse direction: frame placement — "FrameEnvelope at timeline-remapped
/// pts"). `src` must lie in the consumed range `[src_in, src_in + dur*speed]`.
pub fn source_to_timeline(window: &ClipWindow, src: Rational) -> Result<Rational, SeamError> {
    let src_end = window.src_in.add(window.src_dur());
    if src < window.src_in || src > src_end {
        return Err(SeamError::SourceOutsideClip { at: src });
    }
    let offset_from_in = src.sub(window.src_in);
    let offset = offset_from_in.mul(window.speed.recip()); // exact division
    Ok(window.timeline_start.add(offset))
}

/// The integer output-frame span a clip covers at `frame_rate` (frames/s):
/// `[floor(start·rate), ceil(end·rate))` — the render graph's frame loop
/// bounds. Both bounds are exact (ove-time floor/ceil); the span is
/// half-open: frame index k is rendered iff `k ∈ [first, last)`.
pub fn frame_span(window: &ClipWindow, frame_rate: (i64, i64)) -> Result<(i64, i64), SeamError> {
    if frame_rate.0 <= 0 || frame_rate.1 <= 0 {
        return Err(SeamError::RateNotPositive {
            rate_num: frame_rate.0,
            rate_den: frame_rate.1,
        });
    }
    let first = window
        .timeline_start
        .floor_div_rate(frame_rate.0, frame_rate.1);
    let last = window.end().ceil_div_rate(frame_rate.0, frame_rate.1);
    Ok((first, last))
}

/// Seek plan for ONE decode fetch inside one clip (DECODER_SPEC seek modes,
/// D-4/D-5; E-007 mid-GOP correctness).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SeekPlan {
    /// pts to hand the decoder: the keyframe (Snap) or the exact target
    /// (Exact — decoder decodes forward from the ≤ target keyframe and drops).
    pub land_pts: Rational,
    /// The exact source target (= timeline_to_source of the requested time).
    pub target_pts: Rational,
    /// true if land_pts is the keyframe the caller wants frames from (Snap) —
    /// decode starts there; false means the DECODER owns the drop (Exact).
    pub snap: bool,
}

/// Plan one seek: timeline time → source target → (per mode) keyframe land.
///
/// Snap (the copy-route / preview path): land on the ≤ target keyframe —
/// a keyframe-aligned cut NEVER guesses mid-GOP bytes (E-007 72/72 pattern).
/// Exact (the render path): target is exact; the decoder decodes forward from
/// the ≤ target keyframe and drops until the target (D-4 frame identity).
/// Both modes require a keyframe at/before the target when an index exists.
pub fn plan_seek(
    window: &ClipWindow,
    clock: &SourceClock,
    t_timeline: Rational,
    snap: bool,
) -> Result<SeekPlan, SeamError> {
    let target = timeline_to_source(window, t_timeline)?;
    if clock.keyframes.is_empty() {
        // No index (audio, unindexed stills): only Exact is planable; the
        // decoder reports its own capabilities (DECODER_SPEC capability report).
        return Ok(SeekPlan {
            land_pts: target,
            target_pts: target,
            snap: false,
        });
    }
    let land = clock
        .keyframe_floor(target)
        .ok_or(SeamError::NoKeyframeAtOrBefore { at: target })?;
    Ok(SeekPlan {
        land_pts: land,
        target_pts: target,
        snap,
    })
}
