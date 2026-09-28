//! RenderPlan — the ONLY execution authority (RENDER_GRAPH_SPEC §1).
//!
//! A plan is a pure function of (render input, span): identical input →
//! byte-identical plan, pinned by the canonical hash (RG-1). Passes are
//! ordered (layer order = track order bottom-up, tie-broken by explicit clip
//! ids — no layout algorithms, no hidden z-order). Every time-dependent
//! parameter is an exact rational evaluated at the frame's timeline pts;
//! floats appear NOWHERE in v1 (geometry is integer translation; affine
//! fixed-point lands with real decode sources — documented in compile.rs).

use ove_media::ColorTags;
use ove_time::Rational;

/// Working-space declaration: output geometry + cadence + color tags.
/// All five color tags REQUIRED (FRAME_CONTRACT §4 — unknown is a value).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct OutputSpec {
    pub width: u32,
    pub height: u32,
    /// Output frame cadence (exact rational rate, e.g. (30000, 1001)).
    pub rate_num: i64,
    pub rate_den: i64,
    /// The project's working color space (RG-7: conversion happens ONCE,
    /// here — never scattered across adapters).
    pub working_space: ColorTags,
}

impl OutputSpec {
    /// Exact pts of output frame index `k` (frame k spans [k/rate, (k+1)/rate)).
    pub fn frame_pts(&self, k: i64) -> Rational {
        Rational::new(k * self.rate_den, self.rate_num)
    }
}

/// Half-open output frame range `[first, first+count)` at the output rate.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct RenderSpan {
    pub first: i64,
    pub count: u64,
}

/// A source registered in the plan's resource table. Identity is the engine's
/// asset/stream key; ove-render never sees paths or codecs (ADR-013 layering).
pub type SourceId = u64;

/// Surfaces a pass reads/writes. v1 software executor owns exactly these.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum SurfaceId {
    /// The composited output frame (RGBA8, working space).
    Output,
    /// One scratch surface per pass chain step (packed tightly by compile).
    Scratch(u16),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PassKind {
    /// Pull the source frame covering `src_pts` (exact, source axis) into a
    /// scratch surface. The executor resolves `src_pts` → concrete frame via
    /// the FrameSource (greatest pts ≤ target; DECODER_SPEC D-5 floor rule).
    SourceFetch { source: SourceId, src_pts: Rational },
    /// Working-space reconciliation (FRAME_CONTRACT §4.2: happens ONCE, here).
    /// v1 RGBA path: tag stamping (pixels unchanged); matrix conversion is a
    /// no-op for RGB data and becomes pixel work with YUV decode (W6).
    ColorConvert { to: ColorTags },
    /// Integer translation (geometry coefficients; documented float-free v1).
    /// dx/dy are output-pixel offsets of the source frame's top-left corner;
    /// regions outside the frame are transparent.
    Transform { dx: i32, dy: i32 },
    /// Straight-alpha "over" blend of the input surface onto the output.
    /// Exact rational alpha ∈ [0, 1]; blending is integer arithmetic
    /// (u16 intermediates, round-half-up) — deterministic everywhere.
    BlendOver { alpha: Rational },
}

/// One ordered execution step.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Pass {
    pub kind: PassKind,
    /// Explicit clip identity that produced this pass (tie-break provenance).
    pub clip_id: u64,
    pub inputs: Vec<SurfaceId>,
    pub outputs: Vec<SurfaceId>,
}

/// A compiled, ready-to-execute plan for ONE output frame.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RenderPlan {
    pub passes: Vec<Pass>,
    pub output: OutputSpec,
    /// The frame index this plan compiles (plans are per-frame in v1; the
    /// compile entry point takes the span and callers compile per frame).
    pub frame_index: i64,
}

impl RenderPlan {
    /// Canonical FNV-1a hash over the pass list (RG-1 determinism probe).
    /// Same discipline as the timeline state hash: stable byte layout,
    /// wrapping arithmetic, no dependency on HashMap iteration order.
    pub fn canonical_hash(&self) -> u64 {
        let mut h: u64 = 0xcbf2_9ce4_8422_2325;
        let mut w = |bytes: &[u8]| {
            for b in bytes {
                h ^= *b as u64;
                h = h.wrapping_mul(0x0000_0100_0000_01b3);
            }
        };
        w(b"ove-render-plan-v1");
        w(&self.frame_index.to_le_bytes());
        w(&(self.passes.len() as u32).to_le_bytes());
        for p in &self.passes {
            w(&(p.clip_id).to_le_bytes());
            match p.kind {
                PassKind::SourceFetch { source, src_pts } => {
                    w(&[0u8]);
                    w(&source.to_le_bytes());
                    w(&src_pts.num().to_le_bytes());
                    w(&src_pts.den().to_le_bytes());
                }
                PassKind::ColorConvert { .. } => {
                    // tags are constant per plan (the working space) — the
                    // discriminant suffices for plan-identity purposes
                    w(&[1u8]);
                }
                PassKind::Transform { dx, dy } => {
                    w(&[2u8]);
                    w(&dx.to_le_bytes());
                    w(&dy.to_le_bytes());
                }
                PassKind::BlendOver { alpha } => {
                    w(&[3u8]);
                    w(&alpha.num().to_le_bytes());
                    w(&alpha.den().to_le_bytes());
                }
            }
            w(&(p.inputs.len() as u32).to_le_bytes());
            for i in &p.inputs {
                w(&surface_byte(*i));
            }
            w(&(p.outputs.len() as u32).to_le_bytes());
            for o in &p.outputs {
                w(&surface_byte(*o));
            }
        }
        h
    }
}

fn surface_byte(s: SurfaceId) -> [u8; 5] {
    match s {
        SurfaceId::Output => [0, 0, 0, 0, 0],
        SurfaceId::Scratch(n) => [1, 0, (n & 0xff) as u8, (n >> 8) as u8, 0],
    }
}
