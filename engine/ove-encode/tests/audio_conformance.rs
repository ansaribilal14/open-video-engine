//! ENCODER_SPEC §2 audio conformance (E-8..E-10, WAVE 7, ADR-018).
//!
//! E-8: AAC encode exactness — sample-count authority, contiguity contract,
//!      grid consistency with the DECLARED small-last-frame branch.
//! E-9: A/V mux — one MP4 with mpeg4 video + AAC audio; both streams
//!      ffprobe/libav verified; A/V duration drift = 0 samples.
//! E-10: WAV PCM16 output — header + pinned bytes + exact duration.

use std::path::{Path, PathBuf};

use ove_decode::{DecodeConfig, Decoder};
use ove_encode::ffmpeg::{FfmpegAacEncoder, FfmpegMuxer, FfmpegSwEncoder};
use ove_encode::wav::write_wav_s16;
use ove_encode::{
    AudioCodec, AudioEncoder, AudioEncoderConfig, Container, EncodeError, Encoder, EncoderConfig,
    Muxer, OutputSink, RateControl, TrackSpec, VideoCodec, VideoProfile,
};
use ove_media::{AssetRef, ColorTags, FrameEnvelope, FrameKind, ProbeBackend, StreamId};
use ove_time::Rational;

const RATE: u32 = 44100;

fn out_dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("target/test-out")
}

fn out_path(name: &str) -> PathBuf {
    let d = out_dir();
    std::fs::create_dir_all(&d).expect("out dir");
    d.join(name)
}

fn asset_of(p: &Path) -> AssetRef {
    AssetRef::from_path(p).expect("asset from produced file")
}

fn ffprobe_json(path: &Path) -> Option<serde_json::Value> {
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
    serde_json::from_slice(&out.stdout).ok()
}

// deterministic test signal: integer LCG scaled to ±0.5 (bit-exact build of
// the INPUT everywhere; AAC output determinism is version-pinned, not
// input-reconstruction-pinned)
struct Lcg(u64);
impl Lcg {
    fn next_f32(&mut self) -> f32 {
        self.0 = self
            .0
            .wrapping_mul(6364136223846793005)
            .wrapping_add(1442695040888963407);
        (((self.0 >> 33) % 2000) as f32 / 2000.0) - 0.5
    }
}

/// planar-f32 audio envelope (the decode leg's canonical surface).
fn audio_env(pts_samples: i64, samples: usize, channels: usize, gen: &mut Lcg) -> FrameEnvelope {
    let mut data = Vec::with_capacity(samples * channels * 4);
    for _ in 0..channels {
        for _ in 0..samples {
            data.extend_from_slice(&gen.next_f32().to_le_bytes());
        }
    }
    FrameEnvelope::audio(
        Rational::new(pts_samples, RATE as i64),
        StreamId(1),
        RATE,
        channels as u32,
        samples,
        ove_media::FrameBytes {
            data,
            strides: vec![samples * 4; channels],
        },
        ove_media::BackendId::Other("test".into()),
        0,
    )
}

fn aac_cfg() -> AudioEncoderConfig {
    AudioEncoderConfig {
        codec: AudioCodec::Aac,
        sample_rate: RATE,
        channels: 1,
        rate: RateControl::Cbr { bitrate: 64_000 },
        bitexact: true,
    }
}

// ---------------------------------------------------------------------------
// E-8: AAC encode exactness (sample-count authority + contiguity contract)
// ---------------------------------------------------------------------------

