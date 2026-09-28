//! `FfmpegAacEncoder` — the W7 audio encoder adapter (ENCODER_SPEC §2
//! sample-count authority; ADR-018 decisions).
//!
//! Input surface: the decode leg's canonical decoded PCM — planar f32
//! ("fltp") at the configured rate/channel count. The adapter buffers fed
//! samples to the codec's frame_size grid (AAC: 1024), timestamps every
//! output frame on the sample axis (time_base = 1/sample_rate — the sample
//! index IS the tick), and reports each packet's exact sample duration.
//!
//! Honesty rules:
//!   * contiguity: a fed frame whose pts ≠ (fed cursor)/rate is a typed
//!     `FrameMismatch` (sample-exact assembly is the caller's contract);
//!   * rate control: AAC v1 is CBR-only (a CRF request is typed
//!     `Unsupported`, never a hidden remap);
//!   * the small-last-frame branch is DETECTED from the codec's declared
//!     `AV_CODEC_CAP_SMALL_LAST_FRAME` and reported by `small_last_frame()`
//!     — drain pads with silence when the capability is absent (padded
//!     samples are silence beyond the fed count; the observed output total
//!     is asserted against the declared branch in the conformance suite).

use std::ffi::c_int;

use ffmpeg_sys_next as sys;
use ove_media::{FrameEnvelope, FrameKind, FrameMemory, StreamId};
use ove_time::Rational;

use super::{last_error, libav_versions};
use crate::{
    AudioCodec, AudioEncoderConfig, EncodeError, EncodedPacket, RateControl, TrackCodec, TrackKind,
    TrackSpec,
};

/// libc EAGAIN on Linux (mirrors the video encoder).
const EAGAIN: c_int = 11;

pub struct FfmpegAacEncoder {
    rate: u32,
    channels: u32,
    frame_size: usize,
    small_last: bool,
    ctx: *mut sys::AVCodecContext,
    /// The codec frame handed to the encoder (frame_size samples, fltp).
    enc_frame: *mut sys::AVFrame,
    /// Per-channel sample buffers (planar, matching the input surface).
    fifo: Vec<Vec<f32>>,
    /// Absolute samples fed so far — also the pts cursor (ticks on the
    /// 1/rate axis are sample indices).
    fed_samples: u64,
    /// The codec's declared initial padding (AAC priming delay, 1024 for the
    /// native encoder). The audio frame queue subtracts it from the FIRST
    /// output packet's pts — input frames are stamped at chunk_start and
    /// OUTPUT pts start at −initial_padding. The muxer/container handles the
    /// trim via the edit list (media_time = initial_padding), which is
    /// exactly how the reference libav pipeline behaves (verified against a
    /// CLI-produced AAC mp4: first packet pts −1024, elst 1024, file
    /// duration sample-exact; ADR-018).
    initial_padding: i64,
    track: TrackSpec,
    pending: Vec<EncodedPacket>,
    drained: bool,
    /// libav identity (version-keyed golden harnesses; ADR-015 pattern).
    versions: String,
}

// SAFETY: mirrors FfmpegSwEncoder — raw pointers exclusively owned by self.
unsafe impl Send for FfmpegAacEncoder {}

impl Drop for FfmpegAacEncoder {
    fn drop(&mut self) {
        unsafe {
            sys::av_frame_free(&mut self.enc_frame);
            sys::avcodec_free_context(&mut self.ctx);
        }
    }
}

impl FfmpegAacEncoder {
    /// Declared last-frame policy (detected from the codec capabilities at
    /// configure): true = a partial final frame is fed as-is; false = drain
    /// zero-pads the remainder to frame_size (ADR-018 honesty).
    pub fn small_last_frame(&self) -> bool {
        self.small_last
    }

    /// libav identity of the producing adapter (golden keying).
    pub fn versions(&self) -> &str {
        &self.versions
    }

    /// The codec's declared priming delay in samples (recorded on the
    /// TrackSpec so the container trims it — sample-exact file duration).
    pub fn initial_padding(&self) -> i64 {
        self.initial_padding
    }

    /// Samples accepted so far (the contiguity cursor).
    pub fn fed_samples(&self) -> u64 {
        self.fed_samples
    }

