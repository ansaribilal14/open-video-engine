//! FFmpeg software adapter: mappings, probe backend (`FfmpegProbe`), and the
//! `FfmpegSwDecoder` (see `decoder.rs`).
//!
//! This module — and ONLY this crate — may contain libav* linkage
//! (DECODER_SPEC §5). LGPL-only API usage; no GPL-only libav* entry points.

use std::ffi::{c_char, c_int, CStr, CString};

use ffmpeg_sys_next as sys;
use ove_media::{
    AssetRef, AudioDetails, BitDepth, ChromaLoc, ColorTags, ContainerKind, KeyframeEntry,
    KeyframeIndex, MatrixCoeffs, PixelFormat, Primaries, ProbeBackend, ProbeError, ProbeInfo,
    ProbeStream, Range, StreamId, StreamKind, Transfer, VfrReport, VideoDetails,
};
use ove_time::Rational;

pub mod decoder;
pub use decoder::FfmpegSwDecoder;

// ---------------------------------------------------------------------------
// Exact conversion helpers (FRAME_CONTRACT §3: integer → rational is exact)
// ---------------------------------------------------------------------------

/// AVRational → ove Rational, rejecting zero denominators and normalizing a
/// negative denominator by flipping both signs.
pub(crate) fn av_rational_to_rational(r: sys::AVRational) -> Result<Rational, ProbeError> {
    if r.den == 0 {
        return Err(ProbeError::Unsupported("zero time base".into()));
    }
    if r.den > 0 {
        Ok(Rational::new(r.num as i64, r.den as i64))
    } else {
        Ok(Rational::new(-(r.num as i64), -(r.den as i64)))
    }
}

/// Packet/frame ticks (i64) in the given time base → exact Rational.
pub(crate) fn ticks_to_rational(ticks: i64, tb: sys::AVRational) -> Option<Rational> {
    if tb.den == 0 {
        return None;
    }
    let (n, d) = if tb.den > 0 {
        (tb.num as i64, tb.den as i64)
    } else {
        (-(tb.num as i64), -(tb.den as i64))
    };
    // i128 intermediate guards pts * num overflow on huge timestamps
    let num128 = ticks as i128 * n as i128;
    let num = i64::try_from(num128).ok()?;
    Some(Rational::new(num, d))
}

pub(crate) fn last_error(errnum: c_int) -> String {
    let mut buf = [0 as c_char; 256];
    unsafe { sys::av_strerror(errnum, buf.as_mut_ptr(), buf.len()) };
    unsafe { CStr::from_ptr(buf.as_ptr()).to_string_lossy().into_owned() }
}

// ---------------------------------------------------------------------------
// Pixel-format mapping. Deliberately a match on libavformat integer ids
// against the sys-crate variants (as i32) — no transmute of non-exhaustive
// C enums. An unmapped format is a TYPED open failure (v1 declared
// limitation, recorded in STATUS) — never a silent reformat.
// ---------------------------------------------------------------------------

pub(crate) struct MappedFormat {
    pub sys_id: sys::AVPixelFormat,
    pub ours: PixelFormat,
    pub bit_depth: BitDepth,
    pub planes: usize,
}

