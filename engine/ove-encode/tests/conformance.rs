//! ENCODER_SPEC §4 conformance suite (E-1..E-5, E-7 planner/executor gates).
//!
//! Verification strategy (mirrors the decode suite's honesty):
//!   * The engine-under-test produces real MP4s through the FFmpeg adapter
//!     (encoder + muxer + copy routes) at test time.
//!   * Outputs are verified through THREE independent eyes:
//!       1. the ffprobe CLI when present on the host (the directive's
//!          literal "ffprobe must inspect it" gate),
//!       2. libav demux/probe via ove-decode's `FfmpegProbe` (CI-safe —
//!          the same engine ffprobe itself uses),
//!       3. decoded-frame byte equality for content correctness (E-007).
//!   * Byte goldens (E-5) are KEYED ON THE PRODUCING libav identity
//!     (`libav_versions()`); same-run determinism is asserted everywhere.
//!     This is recorded in ADR-015: byte equality is pinned per encoder
//!     build, structure gates are version-agnostic — no silent weakening.

use std::path::{Path, PathBuf};

use ove_decode::ffmpeg::FfmpegSwDecoder;
use ove_decode::{DecodeConfig, Decoder};
use ove_encode::ffmpeg::{libav_versions, FfmpegCopySource, FfmpegMuxer, FfmpegSwEncoder};
use ove_encode::planner::{plan_export, CopyPolicy, TimeRange, TrackInput, TrackKindTag};
use ove_encode::{
    Container, EncodeError, Encoder, EncoderConfig, Muxer, OutputSink, RateControl, TrackSpec,
    VideoCodec, VideoProfile,
};
use ove_media::{
    AssetRef, BackendId, ChromaLoc, ColorTags, MatrixCoeffs, Primaries, ProbeBackend, Range,
    StreamId, Transfer,
};
use ove_time::Rational;
use serde_json::Value;

fn media(name: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests/media")
        .join(name)
}

fn media_asset(name: &str) -> AssetRef {
    AssetRef::from_path(media(name)).expect("corpus file present")
}

fn out_path(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join("ove-encode-conformance");
    std::fs::create_dir_all(&dir).expect("temp dir");
    dir.join(name)
}

fn asset_of(p: &Path) -> AssetRef {
    AssetRef::from_path(p).expect("output asset")
}

// ---------------------------------------------------------------------------
// Synthetic deterministic source frames (YUV420P, no fp anywhere)
// ---------------------------------------------------------------------------

const W: u32 = 320;
const H: u32 = 240;
const FPS: (i64, i64) = (24, 1);

fn yuv420p_frame(k: u64) -> ove_media::FrameEnvelope {
    let mut data = Vec::with_capacity(W as usize * H as usize * 3 / 2);
    for y in 0..H as usize {
        for x in 0..W as usize {
            data.push(((x + 4 * k as usize + y) % 256) as u8);
        }
    }
    for cy in 0..H as usize / 2 {
        for cx in 0..W as usize / 2 {
            data.push(((cx / 2 + cy + k as usize) % 256) as u8);
        }
    }
    for cy in 0..H as usize / 2 {
        for cx in 0..W as usize / 2 {
            data.push((((cx + cy + 2 * k as usize) % 128) * 2) as u8);
        }
    }
    let color = ColorTags {
        primaries: Primaries::Bt709,
        transfer: Transfer::Bt709,
        matrix: MatrixCoeffs::Bt709,
        range: Range::Limited,
        chroma_loc: Some(ChromaLoc::Left),
    };
    ove_media::FrameEnvelope::video_cpu(
        Rational::new(k as i64 * FPS.1, FPS.0), // pts = k/24 s (exact)
        Rational::new(FPS.1, FPS.0),            // duration 1/24 s
        StreamId(0),
        W,
        H,
        ove_media::PixelFormat::Yuv420p,
        ove_media::BitDepth::B8,
        color,
        ove_media::FrameBytes {
            data,
            strides: vec![W as usize, W as usize / 2, W as usize / 2],
        },
        k.is_multiple_of(24), // keyframe every second (matches gop)
        BackendId::Other("test-synthetic".into()),
        0,
    )
}

