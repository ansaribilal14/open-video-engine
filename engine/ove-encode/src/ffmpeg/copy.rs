//! `FfmpegCopySource` — packet-level demux for the StreamCopy export route.
//!
//! NO DECODE HAPPENS HERE (ENCODER_SPEC §3.2): compressed packets pass
//! through byte-identical; color/pts tags ride along untouched. The planner
//! (pure, `crate::planner`) already resolved the keyframe-aligned span; this
//! adapter lands exactly on it:
//!   * `av_seek_frame(..., AVSEEK_FLAG_BACKWARD)` to the snapped start
//!     keyframe (D-5 keyframe-floor landing, encoded positively),
//!   * packets in [start, end) per stream, pts shifted by `-start` so the
//!     output starts at 0 (E-3 start-offset policy),
//!   * every timestamp conversion is exact (typed error otherwise).
//!
//! `ParsedStream` carries everything the muxer needs to rebuild the track
//! (codec, timescale, geometry/samplerate, extradata) without decode.

use std::ffi::{c_int, CStr};

use ffmpeg_sys_next as sys;
use ove_media::{
    AssetRef, ChromaLoc, ColorTags, MatrixCoeffs, PixelFormat, Primaries, Range, StreamId, Transfer,
};
use ove_time::Rational;

use crate::{EncodedPacket, MuxError, TrackCodec, TrackKind, TrackSpec};

use super::{av_rational_to_rational, cstring, exact_ticks, last_error, mux_internal};

/// A demuxed source stream, ready for muxer `TrackSpec` construction.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ParsedStream {
    /// Source stream index (demux order; the CALLER chooses the output
    /// StreamId — explicit remap, no implicit identity).
    pub source_index: usize,
    pub is_video: bool,
    pub codec: TrackCodec,
    pub codec_name: String,
    /// Container time base (exact).
    pub time_base: Rational,
    /// Video: average frame rate when declared (None = VFR source).
    pub avg_frame_rate: Option<Rational>,
    /// Video geometry (video streams only).
    pub width: u32,
    pub height: u32,
    /// Video pixel format (mapped from the libav id; None = unmapped →
    /// typed InvalidTrackSpec on track_spec — declared honesty).
    pub pixel_format: Option<PixelFormat>,
    /// Audio facts (audio streams only).
    pub sample_rate: u32,
    pub channels: u32,
    pub sample_format: String,
    /// Color tags (video; unknown-as-value).
    pub color: ColorTags,
    pub extradata: Vec<u8>,
    /// Stream duration on the source axis (0 when undeclared).
    pub stream_end: Rational,
    /// Codec priming delay in samples (AAC; carried into the muxer TrackSpec
    /// so the container writes the trim edit list — W7/ADR-018).
    pub initial_padding: i64,
}

impl ParsedStream {
    /// Build the muxer track spec with an EXPLICIT output stream id.
    /// Timescale: video = minimal exact axis for the frame rate (falling
    /// back to the container time-base denominator when no frame rate is
    /// declared); audio = sample rate (sample-exact contract).
    pub fn track_spec(&self, out_stream_id: StreamId) -> Result<TrackSpec, MuxError> {
        let (timescale, kind) = if self.is_video {
            let rate = self.avg_frame_rate.ok_or_else(|| {
                MuxError::InvalidTrackSpec(
                    "VFR stream copy needs a declared frame-rate axis (v1: CFR sources)".into(),
                )
            })?;
            let ts = TrackSpec::video_timescale(rate)?;
            let pf = self.pixel_format.clone().ok_or_else(|| {
                MuxError::InvalidTrackSpec(format!(
                    "source pixel format {} not mappable (copy v1)",
                    self.codec_name
                ))
            })?;
            (
                ts,
                TrackKind::Video {
                    width: self.width,
                    height: self.height,
                    pixel_format: pf,
                    color: self.color,
                    frame_rate: rate,
                },
            )
        } else {
            (
                self.sample_rate as i64,
                TrackKind::Audio {
                    sample_rate: self.sample_rate,
                    channels: self.channels,
                    sample_format: self.sample_format.clone(),
                },
            )
        };
        Ok(TrackSpec {
            stream_id: out_stream_id,
            codec: self.codec,
            timescale,
            kind,
            extradata: self.extradata.clone(),
            initial_padding: self.initial_padding,
        })
    }
}

pub struct FfmpegCopySource {
    ctx: *mut sys::AVFormatContext,
    streams: Vec<ParsedStream>,
}

// SAFETY: exclusive ownership of the input context; no shared call sites.
unsafe impl Send for FfmpegCopySource {}

