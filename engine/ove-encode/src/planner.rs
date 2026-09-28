//! Export planner — PURE route logic (ENCODER_SPEC §3 made executable).
//!
//! Decides, per track, how a requested source-time span becomes output:
//! keyframe-aligned stream-copy (E-007/E-007b economics), re-encode, or a
//! mixed segmentation. NO libav, NO I/O — everything here is exact rational
//! arithmetic over a committed keyframe index, so every rule is unit- and
//! property-testable in isolation (tests/planner.rs).
//!
//! Normative rules encoded here (ENCODER_SPEC §3):
//!   1. Cut boundaries MUST resolve to keyframes unless the policy explicitly
//!      allows re-encode at boundaries. Snapping is REPORTED in the receipt —
//!      never silent (rule 3.1).
//!   2. Copy segments are keyframe-start-valid: the first copied packet is a
//!      keyframe, or the copy starts at packet 0 (which is a keyframe in any
//!      closed-GOP source).
//!   3. Audio cut boundaries must land on the codec packet grid (sample-exact);
//!      otherwise the route is re-encode (v1: `Deferred` until the audio
//!      encoder leg lands, W7 — declared, never silently dropped).

use ove_media::StreamId;
use ove_time::Rational;

// ---------------------------------------------------------------------------
// Request side
// ---------------------------------------------------------------------------

/// Half-open time range [start, end) on the SOURCE media axis (exact).
/// Project-axis composition joins at W6; the W3 planner works per segment.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct TimeRange {
    pub start: Rational,
    pub end: Rational,
}

impl TimeRange {
    pub fn new(start: Rational, end: Rational) -> Result<Self, PlanError> {
        if start >= end {
            return Err(PlanError::InvertedRange { start, end });
        }
        Ok(TimeRange { start, end })
    }
    pub fn contains(&self, t: Rational) -> bool {
        t >= self.start && t < self.end
    }
}

/// Snap discipline (ENCODER_SPEC §3.1).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CopyPolicy {
    /// Boundaries resolve to keyframes (round-out end, validity round-in
    /// start). The snapped span is the exported span — reported, consistent.
    KeyframeAlignedOnly,
    /// Non-keyframe boundaries become exact re-encode slivers around the
    /// copyable GOP-aligned core (Mixed route).
    AllowReencodeBoundaries,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TrackKindTag {
    Video,
    Audio,
}

/// Audio packet grid: `frame_samples` at `sample_rate` (e.g. AAC 1024 @ 48k).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct AudioGrid {
    pub frame_samples: u32,
    pub sample_rate: u32,
}

/// Planner input for one track (all exact; index comes from ove-media probe).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TrackInput {
    pub stream_id: StreamId,
    pub kind: TrackKindTag,
    /// Sorted, distinct keyframe presentation timestamps (video).
    pub keyframes: Vec<Rational>,
    /// Exclusive end of the media on the source axis.
    pub media_end: Rational,
    /// Audio grid for alignment checks (audio tracks only).
    pub audio_grid: Option<AudioGrid>,
    /// Whether a re-encode leg can execute for this track in this session.
    pub reencode_available: bool,
}

// ---------------------------------------------------------------------------
// Plan side
// ---------------------------------------------------------------------------