pub(crate) fn map_pixel_format(fmt: c_int) -> Option<MappedFormat> {
    use sys::AVPixelFormat as P;
    use {BitDepth as B, PixelFormat as F};
    let (sys_id, ours, bit_depth, planes) = match fmt {
        x if x == P::AV_PIX_FMT_YUV420P as i32 => (P::AV_PIX_FMT_YUV420P, F::Yuv420p, B::B8, 3),
        x if x == P::AV_PIX_FMT_YUVJ420P as i32 => (P::AV_PIX_FMT_YUVJ420P, F::Yuvj420p, B::B8, 3),
        x if x == P::AV_PIX_FMT_YUV422P as i32 => (P::AV_PIX_FMT_YUV422P, F::Yuv422p, B::B8, 3),
        x if x == P::AV_PIX_FMT_YUV444P as i32 => (P::AV_PIX_FMT_YUV444P, F::Yuv444p, B::B8, 3),
        x if x == P::AV_PIX_FMT_YUV420P10LE as i32 => {
            (P::AV_PIX_FMT_YUV420P10LE, F::Yuv420p10le, B::B10, 3)
        }
        x if x == P::AV_PIX_FMT_YUV422P10LE as i32 => {
            (P::AV_PIX_FMT_YUV422P10LE, F::Yuv422p10le, B::B10, 3)
        }
        x if x == P::AV_PIX_FMT_YUV444P10LE as i32 => {
            (P::AV_PIX_FMT_YUV444P10LE, F::Yuv444p10le, B::B10, 3)
        }
        x if x == P::AV_PIX_FMT_NV12 as i32 => (P::AV_PIX_FMT_NV12, F::Nv12, B::B8, 2),
        x if x == P::AV_PIX_FMT_NV21 as i32 => (P::AV_PIX_FMT_NV21, F::Nv21, B::B8, 2),
        x if x == P::AV_PIX_FMT_P010LE as i32 => (P::AV_PIX_FMT_P010LE, F::P010le, B::B10, 2),
        x if x == P::AV_PIX_FMT_RGBA as i32 => (P::AV_PIX_FMT_RGBA, F::Rgba, B::B8, 1),
        x if x == P::AV_PIX_FMT_BGRA as i32 => (P::AV_PIX_FMT_BGRA, F::Bgra, B::B8, 1),
        x if x == P::AV_PIX_FMT_RGB24 as i32 => (P::AV_PIX_FMT_RGB24, F::Rgb24, B::B8, 1),
        x if x == P::AV_PIX_FMT_BGR24 as i32 => (P::AV_PIX_FMT_BGR24, F::Bgr24, B::B8, 1),
        x if x == P::AV_PIX_FMT_GRAY8 as i32 => (P::AV_PIX_FMT_GRAY8, F::Gray8, B::B8, 1),
        x if x == P::AV_PIX_FMT_GRAY16LE as i32 => (P::AV_PIX_FMT_GRAY16LE, F::Gray16le, B::B16, 1),
        _ => return None,
    };
    Some(MappedFormat {
        sys_id,
        ours,
        bit_depth,
        planes,
    })
}

// ---------------------------------------------------------------------------
// Color-tag mapping (all five tags; UNSPECIFIED → Unknown-as-value)
// ---------------------------------------------------------------------------

pub(crate) fn map_color(
    primaries: sys::AVColorPrimaries,
    transfer: sys::AVColorTransferCharacteristic,
    matrix: sys::AVColorSpace,
    range: sys::AVColorRange,
    chroma_loc: sys::AVChromaLocation,
) -> ColorTags {
    use sys::AVChromaLocation as CL;
    use sys::AVColorPrimaries as CP;
    use sys::AVColorRange as CR;
    use sys::AVColorSpace as CS;
    use sys::AVColorTransferCharacteristic as CT;
    let primaries = match primaries {
        CP::AVCOL_PRI_BT709 => Primaries::Bt709,
        CP::AVCOL_PRI_BT470M => Primaries::Bt470m,
        CP::AVCOL_PRI_BT470BG => Primaries::Bt470bg,
        CP::AVCOL_PRI_SMPTE170M => Primaries::Bt601525,
        // FFmpeg 7 removed AVCOL_PRI_BT601_625: 625-line BT.601 files report
        // BT470BG (mapped above); our Bt601625 variant stays for other backends
        CP::AVCOL_PRI_BT2020 => Primaries::Bt2020,
        CP::AVCOL_PRI_SMPTE431 => Primaries::DciP3,
        _ => Primaries::Unknown,
    };
    let transfer = match transfer {
        CT::AVCOL_TRC_BT709 => Transfer::Bt709,
        CT::AVCOL_TRC_GAMMA22 => Transfer::Gamma22,
        CT::AVCOL_TRC_GAMMA28 => Transfer::Gamma28,
        CT::AVCOL_TRC_IEC61966_2_1 => Transfer::Srgb,
        CT::AVCOL_TRC_LINEAR => Transfer::Linear,
        CT::AVCOL_TRC_LOG => Transfer::Log100,
        CT::AVCOL_TRC_LOG_SQRT => Transfer::Log316,
        CT::AVCOL_TRC_SMPTE2084 => Transfer::Pq,
        CT::AVCOL_TRC_ARIB_STD_B67 => Transfer::Hlg,
        _ => Transfer::Unknown,
    };
    let matrix = match matrix {
        CS::AVCOL_SPC_RGB => MatrixCoeffs::Identity,
        CS::AVCOL_SPC_BT709 => MatrixCoeffs::Bt709,
        CS::AVCOL_SPC_BT470BG => MatrixCoeffs::Bt470bg,
        CS::AVCOL_SPC_SMPTE170M => MatrixCoeffs::Smpte170m,
        CS::AVCOL_SPC_SMPTE240M => MatrixCoeffs::Smpte240m,
        CS::AVCOL_SPC_BT2020_NCL => MatrixCoeffs::Bt2020Ncl,
        CS::AVCOL_SPC_BT2020_CL => MatrixCoeffs::Bt2020Pcl,
        _ => MatrixCoeffs::Unknown,
    };
    let range = match range {
        CR::AVCOL_RANGE_MPEG => Range::Limited,
        CR::AVCOL_RANGE_JPEG => Range::Full,
        _ => Range::Unknown,
    };
    let chroma = match chroma_loc {
        CL::AVCHROMA_LOC_LEFT => Some(ChromaLoc::Left),
        CL::AVCHROMA_LOC_CENTER => Some(ChromaLoc::Center),
        CL::AVCHROMA_LOC_TOPLEFT => Some(ChromaLoc::TopLeft),
        CL::AVCHROMA_LOC_TOP => Some(ChromaLoc::Top),
        CL::AVCHROMA_LOC_BOTTOMLEFT => Some(ChromaLoc::BottomLeft),
        CL::AVCHROMA_LOC_BOTTOM => Some(ChromaLoc::Bottom),
        _ => None,
    };
    ColorTags {
        primaries,
        transfer,
        matrix,
        range,
        chroma_loc: chroma,
    }
}