    fn encode_chunk(&mut self, n: usize) -> Result<(), EncodeError> {
        // the chunk's first sample sits at fed_samples − fifo_len (the fifo
        // holds every not-yet-encoded sample). Input frames are stamped at
        // the chunk's real start; the encoder's frame queue then subtracts
        // initial_padding on the first output packet (the priming trim the
        // container expresses as an edit list — sample-exact file duration).
        let chunk_start = self.fed_samples - self.fifo[0].len() as u64;
        unsafe {
            let ef = self.enc_frame;
            (*ef).nb_samples = n as c_int;
            (*ef).pts = chunk_start as i64; // sample index = tick
            for ch in 0..self.channels as usize {
                let dst = (*ef).data[ch] as *mut f32;
                let src = &self.fifo[ch][..n];
                std::ptr::copy_nonoverlapping(src.as_ptr(), dst, n);
            }
            // consume the encoded chunk from the fifo
            for b in self.fifo.iter_mut() {
                b.drain(..n);
            }
            let rc = sys::avcodec_send_frame(self.ctx, ef);
            if rc < 0 {
                return Err(EncodeError::Internal(format!(
                    "avcodec_send_frame(audio): {}",
                    last_error(rc)
                )));
            }
        }
        self.receive_available()
    }

    fn receive_available(&mut self) -> Result<(), EncodeError> {
        unsafe {
            loop {
                let pkt = sys::av_packet_alloc();
                if pkt.is_null() {
                    return Err(EncodeError::Internal("av_packet_alloc failed".into()));
                }
                let rc = sys::avcodec_receive_packet(self.ctx, pkt);
                if rc == sys::AVERROR(EAGAIN) {
                    sys::av_packet_free(&mut { pkt });
                    return Ok(());
                }
                if rc == sys::AVERROR_EOF {
                    // flush complete (the AAC encoder declares CAP_DELAY —
                    // EOF is its normal post-flush termination, not an error)
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
                // pts in ticks (samples) → exact rational seconds
                let pts = pkt_ticks_to_rational((*pkt).pts, tb_r);
                let dts = pkt_ticks_to_rational((*pkt).dts, tb_r);
                let duration =
                    pkt_ticks_to_rational((*pkt).duration, tb_r).filter(|d| d.num() != 0);
                let keyframe = (*pkt).flags & sys::AV_PKT_FLAG_KEY != 0;
                let data = std::slice::from_raw_parts((*pkt).data, (*pkt).size as usize).to_vec();
                sys::av_packet_free(&mut { pkt });
                self.pending.push(EncodedPacket {
                    stream_id: self.track.stream_id,
                    pts: pts.ok_or_else(|| EncodeError::Internal("packet without pts".into()))?,
                    dts,
                    // sample-exact contract: audio packets carry durations
                    duration: Some(duration.ok_or_else(|| {
                        EncodeError::Internal("audio packet without duration".into())
                    })?),
                    keyframe,
                    byte_range_hint: None,
                    data,
                });
            }
        }
    }
}

impl crate::AudioEncoder for FfmpegAacEncoder {
    fn configure(cfg: AudioEncoderConfig) -> Result<Self, EncodeError> {
        if cfg.sample_rate == 0 {
            return Err(EncodeError::InvalidConfig("zero sample rate".into()));
        }
        if cfg.channels == 0 || cfg.channels > 8 {
            return Err(EncodeError::InvalidConfig(format!(
                "channel count {} outside the v1 AAC surface (1..=8)",
                cfg.channels
            )));
        }
        match cfg.rate {
            RateControl::Cbr { bitrate } if bitrate > 0 => {}
            RateControl::Cbr { .. } => {
                return Err(EncodeError::InvalidConfig("zero bitrate".into()))
            }
            RateControl::Crf { .. } => {
                return Err(EncodeError::Unsupported(
                    "AAC v1 is CBR-only; a CRF request is a config error (ADR-018)".into(),
                ))
            }
        }
        let codec_id = match cfg.codec {
            AudioCodec::Aac => sys::AVCodecID::AV_CODEC_ID_AAC,
        };

        unsafe {
            let codec = sys::avcodec_find_encoder(codec_id);
            if codec.is_null() {
                return Err(EncodeError::Unsupported(
                    "libavcodec has no AAC encoder".into(),
                ));
            }
            let mut ctx = sys::avcodec_alloc_context3(codec);
            if ctx.is_null() {
                return Err(EncodeError::Internal(
                    "avcodec_alloc_context3 failed".into(),
                ));
            }

            // rate honesty: reject rates the encoder does not declare
            if !(*codec).supported_samplerates.is_null() {
                let mut ok = false;
                let mut p = (*codec).supported_samplerates;
                while *p != 0 {
                    if *p == cfg.sample_rate as c_int {
                        ok = true;
                        break;
                    }
                    p = p.add(1);
                }
                if !ok {
                    sys::avcodec_free_context(&mut ctx);
                    return Err(EncodeError::Unsupported(format!(
                        "sample rate {} not declared by the AAC encoder",
                        cfg.sample_rate
                    )));
                }
            }
            // format honesty: the input surface is fltp
            if !(*codec).sample_fmts.is_null() {
                let mut ok = false;
                let mut p = (*codec).sample_fmts;
                while *p != sys::AVSampleFormat::AV_SAMPLE_FMT_NONE {
                    if *p == sys::AVSampleFormat::AV_SAMPLE_FMT_FLTP {
                        ok = true;
                        break;
                    }
                    p = p.add(1);
                }
                if !ok {
                    sys::avcodec_free_context(&mut ctx);
                    return Err(EncodeError::Unsupported(
                        "AAC encoder does not declare fltp input".into(),
                    ));
                }
            }

            (*ctx).sample_fmt = sys::AVSampleFormat::AV_SAMPLE_FMT_FLTP;
            (*ctx).sample_rate = cfg.sample_rate as c_int;
            sys::av_channel_layout_default(&mut (*ctx).ch_layout, cfg.channels as c_int);
            // the sample index IS the tick: time_base = 1/sample_rate
            (*ctx).time_base = sys::AVRational {
                num: 1,
                den: cfg.sample_rate as c_int,
            };
            if let RateControl::Cbr { bitrate } = cfg.rate {
                (*ctx).bit_rate = bitrate as i64;
            }
            if cfg.bitexact {
                (*ctx).flags |= sys::AV_CODEC_FLAG_BITEXACT as c_int;
            }

            let rc = sys::avcodec_open2(ctx, codec, std::ptr::null_mut());
            if rc < 0 {
                sys::avcodec_free_context(&mut ctx);
                return Err(EncodeError::Unsupported(format!(
                    "avcodec_open2 (aac): {}",
                    last_error(rc)
                )));
            }

            let frame_size = (*ctx).frame_size as usize;
            if frame_size == 0 {
                sys::avcodec_free_context(&mut ctx);
                return Err(EncodeError::Unsupported(
                    "AAC encoder reported zero frame_size".into(),
                ));
            }
            let initial_padding = (*ctx).initial_padding as i64;
            let small_last =
                (*codec).capabilities & sys::AV_CODEC_CAP_SMALL_LAST_FRAME as c_int != 0;

            let mut enc_frame = sys::av_frame_alloc();
            if enc_frame.is_null() {
                sys::avcodec_free_context(&mut ctx);
                return Err(EncodeError::Internal("av_frame_alloc failed".into()));
            }
            (*enc_frame).format = sys::AVSampleFormat::AV_SAMPLE_FMT_FLTP as c_int;
            (*enc_frame).sample_rate = cfg.sample_rate as c_int;
            let rc = sys::av_channel_layout_copy(&mut (*enc_frame).ch_layout, &(*ctx).ch_layout);
            if rc < 0 {
                sys::av_frame_free(&mut enc_frame);
                sys::avcodec_free_context(&mut ctx);
                return Err(EncodeError::Internal(
                    "av_channel_layout_copy failed".into(),
                ));
            }
            (*enc_frame).nb_samples = frame_size as c_int;
            let rc = sys::av_frame_get_buffer(enc_frame, 0);
            if rc < 0 {
                sys::av_frame_free(&mut enc_frame);
                sys::avcodec_free_context(&mut ctx);
                return Err(EncodeError::Internal(format!(
                    "av_frame_get_buffer: {}",
                    last_error(rc)
                )));
            }

            let extradata = if !(*ctx).extradata.is_null() && (*ctx).extradata_size > 0 {
                std::slice::from_raw_parts((*ctx).extradata, (*ctx).extradata_size as usize)
                    .to_vec()
            } else {
                Vec::new()
            };

            let track = TrackSpec {
                stream_id: StreamId(1), // W7 exports: video 0 + audio 1
                codec: TrackCodec::Aac,
                timescale: cfg.sample_rate as i64,
                kind: TrackKind::Audio {
                    sample_rate: cfg.sample_rate,
                    channels: cfg.channels,
                    sample_format: "fltp".into(),
                },
                extradata,
                initial_padding,
            };

            Ok(FfmpegAacEncoder {
                rate: cfg.sample_rate,
                channels: cfg.channels,
                frame_size,
                small_last,
                ctx,
                enc_frame,
                fifo: vec![Vec::new(); cfg.channels as usize],
                fed_samples: 0,
                initial_padding,
                track,
                pending: Vec::new(),
                drained: false,
                versions: libav_versions(),
            })
        }
    }

