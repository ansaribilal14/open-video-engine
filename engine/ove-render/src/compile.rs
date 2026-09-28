//! The RenderPlan compiler — a PURE function (RENDER_GRAPH_SPEC §2, RG-1).
//!
//! Purity spine:
//!   * identical input + span → identical plan sequence (canonical-hash equal),
//!     across runs AND threads;
//!   * layer order = track order bottom-up (Vec order of [`RenderInput.tracks`]),
//!     placements in start order, tie-broken by explicit clip ids;
//!   * every time-dependent parameter is exact rational arithmetic at the
//!     frame's timeline pts (seam: `ove_timeline::mapping::timeline_to_source`);
//!   * no floats anywhere in v1 (geometry = integer translation; affine
//!     fixed-point lands with real decode sources in W6);
//!   * culling (§2.5) only when provably output-identical, and RG-6 property-
//!     tests culled vs unculled output byte-equality.

use crate::plan::{OutputSpec, Pass, PassKind, RenderPlan, RenderSpan, SourceId, SurfaceId};
use ove_media::ColorTags;
use ove_time::Rational;
use ove_timeline::mapping::{timeline_to_source, ClipWindow};

/// One clip placement as the compiler sees it. Built by the engine layer from
/// the timeline (`TrackOps::walk` gives derived starts) + probe records.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Placement {
    /// Explicit clip identity (tie-break provenance; never positional).
    pub clip_id: u64,
    /// Exact placement (timeline start/dur/src_in/speed — ADR-013 seam).
    pub window: ClipWindow,
    /// Which source (asset+stream) this placement reads.
    pub source: SourceId,
    /// Constant opacity for v1 (keyframes land in wave 8 — same compile rule:
    /// any time-dependent value becomes exact-rational-at-frame-pts).
    pub alpha: Rational,
    /// Integer pixel offset of the source frame's top-left in the output.
    pub offset: (i32, i32),
    /// The source stream's declared color tags (from probe — known at ingest,
    /// NOT per-frame data). Drives the RG-7 single-conversion decision.
    pub src_color: ColorTags,
}

/// One compositing track, bottom-up order (index 0 = bottom layer).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TrackInput {
    /// Placements in start order (the timeline's walk order).
    pub placements: Vec<Placement>,
}

/// The compiler's view of the project state (v1 stand-in for ProjectState;
/// the engine layer builds it from Timeline + probe records in W5).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RenderInput {
    pub tracks: Vec<TrackInput>,
    pub output: OutputSpec,
}

/// The compile error surface. Compile is total over well-formed input; these
/// are the malformed cases (exact rationals make range bugs loud here rather
/// than silent in the executor).
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum CompileError {
    /// Placement consumes source before t=0 (ingest gate bypassed upstream).
    SourceNegative { clip_id: u64 },
    /// Placement outlives its source per the ingest rule (ADR-013).
    SourceBeyondClip { clip_id: u64 },
    /// Alpha outside [0, 1].
    AlphaOutOfRange { clip_id: u64, alpha: Rational },
    /// Zero/negative frame rate or non-positive geometry in the output spec.
    InvalidOutput,
}

impl std::fmt::Display for CompileError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            CompileError::SourceNegative { clip_id } => {
                write!(f, "placement {clip_id} consumes negative source time")
            }
            CompileError::SourceBeyondClip { clip_id } => {
                write!(f, "placement {clip_id} exceeds its declared source range")
            }
            CompileError::AlphaOutOfRange { clip_id, alpha } => {
                write!(f, "placement {clip_id} alpha {alpha} outside [0,1]")
            }
            CompileError::InvalidOutput => write!(f, "invalid output spec"),
        }
    }
}

impl std::error::Error for CompileError {}

fn validate_placement(p: &Placement) -> Result<(), CompileError> {
    if p.window.src_in < Rational::zero(1) {
        return Err(CompileError::SourceNegative { clip_id: p.clip_id });
    }
    if p.alpha < Rational::zero(1) || p.alpha > Rational::new(1, 1) {
        return Err(CompileError::AlphaOutOfRange {
            clip_id: p.clip_id,
            alpha: p.alpha,
        });
    }
    Ok(())
}