// ---------------------------------------------------------------------------
// Container-kind mapping
// ---------------------------------------------------------------------------

fn container_kind(fmt_name: *const c_char) -> ContainerKind {
    if fmt_name.is_null() {
        return ContainerKind::Other;
    }
    let name = unsafe { CStr::from_ptr(fmt_name).to_string_lossy() };
    if name.contains("mp4") || name.contains("3gp") {
        ContainerKind::Mp4
    } else if name.contains("mov") {
        ContainerKind::Mov
    } else if name.contains("webm") {
        ContainerKind::WebM
    } else if name.contains("matroska") {
        ContainerKind::Mkv
    } else if name == "avi" {
        ContainerKind::Avi
    } else {
        ContainerKind::Other
    }
}

fn sample_format_name(fmt: c_int) -> String {
    // AVSampleFormat ids are small and stable; match instead of transmute.
    use sys::AVSampleFormat as S;
    let known = [
        (S::AV_SAMPLE_FMT_U8 as i32, "u8"),
        (S::AV_SAMPLE_FMT_S16 as i32, "s16"),
        (S::AV_SAMPLE_FMT_S32 as i32, "s32"),
        (S::AV_SAMPLE_FMT_FLT as i32, "flt"),
        (S::AV_SAMPLE_FMT_DBL as i32, "dbl"),
        (S::AV_SAMPLE_FMT_U8P as i32, "u8p"),
        (S::AV_SAMPLE_FMT_S16P as i32, "s16p"),
        (S::AV_SAMPLE_FMT_S32P as i32, "s32p"),
        (S::AV_SAMPLE_FMT_FLTP as i32, "fltp"),
        (S::AV_SAMPLE_FMT_DBLP as i32, "dblp"),
    ];
    known
        .iter()
        .find(|(id, _)| *id == fmt)
        .map(|(_, n)| (*n).to_string())
        .unwrap_or_else(|| format!("sample_fmt#{fmt}"))
}

// ---------------------------------------------------------------------------
// Shared open/stream-facts helpers
// ---------------------------------------------------------------------------

pub(crate) fn open_input(locator: &str) -> Result<*mut sys::AVFormatContext, ProbeError> {
    let cpath = CString::new(locator).map_err(|_| ProbeError::Io("NUL in path".into()))?;
    let mut ctx: *mut sys::AVFormatContext = std::ptr::null_mut();
    let rc = unsafe {
        sys::avformat_open_input(
            &mut ctx,
            cpath.as_ptr(),
            std::ptr::null(),
            std::ptr::null_mut(),
        )
    };
    if rc != 0 || ctx.is_null() {
        // ENOENT-style failures and unparseable headers both read as Unreadable
        return Err(ProbeError::Unreadable);
    }
    let rc = unsafe { sys::avformat_find_stream_info(ctx, std::ptr::null_mut()) };
    if rc < 0 {
        unsafe { sys::avformat_close_input(&mut ctx) };
        return Err(ProbeError::Corrupt(last_error(rc)));
    }
    Ok(ctx)
}

