//! ove-decode — the decoder abstraction and its first real implementation.
//!
//! Design authority: ADR-004 (as amended by ARCHITECTURE_AUDIT #1),
//! DECODER_SPEC v1 (docs/specs/DECODER_SPEC.md), FRAME_CONTRACT v1.
//!
//! Layout:
//!   * `Decoder` trait — session state machine (open → seek → next → flush →
//!     cancel), pull model; callback backends buffer internally (ADR-004 risk
//!     mitigation, contained inside adapters).
//!   * `ffmpeg` module — the FFmpeg software adapter AND the static probe
//!     backend. **This is the only crate in the workspace permitted to link
//!     libav\*** (DECODER_SPEC §5; CI enforces via dependency + symbol check).

pub mod ffmpeg;

use ove_media::{AssetRef, FrameEnvelope, KeyframeIndex, PixelFormat, StreamId};
use ove_time::Rational;

/// Seek semantics (DECODER_SPEC §1/D-4/D-5).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SeekMode {
    /// Decode forward from the ≤ pts keyframe and drop until exact.
    Exact,
    /// Land on the ≤ pts keyframe (stream-copy route); requires a keyframe
    /// index (injected via `DecodeConfig::keyframe_index`).
    Snap,
}

/// Hardware acceleration request. Absence of support is TYPED
/// (DECODER_SPEC D-9: never a hidden downgrade — the caller chooses SW).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum HwAccel {
    /// Any available hw decoder; still explicit: the caller asked.
    Auto,
    Vendor(&'static str),
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DecodeConfig {
    /// None = software decode. Some(acc) = hardware REQUIRED; a software-only
    /// adapter must reject with `DecodeError::Unsupported` (D-9).
    pub hw: Option<HwAccel>,
    /// Internal decoder threads. Decode output must be bit-identical for any
    /// count on the conformance corpus (DECODER_SPEC §4 determinism note).
    pub thread_count: u8,
    /// Prebuilt keyframe index (ove-media owns it, cached by content hash).
    /// Enables `SeekMode::Snap`; without it Snap reports Unsupported.
    pub keyframe_index: Option<KeyframeIndex>,
}

impl Default for DecodeConfig {
    fn default() -> Self {
        DecodeConfig {
            hw: None,
            thread_count: 1,
            keyframe_index: None,
        }
    }
}

/// Declared, not discovered by crashing (DECODER_SPEC §3).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DecoderCaps {
    /// None for software builds (D-11 honesty).
    pub hw: Option<HwInfo>,
    /// Exact always; Snap present iff a keyframe index is available.
    pub seek_modes: Vec<SeekMode>,
    /// Cpu always for SW; Gpu variants per backend.
    pub memory_outputs: Vec<ove_media::FrameMemory>,
    pub pixel_formats: Vec<PixelFormat>,
    pub max_bit_depth: ove_media::BitDepth,
    /// Internal threading (FFmpeg thread_count).
    pub threaded: bool,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct HwInfo {
    pub vendor: String,
}

/// Typed decode errors (D-8: corrupt inputs are values, never panics, never
/// silent skips).
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum DecodeError {
    /// Container/decoder could not open the asset at all.
    Unreadable,
    /// Opened but the bitstream is broken (truncated GOP, bad packet...).
    Corrupt(String),
    /// The requested configuration is not supported by this backend.
    Unsupported(String),
    /// Cooperative cancel (D-7); session unusable afterwards.
    Cancelled,
    Io(String),
    /// Backend-internal failures that map to no cleaner variant. MUST carry
    /// context; empty Internal strings are a review reject.
    Internal(String),
}

impl std::fmt::Display for DecodeError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            DecodeError::Unreadable => write!(f, "unreadable media"),
            DecodeError::Corrupt(d) => write!(f, "corrupt media: {d}"),
            DecodeError::Unsupported(d) => write!(f, "unsupported: {d}"),
            DecodeError::Cancelled => write!(f, "decode cancelled"),
            DecodeError::Io(d) => write!(f, "io: {d}"),
            DecodeError::Internal(d) => write!(f, "internal: {d}"),
        }
    }
}

impl std::error::Error for DecodeError {}

/// The decoder session state machine (DECODER_SPEC §1).
///
/// Pull model: `next()` delivers the frame at/after the internal cursor;
/// callback-based backends adapt via an internal queue. All times are
/// exact rationals (ADR-007); platform integers convert exactly.
pub trait Decoder: Send {
    /// open + probe capabilities; may reject unsupported configs with a
    /// typed error.
    fn open(asset: &AssetRef, stream: StreamId, cfg: DecodeConfig) -> Result<Self, DecodeError>
    where
        Self: Sized;

    /// What THIS instance can actually do (declared honesty).
    fn capabilities(&self) -> &DecoderCaps;

    /// Seek; returns the landed pts (Exact → the requested pts, armed to
    /// drop earlier frames; Snap → the ≤ pts keyframe's pts).
    fn seek(&mut self, pts: Rational, mode: SeekMode) -> Result<Rational, DecodeError>;

    /// Deliver the next frame at/after the internal cursor.
    /// `Ok(None)` = end of stream.
    fn next(&mut self) -> Result<Option<FrameEnvelope>, DecodeError>;

    /// Reset decoder-internal state; `next()` continues from the last
    /// seek/position (an armed Exact-seek drop target is NOT cleared —
    /// D-6 tests that flush introduces no stale state, not a position reset).
    fn flush(&mut self);

    /// Cooperative cancel: stop at the next safe point; session unusable
    /// afterwards (D-7).
    fn cancel(&mut self);
}