fn test_profile() -> VideoProfile {
    VideoProfile {
        width: W,
        height: H,
        frame_rate: Rational::new(FPS.0, FPS.1),
        pixel_format: ove_media::PixelFormat::Yuv420p,
        color: ColorTags {
            primaries: Primaries::Bt709,
            transfer: Transfer::Bt709,
            matrix: MatrixCoeffs::Bt709,
            range: Range::Limited,
            chroma_loc: Some(ChromaLoc::Left),
        },
        gop: 24,
        bitexact: true,
    }
}

/// Encode `n` synthetic frames and mux to `path`. Returns the track spec.
fn encode_n_to(n: u64, path: &Path) -> TrackSpec {
    let cfg = EncoderConfig {
        codec: VideoCodec::Mpeg4,
        profile: test_profile(),
        rate: RateControl::Crf { quality: 6 },
    };
    let mut enc = FfmpegSwEncoder::configure(cfg).expect("configure encoder");
    let track = enc.track_spec().expect("track spec");
    for k in 0..n {
        enc.feed(yuv420p_frame(k)).expect("feed frame");
    }
    let packets = enc.drain().expect("drain");
    assert_eq!(packets.len() as u64, n, "one packet per frame (CFR)");

    let mut mux = FfmpegMuxer::open(
        OutputSink::File(path.to_path_buf()),
        Container::Mp4 {
            faststart: true,
            bitexact: true,
        },
        vec![track.clone()],
    )
    .expect("open muxer");
    for p in packets {
        mux.write(p).expect("mux packet");
    }
    let info = mux.finalize().expect("finalize");
    assert_eq!(info.tracks.len(), 1);
    track
}

fn sha256_file(path: &Path) -> String {
    use sha2::{Digest, Sha256};
    let mut h = Sha256::new();
    let bytes = std::fs::read(path).expect("read output");
    h.update(&bytes);
    h.finalize().iter().map(|b| format!("{b:02x}")).collect()
}

// ---------------------------------------------------------------------------
// ffprobe CLI (the literal directive gate) — runs when the host has it
// ---------------------------------------------------------------------------

fn ffprobe_json(path: &Path) -> Option<Value> {
    let out = std::process::Command::new("ffprobe")
        .args([
            "-v",
            "error",
            "-show_streams",
            "-show_format",
            "-of",
            "json",
        ])
        .arg(path)
        .output()
        .ok()?;
    if !out.status.success() {
        return None;
    }
    serde_json::from_str(&String::from_utf8(out.stdout).ok()?).ok()
}

fn ffprobe_available() -> bool {
    std::process::Command::new("ffprobe")
        .arg("-version")
        .output()
        .is_ok()
}

// ---------------------------------------------------------------------------
// libav-side structural probe (CI-safe verification eye #2)
// ---------------------------------------------------------------------------

fn libav_probe(path: &Path) -> ove_media::ProbeInfo {
    use ove_decode::ffmpeg::FfmpegProbe;
    FfmpegProbe
        .probe(&asset_of(path))
        .expect("libav probe of produced output")
}

fn decode_all(path: &Path) -> Vec<ove_media::FrameEnvelope> {
    let mut dec = FfmpegSwDecoder::open(&asset_of(path), StreamId(0), DecodeConfig::default())
        .expect("decode produced output");
    let mut out = Vec::new();
    while let Some(f) = dec.next().expect("clean decode") {
        out.push(f);
    }
    out
}

// ---------------------------------------------------------------------------
// E-1: ffprobe parses output; nb_frames/frame count matches plan exactly
// ---------------------------------------------------------------------------

const N_FRAMES: u64 = 24; // 1 s @ 24 fps