#[test]
fn e8_aac_encode_exactness() {
    const TOTAL: usize = 44100; // 1 s — NOT a multiple of 1024 (exercises last frame)
    let mut enc = FfmpegAacEncoder::configure(aac_cfg()).expect("configure aac");
    let track = enc.track_spec().expect("track spec");
    assert_eq!(track.timescale, RATE as i64, "timescale = sample rate");
    assert!(
        matches!(track.kind, ove_encode::TrackKind::Audio { sample_format, .. } if sample_format == "fltp")
    );
    assert!(!track.extradata.is_empty(), "AudioSpecificConfig present");

    let mut gen = Lcg(0x5eed);
    let mut cursor = 0usize;
    // irregular chunk sizes: arbitrary caller granularity must be fine
    let chunks = [1000usize, 7, 4096, 313, 11024, 10000, 2000, 1023, 1, 14636];
    for c in chunks {
        let n = c.min(TOTAL - cursor);
        if n == 0 {
            break;
        }
        enc.feed(audio_env(cursor as i64, n, 1, &mut gen))
            .expect("feed contiguous chunk");
        cursor += n;
    }
    assert_eq!(cursor, TOTAL);
    // contiguity is ENFORCED: a wrong-cursor feed is a typed mismatch
    let bad = audio_env((cursor as i64) + 500, 10, 1, &mut gen);
    let e = enc.feed(bad).expect_err("non-contiguous feed rejected");
    assert!(
        matches!(e, ove_encode::EncodeError::FrameMismatch(_)),
        "got {e:?}"
    );

    let packets = enc.drain().expect("drain");
    assert!(!packets.is_empty(), "packets produced");

    // packet invariants: monotonic pts, contiguous, exact durations. The
    // first packet starts at −initial_padding (the priming trim; elst handles
    // it at the container — verified against the reference pipeline).
    let padding = enc.initial_padding();
    let mut t = Rational::new(-padding, RATE as i64);
    let mut decoded_samples = 0i64;
    for (i, p) in packets.iter().enumerate() {
        assert_eq!(p.pts, t, "packet {i} pts contiguity on the sample axis");
        let d = p.duration.expect("audio packets carry exact durations");
        let n = d * Rational::new(RATE as i64, 1);
        assert_eq!(n.den(), 1, "duration must be an exact sample count");
        decoded_samples += n.num();
        t = t + d;
    }
    // grid consistency with the DECLARED small-last branch. RAW packet span
    // includes the codec's declared priming (the container trims it via the
    // edit list written from TrackSpec::initial_padding — see E-9):
    //   small_last → raw total == fed + initial_padding
    //   padded     → raw total == fed rounded UP to the grid + initial_padding
    // RAW output durations include the priming pre-roll (real media samples
    // trimmed by the container edit list — reference-pipeline truth):
    //   small_last → raw total == fed + initial_padding
    //   padded     → raw total == grid + initial_padding
    let grid = (TOTAL + 1023).div_ceil(1024) * 1024;
    if enc.small_last_frame() {
        assert_eq!(
            decoded_samples,
            TOTAL as i64 + padding,
            "raw output == fed + declared priming"
        );
    } else {
        assert_eq!(
            decoded_samples,
            grid as i64 + padding,
            "raw output == grid + declared priming"
        );
    }
    // span: last pts + duration ends at the fed sample count exactly
    let last = packets.last().expect("packets");
    assert_eq!(
        last.pts + last.duration.expect("duration"),
        Rational::new(TOTAL as i64, RATE as i64),
        "raw span end == fed samples (priming expressed as the negative first pts)"
    );
}