pub(crate) fn close_input(ctx: &mut *mut sys::AVFormatContext) {
    unsafe { sys::avformat_close_input(ctx) };
}

pub(crate) fn stream_facts(
    ctx: *mut sys::AVFormatContext,
    i: usize,
) -> Result<StreamFacts, ProbeError> {
    unsafe {
        let st = *(*ctx).streams.add(i);
        let par = (*st).codecpar;
        let kind = match (*par).codec_type {
            sys::AVMediaType::AVMEDIA_TYPE_VIDEO => StreamKind::Video,
            sys::AVMediaType::AVMEDIA_TYPE_AUDIO => StreamKind::Audio,
            sys::AVMediaType::AVMEDIA_TYPE_DATA => StreamKind::Data,
            sys::AVMediaType::AVMEDIA_TYPE_ATTACHMENT => StreamKind::Attachment,
            _ => StreamKind::Unknown,
        };
        let codec = CStr::from_ptr(sys::avcodec_get_name((*par).codec_id))
            .to_string_lossy()
            .into_owned();
        let tb = (*st).time_base;
        let _validated_tb = av_rational_to_rational(tb)?;
        let start_time = if (*st).start_time >= 0 {
            Some((*st).start_time)
        } else {
            None
        };
        let duration = if (*st).duration >= 0 {
            Some((*st).duration)
        } else {
            None
        };
        let avg_frame_rate = av_rational_to_rational((*st).avg_frame_rate)
            .ok()
            .filter(|r| r.num() > 0);
        let nb_frames = if (*st).nb_frames > 0 {
            Some((*st).nb_frames as u64)
        } else {
            None
        };
        let (video, audio) = if kind == StreamKind::Video {
            let fmt = map_pixel_format((*par).format).ok_or_else(|| {
                ProbeError::Unsupported(format!("pixel format id {}", (*par).format))
            })?;
            let color = map_color(
                (*par).color_primaries,
                (*par).color_trc,
                (*par).color_space,
                (*par).color_range,
                (*par).chroma_location,
            );
            (
                Some(VideoDetails {
                    width: (*par).width as u32,
                    height: (*par).height as u32,
                    pixel_format: fmt.ours,
                    bit_depth: fmt.bit_depth,
                    color,
                }),
                None,
            )
        } else if kind == StreamKind::Audio {
            (
                None,
                Some(AudioDetails {
                    sample_rate: (*par).sample_rate as u32,
                    channels: (*par).ch_layout.nb_channels as u32,
                    sample_format: sample_format_name((*par).format),
                }),
            )
        } else {
            (None, None)
        };
        Ok(StreamFacts {
            index: i as u32,
            kind,
            codec,
            time_base: tb,
            start_time,
            duration,
            avg_frame_rate,
            nb_frames,
            video,
            audio,
        })
    }
}

pub(crate) struct StreamFacts {
    pub index: u32,
    pub kind: StreamKind,
    pub codec: String,
    pub time_base: sys::AVRational,
    pub start_time: Option<i64>,
    pub duration: Option<i64>,
    pub avg_frame_rate: Option<Rational>,
    pub nb_frames: Option<u64>,
    pub video: Option<VideoDetails>,
    pub audio: Option<AudioDetails>,
}

fn facts_to_probe_stream(f: StreamFacts) -> ProbeStream {
    let tb = f.time_base;
    ProbeStream {
        id: StreamId(f.index),
        kind: f.kind,
        codec: f.codec,
        time_base: av_rational_to_rational(tb).expect("validated by stream_facts"),
        start_time: f.start_time.and_then(|t| ticks_to_rational(t, tb)),
        duration: f.duration.and_then(|t| ticks_to_rational(t, tb)),
        avg_frame_rate: f.avg_frame_rate,
        nb_frames_hint: f.nb_frames,
        video: f.video,
        audio: f.audio,
    }
}

// ---------------------------------------------------------------------------
// FfmpegProbe — the static probe backend (ProbeBackend impl, libav-confined)
// ---------------------------------------------------------------------------

pub struct FfmpegProbe;

impl ProbeBackend for FfmpegProbe {
    /// Header-level probe (no packet scan). See `probe_full` for the combined
    /// import path that also fills keyframe_index + vfr in ONE packet scan.
    fn probe(&self, asset: &AssetRef) -> Result<ProbeInfo, ProbeError> {
        let mut ctx = open_input(&asset.locator)?;
        let info = self.header_info(&mut ctx)?;
        close_input(&mut ctx);
        Ok(info)
    }