/// One reported snap (ENCODER_SPEC §3.1: "snapping is reported, never
/// silent"). `requested` → `resolved` with the direction named.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SnapRecord {
    pub boundary: Boundary,
    pub requested: Rational,
    pub resolved: Rational,
    pub direction: SnapDirection,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Boundary {
    Start,
    End,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SnapDirection {
    /// Expanded coverage: keyframe floor at start (or EOF at end).
    RoundOut,
    /// Reduced coverage: required for validity when the request starts
    /// before the first keyframe (a copy must open on a keyframe).
    RoundIn,
    /// Aligned to end-of-media.
    ToEof,
}

/// Per-track route decision.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum TrackRoute {
    /// Copy packets in [snapped_start, snapped_end) — keyframe-start-valid;
    /// every snap is recorded.
    StreamCopy {
        span: TimeRange,
        snaps: Vec<SnapRecord>,
    },
    /// Encode this span exactly (no snapping — encoders take any boundary).
    ReEncode { span: TimeRange },
    /// Segmented route: exact requested span = re-encode slivers around a
    /// keyframe-aligned copy core (Mixed; E-7 boundary exactness).
    MixedSegments {
        segments: Vec<(TimeRange, TrackRoute)>,
    },
    /// Planner requires re-encode but the executor leg is not shipped yet
    /// (v1: audio seam re-encode, W7). Declared honesty — never silent.
    Deferred { reason: String },
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PlannedTrack {
    pub stream_id: StreamId,
    pub kind: TrackKindTag,
    pub route: TrackRoute,
}

/// Overall export shape (ENCODER_SPEC §1 `ExportRoute`, planner view).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ExportShape {
    SingleStreamCopy,
    ReEncode,
    /// Per-segment smart-render segmentation (E-7); exact boundaries.
    Mixed,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ExportPlan {
    pub requested: TimeRange,
    pub shape: ExportShape,
    pub tracks: Vec<PlannedTrack>,
}

impl ExportPlan {
    /// Copy spans for a stream (searches both single-copy routes and the
    /// Mixed segmentation's copy core; empty otherwise).
    pub fn copy_span_of(&self, stream_id: StreamId) -> Option<TimeRange> {
        for t in &self.tracks {
            if t.stream_id != stream_id {
                continue;
            }
            match &t.route {
                TrackRoute::StreamCopy { span, .. } => return Some(*span),
                TrackRoute::MixedSegments { segments } => {
                    for (span, r) in segments {
                        if let TrackRoute::StreamCopy { .. } = r {
                            return Some(*span);
                        }
                    }
                }
                _ => {}
            }
        }
        None
    }
    /// All snaps across tracks (the receipt; ENCODER_SPEC §3.1).
    pub fn snaps(&self) -> Vec<(StreamId, SnapRecord)> {
        let mut out = Vec::new();
        for t in &self.tracks {
            if let TrackRoute::StreamCopy { snaps, .. } = &t.route {
                for s in snaps {
                    out.push((t.stream_id, s.clone()));
                }
            }
        }
        out
    }
}

// ---------------------------------------------------------------------------
// Errors
// ---------------------------------------------------------------------------

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum PlanError {
    InvertedRange {
        start: Rational,
        end: Rational,
    },
    EndBeyondMedia {
        end: Rational,
        media_end: Rational,
    },
    /// No keyframes at all: stream copy is impossible (every packet would be
    /// mid-GOP); caller must re-encode.
    NoKeyframes,
    /// After snapping, the copyable span is empty.
    EmptyCopySpan {
        range: TimeRange,
    },
}

impl std::fmt::Display for PlanError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            PlanError::InvertedRange { start, end } => write!(f, "inverted range [{start}, {end})"),
            PlanError::EndBeyondMedia { end, media_end } => {
                write!(f, "end {end} beyond media end {media_end}")
            }
            PlanError::NoKeyframes => write!(f, "no keyframes: stream copy impossible"),
            PlanError::EmptyCopySpan { range } => write!(f, "empty copy span for {range:?}"),
        }
    }
}

impl std::error::Error for PlanError {}

// ---------------------------------------------------------------------------
// Core planning functions
// ---------------------------------------------------------------------------

/// Greatest keyframe <= t (None when t is before the first keyframe).
pub fn kf_floor(keyframes: &[Rational], t: Rational) -> Option<Rational> {
    // binary search: last element <= t
    let mut lo = 0usize;
    let mut hi = keyframes.len();
    while lo < hi {
        let mid = (lo + hi) / 2;
        if keyframes[mid] <= t {
            lo = mid + 1;
        } else {
            hi = mid;
        }
    }
    if lo == 0 {
        None
    } else {
        Some(keyframes[lo - 1])
    }
}

/// Smallest keyframe >= t (None when t is past the last keyframe).
pub fn kf_ceil(keyframes: &[Rational], t: Rational) -> Option<Rational> {
    let mut lo = 0usize;
    let mut hi = keyframes.len();
    while lo < hi {
        let mid = (lo + hi) / 2;
        if keyframes[mid] < t {
            lo = mid + 1;
        } else {
            hi = mid;
        }
    }
    keyframes.get(lo).copied()
}

