//! ove-media — the libav-free media data model of the open-video-engine.
//!
//! Design authority: FRAME_CONTRACT (docs/specs/FRAME_CONTRACT.md), DECODER_SPEC
//! (docs/specs/DECODER_SPEC.md), ADR-004/006/007.
//!
//! This crate owns, deliberately:
//!   * asset identity by content hash (blake3, S-3d7) — the asset is the cache key
//!   * the probe data model (container/streams/color/keyframe index/VFR) and the
//!     `ProbeBackend` trait — backends live OUTSIDE this crate (libav linkage is
//!     confined to ove-decode; DECODER_SPEC §5 gate)
//!   * the ONLY frame type that crosses crate boundaries: `FrameEnvelope`
//!   * the CPU frame pool with generation-based staleness detection
//!
//! It never links libav*, wgpu, or any platform media API — on purpose. Every
//! consumer (ove-decode, ove-render, ove-encode) and every future platform leg
//! (WASM browser core) can depend on these types without transitive C linkage.

pub mod asset;
pub mod frame;
pub mod pool;
pub mod probe;

pub use asset::{AssetRef, ContentHash, StreamId, StreamKind};
pub use frame::{
    BackendId, BitDepth, ChromaLoc, ColorTags, FrameBytes, FrameEnvelope, FrameError, FrameKind,
    FrameMemory, GpuFrameRef, GpuHandle, GpuUploadKind, MatrixCoeffs, PackedSlice, PixelFormat,
    Primaries, Range, Transfer,
};
pub use pool::{FramePool, PoolKey, PoolStats, PooledBuffer};
pub use probe::{
    AudioDetails, ContainerKind, KeyframeEntry, KeyframeIndex, MetadataCache, ProbeBackend,
    ProbeError, ProbeInfo, ProbeStream, VfrReport, VideoDetails,
};