impl Drop for FfmpegCopySource {
    fn drop(&mut self) {
        unsafe { sys::avformat_close_input(&mut self.ctx) };
    }
}

fn map_color_tags(par: *const sys::AVCodecParameters) -> ColorTags {
    unsafe {
        ColorTags {
            primaries: match (*par).color_primaries {
                sys::AVColorPrimaries::AVCOL_PRI_BT709 => Primaries::Bt709,
                sys::AVColorPrimaries::AVCOL_PRI_BT470BG => Primaries::Bt470bg,
                sys::AVColorPrimaries::AVCOL_PRI_SMPTE170M => Primaries::Bt601525,
                sys::AVColorPrimaries::AVCOL_PRI_BT2020 => Primaries::Bt2020,
                sys::AVColorPrimaries::AVCOL_PRI_SMPTE431 => Primaries::DciP3,
                _ => Primaries::Unknown,
            },
            transfer: match (*par).color_trc {
                sys::AVColorTransferCharacteristic::AVCOL_TRC_BT709 => Transfer::Bt709,
                sys::AVColorTransferCharacteristic::AVCOL_TRC_IEC61966_2_1 => Transfer::Srgb,
                sys::AVColorTransferCharacteristic::AVCOL_TRC_SMPTE2084 => Transfer::Pq,
                _ => Transfer::Unknown,
            },
            matrix: match (*par).color_space {
                sys::AVColorSpace::AVCOL_SPC_BT709 => MatrixCoeffs::Bt709,
                sys::AVColorSpace::AVCOL_SPC_BT470BG => MatrixCoeffs::Bt470bg,
                sys::AVColorSpace::AVCOL_SPC_SMPTE170M => MatrixCoeffs::Smpte170m,
                sys::AVColorSpace::AVCOL_SPC_BT2020_NCL => MatrixCoeffs::Bt2020Ncl,
                _ => MatrixCoeffs::Unknown,
            },
            range: match (*par).color_range {
                sys::AVColorRange::AVCOL_RANGE_MPEG => Range::Limited,
                sys::AVColorRange::AVCOL_RANGE_JPEG => Range::Full,
                _ => Range::Unknown,
            },
            chroma_loc: match (*par).chroma_location {
                sys::AVChromaLocation::AVCHROMA_LOC_LEFT => Some(ChromaLoc::Left),
                sys::AVChromaLocation::AVCHROMA_LOC_CENTER => Some(ChromaLoc::Center),
                _ => None,
            },
        }
    }
}

fn map_track_codec(codec_id: sys::AVCodecID) -> Option<TrackCodec> {
    // v1 copy whitelist (typed InvalidTrackSpec for anything else — honest
    // declared surface, grows with the corpus/spec).
    if codec_id == sys::AVCodecID::AV_CODEC_ID_MPEG4 {
        Some(TrackCodec::Mpeg4)
    } else if codec_id == sys::AVCodecID::AV_CODEC_ID_AAC {
        Some(TrackCodec::Aac)
    } else {
        None
    }
}

fn map_pixel_format(fmt: c_int) -> Option<PixelFormat> {
    use sys::AVPixelFormat as P;
    use PixelFormat as F;
    let f = if fmt == P::AV_PIX_FMT_YUV420P as i32 {
        F::Yuv420p
    } else if fmt == P::AV_PIX_FMT_YUVJ420P as i32 {
        F::Yuvj420p
    } else if fmt == P::AV_PIX_FMT_YUV422P as i32 {
        F::Yuv422p
    } else if fmt == P::AV_PIX_FMT_YUV444P as i32 {
        F::Yuv444p
    } else if fmt == P::AV_PIX_FMT_NV12 as i32 {
        F::Nv12
    } else {
        return None;
    };
    Some(f)
}

fn ticks_to_rational(ticks: i64, tb: Rational) -> Option<Rational> {
    // seconds = ticks × tb — always exact as a rational (only i64 overflow
    // fails). Divisibility is required only in the INVERSE direction.
    let numer = (ticks as i128) * (tb.num() as i128);
    let denom = tb.den() as i128;
    if denom == 0 {
        return None;
    }
    let g = gcd128(numer.unsigned_abs(), denom.unsigned_abs()).max(1);
    Some(Rational::new(
        i64::try_from(numer / g).ok()?,
        i64::try_from(denom / g).ok()?,
    ))
}

fn gcd128(a: u128, b: u128) -> i128 {
    let (mut a, mut b) = (a, b);
    while b != 0 {
        let t = a % b;
        a = b;
        b = t;
    }
    a as i128
}

