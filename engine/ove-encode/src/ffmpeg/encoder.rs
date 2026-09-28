//! `FfmpegSwEncoder` — the v1 software video encoder adapter.
//!
//! v1 codec: libavcodec-native **mpeg4** (MPEG-4 Part 2) — LGPL,
//! dependency-free in both the system and bundled FFmpeg builds, and
//! deterministic given pinned libav versions (ENCODER_SPEC §6). `OpenH264`
//! and `SvtAv1` are declared targets whose adapter legs land later; they
//! report TYPED `Unsupported` here (capability honesty, never a downgrade).
//!
//! Pixel handling: the codec works in YUV420P. Inputs already in YUV420P
//! are filled directly (zero conversion); RGB-family inputs declared in the
//! profile are converted via swscale INSIDE the adapter — a visible,
//! caps-declared conversion (colorspace chosen from the frame's matrix tag;
//! never silently assumed).
//!
//! Exactness: the encoder time base is 1/frame_rate; a fed frame whose pts
//! does not divide exactly onto that axis is a typed `NonExactTimestamp`
//! error (E-002) — no silent rounding, ever.

use std::ffi::c_int;

use ffmpeg_sys_next as sys;
use ove_media::{FrameEnvelope, FrameKind, FrameMemory, MatrixCoeffs, PixelFormat};
use ove_time::Rational;

use crate::{
    AudioCodec, EncodeError, EncoderCaps, EncoderConfig, RateControl, TrackCodec, TrackKind,
    TrackSpec, VideoCodec,
};

use super::{encoder_pix_fmt, exact_ticks, last_error, libav_versions};

pub struct FfmpegSwEncoder {
    cfg: EncoderConfig,
    caps: EncoderCaps,
    ctx: *mut sys::AVCodecContext,
    /// Conversion context when the declared input format != YUV420P.
    sws: *mut sys::SwsContext,
    /// The frame handed to the encoder (always the codec's working format).
    enc_frame: *mut sys::AVFrame,
    track: TrackSpec,
    /// Packets produced by `feed` (receive loop runs eagerly to keep adapter
    /// buffering bounded and observable).
    pending: Vec<crate::EncodedPacket>,
    drained: bool,
    fed_frames: u64,
}

// SAFETY: the raw pointers are exclusively owned by `self` for the session's
// lifetime (created in `configure`, freed in `Drop`); no libav call site
// shares them across threads. Mirrors the ove-decode adapter pattern.
unsafe impl Send for FfmpegSwEncoder {}

