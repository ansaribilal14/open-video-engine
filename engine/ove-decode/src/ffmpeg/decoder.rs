//! `FfmpegSwDecoder` — the v1 reference decoder implementation
//! (DECODER_SPEC §4: libavformat/libavcodec/libavutil, LGPL-only, software).

use std::ffi::c_int;
use std::sync::atomic::{AtomicBool, Ordering::SeqCst};

use ffmpeg_sys_next as sys;
use ove_media::{
    AssetRef, BackendId, BitDepth, FrameBytes, FrameEnvelope, FrameMemory, FramePool, PixelFormat,
    PoolKey, StreamId, StreamKind,
};
use ove_time::Rational;

use super::{
    close_input, last_error, map_color, map_pixel_format, open_input, stream_facts,
    ticks_to_rational,
};
use crate::{DecodeConfig, DecodeError, Decoder, DecoderCaps, HwAccel, SeekMode};

/// libc EAGAIN on Linux; the conformance corpus runs on Linux (CI + sandbox).
const EAGAIN: c_int = 11;

/// Exact floor((pts × tb.num) / tb.den) in i128 — never fp (FRAME_CONTRACT §3).
fn rational_to_ticks_floor(pts: Rational, tb: sys::AVRational) -> i64 {
    let (tn, td) = if tb.den > 0 {
        (tb.num as i128, tb.den as i128)
    } else {
        (-(tb.num as i128), -(tb.den as i128))
    };
    let n = pts.num() as i128 * td;
    let d = pts.den() as i128 * tn;
    let mut q = n / d;
    let r = n % d;
    if r != 0 && (r < 0) != (d < 0) {
        q -= 1;
    }
    i64::try_from(q).expect("tick conversion overflows i64")
}

pub struct FfmpegSwDecoder {
    fmt_ctx: *mut sys::AVFormatContext,
    codec_ctx: *mut sys::AVCodecContext,
    stream_index: usize,
    time_base: sys::AVRational,
    packet: *mut sys::AVPacket,
    frame: *mut sys::AVFrame,
    eof_sent: bool,
    drain_done: bool,
    /// Armed by `seek(Exact)`: frames strictly before this are dropped (D-4).
    drop_until: Option<Rational>,
    snap_index: Option<ove_media::KeyframeIndex>,
    caps: DecoderCaps,
    pool: FramePool,
    pool_key: PoolKey,
    pool_strides: Vec<usize>,
    pool_bytes: usize,
    cancelled: AtomicBool,
    stream_id: StreamId,
    width: u32,
    height: u32,
    pixel_format: PixelFormat,
    bit_depth: BitDepth,
    format_sys_id: sys::AVPixelFormat,
}

// Raw-pointer session; sending the session across threads is fine, sharing it
// concurrently is not (Sync intentionally NOT implemented).
unsafe impl Send for FfmpegSwDecoder {}

impl Drop for FfmpegSwDecoder {
    fn drop(&mut self) {
        unsafe {
            sys::av_frame_free(&mut self.frame);
            sys::av_packet_free(&mut self.packet);
            sys::avcodec_free_context(&mut self.codec_ctx);
            close_input(&mut self.fmt_ctx);
        }
    }
}

fn pe2de(e: ove_media::ProbeError) -> DecodeError {
    match e {
        ove_media::ProbeError::Unreadable => DecodeError::Unreadable,
        ove_media::ProbeError::Corrupt(d) => DecodeError::Corrupt(d),
        ove_media::ProbeError::Unsupported(d) => DecodeError::Unsupported(d),
        ove_media::ProbeError::Io(d) => DecodeError::Io(d),
    }
}

