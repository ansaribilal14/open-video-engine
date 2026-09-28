//! `FfmpegMuxer` — MP4 muxing via libavformat (LGPL path).
//!
//! Rules implemented (ENCODER_SPEC §5):
//!   * Per-track timescales chosen to keep rationals exact (video:
//!     frame-rate-derived via `TrackSpec::video_timescale`; audio: sample
//!     rate). Packet timestamps convert EXACTLY onto the track axis —
//!     anything else is a typed `NonExactTimestamp` (E-002). No fp anywhere.
//!   * faststart (moov first) is on; bitexact mode strips identity metadata.
//!   * Edit-list/start policy: outputs start at pts 0 (the copy executor and
//!     encoder sessions emit shifted pts; E-3 asserts the 0 start).
//!   * `finalize()` reports per-track tables + the file's sha256 — an
//!     OUTPUT-ARTIFACT INTEGRITY FINGERPRINT (PROJECT_FORMAT_SPEC §2 hash
//!     policy; asset identity stays BLAKE3 in ove-media).

use std::collections::HashMap;
use std::ffi::c_int;

use ffmpeg_sys_next as sys;
use ove_media::StreamId;
use ove_time::Rational;

use crate::{
    Container, EncodedPacket, MuxError, OutputInfo, OutputSink, OutputTrackInfo, TrackCodec,
    TrackKind, TrackSpec,
};

use super::{apply_color_tags, cstring, encoder_pix_fmt, last_error, mux_internal};

pub struct FfmpegMuxer {
    fmt: *mut sys::AVFormatContext,
    /// stream_id → (stream index, effective tick axis denominator)
    index: HashMap<StreamId, (usize, i64)>,
    /// stream_id → written-span end in track ticks (max over written
    /// packets of pts+duration). The mp4 muxer does not populate
    /// AVStream.duration, so `finalize` reports the container value when
    /// present and this EXACT accounting otherwise (documented in
    /// ENCODER_SPEC §5 annotation).
    ends: HashMap<StreamId, i64>,
    tracks: Vec<TrackSpec>,
    path: std::path::PathBuf,
    /// Our own avio_open'd stream must be closed explicitly.
    owns_io: bool,
    closed: bool,
}

// SAFETY: exclusive ownership of the format context for the session's
// lifetime; no libav call site shares it across threads.
unsafe impl Send for FfmpegMuxer {}

impl Drop for FfmpegMuxer {
    fn drop(&mut self) {
        unsafe {
            if !self.fmt.is_null() {
                if self.owns_io && !(*self.fmt).pb.is_null() && !self.closed {
                    sys::avio_closep(&mut (*self.fmt).pb);
                }
                sys::avformat_free_context(self.fmt);
            }
        }
    }
}

fn codec_id_of(codec: TrackCodec) -> sys::AVCodecID {
    match codec {
        TrackCodec::Mpeg4 => sys::AVCodecID::AV_CODEC_ID_MPEG4,
        TrackCodec::Aac => sys::AVCodecID::AV_CODEC_ID_AAC,
    }
}

/// One output stream from one TrackSpec. Errors free nothing here — the
/// caller frees the context on Err (single ownership path, no leaks).
unsafe fn build_stream(fmt: *mut sys::AVFormatContext, t: &TrackSpec) -> Result<(), MuxError> {
    let st = sys::avformat_new_stream(fmt, std::ptr::null());
    if st.is_null() {
        return Err(mux_internal("avformat_new_stream failed"));
    }
    (*st).time_base = sys::AVRational {
        num: 1,
        den: t.timescale as c_int,
    };
    let par = (*st).codecpar;
    (*par).codec_type = match t.kind {
        TrackKind::Video { .. } => sys::AVMediaType::AVMEDIA_TYPE_VIDEO,
        TrackKind::Audio { .. } => sys::AVMediaType::AVMEDIA_TYPE_AUDIO,
    };
    (*par).codec_id = codec_id_of(t.codec);
    match &t.kind {
        TrackKind::Video {
            width,
            height,
            pixel_format,
            color,
            ..
        } => {
            (*par).width = *width as c_int;
            (*par).height = *height as c_int;
            let f = encoder_pix_fmt(pixel_format.clone()).ok_or_else(|| {
                MuxError::InvalidTrackSpec(format!("pixel format {pixel_format:?} not mappable"))
            })?;
            (*par).format = f as c_int;
            apply_color_tags(par, color);
        }
        TrackKind::Audio {
            sample_rate,
            channels,
            sample_format,
        } => {
            (*par).sample_rate = *sample_rate as c_int;
            sys::av_channel_layout_default(&mut (*par).ch_layout, *channels as c_int);
            (*par).format = sample_format_id(sample_format).ok_or_else(|| {
                MuxError::InvalidTrackSpec(format!("sample format {sample_format:?} not mappable"))
            })?;
        }
    }
    if t.initial_padding > 0 {
        // Codec priming delay (AAC encoder delay, ADR-018): movenc writes the
        // trim edit list from this, keeping the FILE duration sample-exact
        // even though the raw packets carry the padded pre-roll.
        (*par).initial_padding = t.initial_padding as c_int;
    }
    if !t.extradata.is_empty() {
        // codecpar owns its extradata buffer (av_freep'd by
        // avformat_free_context) — deep copy, never alias.
        let sz = t.extradata.len();
        let buf = sys::av_mallocz(sz + sys::AV_INPUT_BUFFER_PADDING_SIZE as usize);
        if buf.is_null() {
            return Err(mux_internal("av_mallocz for extradata failed"));
        }
        std::ptr::copy_nonoverlapping(t.extradata.as_ptr(), buf.cast(), sz);
        (*par).extradata = buf.cast();
        (*par).extradata_size = sz as c_int;
    }
    Ok(())
}

