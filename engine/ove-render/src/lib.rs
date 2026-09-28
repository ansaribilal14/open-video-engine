//! ove-render — the software reference renderer (RENDER_GRAPH_SPEC; BUILD_PLAN
//! wave 3 / directive wave 2).
//!
//! Software-first (directive §11): THIS crate is the correctness reference.
//! GPU (wave 6) and platform legs are backends that must match these bytes on
//! golden frames — the plan is the only execution authority, and the executor
//! never reorders, skips, or merges passes.
//!
//! Structure:
//!   * [`plan`]   — RenderPlan/Pass/PassKind: the compiled execution authority,
//!     with a canonical hash (the RG-1 determinism probe).
//!   * [`compile`]— the PURE compiler (input+span → plans). Layer order =
//!     track order bottom-up; exact rationals for everything time-dependent;
//!     culling only when provably output-identical (RG-6 property-pinned).
//!   * [`exec`]   — the software executor: integer straight-alpha compositing
//!     onto an opaque output, FrameSource-resolved fetch (D-5 floor rule),
//!     single color conversion (FRAME_CONTRACT §4.2).
//!
//! v1 honesty notes (documented, not hidden):
//!   * sources are Cpu RGBA8 (synthetic in tests; real decode binding lands
//!     with the W6 vertical slice — Decode passes then pull ove-decode
//!     sessions behind the same FrameSource trait);
//!   * ColorConvert is a tag stamp for RGB data; YUV matrix conversion arrives
//!     with decode (the plan's convert-pass structure is already the once-
//!     only rule RG-7 asserts);
//!   * Transform is integer translation; affine (fixed-point) lands with real
//!     sources. Floats appear nowhere, per RENDER_GRAPH_SPEC §1 note.
//!
//! Design authority: RENDER_GRAPH_SPEC, FRAME_CONTRACT, ADR-002/006/013.

pub mod compile;
pub mod exec;
pub mod plan;

pub use compile::{compile_frame, compile_span, CompileError, Placement, RenderInput, TrackInput};
pub use exec::{FrameSource, RenderError, SoftwareRenderer};
pub use plan::{OutputSpec, Pass, PassKind, RenderPlan, RenderSpan, SourceId, SurfaceId};