impl FfmpegSwEncoder {
    fn feed_internal(&mut self, frame: FrameEnvelope) -> Result<(), EncodeError> {
        if self.drained {
            return Err(EncodeError::InvalidConfig("feed after drain".into()));
        }
        self.check_frame(&frame)?;

        unsafe {
            // ---- exact pts on the encoder axis ----
            let tb = (*self.ctx).time_base;
            let ticks = exact_ticks(frame.pts, tb)?;

            // ---- fill the encoder frame ----
            let ef = self.enc_frame;
            (*ef).pts = ticks;
            // The frame is REUSED across feeds — reset pict_type every time
            // (a stale I forces every frame intra; frame 0's keyframe flag
            // must not leak into frames 1..n).
            (*ef).key_frame = 0;
            (*ef).pict_type = sys::AVPictureType::AV_PICTURE_TYPE_NONE;
            if frame.keyframe {
                (*ef).key_frame = 1;
                (*ef).pict_type = sys::AVPictureType::AV_PICTURE_TYPE_I;
            }

            let fb = frame
                .cpu_bytes()
                .ok_or_else(|| EncodeError::FrameMismatch("frame has no CPU payload".into()))?;

            let needs_convert = self.sws.is_null();
            if needs_convert {
                // Direct YUV420P fill: 3 planes, explicit strides.
                if fb.strides.len() != 3 {
                    return Err(EncodeError::FrameMismatch(format!(
                        "expected 3 plane strides for YUV420P, got {}",
                        fb.strides.len()
                    )));
                }
                for p in 0..3 {
                    let linesize = (*ef).linesize[p] as usize;
                    let h = if p == 0 {
                        self.cfg.profile.height as usize
                    } else {
                        self.cfg.profile.height.div_ceil(2) as usize
                    };
                    let src = &fb.data[..];
                    // plane offsets follow the standard contiguous layout the
                    // renderer/pool produces: Y then U then V, explicit
                    // strides, packed (no inter-plane padding).
                    let plane_off = plane_offset(&fb.strides, p, self.cfg.profile.height as usize);
                    let w = if p == 0 {
                        self.cfg.profile.width as usize
                    } else {
                        self.cfg.profile.width as usize / 2
                    };
                    if plane_off + h * fb.strides[p] > src.len() {
                        return Err(EncodeError::FrameMismatch(
                            "YUV420P payload smaller than declared geometry".into(),
                        ));
                    }
                    for row in 0..h {
                        let s =
                            src[plane_off + row * fb.strides[p]..][..w.min(fb.strides[p])].as_ptr();
                        let d = (*ef).data[p].add(row * linesize);
                        std::ptr::copy_nonoverlapping(s, d, w.min(fb.strides[p]));
                    }
                }
            } else {
                // RGB → YUV420P through swscale (declared conversion).
                let in_fmt = encoder_pix_fmt(self.cfg.profile.pixel_format.clone())
                    .ok_or_else(|| EncodeError::Unsupported("pixel format".into()))?;
                // Temporary input frame views the caller's planes (no copy).
                let mut src_frame: *mut sys::AVFrame = sys::av_frame_alloc();
                if src_frame.is_null() {
                    return Err(EncodeError::Internal("av_frame_alloc failed".into()));
                }
                (*src_frame).format = in_fmt as c_int;
                (*src_frame).width = self.cfg.profile.width as c_int;
                (*src_frame).height = self.cfg.profile.height as c_int;
                (*src_frame).colorspace = av_colorspace(&frame.color);
                // single-plane packed RGB payloads
                if fb.strides.len() != 1 {
                    sys::av_frame_free(&mut src_frame);
                    return Err(EncodeError::FrameMismatch(
                        "RGB input must carry one packed plane".into(),
                    ));
                }
                (*src_frame).data[0] = fb.data.as_ptr().cast_mut();
                (*src_frame).linesize[0] = fb.strides[0] as c_int;
                let mut src_ptrs: [*const u8; 8] = [std::ptr::null(); 8];
                src_ptrs[0] = (*src_frame).data[0];
                let dst_ptrs: [*mut u8; 8] = {
                    let mut a = [std::ptr::null_mut(); 8];
                    a[..8].copy_from_slice(&(*ef).data);
                    a
                };
                let rc = sys::sws_scale(
                    self.sws,
                    src_ptrs.as_ptr(),
                    (*src_frame).linesize.as_ptr(),
                    0,
                    self.cfg.profile.height as c_int,
                    dst_ptrs.as_ptr(),
                    (*ef).linesize.as_ptr(),
                );
                sys::av_frame_free(&mut src_frame);
                if rc < 0 {
                    return Err(EncodeError::Internal(format!(
                        "sws_scale failed: {}",
                        last_error(rc)
                    )));
                }
                // The converter derives output color metadata; pin ours.
                (*ef).colorspace = av_colorspace(&frame.color);
            }

            // ---- send to encoder + drain available packets ----
            let rc = sys::avcodec_send_frame(self.ctx, ef);
            if rc < 0 {
                return Err(EncodeError::Internal(format!(
                    "avcodec_send_frame: {}",
                    last_error(rc)
                )));
            }
            self.receive_available()?;
        }
        Ok(())
    }

    fn receive_available(&mut self) -> Result<(), EncodeError> {
        unsafe {
            loop {
                let pkt = sys::av_packet_alloc();
                if pkt.is_null() {
                    return Err(EncodeError::Internal("av_packet_alloc failed".into()));
                }
                let rc = sys::avcodec_receive_packet(self.ctx, pkt);
                if rc == sys::AVERROR(EAGAIN_LINUX) {
                    sys::av_packet_free(&mut { pkt });
                    return Ok(());
                }
                if rc < 0 {
                    sys::av_packet_free(&mut { pkt });
                    return Err(EncodeError::Internal(format!(
                        "avcodec_receive_packet: {}",
                        last_error(rc)
                    )));
                }
                let tb = (*self.ctx).time_base;
                let tb_r = Rational::new(tb.num as i64, tb.den as i64);
                let pts = pkt_ticks_to_rational((*pkt).pts, tb_r);
                let dts = pkt_ticks_to_rational((*pkt).dts, tb_r);
                // 0 ticks = the codec did not set a duration (mpeg4 does
                // not) — report None, never a fake 0 (which would corrupt
                // downstream stts tables).
                let duration =
                    pkt_ticks_to_rational((*pkt).duration, tb_r).filter(|d| d.num() != 0);
                let keyframe = (*pkt).flags & sys::AV_PKT_FLAG_KEY != 0;
                let data = std::slice::from_raw_parts((*pkt).data, (*pkt).size as usize).to_vec();
                sys::av_packet_free(&mut { pkt });
                self.pending.push(crate::EncodedPacket {
                    stream_id: self.track.stream_id,
                    pts: pts.ok_or_else(|| EncodeError::Internal("packet without pts".into()))?,
                    dts,
                    duration,
                    keyframe,
                    byte_range_hint: None,
                    data,
                });
            }
        }
    }