/// Compile ONE output frame. Pure: no clocks, no IO, no thread state.
pub fn compile_frame(input: &RenderInput, frame_index: i64) -> Result<RenderPlan, CompileError> {
    if input.output.width == 0
        || input.output.height == 0
        || input.output.rate_num <= 0
        || input.output.rate_den <= 0
    {
        return Err(CompileError::InvalidOutput);
    }
    let t = input.output.frame_pts(frame_index);
    let t_end_incl = t; // a frame belongs to the clip covering its start instant
    let _ = t_end_incl;
    let mut passes: Vec<Pass> = Vec::new();
    let mut next_scratch: u16 = 0;

    for (ti, track) in input.tracks.iter().enumerate() {
        for p in &track.placements {
            // timeline coverage: half-open [start, end) — a frame belongs to
            // the clip covering its start instant (frame_pts = frame start)
            if t < p.window.timeline_start || t >= p.window.end() {
                continue;
            }
            validate_placement(p)?;
            // CULLING (provably output-identical only, §2.5):
            // (a) fully transparent layer contributes nothing to an
            //     "over" composite;
            if p.alpha == Rational::zero(1) {
                continue;
            }
            // (b) translated region entirely outside the output frame.
            let (dx, dy) = (p.offset.0, p.offset.1);
            // source dims are unknown at compile time (frame data at exec);
            // a placement can only be provably off-frame when its OFFSET
            // already pushes the top-left corner fully past an edge AND the
            // executor clips bottom/right — the conservative check here:
            // offset beyond the right/bottom edge can never re-enter.
            if dx as i64 >= input.output.width as i64 || dy as i64 >= input.output.height as i64 {
                continue;
            }
            let _ = ti;

            // exact source pts at this frame's timeline instant (the seam)
            let src_pts = timeline_to_source(&p.window, t)
                .map_err(|_| CompileError::SourceBeyondClip { clip_id: p.clip_id })?;

            let s0 = SurfaceId::Scratch(next_scratch);
            next_scratch += 1;
            passes.push(Pass {
                kind: PassKind::SourceFetch {
                    source: p.source,
                    src_pts,
                },
                clip_id: p.clip_id,
                inputs: vec![],
                outputs: vec![s0],
            });
            let mut cur = s0;
            // RG-7: convert ONCE, only when the declared source tags differ
            // from the working space; the conversion is explicitly tagged.
            if p.src_color != input.output.working_space {
                let s1 = SurfaceId::Scratch(next_scratch);
                next_scratch += 1;
                passes.push(Pass {
                    kind: PassKind::ColorConvert {
                        to: input.output.working_space,
                    },
                    clip_id: p.clip_id,
                    inputs: vec![cur],
                    outputs: vec![s1],
                });
                cur = s1;
            }
            if dx != 0 || dy != 0 {
                let s1 = SurfaceId::Scratch(next_scratch);
                next_scratch += 1;
                passes.push(Pass {
                    kind: PassKind::Transform { dx, dy },
                    clip_id: p.clip_id,
                    inputs: vec![cur],
                    outputs: vec![s1],
                });
                cur = s1;
            }
            passes.push(Pass {
                kind: PassKind::BlendOver { alpha: p.alpha },
                clip_id: p.clip_id,
                inputs: vec![cur],
                outputs: vec![SurfaceId::Output],
            });
        }
    }

    Ok(RenderPlan {
        passes,
        output: input.output.clone(),
        frame_index,
    })
}

/// Compile a whole span: one plan per output frame (still pure — the sequence
/// is a deterministic function of (input, span)). This is the §1 signature
/// shape; per-frame plans keep the software executor trivially simple.
pub fn compile_span(
    input: &RenderInput,
    span: RenderSpan,
) -> Result<Vec<RenderPlan>, CompileError> {
    (span.first..span.first + span.count as i64)
        .map(|k| compile_frame(input, k))
        .collect()
}