impl FfmpegCopySource {
    /// Open the container and parse every media stream (no packet read yet).
    pub fn open(asset: &AssetRef) -> Result<Self, MuxError> {
        let cpath = cstring(&asset.locator, "asset path")?;
        unsafe {
            let mut ctx: *mut sys::AVFormatContext = std::ptr::null_mut();
            let rc = sys::avformat_open_input(
                &mut ctx,
                cpath.as_ptr(),
                std::ptr::null(),
                std::ptr::null_mut(),
            );
            if rc < 0 || ctx.is_null() {
                return Err(MuxError::Io(format!("open: {}", last_error(rc))));
            }
            let rc = sys::avformat_find_stream_info(ctx, std::ptr::null_mut());
            if rc < 0 {
                sys::avformat_close_input(&mut ctx);
                return Err(mux_internal(&format!(
                    "find_stream_info: {}",
                    last_error(rc)
                )));
            }

            let mut streams = Vec::new();
            for i in 0..(*ctx).nb_streams as usize {
                let st = *(*ctx).streams.add(i);
                let par = (*st).codecpar;
                let is_video = (*par).codec_type == sys::AVMediaType::AVMEDIA_TYPE_VIDEO;
                let is_audio = (*par).codec_type == sys::AVMediaType::AVMEDIA_TYPE_AUDIO;
                if !is_video && !is_audio {
                    continue;
                }
                let codec = match map_track_codec((*par).codec_id) {
                    Some(c) => c,
                    None => {
                        let name = CStr::from_ptr(sys::avcodec_get_name((*par).codec_id))
                            .to_string_lossy()
                            .into_owned();
                        sys::avformat_close_input(&mut ctx);
                        return Err(MuxError::InvalidTrackSpec(format!(
                            "stream copy v1 supports mpeg4/aac only; stream {i} is {name}"
                        )));
                    }
                };
                let tb = match av_rational_to_rational((*st).time_base) {
                    Some(t) => t,
                    None => {
                        sys::avformat_close_input(&mut ctx);
                        return Err(mux_internal("zero time base"));
                    }
                };
                let codec_name = CStr::from_ptr(sys::avcodec_get_name((*par).codec_id))
                    .to_string_lossy()
                    .into_owned();
                let extradata = if !(*par).extradata.is_null() && (*par).extradata_size > 0 {
                    std::slice::from_raw_parts((*par).extradata, (*par).extradata_size as usize)
                        .to_vec()
                } else {
                    Vec::new()
                };
                let stream_end = if (*st).duration >= 0 {
                    ticks_to_rational((*st).duration, tb).unwrap_or(Rational::zero(1))
                } else {
                    Rational::zero(1)
                };
                let (
                    avg_frame_rate,
                    width,
                    height,
                    pixel_format,
                    sample_rate,
                    channels,
                    sample_format,
                ) = if is_video {
                    (
                        av_rational_to_rational((*st).avg_frame_rate).filter(|r| r.num() > 0),
                        (*par).width as u32,
                        (*par).height as u32,
                        map_pixel_format((*par).format),
                        0,
                        0,
                        String::new(),
                    )
                } else {
                    (
                        None,
                        0,
                        0,
                        None,
                        (*par).sample_rate as u32,
                        (*par).ch_layout.nb_channels as u32,
                        sample_format_name((*par).format),
                    )
                };
                streams.push(ParsedStream {
                    source_index: i,
                    is_video,
                    codec,
                    codec_name,
                    time_base: tb,
                    avg_frame_rate,
                    width,
                    height,
                    pixel_format,
                    sample_rate,
                    channels,
                    sample_format,
                    color: map_color_tags(par),
                    extradata,
                    stream_end,
                    initial_padding: (*par).initial_padding as i64,
                });
            }
            Ok(FfmpegCopySource { ctx, streams })
        }
    }

    pub fn streams(&self) -> &[ParsedStream] {
        &self.streams
    }

    /// Exclusive end of media on the source axis (max stream duration).
    pub fn media_end(&self) -> Rational {
        self.streams
            .iter()
            .map(|s| s.stream_end)
            .max()
            .unwrap_or_else(|| Rational::zero(1))
    }

    pub fn stream_by_source_index(&self, idx: usize) -> Option<&ParsedStream> {
        self.streams.iter().find(|s| s.source_index == idx)
    }