/// Plan the video copy span for `range` under `policy`.
///
/// Start boundary (validity rule §3.2):
///   * t == keyframe → exact, no snap
///   * t > first keyframe → floor keyframe (RoundOut, reported)
///   * t < first keyframe → first keyframe (RoundIn, reported — a copy must
///     open on a keyframe; leading packets would be undecodable)
///
/// End boundary (KeyframeAlignedOnly):
///   * t == keyframe → exact, no snap
///   * t < last keyframe → ceil keyframe (RoundOut, reported)
///   * t >= last keyframe → media_end (ToEof, reported)
#[allow(clippy::too_many_lines)] // the branch table IS the spec; kept flat on purpose
pub fn plan_video_copy(
    keyframes: &[Rational],
    media_end: Rational,
    range: TimeRange,
    policy: CopyPolicy,
) -> Result<TrackRoute, PlanError> {
    if range.end > media_end {
        return Err(PlanError::EndBeyondMedia {
            end: range.end,
            media_end,
        });
    }
    if keyframes.is_empty() {
        return Err(PlanError::NoKeyframes);
    }

    // ---- start boundary ----
    let first = keyframes[0];
    let (start, mut snaps) = if keyframes.contains(&range.start) {
        (range.start, Vec::new())
    } else if range.start > first {
        let s = kf_floor(keyframes, range.start).expect("start > first => floor exists");
        (
            s,
            vec![SnapRecord {
                boundary: Boundary::Start,
                requested: range.start,
                resolved: s,
                direction: SnapDirection::RoundOut,
            }],
        )
    } else {
        (
            first,
            vec![SnapRecord {
                boundary: Boundary::Start,
                requested: range.start,
                resolved: first,
                direction: SnapDirection::RoundIn,
            }],
        )
    };

    // ---- end boundary ----
    let (end, end_snap) = if keyframes.contains(&range.end) {
        (range.end, None)
    } else {
        match kf_ceil(keyframes, range.end) {
            Some(k) => (
                k,
                Some(SnapRecord {
                    boundary: Boundary::End,
                    requested: range.end,
                    resolved: k,
                    direction: SnapDirection::RoundOut,
                }),
            ),
            None => (
                media_end,
                Some(SnapRecord {
                    boundary: Boundary::End,
                    requested: range.end,
                    resolved: media_end,
                    direction: SnapDirection::ToEof,
                }),
            ),
        }
    };
    if let Some(s) = end_snap {
        snaps.push(s);
    }

    if start >= end {
        return Err(PlanError::EmptyCopySpan { range });
    }

    match policy {
        CopyPolicy::KeyframeAlignedOnly => Ok(TrackRoute::StreamCopy {
            span: TimeRange { start, end },
            snaps,
        }),
        CopyPolicy::AllowReencodeBoundaries => {
            // The copyable core is the same keyframe-aligned span; slivers
            // outside it are exact re-encode segments. Segmentation is
            // handled by `plan_video_mixed`; this fn stays the core builder.
            Ok(TrackRoute::StreamCopy {
                span: TimeRange { start, end },
                snaps,
            })
        }
    }
}

/// Mixed segmentation (policy AllowReencodeBoundaries): the EXACT requested
/// span = re-encode slivers around the copyable GOP-aligned core:
///   * copy core = [kf_ceil(start), kf_floor(end)) — whole GOPs only,
///   * head = [start, core.start) and tail = [core.end, end) re-encoded
///     (partial GOPs at the edges).
///
/// Boundaries are EXACT — E-7 asserts them. (Contrast with
/// `KeyframeAlignedOnly`, which rounds OUT and reports the superset.)
pub fn plan_video_mixed(
    keyframes: &[Rational],
    media_end: Rational,
    range: TimeRange,
) -> Result<Vec<(TimeRange, TrackRoute)>, PlanError> {
    if range.end > media_end {
        return Err(PlanError::EndBeyondMedia {
            end: range.end,
            media_end,
        });
    }
    // No keyframes: nothing copyable — the whole span re-encodes (an exact
    // route exists because encoders take arbitrary boundaries).
    if keyframes.is_empty() {
        return Ok(vec![(range, TrackRoute::ReEncode { span: range })]);
    }
    let core_start = kf_ceil(keyframes, range.start);
    let core_end = kf_floor(keyframes, range.end);
    let (cs, ce) = match (core_start, core_end) {
        (Some(s), Some(e)) if s < e => (s, e),
        // no whole GOP inside the range → single exact re-encode segment
        _ => return Ok(vec![(range, TrackRoute::ReEncode { span: range })]),
    };
    let mut segs: Vec<(TimeRange, TrackRoute)> = Vec::new();
    if range.start < cs {
        segs.push((
            TimeRange {
                start: range.start,
                end: cs,
            },
            TrackRoute::ReEncode {
                span: TimeRange {
                    start: range.start,
                    end: cs,
                },
            },
        ));
    }
    segs.push((
        TimeRange { start: cs, end: ce },
        TrackRoute::StreamCopy {
            span: TimeRange { start: cs, end: ce },
            snaps: Vec::new(),
        },
    ));
    if ce < range.end {
        segs.push((
            TimeRange {
                start: ce,
                end: range.end,
            },
            TrackRoute::ReEncode {
                span: TimeRange {
                    start: ce,
                    end: range.end,
                },
            },
        ));
    }
    Ok(segs)
}

/// Sample-exact audio boundary check: `t` aligns to the packet grid iff
/// t × sample_rate is an exact integer AND divisible by frame_samples.
pub fn audio_aligns(t: Rational, grid: AudioGrid) -> bool {
    let samples = (t.num() as i128) * (grid.sample_rate as i128);
    let den = t.den() as i128;
    if samples % den != 0 {
        return false;
    }
    let whole = samples / den;
    whole % grid.frame_samples as i128 == 0
}