    /// Full packet scan: keyframe index (packet-level K flags + byte pos) and
    /// VFR detection (distinct inter-frame pts deltas) for the FIRST video
    /// stream. E-007's lesson encoded: boundaries come from a real index.
    fn keyframe_index(&self, asset: &AssetRef) -> Result<KeyframeIndex, ProbeError> {
        self.scan(asset).map(|(_, idx)| idx)
    }
}

impl FfmpegProbe {
    fn header_info(&self, ctx: &mut *mut sys::AVFormatContext) -> Result<ProbeInfo, ProbeError> {
        let c = *ctx; // one deref: c is the raw context pointer
        unsafe {
            let mut info = ProbeInfo {
                container: container_kind((*(*c).iformat).name),
                // container duration is in AV_TIME_BASE (µs) — exact as a
                // rational but µs-precision; per-stream durations are the
                // exact authority
                duration: if (*c).duration >= 0 {
                    Some(Rational::new((*c).duration, 1_000_000))
                } else {
                    None
                },
                streams: Vec::new(),
                keyframe_index: None,
                vfr: None,
            };
            let nb = (*c).nb_streams as usize;
            for i in 0..nb {
                info.streams
                    .push(facts_to_probe_stream(stream_facts(c, i)?));
            }
            Ok(info)
        }
    }

    /// One-pass scan of the first video stream: pts list + keyframe index.
    /// Used by `keyframe_index` and `probe_full`.
    fn scan(&self, asset: &AssetRef) -> Result<(Vec<Rational>, KeyframeIndex), ProbeError> {
        let mut ctx = open_input(&asset.locator)?;
        unsafe {
            // first video stream
            let vidx = (0..(*ctx).nb_streams as usize)
                .find(|&i| {
                    let st = *(*ctx).streams.add(i);
                    (*(*st).codecpar).codec_type == sys::AVMediaType::AVMEDIA_TYPE_VIDEO
                })
                .ok_or_else(|| ProbeError::Unsupported("no video stream".into()))?;
            let st = *(*ctx).streams.add(vidx);
            let tb = (*st).time_base;

            let mut pts_list: Vec<Rational> = Vec::new();
            let mut entries: Vec<KeyframeEntry> = Vec::new();

            let mut pkt = sys::av_packet_alloc();
            if pkt.is_null() {
                close_input(&mut ctx);
                return Err(ProbeError::Io("av_packet_alloc failed".into()));
            }
            loop {
                let rc = sys::av_read_frame(ctx, pkt);
                if rc == sys::AVERROR_EOF {
                    break;
                }
                if rc < 0 {
                    let d = last_error(rc);
                    sys::av_packet_free(&mut pkt);
                    close_input(&mut ctx);
                    return Err(ProbeError::Corrupt(d));
                }
                if (*pkt).stream_index as usize == vidx && (*pkt).pts >= 0 {
                    if let Some(pts) = ticks_to_rational((*pkt).pts, tb) {
                        pts_list.push(pts);
                        if (*pkt).flags & sys::AV_PKT_FLAG_KEY != 0 {
                            entries.push(KeyframeEntry {
                                pts,
                                byte_pos: if (*pkt).pos >= 0 {
                                    Some((*pkt).pos as u64)
                                } else {
                                    None
                                },
                            });
                        }
                    }
                }
                sys::av_packet_unref(pkt);
            }
            sys::av_packet_free(&mut pkt);
            close_input(&mut ctx);

            if entries.is_empty() {
                return Err(ProbeError::Corrupt("no video packets with pts".into()));
            }
            Ok((pts_list, KeyframeIndex { entries }))
        }
    }

    /// Combined import path: header probe + one packet scan filling
    /// `keyframe_index` and `vfr` in the returned `ProbeInfo`.
    pub fn probe_full(&self, asset: &AssetRef) -> Result<ProbeInfo, ProbeError> {
        let mut ctx = open_input(&asset.locator)?;
        let mut info = self.header_info(&mut ctx)?;
        close_input(&mut ctx);
        let (pts_list, idx) = self.scan(asset)?;
        info.vfr = Some(VfrReport::from_pts(&pts_list));
        info.keyframe_index = Some(idx);
        Ok(info)
    }
}