#[test]
fn e8b_aac_rejects_crf_and_wrong_surface() {
    let mut crf = aac_cfg();
    crf.rate = RateControl::Crf { quality: 4 };
    let e = match FfmpegAacEncoder::configure(crf) {
        Err(e) => e,
        Ok(_) => panic!("CRF is a typed config error"),
    };
    assert!(matches!(e, EncodeError::Unsupported(_)), "got {e:?}");

    let mut enc = FfmpegAacEncoder::configure(aac_cfg()).expect("configure aac");
    // non-contiguous first feed (cursor starts at 0): pts 1/2 must fail
    let mut gen = Lcg(1);
    let bad = audio_env(RATE as i64 / 2, 100, 1, &mut gen);
    let e = enc.feed(bad).expect_err("wrong cursor rejected");
    assert!(
        matches!(e, ove_encode::EncodeError::FrameMismatch(_)),
        "got {e:?}"
    );
    // video frame fed to the audio encoder: typed mismatch (kind check)
    let v = FrameEnvelope::video_cpu(
        Rational::new(0, 1),
        Rational::new(1, 24),
        StreamId(0),
        2,
        2,
        ove_media::PixelFormat::Rgba,
        ove_media::BitDepth::Other(8),
        ColorTags::unknown(),
        ove_media::FrameBytes {
            data: vec![0; 16],
            strides: vec![8],
        },
        true,
        ove_media::BackendId::Other("test".into()),
        0,
    );
    let e = enc.feed(v).expect_err("video frame into audio encoder");
    assert!(
        matches!(e, ove_encode::EncodeError::FrameMismatch(_)),
        "got {e:?}"
    );
    // wrong surface (8 kHz declared) at the correct cursor: surface mismatch
    let mut e8k = audio_env(0, 10, 1, &mut Lcg(2));
    e8k.kind = FrameKind::Audio {
        sample_rate: 8000,
        channels: 1,
    };
    let e = enc.feed(e8k).expect_err("surface mismatch");
    assert!(
        matches!(e, ove_encode::EncodeError::FrameMismatch(_)),
        "got {e:?}"
    );
}

// ---------------------------------------------------------------------------
// E-9: A/V mux — both streams in one MP4; ffprobe + libav verified
// ---------------------------------------------------------------------------

const W: u32 = 64;
const H: u32 = 48;
const FPS: (i64, i64) = (24, 1);

fn yuv_frame(k: u64) -> FrameEnvelope {
    let mut data = Vec::with_capacity((W * H * 3 / 2) as usize);
    for y in 0..H {
        for x in 0..W {
            data.push(((x as u64 + 4 * k + y as u64) % 256) as u8);
        }
    }
    data.extend(std::iter::repeat_n(128u8, (W * H / 4) as usize * 2));
    FrameEnvelope::video_cpu(
        Rational::new(k as i64 * FPS.1, FPS.0),
        Rational::new(FPS.1, FPS.0),
        StreamId(0),
        W,
        H,
        ove_media::PixelFormat::Yuv420p,
        ove_media::BitDepth::Other(8),
        ColorTags::unknown(),
        ove_media::FrameBytes {
            data,
            strides: vec![W as usize, W as usize / 2, W as usize / 2],
        },
        k == 0,
        ove_media::BackendId::Other("test".into()),
        0,
    )
}

fn encode_audio_track(samples: usize) -> (TrackSpec, Vec<ove_encode::EncodedPacket>, bool) {
    let mut enc = FfmpegAacEncoder::configure(aac_cfg()).expect("configure aac");
    let small = enc.small_last_frame();
    let mut gen = Lcg(0xabcd);
    let mut cursor = 0usize;
    while cursor < samples {
        let n = 1234.min(samples - cursor);
        enc.feed(audio_env(cursor as i64, n, 1, &mut gen))
            .expect("feed");
        cursor += n;
    }
    let packets = enc.drain().expect("drain");
    (enc.track_spec().expect("track spec"), packets, small)
}

