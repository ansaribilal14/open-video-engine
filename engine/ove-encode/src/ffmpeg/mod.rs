//! FFmpeg adapter for the encode leg: software encoder (`encoder.rs`, native
//! mpeg4 in v1), MP4 muxer (`muxer.rs`), and the packet-level stream-copy
//! source (`copy.rs`).
//!
//! This module — and ONLY the adapter crates {ove-decode, ove-encode} — may
//! contain libav* linkage (DECODER_SPEC §5 + ENCODER_SPEC §5, ADR-015).
//! LGPL-only API usage; no GPL-only libav* entry points (no libx264 —
//! ADR-005 license decision).

use std::ffi::{c_char, c_int, CStr, CString};

use ffmpeg_sys_next as sys;
use ove_media::{ColorTags, PixelFormat, Range, StreamId};
use ove_time::Rational;

use crate::{EncodeError, MuxError};

pub mod aac;
pub mod copy;
pub mod encoder;
pub mod muxer;

pub use aac::FfmpegAacEncoder;
pub use copy::{FfmpegCopySource, ParsedStream};
pub use encoder::FfmpegSwEncoder;
pub use muxer::FfmpegMuxer;

// ---------------------------------------------------------------------------
// Exact conversion helpers (mirror ove-decode/ffmpeg/mod.rs — kept local so
// the two adapter crates stay independent; shared helper crate is a later
// refactor with its own ADR).
// ---------------------------------------------------------------------------

/// AVRational → ove Rational, rejecting zero denominators and normalizing a
/// negative denominator by flipping both signs.
pub(crate) fn av_rational_to_rational(r: sys::AVRational) -> Option<Rational> {
    if r.den == 0 {
        return None;
    }
    if r.den > 0 {
        Some(Rational::new(r.num as i64, r.den as i64))
    } else {
        Some(Rational::new(-(r.num as i64), -(r.den as i64)))
    }
}

/// Exact ticks for `pts` on a 1/tb axis: pts × (tb.den/tb.num). Err when not
/// exactly representable (E-002: conversion never rounds).
pub(crate) fn exact_ticks(pts: Rational, tb: sys::AVRational) -> Result<i64, EncodeError> {
    if tb.num == 0 {
        return Err(EncodeError::Internal("zero tick axis".into()));
    }
    // ticks = pts * (tb.den / tb.num) = (num * tb.den) / (den * tb.num)
    let numer = (pts.num() as i128) * (tb.den as i128);
    let denom = (pts.den() as i128) * (tb.num as i128);
    if numer % denom != 0 {
        return Err(EncodeError::NonExactTimestamp {
            pts,
            time_base: Rational::new(tb.num as i64, tb.den as i64),
        });
    }
    i64::try_from(numer / denom).map_err(|_| EncodeError::Internal("tick count exceeds i64".into()))
}

pub(crate) fn last_error(errnum: c_int) -> String {
    let mut buf = [0 as c_char; 256];
    unsafe { sys::av_strerror(errnum, buf.as_mut_ptr(), buf.len()) };
    unsafe { CStr::from_ptr(buf.as_ptr()).to_string_lossy().into_owned() }
}

/// Silence libav logging (the engine reports errors as typed values; libav
/// stderr noise pollutes CLI/test output). Idempotent, call once.
pub fn quiet_libav() {
    unsafe { sys::av_log_set_level(sys::AV_LOG_QUIET) };
}

// ---------------------------------------------------------------------------
// Pixel-format mapping (encoder side; subset of the decode-side table that
// encoders realistically accept, plus the RGBA input the renderer produces).
// ---------------------------------------------------------------------------

pub(crate) fn encoder_pix_fmt(ours: PixelFormat) -> Option<sys::AVPixelFormat> {
    use sys::AVPixelFormat as P;
    use PixelFormat as F;
    let f = match ours {
        F::Yuv420p => P::AV_PIX_FMT_YUV420P,
        F::Yuvj420p => P::AV_PIX_FMT_YUVJ420P,
        F::Yuv422p => P::AV_PIX_FMT_YUV422P,
        F::Yuv444p => P::AV_PIX_FMT_YUV444P,
        F::Yuv420p10le => P::AV_PIX_FMT_YUV420P10LE,
        F::Yuv422p10le => P::AV_PIX_FMT_YUV422P10LE,
        F::Yuv444p10le => P::AV_PIX_FMT_YUV444P10LE,
        F::Nv12 => P::AV_PIX_FMT_NV12,
        F::Nv21 => P::AV_PIX_FMT_NV21,
        F::P010le => P::AV_PIX_FMT_P010LE,
        F::Rgba => P::AV_PIX_FMT_RGBA,
        F::Bgra => P::AV_PIX_FMT_BGRA,
        F::Rgb24 => P::AV_PIX_FMT_RGB24,
        F::Bgr24 => P::AV_PIX_FMT_BGR24,
        F::Gray8 => P::AV_PIX_FMT_GRAY8,
        F::Gray16le => P::AV_PIX_FMT_GRAY16LE,
        _ => return None,
    };
    Some(f)
}

// ---------------------------------------------------------------------------
// Color-tag → libav (encoder side; the inverse of the decode mapping).
// Unknown stays unspecified (0): declared unknown-as-value, never invented.
// ---------------------------------------------------------------------------