#[test]
fn e1_frame_count_matches_plan() {
    let p = out_path("e1.mp4");
    encode_n_to(N_FRAMES, &p);

    // eye 2: libav probe (always)
    let info = libav_probe(&p);
    let v = &info.streams[0];
    assert_eq!(info.container, ove_media::ContainerKind::Mp4);
    assert_eq!(v.nb_frames_hint, Some(N_FRAMES), "container frame count");

    // decoded frame count == plan exactly
    let frames = decode_all(&p);
    assert_eq!(frames.len() as u64, N_FRAMES, "decoded count");

    // eye 1: ffprobe CLI when present
    if ffprobe_available() {
        let j = ffprobe_json(&p).expect("ffprobe parses our output");
        assert_eq!(j["streams"][0]["codec_name"], "mpeg4");
        assert_eq!(
            j["streams"][0]["nb_frames"]
                .as_str()
                .unwrap()
                .parse::<u64>()
                .unwrap(),
            N_FRAMES
        );
    }
}

// ---------------------------------------------------------------------------
// E-2: duration exactness — container duration == plan span ± 0 frames
// ---------------------------------------------------------------------------

#[test]
fn e2_duration_exact_cfr24() {
    let p = out_path("e2_cfr24.mp4");
    encode_n_to(N_FRAMES, &p);
    // exact span: 24 frames at 1/24 s = 1 s
    let info = libav_probe(&p);
    let dur = info.streams[0].duration.expect("stream duration");
    assert_eq!(dur, Rational::new(1, 1), "duration ± 0 frames");
}

#[test]
fn e2b_duration_exact_ntsc() {
    // 71 frames at 30000/1001 → duration 71*1001/30000 (exact rational,
    // the classic non-decimal axis; fp would fail this assert)
    let cfg = EncoderConfig {
        codec: VideoCodec::Mpeg4,
        profile: VideoProfile {
            frame_rate: Rational::new(30000, 1001),
            ..test_profile()
        },
        rate: RateControl::Crf { quality: 6 },
    };
    let mut enc = FfmpegSwEncoder::configure(cfg).expect("configure ntsc encoder");
    let track = enc.track_spec().expect("track spec");
    assert_eq!(track.timescale, 30000, "minimal exact timescale");
    const N: u64 = 71;
    for k in 0..N {
        let mut f = yuv420p_frame(k);
        f.pts = Rational::new(k as i64 * 1001, 30000);
        f.duration = Rational::new(1001, 30000);
        enc.feed(f).expect("feed ntsc frame");
    }
    let packets = enc.drain().expect("drain");
    let p = out_path("e2_ntsc.mp4");
    let mut mux = FfmpegMuxer::open(
        OutputSink::File(p.clone()),
        Container::Mp4 {
            faststart: true,
            bitexact: true,
        },
        vec![track],
    )
    .expect("open muxer");
    for pkt in packets {
        mux.write(pkt).expect("mux");
    }
    let info = mux.finalize().expect("finalize");
    let t = &info.tracks[0];
    assert_eq!(
        t.duration,
        Some(Rational::new(71 * 1001, 30000)),
        "exact ntsc duration"
    );
}

// ---------------------------------------------------------------------------
// E-3: timestamp monotonicity + start offsets (0 start policy)
// ---------------------------------------------------------------------------

#[test]
fn e3_monotonic_zero_start() {
    let p = out_path("e3.mp4");
    encode_n_to(48, &p);
    let frames = decode_all(&p);
    assert_eq!(frames.len(), 48);
    for (i, f) in frames.iter().enumerate() {
        assert_eq!(f.pts, Rational::new(i as i64, 24), "pts i/24 monotonic");
        assert_eq!(f.duration, Rational::new(1, 24), "1/24 duration");
    }
    assert_eq!(frames[0].pts, Rational::new(0, 1), "output starts at 0");
}

// ---------------------------------------------------------------------------
// E-4: codec parameters round-trip (profile/level + colour tags)
// ---------------------------------------------------------------------------

