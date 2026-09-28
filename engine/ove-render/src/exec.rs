//! Software reference executor — CPU RGBA, single-threaded, deterministic
//! (RENDER_GRAPH_SPEC §3). THE correctness reference: every future backend
//! (GPU wave 6, platform legs) must match this byte-for-byte on golden frames.
//!
//! Determinism rules:
//!   * compositing is integer-only: straight-alpha "over" onto an OPAQUE
//!     output (black base), u64 intermediates, round-half-up — identical on
//!     every platform;
//!   * scheduling is exact rational (frame pts from OutputSpec::frame_pts;
//!     source pts from the plan's SourceFetch params) — never fp;
//!   * the executor never reorders, skips, or merges passes (§2.4).

use std::collections::HashMap;

use crate::plan::{PassKind, RenderPlan, SourceId, SurfaceId};
use ove_media::{BackendId, BitDepth, FrameBytes, FrameEnvelope, PixelFormat, StreamId};
use ove_time::Rational;

/// A source of decoded/synthetic frames, resolved by the executor.
///
/// Contract: `fetch(target)` returns the frame with the GREATEST pts ≤ target
/// (the D-5 floor rule — the same discipline keyframe-aligned seeks use), or
/// None when target precedes the first frame. Frames MUST be Cpu RGBA8.
pub trait FrameSource {
    fn fetch(&self, target: Rational) -> Option<FrameEnvelope>;
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum RenderError {
    /// No frame at/before the requested source pts.
    SourceFrameMissing { source: SourceId, at: Rational },
    /// The fetched frame is not Cpu RGBA8 (FrameSource contract violation).
    FrameNotCpuRgba { source: SourceId },
    /// The fetched frame carries tags ≠ the plan assumed at compile time
    /// (probe lied): the plan has NO ColorConvert for this placement, so
    /// honoring it blindly would silently skip the single-conversion rule
    /// (RG-7 / FRAME_CONTRACT §4.2).
    TagMismatch {
        source: SourceId,
        frame: ove_media::ColorTags,
        expected: ove_media::ColorTags,
    },
    /// Executor bug guard: pass graph references an un-produced surface.
    MissingSurface(SurfaceId),
}

impl std::fmt::Display for RenderError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            RenderError::SourceFrameMissing { source, at } => {
                write!(f, "source {source} has no frame at/before {at}")
            }
            RenderError::FrameNotCpuRgba { source } => {
                write!(f, "source {source} delivered a non-Cpu-RGBA8 frame")
            }
            RenderError::TagMismatch {
                source,
                frame,
                expected,
            } => write!(
                f,
                "source {source} frame tags {frame:?} != declared {expected:?} (no convert pass in plan)"
            ),
            RenderError::MissingSurface(s) => write!(f, "surface {s:?} missing at execution"),
        }
    }
}

impl std::error::Error for RenderError {}

/// One execution surface: pixels + geometry + tags.
struct Surface {
    data: Vec<u8>,
    width: u32,
    height: u32,
    tags: ove_media::ColorTags,
}

/// The v1 software renderer. Sources are registered by id; plans execute
/// frame-at-a-time (frame-level parallelism later is safe by plan purity).
pub struct SoftwareRenderer<'a> {
    sources: HashMap<SourceId, &'a dyn FrameSource>,
}

impl<'a> SoftwareRenderer<'a> {
    pub fn new(sources: HashMap<SourceId, &'a dyn FrameSource>) -> Self {
        SoftwareRenderer { sources }
    }