fn sample_format_id(name: &str) -> Option<c_int> {
    // Mirrors the decode-side table (string ids are the engine's honest
    // audio format surface; ove-media AudioDetails uses the same names).
    use sys::AVSampleFormat as S;
    let known: &[(&str, S)] = &[
        ("u8", S::AV_SAMPLE_FMT_U8),
        ("s16", S::AV_SAMPLE_FMT_S16),
        ("s32", S::AV_SAMPLE_FMT_S32),
        ("flt", S::AV_SAMPLE_FMT_FLT),
        ("dbl", S::AV_SAMPLE_FMT_DBL),
        ("u8p", S::AV_SAMPLE_FMT_U8P),
        ("s16p", S::AV_SAMPLE_FMT_S16P),
        ("s32p", S::AV_SAMPLE_FMT_S32P),
        ("fltp", S::AV_SAMPLE_FMT_FLTP),
        ("dblp", S::AV_SAMPLE_FMT_DBLP),
    ];
    known
        .iter()
        .find(|(n, _)| *n == name)
        .map(|(_, f)| *f as c_int)
}

impl crate::Muxer for FfmpegMuxer {
    fn open(
        sink: OutputSink,
        container: Container,
        tracks: Vec<TrackSpec>,
    ) -> Result<Self, MuxError> {
        if tracks.is_empty() {
            return Err(MuxError::InvalidTrackSpec("no tracks".into()));
        }
        let OutputSink::File(path) = sink;
        let (fmt_name, faststart, bitexact) = match container {
            Container::Mp4 {
                faststart,
                bitexact,
            } => ("mp4", faststart, bitexact),
        };
        unsafe {
            let mut fmt: *mut sys::AVFormatContext = std::ptr::null_mut();
            let cfmt = cstring(fmt_name, "container name")?;
            let cpath = cstring(
                path.to_str()
                    .ok_or_else(|| MuxError::Io("output path is not valid UTF-8".into()))?,
                "output path",
            )?;
            let rc = sys::avformat_alloc_output_context2(
                &mut fmt,
                std::ptr::null(),
                cfmt.as_ptr(),
                cpath.as_ptr(),
            );
            if rc < 0 || fmt.is_null() {
                return Err(mux_internal(&format!(
                    "avformat_alloc_output_context2: {}",
                    last_error(rc)
                )));
            }

            // Track-building; any error must free the context (no leaks).
            for t in &tracks {
                if let Err(e) = build_stream(fmt, t) {
                    sys::avformat_free_context(fmt);
                    return Err(e);
                }
            }

            if bitexact {
                (*fmt).flags |= sys::AVFMT_FLAG_BITEXACT;
            }

            let oflags = if (*fmt).oformat.is_null() {
                0
            } else {
                (*(*fmt).oformat).flags
            };
            // AVFMT_NOFILE means "the muxer opens its own IO". mp4 does NOT
            // set it — the CALLER must provide (*fmt).pb via avio_open.
            // (Getting this backwards leaves pb NULL and libav segfaults in
            // avformat_init_output instead of returning EINVAL — verified
            // empirically on libavformat 7.1.5, 2026-09-29.)
            let caller_needs_pb = oflags & sys::AVFMT_NOFILE == 0;
            if caller_needs_pb {
                let mut pb: *mut sys::AVIOContext = std::ptr::null_mut();
                let rc = sys::avio_open(&mut pb, cpath.as_ptr(), sys::AVIO_FLAG_WRITE);
                if rc < 0 {
                    sys::avformat_free_context(fmt);
                    return Err(MuxError::Io(format!("avio_open: {}", last_error(rc))));
                }
                (*fmt).pb = pb;
            }
            let owns_io = caller_needs_pb;

            // faststart: moov moved to file start at finalize
            let mut opts: *mut sys::AVDictionary = std::ptr::null_mut();
            if faststart {
                let key = cstring("movflags", "option key")?;
                let val = cstring("+faststart", "option value")?;
                sys::av_dict_set(&mut opts, key.as_ptr(), val.as_ptr(), 0);
            }
            let rc = sys::avformat_write_header(fmt, &mut opts);
            sys::av_dict_free(&mut opts);
            if rc < 0 {
                if owns_io {
                    sys::avio_closep(&mut (*fmt).pb);
                }
                sys::avformat_free_context(fmt);
                return Err(mux_internal(&format!(
                    "avformat_write_header: {}",
                    last_error(rc)
                )));
            }

            // effective axes (muxer may adjust; exactness uses the real one)
            let mut index = HashMap::new();
            for (i, t) in tracks.iter().enumerate() {
                let st = *(*fmt).streams.add(i);
                let tb = (*st).time_base;
                if tb.num != 1 {
                    if owns_io {
                        sys::avio_closep(&mut (*fmt).pb);
                    }
                    sys::avformat_free_context(fmt);
                    return Err(mux_internal(&format!(
                        "post-header time base {}/{} is not 1/N",
                        tb.num, tb.den
                    )));
                }
                index.insert(t.stream_id, (i, tb.den as i64));
            }

            Ok(FfmpegMuxer {
                fmt,
                index,
                ends: HashMap::new(),
                tracks,
                path,
                owns_io,
                closed: false,
            })
        }
    }