#[test]
fn e4_codec_params_round_trip() {
    let p = out_path("e4.mp4");
    encode_n_to(24, &p);
    let info = libav_probe(&p);
    let v = &info.streams[0];
    assert_eq!(v.codec, "mpeg4");
    let vd = v.video.as_ref().expect("video details");
    assert_eq!(vd.width, W);
    assert_eq!(vd.height, H);
    assert_eq!(vd.pixel_format, ove_media::PixelFormat::Yuv420p);
    // colour tags round-trip (E-4): what we configured is what comes back
    assert_eq!(vd.color.primaries, Primaries::Bt709);
    assert_eq!(vd.color.transfer, Transfer::Bt709);
    assert_eq!(vd.color.matrix, MatrixCoeffs::Bt709);
    assert_eq!(vd.color.range, Range::Limited);
}

// ---------------------------------------------------------------------------
// E-5: golden bytes — version-keyed + same-run determinism
// ---------------------------------------------------------------------------

#[test]
fn e5_determinism_and_golden_bytes() {
    let p1 = out_path("e5_run1.mp4");
    let p2 = out_path("e5_run2.mp4");
    encode_n_to(48, &p1);
    encode_n_to(48, &p2);
    let h1 = sha256_file(&p1);
    let h2 = sha256_file(&p2);
    assert_eq!(h1, h2, "same libav build: encode+mux must be deterministic");

    // golden: keyed on the producing libav identity (ADR-015)
    let golden_path = media("golden/encode_golden.json");
    let versions = libav_versions();
    if std::env::var("OVE_REGEN_ENCODE_GOLDEN").is_ok() {
        let manifest = serde_json::json!({
            "libav_versions": versions,
            "sha256": h1,
            "frames": 48,
            "config": "320x240 yuv420p 24fps gop24 qscale6 bitexact mp4 faststart+bitexact",
        });
        std::fs::write(
            &golden_path,
            serde_json::to_string_pretty(&manifest).unwrap(),
        )
        .expect("write golden manifest");
        return;
    }
    let manifest: Value = serde_json::from_str(
        &std::fs::read_to_string(&golden_path).expect("encode golden manifest committed"),
    )
    .unwrap();
    let recorded = manifest["libav_versions"].as_str().expect("versions");
    if recorded == versions {
        let g = manifest["sha256"].as_str().expect("golden sha256");
        assert_eq!(h1, g, "byte golden for pinned libav {versions}");
    } else {
        // explicit, reported skip — never silent
        eprintln!(
            "E-5 byte golden keyed to {recorded}; running {versions} — \
             determinism asserted, byte gate applies on the pinned build"
        );
    }
}

// ---------------------------------------------------------------------------
// E-7: stream-copy route — video copy exactness + content correctness
// ---------------------------------------------------------------------------