    fn check_frame(&self, frame: &FrameEnvelope) -> Result<(), EncodeError> {
        if frame.memory != FrameMemory::Cpu {
            return Err(EncodeError::FrameMismatch(
                "encoder v1 accepts CPU frames only (GPU readback is the caller's visible step)"
                    .into(),
            ));
        }
        if frame.width != self.cfg.profile.width || frame.height != self.cfg.profile.height {
            return Err(EncodeError::FrameMismatch(format!(
                "frame {}x{} != configured {}x{}",
                frame.width, frame.height, self.cfg.profile.width, self.cfg.profile.height
            )));
        }
        if frame.pixel_format != self.cfg.profile.pixel_format {
            return Err(EncodeError::FrameMismatch(format!(
                "frame format {:?} != declared input format {:?}",
                frame.pixel_format, self.cfg.profile.pixel_format
            )));
        }
        if !matches!(frame.kind, FrameKind::Video) {
            return Err(EncodeError::FrameMismatch("non-video frame fed".into()));
        }
        Ok(())
    }
}

fn plane_offset(strides: &[usize], plane: usize, height: usize) -> usize {
    match plane {
        0 => 0,
        1 => strides[0] * height,
        2 => strides[0] * height + strides[1] * height.div_ceil(2),
        _ => 0,
    }
}

fn sws_colorspace(color: &ove_media::ColorTags) -> c_int {
    match color.matrix {
        MatrixCoeffs::Bt709 | MatrixCoeffs::Identity => sys::SWS_CS_ITU709,
        MatrixCoeffs::Smpte170m | MatrixCoeffs::Bt470bg => sys::SWS_CS_ITU601,
        MatrixCoeffs::Smpte240m => sys::SWS_CS_SMPTE240M,
        _ => sys::SWS_CS_DEFAULT,
    }
}

/// AVFrame.colorspace is the libav enum (bindgen type), distinct from the
/// SWS_CS_* param ids used by sws_getContext.
fn av_colorspace(color: &ove_media::ColorTags) -> sys::AVColorSpace {
    match color.matrix {
        MatrixCoeffs::Identity => sys::AVColorSpace::AVCOL_SPC_RGB,
        MatrixCoeffs::Bt709 => sys::AVColorSpace::AVCOL_SPC_BT709,
        MatrixCoeffs::Bt470bg => sys::AVColorSpace::AVCOL_SPC_BT470BG,
        MatrixCoeffs::Smpte170m => sys::AVColorSpace::AVCOL_SPC_SMPTE170M,
        MatrixCoeffs::Smpte240m => sys::AVColorSpace::AVCOL_SPC_SMPTE240M,
        MatrixCoeffs::Bt2020Ncl => sys::AVColorSpace::AVCOL_SPC_BT2020_NCL,
        MatrixCoeffs::Bt2020Pcl => sys::AVColorSpace::AVCOL_SPC_BT2020_CL,
        _ => sys::AVColorSpace::AVCOL_SPC_UNSPECIFIED,
    }
}