    fn write(&mut self, pkt: EncodedPacket) -> Result<(), MuxError> {
        unsafe {
            let &(idx, axis) = self
                .index
                .get(&pkt.stream_id)
                .ok_or(MuxError::UnknownTrack(pkt.stream_id))?;
            let track = &self.tracks[idx];

            // Ticks are computed against the EFFECTIVE axis — the one we
            // label the packet with (pkt->time_base = 1/axis). Using the
            // requested TrackSpec.timescale here would mix two axes when
            // the muxer widens the timescale (e.g. 24 -> 12288).
            let ticks_for_axis = |pts: Rational| -> Result<i64, MuxError> {
                let numer = (pts.num() as i128) * (axis as i128);
                let den = pts.den() as i128;
                if numer % den != 0 {
                    return Err(MuxError::NonExactTimestamp {
                        pts,
                        timescale: axis,
                    });
                }
                i64::try_from(numer / den).map_err(|_| mux_internal("tick count exceeds i64"))
            };
            let pts_ticks = ticks_for_axis(pkt.pts)?;
            let dts_ticks = match pkt.dts {
                Some(d) => ticks_for_axis(d)?,
                None => pts_ticks,
            };
            let dur_ticks = match pkt.duration {
                Some(d) => ticks_for_axis(d)?,
                None => match &track.kind {
                    TrackKind::Video { frame_rate, .. } => {
                        // One frame on the EFFECTIVE axis: axis × den / num
                        // ticks (movenc may widen the timescale — e.g. 24 ->
                        // 12288 — so 1 tick is NOT one frame). For 24 fps:
                        // 12288 × 1/24 = 512; for 30000/1001 @ 30000:
                        // 30000 × 1001/30000 = 1001. Must divide exactly;
                        // anything else is a typed error (E-002).
                        let num = frame_rate.num() as i128;
                        let den = frame_rate.den() as i128;
                        let ticks128 = (axis as i128) * den;
                        if num == 0 || ticks128 % num != 0 {
                            return Err(MuxError::NonExactTimestamp {
                                pts: pkt.pts,
                                timescale: axis,
                            });
                        }
                        (ticks128 / num) as i64
                    }
                    TrackKind::Audio { .. } => {
                        return Err(MuxError::InvalidTrackSpec(
                            "audio packets must carry an exact duration (sample-exact contract)"
                                .into(),
                        ));
                    }
                },
            };

            let payload = pkt.data;
            let mut p = sys::av_packet_alloc();
            if p.is_null() {
                return Err(mux_internal("av_packet_alloc failed"));
            }
            // Refcounted pattern: av_new_packet allocates a padded libav
            // buffer we copy into; av_interleaved_write_frame takes
            // ownership of the reference (move_ref semantics blank our
            // struct), so av_packet_free afterwards is safe and the payload
            // lifetime is libav's — no aliasing into our Vec.
            let rc_new = sys::av_new_packet(p, payload.len() as c_int);
            if rc_new < 0 {
                sys::av_packet_free(&mut p);
                return Err(mux_internal(&format!(
                    "av_new_packet: {}",
                    last_error(rc_new)
                )));
            }
            std::ptr::copy_nonoverlapping(payload.as_ptr(), (*p).data.cast(), payload.len());
            (*p).pts = pts_ticks;
            (*p).dts = dts_ticks;
            (*p).duration = dur_ticks;
            (*p).stream_index = idx as c_int;
            (*p).time_base = sys::AVRational {
                num: 1,
                den: axis as c_int,
            };
            if pkt.keyframe {
                (*p).flags |= sys::AV_PKT_FLAG_KEY;
            }
            if std::env::var("OVE_MUX_DEBUG").is_ok() {
                static SEEN: std::sync::atomic::AtomicUsize =
                    std::sync::atomic::AtomicUsize::new(0);
                let n = SEEN.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
                if n < 4 {
                    let st = *(*self.fmt).streams.add(idx);
                    eprintln!("DBG write[{n}]: idx={idx} axis={axis} pkt_tb={}/{} file_tb={}/{} pts={pts_ticks} dts={dts_ticks} dur={dur_ticks}",
                        (*p).time_base.num, (*p).time_base.den,
                        (*st).time_base.num, (*st).time_base.den);
                }
            }
            // written-span accounting (pts + duration; monotonic per track)
            let end = pts_ticks
                .checked_add(dur_ticks)
                .ok_or_else(|| mux_internal("tick end exceeds i64"))?;
            let e = self.ends.entry(pkt.stream_id).or_insert(0);
            if end > *e {
                *e = end;
            }
            let rc = sys::av_interleaved_write_frame(self.fmt, p);
            sys::av_packet_free(&mut p);
            if rc < 0 {
                return Err(mux_internal(&format!(
                    "av_interleaved_write_frame: {}",
                    last_error(rc)
                )));
            }
        }
        Ok(())
    }