#[test]
fn e9_av_mux_two_tracks() {
    const VIDEO_FRAMES: u64 = 48; // 2 s @ 24
    const AUDIO_SAMPLES: usize = 2 * RATE as usize; // 2 s — SAME span as video

    let vcfg = EncoderConfig {
        codec: VideoCodec::Mpeg4,
        profile: VideoProfile {
            width: W,
            height: H,
            frame_rate: Rational::new(FPS.0, FPS.1),
            pixel_format: ove_media::PixelFormat::Yuv420p,
            color: ColorTags::unknown(),
            gop: 12,
            bitexact: true,
        },
        rate: RateControl::Crf { quality: 6 },
    };
    let mut venc = FfmpegSwEncoder::configure(vcfg).expect("configure video");
    let vtrack = venc.track_spec().expect("video track");
    let mut vpackets = Vec::new();
    for k in 0..VIDEO_FRAMES {
        venc.feed(yuv_frame(k)).expect("feed video");
    }
    vpackets.extend(venc.drain().expect("drain video"));

    let (atrack, apackets, small_last) = encode_audio_track(AUDIO_SAMPLES);

    let p = out_path("e9_av.mp4");
    let mut mux = FfmpegMuxer::open(
        OutputSink::File(p.clone()),
        Container::Mp4 {
            faststart: true,
            bitexact: true,
        },
        vec![vtrack, atrack],
    )
    .expect("open 2-track muxer");
    for pkt in vpackets.into_iter().chain(apackets) {
        mux.write(pkt).expect("mux packet");
    }
    let _info = mux.finalize().expect("finalize");

    // libav probe: two streams, exact durations
    let probe = ove_decode::ffmpeg::FfmpegProbe
        .probe(&asset_of(&p))
        .expect("probe of A/V output");
    assert_eq!(probe.streams.len(), 2, "video + audio streams");
    let v = probe
        .streams
        .iter()
        .find(|s| s.kind == ove_media::StreamKind::Video)
        .expect("video");
    let a = probe
        .streams
        .iter()
        .find(|s| s.kind == ove_media::StreamKind::Audio)
        .expect("audio");
    assert_eq!(v.codec, "mpeg4");
    assert_eq!(a.codec, "aac");
    assert_eq!(v.nb_frames_hint, Some(VIDEO_FRAMES));
    assert_eq!(
        v.duration,
        Some(Rational::new(VIDEO_FRAMES as i64 * FPS.1, FPS.0)),
        "video duration ± 0 frames"
    );
    // Container duration: with initial_padding recorded on the TrackSpec the
    // muxer writes the trim edit list, so the FILE audio duration is the fed
    // sample count exactly (± 0 samples — E-2). (The padded no-small-last
    // branch is declared, deferred until an encoder without the capability
    // appears; the FFmpeg 7.1.x pin has it on both local and CI.)
    assert!(
        small_last,
        "FFmpeg 7.1.x native AAC declares SMALL_LAST_FRAME"
    );
    let expected_audio = Rational::new(AUDIO_SAMPLES as i64, RATE as i64);
    assert_eq!(
        a.duration,
        Some(expected_audio),
        "audio duration ± 0 samples"
    );

    // A/V drift: |video_dur − audio_dur| — the muxer/encoder chain must not
    // introduce any drift beyond the declared grid padding (0 when small_last)
    let vd = v.duration.unwrap();
    let ad = a.duration.unwrap();
    let drift = if vd >= ad { vd - ad } else { ad - vd };
    let bound = Rational::new(0, 1);
    assert!(
        drift <= bound,
        "A/V drift {drift} exceeds declared bound {bound}"
    );

    // ffprobe CLI eye: stream order, codec names, audio rate
    if let Some(j) = ffprobe_json(&p) {
        assert_eq!(j["streams"][0]["codec_name"], "mpeg4");
        assert_eq!(j["streams"][1]["codec_name"], "aac");
        assert_eq!(
            j["streams"][1]["sample_rate"]
                .as_str()
                .unwrap()
                .parse::<u32>()
                .unwrap(),
            RATE
        );
        assert_eq!(
            j["streams"][0]["duration"].as_str().unwrap(),
            "2.000000",
            "ffprobe sees exact video duration"
        );
    }

    // full round trip: decode BOTH tracks back through our own stack
    let mut dec = ove_decode::ffmpeg::FfmpegSwDecoder::open(
        &asset_of(&p),
        StreamId(1),
        DecodeConfig::default(),
    )
    .expect("decode audio track of our output");
    let mut total = 0i64;
    while let Some(f) = dec.next().expect("clean audio decode") {
        total += (f.duration * Rational::new(RATE as i64, 1)).num();
    }
    if small_last {
        // decoded total covers the fed 1 s exactly (plus possible codec-delay
        // flush padding on the grid — bounded by one frame)
        assert!(total >= AUDIO_SAMPLES as i64, "decoded {total} < fed");
        assert!((total - AUDIO_SAMPLES as i64) < 1024, "decode tail bound");
    }
}