impl Decoder for FfmpegSwDecoder {
    fn open(asset: &AssetRef, stream: StreamId, cfg: DecodeConfig) -> Result<Self, DecodeError> {
        // D-9: hardware requested but this is a software adapter — typed
        // rejection, never a hidden downgrade.
        if let Some(acc) = cfg.hw {
            return Err(DecodeError::Unsupported(match acc {
                HwAccel::Auto => {
                    "hardware decode requested; FFmpeg-SW adapter is software-only".into()
                }
                HwAccel::Vendor(v) => {
                    format!("hardware decode ({v}) requested; FFmpeg-SW adapter is software-only")
                }
            }));
        }

        let mut ctx = open_input(&asset.locator).map_err(pe2de)?;

        if stream.0 as usize >= unsafe { (*ctx).nb_streams } as usize {
            close_input(&mut ctx);
            return Err(DecodeError::Unsupported(format!(
                "stream {stream:?} does not exist"
            )));
        }
        let facts = match stream_facts(ctx, stream.0 as usize) {
            Ok(f) => f,
            Err(e) => {
                close_input(&mut ctx);
                return Err(pe2de(e));
            }
        };
        if facts.kind == StreamKind::Audio {
            close_input(&mut ctx);
            return Err(DecodeError::Unsupported(
                "audio decode is out of Wave-2 scope (typed rejection; see STATUS)".into(),
            ));
        }
        if facts.kind != StreamKind::Video {
            close_input(&mut ctx);
            return Err(DecodeError::Unsupported(format!(
                "stream {stream:?} is not a video stream"
            )));
        }

        let st = unsafe { *(*ctx).streams.add(stream.0 as usize) };
        let par = unsafe { (*st).codecpar };
        let mapped = match map_pixel_format(unsafe { (*par).format }) {
            Some(m) => m,
            None => {
                close_input(&mut ctx);
                return Err(DecodeError::Unsupported(format!(
                    "pixel format id {} not mapped in v1 (declared limitation)",
                    unsafe { (*par).format }
                )));
            }
        };

        let codec = unsafe { sys::avcodec_find_decoder((*par).codec_id) };
        if codec.is_null() {
            close_input(&mut ctx);
            return Err(DecodeError::Unsupported("no libavcodec decoder".into()));
        }
        let codec_ctx = unsafe { sys::avcodec_alloc_context3(codec) };
        if codec_ctx.is_null() {
            close_input(&mut ctx);
            return Err(DecodeError::Io("avcodec_alloc_context3 failed".into()));
        }
        let rc = unsafe { sys::avcodec_parameters_to_context(codec_ctx, par) };
        if rc < 0 {
            close_input(&mut ctx);
            return Err(DecodeError::Corrupt(last_error(rc)));
        }
        unsafe {
            // exact time basis for decoded frames (FRAME_CONTRACT §3)
            (*codec_ctx).pkt_timebase = (*st).time_base;
            (*codec_ctx).thread_count = cfg.thread_count as c_int;
            (*codec_ctx).thread_type = (sys::FF_THREAD_FRAME | sys::FF_THREAD_SLICE) as c_int;
        }
        let rc = unsafe { sys::avcodec_open2(codec_ctx, codec, std::ptr::null_mut()) };
        if rc < 0 {
            close_input(&mut ctx);
            return Err(DecodeError::Unsupported(last_error(rc)));
        }

        // compact (align=1) buffer geometry for this format/size
        let mut linesizes = [0 as c_int; 4];
        let (width, height) = (unsafe { (*par).width as u32 }, unsafe {
            (*par).height as u32
        });
        let rc = unsafe {
            sys::av_image_fill_linesizes(linesizes.as_mut_ptr(), mapped.sys_id, width as c_int)
        };
        if rc < 0 {
            close_input(&mut ctx);
            return Err(DecodeError::Internal(last_error(rc)));
        }
        let strides: Vec<usize> = linesizes[..mapped.planes]
            .iter()
            .map(|&l| l as usize)
            .collect();
        let buf_size = unsafe {
            sys::av_image_get_buffer_size(mapped.sys_id, width as c_int, height as c_int, 1)
        };
        if buf_size <= 0 {
            close_input(&mut ctx);
            return Err(DecodeError::Internal(
                "av_image_get_buffer_size failed".into(),
            ));
        }
        let pool_key = PoolKey::from_strides(mapped.ours.clone(), width, height, &strides);

        let mut seek_modes = vec![SeekMode::Exact];
        if cfg.keyframe_index.is_some() {
            seek_modes.push(SeekMode::Snap);
        }
        let caps = DecoderCaps {
            hw: None,
            seek_modes,
            memory_outputs: vec![FrameMemory::Cpu],
            pixel_formats: vec![mapped.ours.clone()],
            max_bit_depth: mapped.bit_depth,
            threaded: cfg.thread_count > 1,
        };

        let packet = unsafe { sys::av_packet_alloc() };
        let frame = unsafe { sys::av_frame_alloc() };
        if packet.is_null() || frame.is_null() {
            close_input(&mut ctx);
            return Err(DecodeError::Io("av_packet/av_frame_alloc failed".into()));
        }

        Ok(FfmpegSwDecoder {
            fmt_ctx: ctx,
            codec_ctx,
            stream_index: stream.0 as usize,
            time_base: unsafe { (*st).time_base },
            packet,
            frame,
            eof_sent: false,
            drain_done: false,
            drop_until: None,
            snap_index: cfg.keyframe_index,
            caps,
            pool: FramePool::new(8),
            pool_key,
            pool_strides: strides,
            pool_bytes: buf_size as usize,
            cancelled: AtomicBool::new(false),
            stream_id: stream,
            width,
            height,
            pixel_format: mapped.ours,
            bit_depth: mapped.bit_depth,
            format_sys_id: mapped.sys_id,
        })
    }