    fn finalize(mut self) -> Result<OutputInfo, MuxError> {
        unsafe {
            let rc = sys::av_write_trailer(self.fmt);
            if rc < 0 {
                return Err(mux_internal(&format!(
                    "av_write_trailer: {}",
                    last_error(rc)
                )));
            }
            let mut out_tracks = Vec::new();
            for t in &self.tracks {
                let &(idx, axis) = &self.index[&t.stream_id];
                if std::env::var("OVE_MUX_DEBUG").is_ok() {
                    let st = *(*self.fmt).streams.add(idx);
                    eprintln!(
                        "DBG final: axis={axis} st_dur={} st_tb={}/{} end_acct={}",
                        (*st).duration,
                        (*st).time_base.num,
                        (*st).time_base.den,
                        self.ends.get(&t.stream_id).copied().unwrap_or(0)
                    );
                }
                let st = *(*self.fmt).streams.add(idx);
                let duration = if (*st).duration > 0 {
                    Some(Rational::new((*st).duration, axis))
                } else {
                    // container did not declare a stream duration — report
                    // the exact written-span accounting (pts+duration of
                    // the last written packet, per track)
                    let end = self.ends.get(&t.stream_id).copied().unwrap_or(0);
                    if end > 0 {
                        Some(Rational::new(end, axis))
                    } else {
                        None
                    }
                };
                let nb_frames = if (*st).nb_frames > 0 {
                    Some((*st).nb_frames as u64)
                } else {
                    None
                };
                out_tracks.push(OutputTrackInfo {
                    stream_id: t.stream_id,
                    timescale: axis,
                    duration,
                    nb_frames,
                });
            }
            if self.owns_io && !(*self.fmt).pb.is_null() {
                let rc = sys::avio_closep(&mut (*self.fmt).pb);
                self.closed = true;
                if rc < 0 {
                    return Err(MuxError::Io(format!("avio_closep: {}", last_error(rc))));
                }
            }

            // Artifact integrity fingerprint (hash policy: sha256 is the
            // output-record fingerprint; identity stays BLAKE3).
            let bytes = std::fs::read(&self.path)
                .map_err(|e| MuxError::Io(format!("finalize read: {e}")))?;
            let file_size = bytes.len() as u64;
            let digest = {
                use sha2::{Digest, Sha256};
                let mut h = Sha256::new();
                h.update(&bytes);
                h.finalize()
            };
            let mut hex = String::with_capacity(64);
            for b in digest {
                hex.push_str(&format!("{b:02x}"));
            }
            let path = self.path.clone();
            // suppress Drop double-close: context free happens in Drop
            let fmt = self.fmt;
            self.fmt = std::ptr::null_mut();
            sys::avformat_free_context(fmt);
            Ok(OutputInfo {
                path,
                tracks: out_tracks,
                file_sha256: hex,
                file_size,
            })
        }
    }
}