/// Plan one track.
pub fn plan_track(
    input: &TrackInput,
    range: TimeRange,
    policy: CopyPolicy,
) -> Result<PlannedTrack, PlanError> {
    match input.kind {
        TrackKindTag::Video => {
            let route = match policy {
                CopyPolicy::KeyframeAlignedOnly => {
                    plan_video_copy(&input.keyframes, input.media_end, range, policy)?
                }
                CopyPolicy::AllowReencodeBoundaries => {
                    // v1 executor shape: the Mixed segmentation is returned as
                    // the route of record for the track; the single-segment
                    // case collapses to a plain copy or re-encode.
                    let segs = plan_video_mixed(&input.keyframes, input.media_end, range)?;
                    match segs.as_slice() {
                        [(_, TrackRoute::StreamCopy { span, snaps })] => TrackRoute::StreamCopy {
                            span: *span,
                            snaps: snaps.clone(),
                        },
                        [(only_span, TrackRoute::ReEncode { span })] => {
                            let _ = only_span;
                            TrackRoute::ReEncode { span: *span }
                        }
                        // A ReEncode-only multi-segment list (degenerate;
                        // kept total so the planner never panics on odd
                        // indexes) — exact spans are preserved verbatim.
                        _ if segs
                            .iter()
                            .all(|(_, r)| matches!(r, TrackRoute::ReEncode { .. })) =>
                        {
                            let last = segs.last().expect("non-empty").1.clone();
                            match last {
                                TrackRoute::ReEncode { span } => TrackRoute::ReEncode { span },
                                _ => unreachable!("guarded above"),
                            }
                        }
                        _ => {
                            // head/tail slivers + core — represent as Mixed by
                            // delegating to the segment list on the plan; v1
                            // stores the copy core and defers nothing.
                            return Ok(PlannedTrack {
                                stream_id: input.stream_id,
                                kind: input.kind,
                                route: TrackRoute::MixedSegments {
                                    segments: segs.into_iter().collect(),
                                },
                            });
                        }
                    }
                }
            };
            Ok(PlannedTrack {
                stream_id: input.stream_id,
                kind: input.kind,
                route,
            })
        }
        TrackKindTag::Audio => {
            let grid = input.audio_grid.ok_or(PlanError::NoKeyframes)?; // audio needs a grid
            if audio_aligns(range.start, grid) && audio_aligns(range.end, grid) {
                Ok(PlannedTrack {
                    stream_id: input.stream_id,
                    kind: input.kind,
                    route: TrackRoute::StreamCopy {
                        span: range,
                        snaps: Vec::new(),
                    },
                })
            } else if input.reencode_available {
                Ok(PlannedTrack {
                    stream_id: input.stream_id,
                    kind: input.kind,
                    route: TrackRoute::ReEncode { span: range },
                })
            } else {
                Ok(PlannedTrack {
                    stream_id: input.stream_id,
                    kind: input.kind,
                    route: TrackRoute::Deferred {
                        reason: "audio boundaries not sample-aligned; AAC seam re-encode lands "
                            .to_string()
                            + "with the audio wave (W7) — ENCODER_SPEC §3.3",
                    },
                })
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Top-level export planning
// ---------------------------------------------------------------------------

/// Plan a whole export: every track against the SAME requested span, then
/// classify the overall shape (ENCODER_SPEC §1 ExportRoute, planner view).
pub fn plan_export(
    tracks: &[TrackInput],
    range: TimeRange,
    policy: CopyPolicy,
) -> Result<ExportPlan, PlanError> {
    let mut planned = Vec::with_capacity(tracks.len());
    for t in tracks {
        planned.push(plan_track(t, range, policy)?);
    }
    let shape = shape_of(&planned);
    Ok(ExportPlan {
        requested: range,
        shape,
        tracks: planned,
    })
}

/// Shape classification: single copy / re-encode / mixed. A track that only
/// differs by being `Deferred` does not change the copy/encode economics —
/// it is reported on the track route (receipt), not on the shape.
fn shape_of(tracks: &[PlannedTrack]) -> ExportShape {
    let mut any_copy = false;
    let mut any_encode = false;
    for t in tracks {
        match &t.route {
            TrackRoute::StreamCopy { .. } => any_copy = true,
            TrackRoute::ReEncode { .. } => any_encode = true,
            TrackRoute::MixedSegments { segments } => {
                for (_, r) in segments {
                    match r {
                        TrackRoute::StreamCopy { .. } => any_copy = true,
                        TrackRoute::ReEncode { .. } => any_encode = true,
                        _ => {}
                    }
                }
            }
            TrackRoute::Deferred { .. } => {}
        }
    }
    match (any_copy, any_encode) {
        (true, true) => ExportShape::Mixed,
        (true, false) => ExportShape::SingleStreamCopy,
        (false, _) => ExportShape::ReEncode,
    }
}