fn pkt_ticks_to_rational(ticks: i64, tb: Rational) -> Option<Rational> {
    // seconds = ticks × tb = (ticks × tb.num) / tb.den — ALWAYS exact as a
    // rational (the axis IS the rational); only i64 overflow is a failure.
    // (Contrast with the inverse direction, exact_ticks, which legitimately
    // demands divisibility because ticks must be integers.)
    let numer = (ticks as i128) * (tb.num() as i128);
    let denom = tb.den() as i128;
    if denom == 0 {
        return None;
    }
    let g = gcd128(numer.unsigned_abs(), denom.unsigned_abs()).max(1);
    let n = numer / g;
    let d = denom / g;
    Some(Rational::new(
        i64::try_from(n).ok()?,
        i64::try_from(d).ok()?,
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

/// Linux EAGAIN (11): receive-packet would-block. CI targets Linux only
/// (both the system and bundled paths); revisit with a platform port.
const EAGAIN_LINUX: i32 = 11;

impl Drop for FfmpegSwEncoder {
    fn drop(&mut self) {
        unsafe {
            if !self.sws.is_null() {
                sys::sws_freeContext(self.sws);
            }
            if !self.enc_frame.is_null() {
                sys::av_frame_free(&mut self.enc_frame);
            }
            if !self.ctx.is_null() {
                sys::avcodec_free_context(&mut self.ctx);
            }
        }
    }
}

impl crate::Encoder for FfmpegSwEncoder {
    fn configure(cfg: EncoderConfig) -> Result<Self, EncodeError> {
        cfg.profile.validate()?;
        let codec_id = match cfg.codec {
            VideoCodec::Mpeg4 => sys::AVCodecID::AV_CODEC_ID_MPEG4,
            VideoCodec::OpenH264 => {
                return Err(EncodeError::Unsupported(
                    "OpenH264 adapter leg not landed (ADR-015); use Mpeg4 in v1".into(),
                ))
            }
            VideoCodec::SvtAv1 => {
                return Err(EncodeError::Unsupported(
                    "SVT-AV1 adapter leg not landed (ADR-015); use Mpeg4 in v1".into(),
                ))
            }
        };

        unsafe {
            let codec = sys::avcodec_find_encoder(codec_id);
            if codec.is_null() {
                return Err(EncodeError::Unsupported(format!(
                    "libavcodec has no encoder for codec id {codec_id:?}"
                )));
            }
            let mut ctx = sys::avcodec_alloc_context3(codec);
            if ctx.is_null() {
                return Err(EncodeError::Internal(
                    "avcodec_alloc_context3 failed".into(),
                ));
            }

            let rate = cfg.profile.frame_rate;
            // encoder time base = 1/frame_rate (exact rational axis)
            (*ctx).time_base = sys::AVRational {
                num: rate.den() as c_int,
                den: rate.num() as c_int,
            };
            (*ctx).width = cfg.profile.width as c_int;
            (*ctx).height = cfg.profile.height as c_int;
            // v1 working format: 4:2:0 for mpeg4 (validated even dims above)
            (*ctx).pix_fmt = sys::AVPixelFormat::AV_PIX_FMT_YUV420P;
            (*ctx).gop_size = cfg.profile.gop as c_int;
            (*ctx).max_b_frames = 0; // deterministic simple path (v1, ADR-015)
            if cfg.profile.bitexact {
                (*ctx).flags |= sys::AV_CODEC_FLAG_BITEXACT as c_int;
            }
            match cfg.rate {
                RateControl::Cbr { bitrate } => (*ctx).bit_rate = bitrate as i64,
                RateControl::Crf { quality } => {
                    (*ctx).flags |= sys::AV_CODEC_FLAG_QSCALE as c_int;
                    (*ctx).global_quality = sys::FF_QP2LAMBDA * quality as i32;
                }
            }

            let rc = sys::avcodec_open2(ctx, codec, std::ptr::null_mut());
            if rc < 0 {
                sys::avcodec_free_context(&mut ctx);
                return Err(EncodeError::Unsupported(format!(
                    "avcodec_open2 failed: {}",
                    last_error(rc)
                )));
            }

            // extradata (global header) → track spec
            let extradata = if !(*ctx).extradata.is_null() && (*ctx).extradata_size > 0 {
                std::slice::from_raw_parts((*ctx).extradata, (*ctx).extradata_size as usize)
                    .to_vec()
            } else {
                Vec::new()
            };

            // conversion context when input != YUV420P
            let in_fmt = encoder_pix_fmt(cfg.profile.pixel_format.clone())
                .ok_or_else(|| EncodeError::Unsupported("input pixel format".into()))?;
            let sws = if in_fmt != sys::AVPixelFormat::AV_PIX_FMT_YUV420P {
                // declared conversion carries the declared matrix tag into
                // the conversion (no silent default colorspace); SWS param
                // is f64 (src, dst colorspace ids)
                let cs = sws_colorspace(&cfg.profile.color) as f64;
                let param = [cs, cs];
                let c = sys::sws_getContext(
                    cfg.profile.width as c_int,
                    cfg.profile.height as c_int,
                    in_fmt,
                    cfg.profile.width as c_int,
                    cfg.profile.height as c_int,
                    sys::AVPixelFormat::AV_PIX_FMT_YUV420P,
                    sys::SWS_BILINEAR,
                    std::ptr::null_mut(),
                    std::ptr::null_mut(),
                    param.as_ptr(),
                );
                if c.is_null() {
                    sys::avcodec_free_context(&mut ctx);
                    return Err(EncodeError::Internal("sws_getContext failed".into()));
                }
                c
            } else {
                std::ptr::null_mut()
            };

            let mut enc_frame = sys::av_frame_alloc();
            if enc_frame.is_null() {
                sys::avcodec_free_context(&mut ctx);
                return Err(EncodeError::Internal("av_frame_alloc failed".into()));
            }
            (*enc_frame).format = sys::AVPixelFormat::AV_PIX_FMT_YUV420P as c_int;
            (*enc_frame).width = cfg.profile.width as c_int;
            (*enc_frame).height = cfg.profile.height as c_int;
            let rc = sys::av_frame_get_buffer(enc_frame, 0);
            if rc < 0 {
                sys::av_frame_free(&mut enc_frame);
                sys::avcodec_free_context(&mut ctx);
                return Err(EncodeError::Internal(format!(
                    "av_frame_get_buffer: {}",
                    last_error(rc)
                )));
            }

            let timescale = TrackSpec::video_timescale(rate)
                .map_err(|e| EncodeError::InvalidConfig(e.to_string()))?;
            let track = TrackSpec {
                stream_id: cfg_track_stream(&cfg),
                codec: TrackCodec::Mpeg4,
                timescale,
                kind: TrackKind::Video {
                    width: cfg.profile.width,
                    height: cfg.profile.height,
                    pixel_format: PixelFormat::Yuv420p,
                    color: cfg.profile.color,
                    frame_rate: rate,
                },
                extradata,
            };

            let caps = EncoderCaps {
                codecs: vec![VideoCodec::Mpeg4],
                memory_inputs: vec![PixelFormat::Yuv420p],
                converted_inputs: if sws.is_null() {
                    Vec::new()
                } else {
                    vec![cfg.profile.pixel_format.clone()]
                },
                threaded: false, // single-threaded encode: deterministic (v1)
                deterministic: true,
                versions: libav_versions(),
            };

            Ok(FfmpegSwEncoder {
                caps,
                cfg,
                ctx,
                sws,
                enc_frame,
                track,
                pending: Vec::new(),
                drained: false,
                fed_frames: 0,
            })
        }
    }

    fn capabilities(&self) -> &EncoderCaps {
        &self.caps
    }

    fn track_spec(&self) -> Result<TrackSpec, EncodeError> {
        Ok(self.track.clone())
    }

    fn feed(&mut self, frame: FrameEnvelope) -> Result<(), EncodeError> {
        self.fed_frames += 1;
        self.feed_internal(frame)
    }

    fn drain(&mut self) -> Result<Vec<crate::EncodedPacket>, EncodeError> {
        if self.drained {
            return Err(EncodeError::InvalidConfig("drain called twice".into()));
        }
        self.drained = true;
        unsafe {
            // flush: NULL frame → buffered packets, then EOF
            let rc = sys::avcodec_send_frame(self.ctx, std::ptr::null());
            if rc < 0 && rc != sys::AVERROR_EOF {
                return Err(EncodeError::Internal(format!(
                    "flush send_frame: {}",
                    last_error(rc)
                )));
            }
            loop {
                let pkt = sys::av_packet_alloc();
                if pkt.is_null() {
                    return Err(EncodeError::Internal("av_packet_alloc failed".into()));
                }
                let rc = sys::avcodec_receive_packet(self.ctx, pkt);
                if rc < 0 {
                    sys::av_packet_free(&mut { pkt });
                    // AVERROR_EOF = flush complete
                    break;
                }
                let tb = (*self.ctx).time_base;
                let tb_r = Rational::new(tb.num as i64, tb.den as i64);
                let pts = pkt_ticks_to_rational((*pkt).pts, tb_r);
                let dts = pkt_ticks_to_rational((*pkt).dts, tb_r);
                let duration =
                    pkt_ticks_to_rational((*pkt).duration, tb_r).filter(|d| d.num() != 0);
                let keyframe = (*pkt).flags & sys::AV_PKT_FLAG_KEY != 0;
                let data = std::slice::from_raw_parts((*pkt).data, (*pkt).size as usize).to_vec();
                sys::av_packet_free(&mut { pkt });
                self.pending.push(crate::EncodedPacket {
                    stream_id: self.track.stream_id,
                    pts: pts
                        .ok_or_else(|| EncodeError::Internal("flush packet without pts".into()))?,
                    dts,
                    duration,
                    keyframe,
                    byte_range_hint: None,
                    data,
                });
            }
        }
        Ok(std::mem::take(&mut self.pending))
    }
}

fn cfg_track_stream(_cfg: &EncoderConfig) -> ove_media::StreamId {
    // v1 single-video-track sessions; multi-track composition joins at W6.
    ove_media::StreamId(0)
}

/// Silence an unused-import lint when features shift; AudioCodec is part of
/// the public surface re-exported through lib.rs.
#[allow(dead_code)]
fn _audio_codec_surface(_c: AudioCodec) {}