#[test]
fn e7_copy_video_exact_frames_and_content() {
    let src = media_asset("copy24.mp4");

    // real keyframe index from the real probe backend
    use ove_decode::ffmpeg::FfmpegProbe;
    let idx = FfmpegProbe.keyframe_index(&src).expect("keyframe index");
    let kfs: Vec<Rational> = idx.entries.iter().map(|e| e.pts).collect();

    // plan [1 s, 3 s) — exact keyframe boundaries on this corpus
    let input = TrackInput {
        stream_id: StreamId(0),
        kind: TrackKindTag::Video,
        keyframes: kfs.clone(),
        media_end: Rational::new(6, 1),
        audio_grid: None,
        reencode_available: false,
    };
    let plan = plan_export(
        &[input],
        TimeRange::new(Rational::new(1, 1), Rational::new(3, 1)).unwrap(),
        CopyPolicy::KeyframeAlignedOnly,
    )
    .expect("plan copy");
    assert!(plan.snaps().is_empty(), "aligned corpus cut: no snaps");
    let span = plan.copy_span_of(StreamId(0)).expect("copy span");
    assert_eq!(span.start, Rational::new(1, 1));
    assert_eq!(span.end, Rational::new(3, 1));

    // execute: demux source packets in span, shift to 0, mux mp4
    let mut source = FfmpegCopySource::open(&src).expect("open copy source");
    let parsed = source.stream_by_source_index(0).expect("video stream");
    let track = parsed.track_spec(StreamId(0)).expect("track spec");
    let packets = source
        .read_packets(&span, &[0], &[(0, StreamId(0))])
        .expect("read copy packets");
    assert_eq!(packets.len(), 48, "2 s @ 24 fps = 48 packets");
    assert!(packets[0].keyframe, "copy opens on a keyframe (E-007)");
    assert_eq!(packets[0].pts, Rational::new(0, 1), "shifted start");

    let p = out_path("e7_copy.mp4");
    let mut mux = FfmpegMuxer::open(
        OutputSink::File(p.clone()),
        Container::Mp4 {
            faststart: true,
            bitexact: true,
        },
        vec![track],
    )
    .expect("open muxer");
    for pkt in packets.clone() {
        mux.write(pkt).expect("mux copied packet");
    }
    let info = mux.finalize().expect("finalize");
    assert_eq!(info.tracks[0].nb_frames, Some(48));
    assert_eq!(
        info.tracks[0].duration,
        Some(Rational::new(2, 1)),
        "copied span duration exact"
    );

    // E-7 content correctness, byte level: output packets == source GOP
    // range packets (byte-identical payloads, pts shifted by exactly 1 s)
    let mut out_reader = FfmpegCopySource::open(&asset_of(&p)).expect("reopen output");
    let out_pkts = out_reader
        .read_packets(
            &TimeRange::new(Rational::new(0, 1), Rational::new(2, 1)).unwrap(),
            &[0],
            &[(0, StreamId(0))],
        )
        .expect("read output packets");
    assert_eq!(out_pkts.len(), packets.len());
    for (o, s) in out_pkts.iter().zip(packets.iter()) {
        assert_eq!(o.data, s.data, "byte-identical copy (E-007 §3.4)");
        assert_eq!(o.pts, s.pts, "pts preserved through the copy round-trip");
        assert_eq!(o.keyframe, s.keyframe);
    }

    // E-7 content correctness, decode level: pixels identical to source
    let copied = decode_all(&p);
    let mut sdec =
        FfmpegSwDecoder::open(&src, StreamId(0), DecodeConfig::default()).expect("decode source");
    let mut source_frames = Vec::new();
    while let Some(f) = sdec.next().expect("clean source decode") {
        let keep = f.pts >= Rational::new(1, 1) && f.pts < Rational::new(3, 1);
        let stop = f.pts >= Rational::new(3, 1);
        if keep {
            source_frames.push(f);
        }
        if stop {
            break;
        }
    }
    assert_eq!(copied.len(), source_frames.len());
    for (c, s) in copied.iter().zip(source_frames.iter()) {
        let cb = c.cpu_bytes().expect("cpu payload");
        let sb = s.cpu_bytes().expect("cpu payload");
        assert_eq!(cb.data, sb.data, "decoded pixels bit-identical");
        assert_eq!(cb.strides, sb.strides);
    }
}

// ---------------------------------------------------------------------------
// E-7b: A/V export — video re-encode + audio stream-copy, per-track
// exactness (audio ± 0 samples)
// ---------------------------------------------------------------------------