    fn capabilities(&self) -> &DecoderCaps {
        &self.caps
    }

    fn seek(&mut self, pts: Rational, mode: SeekMode) -> Result<Rational, DecodeError> {
        if self.cancelled.load(SeqCst) {
            return Err(DecodeError::Cancelled);
        }
        let (target_ticks, drop_until, landed) = match mode {
            SeekMode::Snap => {
                let index = self.snap_index.as_ref().ok_or_else(|| {
                    DecodeError::Unsupported(
                        "Snap seek requires a keyframe index (DecodeConfig::keyframe_index)".into(),
                    )
                })?;
                let entry = index.floor(pts).ok_or_else(|| {
                    DecodeError::Unsupported(format!(
                        "snap target {pts} is before the first keyframe"
                    ))
                })?;
                (
                    rational_to_ticks_floor(entry.pts, self.time_base),
                    None,
                    Some(entry.pts),
                )
            }
            SeekMode::Exact => {
                // decode forward from the ≤ pts keyframe, drop until exact
                (
                    rational_to_ticks_floor(pts, self.time_base),
                    Some(pts),
                    Some(pts),
                )
            }
        };
        let rc = unsafe {
            sys::avformat_seek_file(
                self.fmt_ctx,
                self.stream_index as c_int,
                i64::MIN,
                target_ticks,
                target_ticks,
                sys::AVSEEK_FLAG_BACKWARD as c_int,
            )
        };
        if rc < 0 {
            return Err(DecodeError::Internal(last_error(rc)));
        }
        unsafe { sys::avcodec_flush_buffers(self.codec_ctx) };
        self.eof_sent = false;
        self.drain_done = false;
        self.drop_until = drop_until;
        Ok(landed.expect("both modes return a landed pts"))
    }

    fn next(&mut self) -> Result<Option<FrameEnvelope>, DecodeError> {
        if self.cancelled.load(SeqCst) {
            return Err(DecodeError::Cancelled);
        }
        loop {
            let rc = unsafe { sys::avcodec_receive_frame(self.codec_ctx, self.frame) };
            if rc == 0 {
                let pts_ticks = unsafe { (*self.frame).pts };
                if pts_ticks < 0 {
                    unsafe { sys::av_frame_unref(self.frame) };
                    return Err(DecodeError::Corrupt("decoded frame without pts".into()));
                }
                let pts = match ticks_to_rational(pts_ticks, self.time_base) {
                    Some(p) => p,
                    None => {
                        unsafe { sys::av_frame_unref(self.frame) };
                        return Err(DecodeError::Corrupt("pts out of rational range".into()));
                    }
                };
                // D-4 drop-until: skip frames strictly before the Exact target
                if let Some(target) = self.drop_until {
                    if pts < target {
                        unsafe { sys::av_frame_unref(self.frame) };
                        continue;
                    }
                }
                let env = self.build_frame(pts)?;
                unsafe { sys::av_frame_unref(self.frame) };
                return Ok(Some(env));
            }
            if rc == -EAGAIN {
                // decoder wants input: feed one packet (or drain at EOF)
                self.feed_one_packet()?;
                continue;
            }
            if rc == sys::AVERROR_EOF {
                self.drain_done = true;
                return Ok(None);
            }
            return Err(DecodeError::Corrupt(last_error(rc)));
        }
    }

    fn flush(&mut self) {
        // D-6: reset decoder-internal state only; position and any armed
        // Exact drop target stay (next() continues from the last seek).
        unsafe { sys::avcodec_flush_buffers(self.codec_ctx) };
        self.eof_sent = false;
        self.drain_done = false;
    }

    fn cancel(&mut self) {
        self.cancelled.store(true, SeqCst);
        // cooperative: the next next() observes the flag and returns
        // Cancelled; Drop then frees all libav resources (D-7).
    }
}