pub(crate) fn apply_color_tags(par: *mut sys::AVCodecParameters, color: &ColorTags) {
    use ove_media::{ChromaLoc, MatrixCoeffs, Primaries, Transfer};
    unsafe {
        (*par).color_primaries = match color.primaries {
            Primaries::Bt709 => sys::AVColorPrimaries::AVCOL_PRI_BT709,
            Primaries::Bt470m => sys::AVColorPrimaries::AVCOL_PRI_BT470M,
            Primaries::Bt470bg => sys::AVColorPrimaries::AVCOL_PRI_BT470BG,
            Primaries::Bt601525 => sys::AVColorPrimaries::AVCOL_PRI_SMPTE170M,
            Primaries::Bt2020 => sys::AVColorPrimaries::AVCOL_PRI_BT2020,
            Primaries::DciP3 => sys::AVColorPrimaries::AVCOL_PRI_SMPTE431,
            _ => sys::AVColorPrimaries::AVCOL_PRI_UNSPECIFIED,
        };
        (*par).color_trc = match color.transfer {
            Transfer::Bt709 => sys::AVColorTransferCharacteristic::AVCOL_TRC_BT709,
            Transfer::Gamma22 => sys::AVColorTransferCharacteristic::AVCOL_TRC_GAMMA22,
            Transfer::Gamma28 => sys::AVColorTransferCharacteristic::AVCOL_TRC_GAMMA28,
            Transfer::Srgb => sys::AVColorTransferCharacteristic::AVCOL_TRC_IEC61966_2_1,
            Transfer::Linear => sys::AVColorTransferCharacteristic::AVCOL_TRC_LINEAR,
            Transfer::Log100 => sys::AVColorTransferCharacteristic::AVCOL_TRC_LOG,
            Transfer::Log316 => sys::AVColorTransferCharacteristic::AVCOL_TRC_LOG_SQRT,
            Transfer::Pq => sys::AVColorTransferCharacteristic::AVCOL_TRC_SMPTE2084,
            Transfer::Hlg => sys::AVColorTransferCharacteristic::AVCOL_TRC_ARIB_STD_B67,
            _ => sys::AVColorTransferCharacteristic::AVCOL_TRC_UNSPECIFIED,
        };
        (*par).color_space = match color.matrix {
            MatrixCoeffs::Identity => sys::AVColorSpace::AVCOL_SPC_RGB,
            MatrixCoeffs::Bt709 => sys::AVColorSpace::AVCOL_SPC_BT709,
            MatrixCoeffs::Bt470bg => sys::AVColorSpace::AVCOL_SPC_BT470BG,
            MatrixCoeffs::Smpte170m => sys::AVColorSpace::AVCOL_SPC_SMPTE170M,
            MatrixCoeffs::Smpte240m => sys::AVColorSpace::AVCOL_SPC_SMPTE240M,
            MatrixCoeffs::Bt2020Ncl => sys::AVColorSpace::AVCOL_SPC_BT2020_NCL,
            MatrixCoeffs::Bt2020Pcl => sys::AVColorSpace::AVCOL_SPC_BT2020_CL,
            _ => sys::AVColorSpace::AVCOL_SPC_UNSPECIFIED,
        };
        (*par).color_range = match color.range {
            Range::Limited => sys::AVColorRange::AVCOL_RANGE_MPEG,
            Range::Full => sys::AVColorRange::AVCOL_RANGE_JPEG,
            _ => sys::AVColorRange::AVCOL_RANGE_UNSPECIFIED,
        };
        (*par).chroma_location = match color.chroma_loc {
            Some(ChromaLoc::Left) => sys::AVChromaLocation::AVCHROMA_LOC_LEFT,
            Some(ChromaLoc::Center) => sys::AVChromaLocation::AVCHROMA_LOC_CENTER,
            Some(ChromaLoc::TopLeft) => sys::AVChromaLocation::AVCHROMA_LOC_TOPLEFT,
            Some(ChromaLoc::Top) => sys::AVChromaLocation::AVCHROMA_LOC_TOP,
            Some(ChromaLoc::BottomLeft) => sys::AVChromaLocation::AVCHROMA_LOC_BOTTOMLEFT,
            Some(ChromaLoc::Bottom) => sys::AVChromaLocation::AVCHROMA_LOC_BOTTOM,
            _ => sys::AVChromaLocation::AVCHROMA_LOC_UNSPECIFIED,
        };
    }
}

// ---------------------------------------------------------------------------
// Stream-id ↔ stream-index mapping for the muxer (tracks are explicit; no
// implicit ordering — directive: explicit IDs everywhere).
// ---------------------------------------------------------------------------

pub(crate) fn cstring(s: &str, ctx: &str) -> Result<CString, MuxError> {
    CString::new(s).map_err(|_| MuxError::Internal(format!("NUL in {ctx}")))
}

pub(crate) fn mux_internal(context: &str) -> MuxError {
    MuxError::Internal(context.to_string())
}

/// Version identity string for golden keying (ENCODER_SPEC §6: "record
/// versions in output metadata"). `av_version_info` is the full build
/// identity (e.g. "7.1.5-0+deb13u1"); numeric per-lib versions add the API
/// versions (100xxx-style) as a belt-and-braces pair.
pub fn libav_versions() -> String {
    unsafe {
        let info = sys::av_version_info();
        let info = if info.is_null() {
            "unknown".to_string()
        } else {
            CStr::from_ptr(info).to_string_lossy().into_owned()
        };
        format!(
            "{info} (avutil={}, avcodec={}, avformat={}, swscale={})",
            sys::avutil_version(),
            sys::avcodec_version(),
            sys::avformat_version(),
            sys::swscale_version(),
        )
    }
}

/// The stream id used by single-track export sessions (explicit, not magic:
/// every TrackSpec/EncodedPacket carries it).
pub const DEFAULT_STREAM: StreamId = StreamId(0);