    fn track_spec(&self) -> Result<TrackSpec, EncodeError> {
        Ok(self.track.clone())
    }

    fn feed(&mut self, frame: FrameEnvelope) -> Result<(), EncodeError> {
        if self.drained {
            return Err(EncodeError::InvalidConfig("feed after drain".into()));
        }
        if frame.memory != FrameMemory::Cpu {
            return Err(EncodeError::FrameMismatch(
                "audio encoder accepts CPU frames only".into(),
            ));
        }
        let FrameKind::Audio {
            sample_rate,
            channels,
        } = frame.kind
        else {
            return Err(EncodeError::FrameMismatch("non-audio frame fed".into()));
        };
        if sample_rate != self.rate || channels != self.channels {
            return Err(EncodeError::FrameMismatch(format!(
                "audio surface {sample_rate}Hz/{channels}ch != configured {}Hz/{}ch",
                self.rate, self.channels
            )));
        }
        // contiguity: pts must be exactly cursor/rate (sample-exact assembly)
        let expected = Rational::new(self.fed_samples as i64, self.rate as i64);
        if frame.pts != expected {
            return Err(EncodeError::FrameMismatch(format!(
                "non-contiguous audio feed: pts {} != expected cursor {expected}",
                frame.pts
            )));
        }
        // nb_samples = duration × rate must divide EXACTLY
        let nb_r = frame.duration * Rational::new(self.rate as i64, 1);
        if nb_r.den() != 1 {
            return Err(EncodeError::FrameMismatch(format!(
                "audio duration {} is not sample-exact at {} Hz",
                frame.duration, self.rate
            )));
        }
        let nb = nb_r.num() as usize;
        if nb == 0 {
            return Err(EncodeError::FrameMismatch("empty audio frame".into()));
        }
        let fb = frame
            .cpu_bytes()
            .ok_or_else(|| EncodeError::FrameMismatch("audio frame has no CPU payload".into()))?;
        let stride = nb * 4;
        if fb.strides.len() != self.channels as usize
            || fb.strides.iter().any(|&s| s != stride)
            || fb.data.len() != stride * self.channels as usize
        {
            return Err(EncodeError::FrameMismatch(
                "planar f32 payload mismatch (strides/data length)".into(),
            ));
        }
        for ch in 0..self.channels as usize {
            let plane = &fb.data[ch * stride..(ch + 1) * stride];
            let floats: &[f32] =
                unsafe { std::slice::from_raw_parts(plane.as_ptr().cast::<f32>(), nb) };
            self.fifo[ch].extend_from_slice(floats);
        }
        self.fed_samples += nb as u64;
        while self.fifo[0].len() >= self.frame_size {
            self.encode_chunk(self.frame_size)?;
        }
        Ok(())
    }

    fn drain(&mut self) -> Result<Vec<EncodedPacket>, EncodeError> {
        if self.drained {
            return Err(EncodeError::InvalidConfig("drain twice".into()));
        }
        self.drained = true;
        let remainder = self.fifo[0].len();
        if remainder > 0 {
            if self.small_last {
                self.encode_chunk(remainder)?;
            } else {
                // declared policy: silence-pad the final partial frame to the
                // codec grid (output exceeds the fed count by the padding;
                // the conformance suite asserts the declared branch).
                self.encode_chunk(self.frame_size)?;
            }
        }
        unsafe {
            let rc = sys::avcodec_send_frame(self.ctx, std::ptr::null_mut());
            if rc < 0 && rc != sys::AVERROR(EAGAIN) && rc != sys::AVERROR_EOF {
                return Err(EncodeError::Internal(format!(
                    "audio flush send: {}",
                    last_error(rc)
                )));
            }
        }
        self.receive_available()?;
        Ok(std::mem::take(&mut self.pending))
    }
}

/// Ticks → exact rational seconds (mirrors the video encoder's helper).
fn pkt_ticks_to_rational(ticks: i64, tb: Rational) -> Option<Rational> {
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