impl FfmpegSwDecoder {
    /// Reclaim a consumed frame's CPU bytes into the pool (visible, checked).
    /// Holding a frame across a pool recycle is the contract's stale-hold bug;
    /// the generation tag makes it detectable (FRAME_CONTRACT §5.2).
    pub fn reclaim(&mut self, frame: FrameEnvelope) -> Result<(), DecodeError> {
        let gen = frame.generation;
        let data = match frame.cpu_bytes() {
            Some(FrameBytes { data, .. }) => data.clone(),
            None => return Err(DecodeError::Internal("reclaim of non-CPU frame".into())),
        };
        self.pool
            .check_live(&self.pool_key, gen)
            .map_err(|e| DecodeError::Internal(e.to_string()))?;
        self.pool
            .release_bytes(&self.pool_key, data, gen)
            .map_err(|e| DecodeError::Internal(e.to_string()))
    }

    pub fn pool_stats(&self) -> ove_media::PoolStats {
        self.pool.stats()
    }

    /// Convert the already-received AVFrame (not yet unreffed) into an owned
    /// FrameEnvelope with pooled, compact-layout CPU bytes.
    fn build_frame(&mut self, pts: Rational) -> Result<FrameEnvelope, DecodeError> {
        unsafe {
            // FRAME_CONTRACT §3: duration EXACT, never inferred from a rate.
            // A frame without a duration field is corrupt input, honestly typed.
            let dur_ticks = (*self.frame).duration;
            let duration = if dur_ticks > 0 {
                ticks_to_rational(dur_ticks, self.time_base)
                    .ok_or_else(|| DecodeError::Corrupt("duration out of rational range".into()))?
            } else {
                return Err(DecodeError::Corrupt("frame without duration".into()));
            };

            let keyframe = (*self.frame).flags & sys::AV_FRAME_FLAG_KEY != 0;
            let color = map_color(
                (*self.frame).color_primaries,
                (*self.frame).color_trc,
                (*self.frame).colorspace,
                (*self.frame).color_range,
                (*self.frame).chroma_location,
            );

            let mut buf =
                self.pool
                    .acquire(&self.pool_key, self.pool_bytes, self.pool_strides.clone());
            let rc = sys::av_image_copy_to_buffer(
                buf.data.as_mut_ptr(),
                buf.data.len() as c_int,
                (*self.frame).data.as_ptr() as *const *const u8,
                (*self.frame).linesize.as_ptr(),
                self.format_sys_id,
                self.width as c_int,
                self.height as c_int,
                1, // compact layout: explicit strides, no padding assumptions
            );
            if rc < 0 {
                let gen = buf.generation;
                let data = std::mem::take(&mut buf.data);
                let _ = self.pool.release_bytes(&self.pool_key, data, gen);
                return Err(DecodeError::Corrupt(last_error(rc)));
            }
            let generation = buf.generation;
            let bytes = buf.into_frame_bytes();

            Ok(FrameEnvelope::video_cpu(
                pts,
                duration,
                self.stream_id,
                self.width,
                self.height,
                self.pixel_format.clone(),
                self.bit_depth,
                color,
                bytes,
                keyframe,
                BackendId::FFmpegSw,
                generation,
            ))
        }
    }

    /// Read packets until one packet of our stream is fed to the decoder, or
    /// EOF is reached and the drain packet is sent.
    fn feed_one_packet(&mut self) -> Result<(), DecodeError> {
        loop {
            let rc = unsafe { sys::av_read_frame(self.fmt_ctx, self.packet) };
            if rc == sys::AVERROR_EOF {
                if !self.eof_sent {
                    let rc = unsafe { sys::avcodec_send_packet(self.codec_ctx, std::ptr::null()) };
                    if rc < 0 && rc != -EAGAIN {
                        return Err(DecodeError::Corrupt(last_error(rc)));
                    }
                    self.eof_sent = true;
                }
                return Ok(());
            }
            if rc < 0 {
                return Err(DecodeError::Corrupt(last_error(rc)));
            }
            let ours = unsafe { (*self.packet).stream_index as usize } == self.stream_index;
            if ours {
                let rc = unsafe { sys::avcodec_send_packet(self.codec_ctx, self.packet) };
                unsafe { sys::av_packet_unref(self.packet) };
                if rc < 0 && rc != -EAGAIN {
                    return Err(DecodeError::Corrupt(last_error(rc)));
                }
                return Ok(()); // fed (EAGAIN from send just means: receive first)
            }
            unsafe { sys::av_packet_unref(self.packet) };
        }
    }
}