    /// Read packets of the given source streams inside `range` (half-open,
    /// SOURCE time), pts shifted by `-range.start` so output starts at 0.
    ///
    /// `wanted`: source stream indices to copy. `remap`: (source index,
    /// output StreamId). The FIRST video packet delivered MUST be a keyframe
    /// (the planner's start snap guarantees it; asserted here — E-007
    /// honesty: a mid-GOP copy start is a typed failure, never silent).
    pub fn read_packets(
        &mut self,
        range: &crate::planner::TimeRange,
        wanted: &[usize],
        remap: &[(usize, StreamId)],
    ) -> Result<Vec<EncodedPacket>, MuxError> {
        unsafe {
            let mut pkt = sys::av_packet_alloc();
            if pkt.is_null() {
                return Err(mux_internal("av_packet_alloc failed"));
            }
            // Seek: target = the planner's snapped start (a keyframe pts);
            // BACKWARD lands exactly there. Seek on the first video stream.
            if let Some(v) = wanted
                .iter()
                .find_map(|&w| self.stream_by_source_index(w))
                .filter(|s| s.is_video)
            {
                let raw_tb = (*(*(*self.ctx).streams.add(v.source_index))).time_base;
                let target = match exact_ticks(range.start, raw_tb) {
                    Ok(k) => k,
                    Err(_) => {
                        sys::av_packet_free(&mut pkt);
                        return Err(mux_internal("start not exact on source axis"));
                    }
                };
                let rc = sys::av_seek_frame(
                    self.ctx,
                    v.source_index as c_int,
                    target,
                    sys::AVSEEK_FLAG_BACKWARD,
                );
                if rc < 0 {
                    sys::av_packet_free(&mut pkt);
                    return Err(MuxError::Io(format!("seek: {}", last_error(rc))));
                }
            }

            let mut out: Vec<EncodedPacket> = Vec::new();
            let mut finished = vec![false; wanted.len()];
            let mut first_video_done = false;
            loop {
                let rc = sys::av_read_frame(self.ctx, pkt);
                if rc == sys::AVERROR_EOF {
                    break;
                }
                if rc < 0 {
                    sys::av_packet_free(&mut pkt);
                    return Err(mux_internal(&format!("read: {}", last_error(rc))));
                }
                let si = (*pkt).stream_index as usize;
                if let Some(pi) = wanted.iter().position(|&w| w == si) {
                    let src = self
                        .stream_by_source_index(si)
                        .ok_or_else(|| mux_internal("stream vanished"))?;
                    let tb = src.time_base;
                    let pts = (*pkt).pts;
                    let dts = (*pkt).dts;
                    let dur = (*pkt).duration;
                    let key = (*pkt).flags & sys::AV_PKT_FLAG_KEY != 0;
                    let byte_pos = if (*pkt).pos >= 0 {
                        Some((*pkt).pos as u64)
                    } else {
                        None
                    };
                    let data =
                        std::slice::from_raw_parts((*pkt).data, (*pkt).size as usize).to_vec();

                    if pts != sys::AV_NOPTS_VALUE {
                        let pts_r = match ticks_to_rational(pts, tb) {
                            Some(p) => p,
                            None => {
                                sys::av_packet_free(&mut pkt);
                                return Err(mux_internal("non-exact source pts"));
                            }
                        };
                        if pts_r >= range.end {
                            finished[pi] = true;
                            if finished.iter().all(|&d| d) {
                                sys::av_packet_unref(pkt);
                                break;
                            }
                        } else if pts_r >= range.start {
                            if src.is_video && !first_video_done && !key {
                                sys::av_packet_free(&mut pkt);
                                return Err(MuxError::InvalidTrackSpec(
                                    "copy plan violated: first video packet is not a keyframe"
                                        .into(),
                                ));
                            }
                            if src.is_video {
                                first_video_done = true;
                            }
                            let out_id = remap
                                .iter()
                                .find_map(|(s, id)| if *s == si { Some(*id) } else { None })
                                .ok_or(MuxError::UnknownTrack(StreamId(si as u32)))?;
                            // shift to output axis: pts - start (exact)
                            let shifted = pts_r.sub(range.start);
                            let dts_r = if dts != sys::AV_NOPTS_VALUE {
                                ticks_to_rational(dts, tb)
                            } else {
                                None
                            };
                            let dur_r = if dur > 0 {
                                ticks_to_rational(dur, tb)
                            } else {
                                None
                            };
                            out.push(EncodedPacket {
                                stream_id: out_id,
                                pts: shifted,
                                dts: dts_r.map(|d| d.sub(range.start)),
                                duration: dur_r,
                                keyframe: key,
                                byte_range_hint: byte_pos,
                                data,
                            });
                        }
                    }
                }
                sys::av_packet_unref(pkt);
            }
            sys::av_packet_free(&mut pkt);
            out.sort_by_key(|p| (p.stream_id.0, p.pts));
            Ok(out)
        }
    }
}

fn sample_format_name(fmt: c_int) -> String {
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