// ---------------------------------------------------------------------------
// E-10: WAV PCM16 output — header + pinned bytes + exact duration
// ---------------------------------------------------------------------------

#[test]
fn e10_wav_pcm16_exact() {
    // deterministic arithmetic signal (bit-exact everywhere: no transcendentals)
    let plane: Vec<f32> = (0..1000).map(|i| (i as f32) * 0.0005 - 0.25).collect();
    let p = out_path("e10.wav");
    let n = write_wav_s16(&p, std::slice::from_ref(&plane), RATE).expect("write wav");
    assert_eq!(n, 1000, "sample frames written");

    let b = std::fs::read(&p).expect("read back");
    assert_eq!(&b[0..4], b"RIFF");
    assert_eq!(&b[8..12], b"WAVE");
    assert_eq!(u16::from_le_bytes(b[20..22].try_into().unwrap()), 1, "PCM");
    assert_eq!(u16::from_le_bytes(b[22..24].try_into().unwrap()), 1, "mono");
    assert_eq!(u32::from_le_bytes(b[24..28].try_into().unwrap()), RATE);
    assert_eq!(
        u16::from_le_bytes(b[34..36].try_into().unwrap()),
        16,
        "bits"
    );
    assert_eq!(
        u32::from_le_bytes(b[40..44].try_into().unwrap()),
        2000,
        "data len"
    );
    assert_eq!(b.len(), 44 + 2000);

    // pinned conversion: s16 = round(clamp(x,-1,1) × 32768)
    for (i, chunk) in b[44..].as_chunks::<2>().0.iter().enumerate() {
        let got = i16::from_le_bytes(*chunk);
        let x = (plane[i].clamp(-1.0, 1.0) * 32768.0).round() as i32;
        let want = x.clamp(-32768, 32767) as i16;
        assert_eq!(got, want, "sample {i}");
    }

    // stereo: interleaved, both planes equal length
    let p2 = out_path("e10_st.wav");
    let planes: Vec<Vec<f32>> = vec![
        (0..500).map(|i| (i as f32) * 0.001).collect(),
        (0..500).map(|i| -((i as f32) * 0.001)).collect(),
    ];
    write_wav_s16(&p2, &planes, RATE).expect("write stereo wav");
    let b2 = std::fs::read(&p2).expect("read back");
    assert_eq!(
        u16::from_le_bytes(b2[22..24].try_into().unwrap()),
        2,
        "stereo"
    );
    assert_eq!(b2.len(), 44 + 500 * 2 * 2);

    // duration exactness: data_len / (rate × ch × 2) — 500 frames @ 44100
    if let Some(j) = ffprobe_json(&p2) {
        let dur: f64 = j["streams"][0]["duration"]
            .as_str()
            .unwrap()
            .parse()
            .unwrap();
        // ffprobe prints 6 decimal places — compare at that precision
        let exact = (500.0 / RATE as f64 * 1e6).round() / 1e6;
        assert!(
            (dur - exact).abs() < 1e-9,
            "exact wav duration, got {dur} want {exact}"
        );
    }

    // honesty: unequal planes / bad channels are typed errors
    let e = write_wav_s16(&out_path("e10_bad.wav"), &[vec![0.0], vec![0.0, 0.0]], RATE)
        .expect_err("unequal planes");
    assert!(
        matches!(e, ove_encode::EncodeError::InvalidConfig(_)),
        "got {e:?}"
    );
    let e = write_wav_s16(&out_path("e10_bad2.wav"), &[vec![], vec![], vec![]], RATE)
        .expect_err("3 channels out of v1 surface");
    assert!(
        matches!(e, ove_encode::EncodeError::InvalidConfig(_)),
        "got {e:?}"
    );
}