#[test]
fn e7b_av_mux_per_track_exactness() {
    // 1) video: 24 synthetic frames (1 s)
    let cfg = EncoderConfig {
        codec: VideoCodec::Mpeg4,
        profile: test_profile(),
        rate: RateControl::Crf { quality: 6 },
    };
    let mut enc = FfmpegSwEncoder::configure(cfg).expect("encoder");
    let vtrack = enc.track_spec().expect("v track");
    for k in 0..24 {
        enc.feed(yuv420p_frame(k)).expect("feed");
    }
    let vpkts = enc.drain().expect("drain video");

    // 2) audio: copy 48 AAC packets = 48*1024 samples = 49152 samples
    let src = media_asset("copyav.mp4");
    let mut source = FfmpegCopySource::open(&src).expect("copy source");
    let audio = source
        .streams()
        .iter()
        .find(|s| !s.is_video)
        .expect("audio stream")
        .clone();
    assert_eq!(audio.codec_name, "aac");
    let atrack = audio.track_spec(StreamId(1)).expect("a track");
    assert_eq!(atrack.timescale, 48000, "audio timescale = sample rate");
    // audio-aligned span [0, 49152/48000) — exact on the AAC grid
    let a_span = TimeRange::new(Rational::new(0, 1), Rational::new(49152, 48000)).unwrap();
    let apkts = source
        .read_packets(&a_span, &[1], &[(1, StreamId(1))])
        .expect("audio copy packets");
    assert_eq!(apkts.len(), 48, "48 whole AAC packets");
    let audio_duration = apkts
        .iter()
        .map(|p| p.duration.expect("aac duration"))
        .fold(Rational::new(0, 48000), |a, d| a.add(d));
    assert_eq!(
        audio_duration,
        Rational::new(49152, 48000),
        "audio span ± 0 samples"
    );

    // 3) mux both tracks
    let p = out_path("e7b_av.mp4");
    let mut mux = FfmpegMuxer::open(
        OutputSink::File(p.clone()),
        Container::Mp4 {
            faststart: true,
            bitexact: true,
        },
        vec![vtrack, atrack],
    )
    .expect("open av muxer");
    for pkt in vpkts.into_iter().chain(apkts) {
        mux.write(pkt).expect("mux av packet");
    }
    let info = mux.finalize().expect("finalize");
    assert_eq!(info.tracks.len(), 2);
    let v = info
        .tracks
        .iter()
        .find(|t| t.stream_id == StreamId(0))
        .unwrap();
    let a = info
        .tracks
        .iter()
        .find(|t| t.stream_id == StreamId(1))
        .unwrap();
    assert_eq!(v.duration, Some(Rational::new(1, 1)), "video ± 0 frames");
    assert_eq!(
        a.duration,
        Some(Rational::new(49152, 48000)),
        "audio ± 0 samples"
    );

    // 4) ffprobe (when present) sees both streams with exact durations
    if ffprobe_available() {
        let j = ffprobe_json(&p).expect("ffprobe parses av output");
        assert_eq!(j["streams"].as_array().unwrap().len(), 2);
        assert_eq!(j["streams"][0]["codec_name"], "mpeg4");
        assert_eq!(j["streams"][1]["codec_name"], "aac");
    }
}

// ---------------------------------------------------------------------------
// Receipt honesty: misaligned audio in a copy request → declared deferral
// ---------------------------------------------------------------------------

#[test]
fn e7c_audio_misalignment_is_declared_not_silent() {
    let input = TrackInput {
        stream_id: StreamId(1),
        kind: TrackKindTag::Audio,
        keyframes: vec![],
        media_end: Rational::new(6, 1),
        audio_grid: Some(ove_encode::planner::AudioGrid {
            frame_samples: 1024,
            sample_rate: 48000,
        }),
        reencode_available: false,
    };
    let plan = plan_export(
        &[input],
        TimeRange::new(Rational::new(1, 1), Rational::new(3, 1)).unwrap(),
        CopyPolicy::KeyframeAlignedOnly,
    )
    .expect("plan");
    let t = &plan.tracks[0];
    match &t.route {
        ove_encode::planner::TrackRoute::Deferred { reason } => {
            assert!(reason.contains("W7"));
        }
        other => panic!("expected declared deferral, got {other:?}"),
    }
}

// ---------------------------------------------------------------------------
// Capability honesty + exactness honesty (typed, never silent)
// ---------------------------------------------------------------------------

#[test]
fn h1_unsupported_codecs_are_typed() {
    for codec in [VideoCodec::OpenH264, VideoCodec::SvtAv1] {
        let err = match FfmpegSwEncoder::configure(EncoderConfig {
            codec,
            profile: test_profile(),
            rate: RateControl::Crf { quality: 6 },
        }) {
            Err(e) => e,
            Ok(_) => panic!("{codec:?} must NOT configure in v1"),
        };
        assert!(
            matches!(err, EncodeError::Unsupported(ref d) if !d.is_empty()),
            "{codec:?} must be a NAMED typed unsupported"
        );
    }
}

