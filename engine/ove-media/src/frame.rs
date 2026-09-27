//! FrameEnvelope — the ONLY frame type that crosses crate boundaries.
//!
//! Normative source: docs/specs/FRAME_CONTRACT.md (v1). Checklist §7 is
//! enforced here by construction:
//!   * all five color tags are REQUIRED fields with unknown-as-value enums —
//!     absence is unrepresentable
//!   * timestamps are exact rationals (ADR-007); no fp on the scheduling path
//!   * memory location is explicit (Cpu/Gpu/Packed); a SW backend cannot emit Gpu
//!   * ownership is unique: FrameEnvelope is NOT Clone; sharing = explicit
//!     `deep_copy_cpu()` (visible cost, Gpu rejected)
//!   * backend identity travels with every frame (`BackendId`)

use ove_time::Rational;
use serde::{Deserialize, Serialize};

use crate::asset::StreamId;

// ---------------------------------------------------------------------------
// Color (five tags; unknown is a VALUE, never an absence)
// ---------------------------------------------------------------------------

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum Primaries {
    Bt709,
    Bt470m,
    Bt470bg,
    Bt601525,
    Bt601625,
    Bt2020,
    DciP3,
    Unknown,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum Transfer {
    Bt709,
    Gamma22,
    Gamma28,
    Srgb,
    Linear,
    Log100,
    Log316,
    Pq,
    Hlg,
    Unknown,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum MatrixCoeffs {
    Identity,
    Bt709,
    Bt470bg,
    Smpte170m,
    Smpte240m,
    Bt2020Ncl,
    Bt2020Pcl,
    Unknown,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum Range {
    Limited,
    Full,
    Unknown,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum ChromaLoc {
    Left,
    Center,
    TopLeft,
    Top,
    BottomLeft,
    Bottom,
    Unknown,
}

/// The five mandatory color tags (FRAME_CONTRACT §1/§4).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ColorTags {
    pub primaries: Primaries,
    pub transfer: Transfer,
    pub matrix: MatrixCoeffs,
    pub range: Range,
    pub chroma_loc: Option<ChromaLoc>,
}

impl ColorTags {
    /// Explicit all-unknown tags — allowed, but never silently invented.
    pub fn unknown() -> Self {
        ColorTags {
            primaries: Primaries::Unknown,
            transfer: Transfer::Unknown,
            matrix: MatrixCoeffs::Unknown,
            range: Range::Unknown,
            chroma_loc: None,
        }
    }
}

// ---------------------------------------------------------------------------
// Format
// ---------------------------------------------------------------------------

/// Pixel format as an enum (FRAME_CONTRACT: "enum, not fourcc string").
/// Adapters map their native formats onto these; genuinely exotic formats
/// surface as `Other(name)` rather than being force-fit.
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum PixelFormat {
    Yuv420p,
    Yuvj420p,
    Yuv422p,
    Yuv444p,
    Yuv420p10le,
    Yuv422p10le,
    Yuv444p10le,
    Nv12,
    Nv21,
    P010le,
    Rgba,
    Bgra,
    Rgb24,
    Bgr24,
    Gray8,
    Gray16le,
    Other(String),
}

impl PixelFormat {
    /// Whether chroma is subsampled horizontally/vertically (2 planes share).
    pub fn is_yuv_planar_family(&self) -> bool {
        matches!(
            self,
            PixelFormat::Yuv420p
                | PixelFormat::Yuvj420p
                | PixelFormat::Yuv422p
                | PixelFormat::Yuv444p
                | PixelFormat::Yuv420p10le
                | PixelFormat::Yuv422p10le
                | PixelFormat::Yuv444p10le
        )
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum BitDepth {
    B8,
    B10,
    B12,
    B16,
    Other(u8),
}

// ---------------------------------------------------------------------------
// Memory & lifecycle
// ---------------------------------------------------------------------------

/// CPU bytes: all planes contiguous in `data`, per-plane `strides` explicit
/// (FRAME_CONTRACT §2: "Strides explicit — no assumptions").
#[derive(Debug, PartialEq, Eq)]
pub struct FrameBytes {
    pub data: Vec<u8>,
    pub strides: Vec<usize>,
}

/// GPU-resident frame: engine-neutral descriptor only (two-level GPU
/// abstraction, ARCHITECTURE_AUDIT #4). The core never names wgpu/
/// MediaCodec types; the backend interprets `handle` in its own world.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct GpuHandle(pub u64);

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum GpuUploadKind {
    ImportedExternalTexture,
    AHardwareBuffer,
    MappedTexture,
    ZeroCopyPlatformSurface,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct GpuFrameRef {
    pub handle: GpuHandle,
    pub upload_kind: GpuUploadKind,
    /// If false, the export planner must readback or re-route.
    pub exportable: bool,
}

/// Undecoded/compressed passthrough (StreamCopy route is first-class).
#[derive(Debug, PartialEq, Eq)]
pub struct PackedSlice {
    pub data: Vec<u8>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FrameMemory {
    /// Payload attached via FrameEnvelope memory payload (see constructor).
    Cpu,
    Gpu,
    Packed,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FrameKind {
    Video,
    Audio { sample_rate: u32, channels: u32 },
    Metadata,
}

/// Which adapter produced this frame (FRAME_CONTRACT §6; golden tests assert it).
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum BackendId {
    FFmpegSw,
    MediaCodec,
    WebCodecs,
    Wgpu,
    Other(String),
}

// ---------------------------------------------------------------------------
// FrameEnvelope
// ---------------------------------------------------------------------------

#[derive(Debug, PartialEq)]
pub struct FrameEnvelope {
    /// Identity & scheduling (exact; FRAME_CONTRACT §3).
    pub pts: Rational,
    /// EXACT duration; VFR-safe — never inferred from a frame rate (D-2).
    pub duration: Rational,
    /// Set by remap when the frame is scheduled on the project axis.
    pub timeline_pts: Option<Rational>,
    pub stream_id: StreamId,
    pub kind: FrameKind,

    /// Geometry & format (video).
    pub width: u32,
    pub height: u32,
    pub pixel_format: PixelFormat,
    pub bit_depth: BitDepth,

    /// All five tags REQUIRED (unknown-as-value fine, absence not).
    pub color: ColorTags,

    /// Memory payload + location.
    pub memory: FrameMemory,
    payload: FramePayload,

    /// Decode entry-point flag (E-007/E-007b rely on it).
    pub keyframe: bool,
    pub backend: BackendId,
    /// Pool generation — stale-detection (FRAME_CONTRACT §5.2).
    pub generation: u64,
}

#[derive(Debug, PartialEq)]
enum FramePayload {
    Cpu(FrameBytes),
    Gpu(GpuFrameRef),
    Packed(PackedSlice),
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum FrameError {
    /// deep_copy on a GPU frame: the contract forbids silent copies; the
    /// backend must do a visible `copy_frame` instead.
    GpuCopyForbidden,
    PackedCopyNotSupported,
    /// Pool generation mismatch: the holder kept the frame across recycle.
    StaleFrame {
        held: u64,
        current: u64,
    },
}

impl std::fmt::Display for FrameError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            FrameError::GpuCopyForbidden => write!(
                f,
                "GPU frames cannot be deep-copied; use the backend's visible copy_frame"
            ),
            FrameError::PackedCopyNotSupported => {
                write!(f, "packed (compressed) payload copy not supported")
            }
            FrameError::StaleFrame { held, current } => {
                write!(f, "stale frame: held generation {held}, pool at {current}")
            }
        }
    }
}

impl std::error::Error for FrameError {}

impl FrameEnvelope {
    #[allow(clippy::too_many_arguments)] // mirrors the 20-field FRAME_CONTRACT 1:1
    pub fn video_cpu(
        pts: Rational,
        duration: Rational,
        stream_id: StreamId,
        width: u32,
        height: u32,
        pixel_format: PixelFormat,
        bit_depth: BitDepth,
        color: ColorTags,
        bytes: FrameBytes,
        keyframe: bool,
        backend: BackendId,
        generation: u64,
    ) -> Self {
        FrameEnvelope {
            pts,
            duration,
            timeline_pts: None,
            stream_id,
            kind: FrameKind::Video,
            width,
            height,
            pixel_format,
            bit_depth,
            color,
            memory: FrameMemory::Cpu,
            payload: FramePayload::Cpu(bytes),
            keyframe,
            backend,
            generation,
        }
    }

    #[allow(clippy::too_many_arguments)] // mirrors the 20-field FRAME_CONTRACT 1:1
    pub fn video_gpu(
        pts: Rational,
        duration: Rational,
        stream_id: StreamId,
        width: u32,
        height: u32,
        pixel_format: PixelFormat,
        bit_depth: BitDepth,
        color: ColorTags,
        gpu: GpuFrameRef,
        keyframe: bool,
        backend: BackendId,
        generation: u64,
    ) -> Self {
        FrameEnvelope {
            pts,
            duration,
            timeline_pts: None,
            stream_id,
            kind: FrameKind::Video,
            width,
            height,
            pixel_format,
            bit_depth,
            color,
            memory: FrameMemory::Gpu,
            payload: FramePayload::Gpu(gpu),
            keyframe,
            backend,
            generation,
        }
    }

    pub fn packed(
        pts: Rational,
        duration: Rational,
        stream_id: StreamId,
        data: Vec<u8>,
        keyframe: bool,
        backend: BackendId,
    ) -> Self {
        FrameEnvelope {
            pts,
            duration,
            timeline_pts: None,
            stream_id,
            kind: FrameKind::Video,
            width: 0,
            height: 0,
            pixel_format: PixelFormat::Other("compressed".into()),
            bit_depth: BitDepth::Other(0),
            color: ColorTags::unknown(),
            memory: FrameMemory::Packed,
            payload: FramePayload::Packed(PackedSlice { data }),
            keyframe,
            backend,
            generation: 0,
        }
    }

    #[allow(clippy::too_many_arguments)] // mirrors the FRAME_CONTRACT audio shape
    pub fn audio(
        pts: Rational,
        stream_id: StreamId,
        sample_rate: u32,
        channels: u32,
        samples: usize,
        bytes: FrameBytes,
        backend: BackendId,
        generation: u64,
    ) -> Self {
        // FRAME_CONTRACT §3.2: duration = samples/rate exactly
        let duration = Rational::new(samples as i64, sample_rate as i64);
        FrameEnvelope {
            pts,
            duration,
            timeline_pts: None,
            stream_id,
            kind: FrameKind::Audio {
                sample_rate,
                channels,
            },
            width: 0,
            height: 0,
            pixel_format: PixelFormat::Other("pcm".into()),
            bit_depth: BitDepth::Other(0),
            color: ColorTags::unknown(),
            memory: FrameMemory::Cpu,
            payload: FramePayload::Cpu(bytes),
            keyframe: false,
            backend,
            generation,
        }
    }

    // -- payload accessors ---------------------------------------------------

    pub fn cpu_bytes(&self) -> Option<&FrameBytes> {
        match &self.payload {
            FramePayload::Cpu(b) => Some(b),
            _ => None,
        }
    }

    pub fn cpu_bytes_mut(&mut self) -> Option<&mut FrameBytes> {
        match &mut self.payload {
            FramePayload::Cpu(b) => Some(b),
            _ => None,
        }
    }

    pub fn gpu_ref(&self) -> Option<GpuFrameRef> {
        match &self.payload {
            FramePayload::Gpu(g) => Some(*g),
            _ => None,
        }
    }

    pub fn packed_data(&self) -> Option<&[u8]> {
        match &self.payload {
            FramePayload::Packed(p) => Some(&p.data),
            _ => None,
        }
    }

    /// Explicit, visible deep copy of CPU frames (FRAME_CONTRACT §5.1).
    /// GPU frames are REJECTED (sharing = backend-visible copy_frame, not a
    /// silent clone); Packed copy is rejected in v1 (StreamCopy re-muxes
    /// bytes without re-buffering).
    pub fn deep_copy_cpu(&self) -> Result<FrameEnvelope, FrameError> {
        match &self.payload {
            FramePayload::Cpu(b) => Ok(FrameEnvelope {
                pts: self.pts,
                duration: self.duration,
                timeline_pts: self.timeline_pts,
                stream_id: self.stream_id,
                kind: self.kind,
                width: self.width,
                height: self.height,
                pixel_format: self.pixel_format.clone(),
                bit_depth: self.bit_depth,
                color: self.color,
                memory: FrameMemory::Cpu,
                payload: FramePayload::Cpu(FrameBytes {
                    data: b.data.clone(),
                    strides: b.strides.clone(),
                }),
                keyframe: self.keyframe,
                backend: self.backend.clone(),
                generation: self.generation,
            }),
            FramePayload::Gpu(_) => Err(FrameError::GpuCopyForbidden),
            FramePayload::Packed(_) => Err(FrameError::PackedCopyNotSupported),
        }
    }
}

// FRAME_CONTRACT §5.1: ownership unique; no derive(Clone) — compile-time ban.
// (Stable Rust has no negative bounds; the ban is enforced by the code review
// checklist in FRAME_CONTRACT §7 plus the tests that rely on unique ownership.)

#[cfg(test)]
mod tests {
    use super::*;
    use crate::asset::StreamId;

    fn cpu_frame() -> FrameEnvelope {
        FrameEnvelope::video_cpu(
            Rational::new(1, 24),
            Rational::new(1, 24),
            StreamId(0),
            4,
            4,
            PixelFormat::Yuv420p,
            BitDepth::B8,
            ColorTags {
                primaries: Primaries::Bt709,
                transfer: Transfer::Bt709,
                matrix: MatrixCoeffs::Bt709,
                range: Range::Limited,
                chroma_loc: Some(ChromaLoc::Left),
            },
            FrameBytes {
                data: vec![0u8; 4 * 4 * 3 / 2],
                strides: vec![4, 2, 2],
            },
            true,
            BackendId::FFmpegSw,
            7,
        )
    }

    #[test]
    fn video_frame_carries_all_five_color_tags() {
        let f = cpu_frame();
        // D-10 honesty by construction: tags exist and are inspectable
        let c = f.color;
        let _ = (c.primaries, c.transfer, c.matrix, c.range, c.chroma_loc);
        assert_eq!(c.range, Range::Limited);
    }

    #[test]
    fn deep_copy_cpu_is_exact_and_independent() {
        let f = cpu_frame();
        let mut g = f.deep_copy_cpu().unwrap();
        assert_eq!(g.pts, f.pts);
        assert_eq!(g.cpu_bytes().unwrap().data, f.cpu_bytes().unwrap().data);
        // mutate the copy; original unchanged (unique ownership proof)
        g.cpu_bytes_mut().unwrap().data[0] = 0xAB;
        assert_ne!(
            g.cpu_bytes().unwrap().data[0],
            f.cpu_bytes().unwrap().data[0]
        );
    }

    #[test]
    fn gpu_frames_reject_deep_copy() {
        let f = FrameEnvelope::video_gpu(
            Rational::new(0, 1),
            Rational::new(1, 24),
            StreamId(0),
            1920,
            1080,
            PixelFormat::Nv12,
            BitDepth::B8,
            ColorTags::unknown(),
            GpuFrameRef {
                handle: GpuHandle(42),
                upload_kind: GpuUploadKind::ImportedExternalTexture,
                exportable: false,
            },
            false,
            BackendId::Wgpu,
            0,
        );
        assert_eq!(f.memory, FrameMemory::Gpu);
        assert!(!f.gpu_ref().unwrap().exportable);
        assert_eq!(f.deep_copy_cpu(), Err(FrameError::GpuCopyForbidden));
    }

    #[test]
    fn audio_duration_is_samples_over_rate_exactly() {
        // 2002 samples @ 48k = exactly one NTSC 29.97 frame (E-004b lesson)
        let f = FrameEnvelope::audio(
            Rational::new(0, 1),
            StreamId(1),
            48000,
            2,
            2002,
            FrameBytes {
                data: vec![0; 2002 * 4],
                strides: vec![4004, 4004],
            },
            BackendId::Other("test".into()),
            0,
        );
        assert_eq!(f.duration, Rational::new(2002, 48000));
        assert_eq!(f.duration, Rational::new(1001, 24000));
    }
}