    /// Execute one compiled plan → one output FrameEnvelope (Cpu RGBA8,
    /// opaque, working-space tags, timeline pts set).
    pub fn execute_frame(&self, plan: &RenderPlan) -> Result<FrameEnvelope, RenderError> {
        let spec = &plan.output;
        let px = (spec.width as usize) * (spec.height as usize) * 4;
        // OPAQUE black base (video convention; FRAME_CONTRACT §4.2: the
        // output is declared in the working space).
        let mut output = Surface {
            data: vec![0u8; px],
            width: spec.width,
            height: spec.height,
            tags: spec.working_space,
        };
        for b in output.data.iter_mut().skip(3).step_by(4) {
            *b = 255;
        }
        let mut scratches: HashMap<SurfaceId, Surface> = HashMap::new();

        for pass in &plan.passes {
            match pass.kind {
                PassKind::SourceFetch { source, src_pts } => {
                    let src = self
                        .sources
                        .get(&source)
                        .ok_or(RenderError::SourceFrameMissing {
                            source,
                            at: src_pts,
                        })?;
                    let env = src.fetch(src_pts).ok_or(RenderError::SourceFrameMissing {
                        source,
                        at: src_pts,
                    })?;
                    let bytes = env
                        .cpu_bytes()
                        .ok_or(RenderError::FrameNotCpuRgba { source })?;
                    if env.pixel_format != PixelFormat::Rgba || env.bit_depth != BitDepth::B8 {
                        return Err(RenderError::FrameNotCpuRgba { source });
                    }
                    // The single-conversion rule: the plan carries a
                    // ColorConvert pass exactly when declared tags ≠ working
                    // space. A frame that arrives mismatched WITHOUT a convert
                    // pass means the probe declaration lied — fail loudly.
                    if env.color != spec.working_space {
                        let has_convert = plan.passes.iter().any(|p| {
                            p.clip_id == pass.clip_id
                                && matches!(p.kind, PassKind::ColorConvert { .. })
                        });
                        if !has_convert {
                            return Err(RenderError::TagMismatch {
                                source,
                                frame: env.color,
                                expected: spec.working_space,
                            });
                        }
                    }
                    // Normalize into an OUTPUT-SIZED transparent surface,
                    // pasting the fetched frame at the origin (clipped). Every
                    // downstream surface is output-sized — blends zip 1:1.
                    let ow = spec.width as usize;
                    let oh = spec.height as usize;
                    let sw = env.width as usize;
                    let sh = env.height as usize;
                    let stride = bytes.strides.first().copied().unwrap_or(sw * 4);
                    let mut data = vec![0u8; ow * oh * 4];
                    for y in 0..sh.min(oh) {
                        let from = y * stride;
                        let copy = (sw.min(ow)) * 4;
                        data[y * ow * 4..y * ow * 4 + copy]
                            .copy_from_slice(&bytes.data[from..from + copy]);
                    }
                    scratches.insert(
                        s_of(pass.outputs.first().copied()),
                        Surface {
                            data,
                            width: spec.width,
                            height: spec.height,
                            tags: env.color,
                        },
                    );
                }
                PassKind::ColorConvert { to } => {
                    // v1 RGBA path: tag stamp only (FRAME_CONTRACT §4.2 — the
                    // conversion is declared in the plan; pixel-level matrix
                    // work applies to YUV sources and lands with decode W6).
                    let in_id = s_of(pass.inputs.first().copied());
                    let out_id = s_of(pass.outputs.first().copied());
                    let mut s = scratches
                        .remove(&in_id)
                        .ok_or(RenderError::MissingSurface(in_id))?;
                    s.tags = to;
                    scratches.insert(out_id, s);
                }
                PassKind::Transform { dx, dy } => {
                    let input = scratches
                        .remove(&s_of(pass.inputs.first().copied()))
                        .ok_or(RenderError::MissingSurface(s_of(
                            pass.inputs.first().copied(),
                        )))?;
                    debug_assert_eq!(input.width, spec.width);
                    debug_assert_eq!(input.height, spec.height);
                    let mut data = vec![0u8; px];
                    // TRANSPARENT outside the translated region (alpha 0) —
                    // the following blend-over must leave the destination
                    // unchanged there. Only the output itself is opaque.
                    // translate the surface content by (dx, dy), clipping at
                    // the frame edges (integer clip — deterministic)
                    let ow = spec.width as i64;
                    let oh = spec.height as i64;
                    let x0 = (dx as i64).max(0);
                    let y0 = (dy as i64).max(0);
                    let x1 = (ow + dx as i64).min(ow);
                    let y1 = (oh + dy as i64).min(oh);
                    for y in y0..y1 {
                        let sy = (y - dy as i64) as usize;
                        let src_row = sy * (ow as usize) * 4;
                        let dst_row = (y as usize) * (ow as usize) * 4;
                        for x in x0..x1 {
                            let sx = (x - dx as i64) as usize;
                            let from = src_row + sx * 4;
                            let to = dst_row + (x as usize) * 4;
                            data[to..to + 4].copy_from_slice(&input.data[from..from + 4]);
                        }
                    }
                    let tags = input.tags;
                    scratches.insert(
                        s_of(pass.outputs.first().copied()),
                        Surface {
                            data,
                            width: spec.width,
                            height: spec.height,
                            tags,
                        },
                    );
                }
                PassKind::BlendOver { alpha } => {
                    let input = scratches
                        .remove(&s_of(pass.inputs.first().copied()))
                        .ok_or(RenderError::MissingSurface(s_of(
                            pass.inputs.first().copied(),
                        )))?;
                    // integer straight-alpha over an opaque dst
                    // N_s = src_alpha * alpha_num ; D = 255 * alpha_den
                    let d = 255u64 * alpha.den() as u64;
                    let a_num = alpha.num().max(0) as u64;
                    let dst_pixels = output.data.as_chunks_mut::<4>().0.iter_mut();
                    let src_pixels = input.data.as_chunks::<4>().0.iter();
                    for (dst_px, src_px) in dst_pixels.zip(src_pixels) {
                        let sa = src_px[3] as u64;
                        let n_s = (sa * a_num).min(d); // clamp: exec does not trust plan validation
                        let inv = d - n_s;
                        for c in 0..3 {
                            let cs = src_px[c] as u64;
                            let cd = dst_px[c] as u64;
                            // round-half-up division
                            dst_px[c] = ((cs * n_s + cd * inv + d / 2) / d).min(255) as u8;
                        }
                        // output stays opaque
                        dst_px[3] = 255;
                    }
                }
            }
        }

        let pts = spec.frame_pts(plan.frame_index);
        let duration = Rational::new(spec.rate_den, spec.rate_num);
        let mut env = FrameEnvelope::video_cpu(
            pts,
            duration,
            StreamId(0), // render output stream (producer-tagged, not an asset)
            spec.width,
            spec.height,
            PixelFormat::Rgba,
            BitDepth::B8,
            spec.working_space,
            FrameBytes {
                data: output.data,
                strides: vec![(spec.width as usize) * 4],
            },
            plan.frame_index == 0, // first frame is the seek entry point
            BackendId::Other("ove-render/software".into()),
            0, // no pool yet (v1 allocates; pool wiring lands with decode W6)
        );
        env.timeline_pts = Some(pts);
        Ok(env)
    }
}

fn s_of(v: Option<SurfaceId>) -> SurfaceId {
    v.unwrap_or(SurfaceId::Output)
}