#[test]
fn h2_odd_420_dimensions_rejected() {
    let err = match FfmpegSwEncoder::configure(EncoderConfig {
        codec: VideoCodec::Mpeg4,
        profile: VideoProfile {
            width: 321,
            ..test_profile()
        },
        rate: RateControl::Crf { quality: 6 },
    }) {
        Err(e) => e,
        Ok(_) => panic!("odd 4:2:0 dimensions must be rejected"),
    };
    assert!(matches!(err, EncodeError::InvalidConfig(_)));
}

#[test]
fn h3_non_exact_pts_is_typed() {
    let mut enc = FfmpegSwEncoder::configure(EncoderConfig {
        codec: VideoCodec::Mpeg4,
        profile: test_profile(),
        rate: RateControl::Crf { quality: 6 },
    })
    .expect("encoder");
    let mut f = yuv420p_frame(0);
    f.pts = Rational::new(1, 48); // half-frame — not on the 24 fps axis
    let err = enc.feed(f).unwrap_err();
    match err {
        EncodeError::NonExactTimestamp { pts, time_base } => {
            assert_eq!(pts, Rational::new(1, 48));
            assert_eq!(time_base, Rational::new(1, 24));
        }
        other => panic!("expected NonExactTimestamp, got {other:?}"),
    }
}

#[test]
fn h4_muxer_rejects_unknown_track() {
    let mut mux = FfmpegMuxer::open(
        OutputSink::File(out_path("h4.mp4")),
        Container::Mp4 {
            faststart: false,
            bitexact: true,
        },
        vec![TrackSpec {
            stream_id: StreamId(7),
            codec: ove_encode::TrackCodec::Mpeg4,
            timescale: 24,
            kind: video_track_kind(),
            extradata: vec![],
        }],
    )
    .expect("open muxer");
    let pkt = ove_encode::EncodedPacket {
        stream_id: StreamId(3), // not a configured track
        pts: Rational::new(0, 1),
        dts: None,
        duration: Some(Rational::new(1, 24)),
        keyframe: true,
        byte_range_hint: None,
        data: vec![0, 0, 0, 1],
    };
    let err = mux.write(pkt).unwrap_err();
    assert_eq!(err, ove_encode::MuxError::UnknownTrack(StreamId(3)));
}

fn video_track_kind() -> ove_encode::TrackKind {
    ove_encode::TrackKind::Video {
        width: W,
        height: H,
        pixel_format: ove_media::PixelFormat::Yuv420p,
        color: ColorTags {
            primaries: Primaries::Unknown,
            transfer: Transfer::Unknown,
            matrix: MatrixCoeffs::Unknown,
            range: Range::Unknown,
            chroma_loc: None,
        },
        frame_rate: Rational::new(24, 1),
    }
}

#[test]
fn h5_muxer_rejects_non_exact_pts() {
    let mut mux = FfmpegMuxer::open(
        OutputSink::File(out_path("h5.mp4")),
        Container::Mp4 {
            faststart: false,
            bitexact: true,
        },
        vec![TrackSpec {
            stream_id: StreamId(0),
            codec: ove_encode::TrackCodec::Mpeg4,
            timescale: 24,
            kind: video_track_kind(),
            extradata: vec![],
        }],
    )
    .expect("open muxer");
    let pkt = ove_encode::EncodedPacket {
        stream_id: StreamId(0),
        // 1/7 s is exact on neither 24 nor movenc-widened 12288 axes
        pts: Rational::new(1, 7),
        dts: None,
        duration: Some(Rational::new(1, 24)),
        keyframe: true,
        byte_range_hint: None,
        data: vec![0, 0, 0, 1],
    };
    let err = mux.write(pkt).unwrap_err();
    assert!(matches!(
        err,
        ove_encode::MuxError::NonExactTimestamp { .. }
    ));
}

// frame.duration is a plain Rational on the envelope; the ntsc test
// rewrites pts/duration on the 1001/30000 grid above.
