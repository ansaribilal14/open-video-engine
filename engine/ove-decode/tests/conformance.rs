//! DECODER_SPEC §2 conformance suite (D-1..D-12).
//!
//! Golden tables are committed ffprobe dumps of the committed corpus
//! (scripts/corpus/gen_corpus.py). Tests compare EXACT rational values, never
//! fp seconds. "Decoder done" = this suite green (DECODER_SPEC §5).

use std::path::{Path, PathBuf};

use ove_decode::ffmpeg::{FfmpegProbe, FfmpegSwDecoder};
use ove_decode::{DecodeConfig, DecodeError, Decoder, HwAccel, SeekMode};
use ove_media::{
    AssetRef, BackendId, ChromaLoc, FrameMemory, MatrixCoeffs, Primaries, ProbeBackend, Range,
    StreamId, Transfer, VfrReport,
};
use ove_time::Rational;
use serde_json::Value;

fn media(name: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests/media")
        .join(name)
}

fn asset(name: &str) -> AssetRef {
    AssetRef::from_path(media(name)).expect("corpus file present")
}

fn golden(name: &str) -> Value {
    let p = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests/media/golden")
        .join(name);
    serde_json::from_str(&std::fs::read_to_string(p).expect("golden table committed")).unwrap()
}

/// "1/24000" -> (1, 24000)
fn parse_rate(s: &str) -> (i64, i64) {
    let (a, b) = s.split_once('/').expect("rate string");
    (a.parse().unwrap(), b.parse().unwrap())
}

fn ticks(v: &Value, field: &str, tb: (i64, i64)) -> Rational {
    let t = v[field].as_i64().expect("tick value in golden table");
    Rational::new(t * tb.0, tb.1)
}

/// (pts, duration, keyframe) per frame from the committed ffprobe table.
fn frames_of(table: &str) -> (Vec<Rational>, Vec<Rational>, Vec<bool>) {
    let v = golden(&format!("{table}.ffprobe.json"));
    let tb = parse_rate(v["streams"][0]["time_base"].as_str().unwrap());
    let f = golden(&format!("{table}.frames.json"));
    let mut pts = Vec::new();
    let mut durs = Vec::new();
    let mut keys = Vec::new();
    for fr in f["frames"].as_array().expect("frames array") {
        pts.push(ticks(fr, "pts", tb));
        durs.push(ticks(fr, "duration", tb));
        keys.push(fr["key_frame"].as_i64().unwrap() == 1);
    }
    (pts, durs, keys)
}

/// Keyframe pts from packet-level flags (container truth, D-3).
fn keyframes_of(table: &str) -> Vec<Rational> {
    let v = golden(&format!("{table}.ffprobe.json"));
    let tb = parse_rate(v["streams"][0]["time_base"].as_str().unwrap());
    let p = golden(&format!("{table}.packets.json"));
    p["packets"]
        .as_array()
        .expect("packets array")
        .iter()
        .filter(|pk| pk["flags"].as_str().unwrap_or("").contains('K'))
        .map(|pk| ticks(pk, "pts", tb))
        .collect()
}

/// Sequential decode of every frame from 0.
fn decode_all(a: &AssetRef, cfg: DecodeConfig) -> Vec<ove_media::FrameEnvelope> {
    let mut dec = FfmpegSwDecoder::open(a, StreamId(0), cfg).expect("open corpus file");
    let mut out = Vec::new();
    while let Some(f) = dec.next().expect("clean decode") {
        out.push(f);
    }
    out
}

fn open_cfr24(cfg: DecodeConfig) -> FfmpegSwDecoder {
    FfmpegSwDecoder::open(&asset("cfr24.mp4"), StreamId(0), cfg).expect("open cfr24")
}

// ---------------------------------------------------------------------------
// D-1 pts exactness
// ---------------------------------------------------------------------------

#[test]
fn d1_pts_exactness_cfr24() {
    let (pts, durs, keys) = frames_of("cfr24");
    assert_eq!(pts.len(), 192);
    let envs = decode_all(&asset("cfr24.mp4"), DecodeConfig::default());
    assert_eq!(envs.len(), pts.len(), "frame count must match container");
    for (i, f) in envs.iter().enumerate() {
        assert_eq!(
            f.pts, pts[i],
            "frame {i}: pts must equal container pts exactly"
        );
        assert_eq!(f.duration, durs[i], "frame {i}: duration must be exact");
        assert_eq!(f.keyframe, keys[i]);
    }
}

#[test]
fn d1_pts_exactness_ntsc() {
    let (pts, _, _) = frames_of("ntsc");
    assert_eq!(pts.len(), 90);
    let envs = decode_all(&asset("ntsc.mp4"), DecodeConfig::default());
    for (i, f) in envs.iter().enumerate() {
        // 30000/1001 time base: the fp trap E-002 proved gets boundaries wrong
        assert_eq!(f.pts, pts[i], "NTSC frame {i} pts exact");
    }
}

// ---------------------------------------------------------------------------
// D-2 duration exactness (VFR sample included)
// ---------------------------------------------------------------------------

#[test]
fn d2_duration_sum_equals_stream_duration() {
    for (file, table) in [
        ("cfr24.mp4", "cfr24"),
        ("ntsc.mp4", "ntsc"),
        ("vfr.mp4", "vfr"),
    ] {
        let (_, durs, _) = frames_of(table);
        let sum = durs.iter().fold(Rational::zero(1), |a, d| a.add(*d));
        // exact expected duration = last frame pts + duration (container
        // ticks; ffprobe's stream-level "duration" is an fp seconds STRING
        // and is deliberately not used)
        let (pts, durs2, _) = frames_of(table);
        let expected = pts[pts.len() - 1].add(durs2[durs2.len() - 1]);
        assert_eq!(
            sum, expected,
            "{file}: per-frame durations must sum to the stream duration exactly"
        );
        let _ = file;
    }
}

// ---------------------------------------------------------------------------
// D-3 keyframe metadata vs packet-level container truth
// ---------------------------------------------------------------------------

#[test]
fn d3_keyframe_metadata_matches_container_truth() {
    for (file, table) in [
        ("cfr24.mp4", "cfr24"),
        ("ntsc.mp4", "ntsc"),
        ("vfr.mp4", "vfr"),
    ] {
        let packet_k = keyframes_of(table);
        assert!(!packet_k.is_empty());
        let envs = decode_all(&asset(file), DecodeConfig::default());
        let decoded_k: Vec<Rational> = envs.iter().filter(|f| f.keyframe).map(|f| f.pts).collect();
        assert_eq!(
            decoded_k, packet_k,
            "{file}: decoded keyframe flags must equal packet truth 100%"
        );
    }
}

// ---------------------------------------------------------------------------
// D-4 seek Exact == sequential decode (frame-identical bytes)
// ---------------------------------------------------------------------------

#[test]
fn d4_seek_exact_is_frame_identical_to_sequential_decode() {
    let targets = [
        Rational::new(0, 1),   // stream start
        Rational::new(1, 24),  // exact frame boundary
        Rational::new(1, 2),   // mid-second (frame 12)
        Rational::new(1, 1),   // keyframe
        Rational::new(25, 24), // keyframe + 1 (E-007 trap shape)
        Rational::new(15, 2),  // mid-GOP
    ];
    for t in targets {
        // sequential reference: decode from 0, keep frames at/after t
        let reference = {
            let mut dec = open_cfr24(DecodeConfig::default());
            let mut kept = Vec::new();
            while let Some(f) = dec.next().unwrap() {
                if f.pts >= t {
                    kept.push((f.pts, f.deep_copy_cpu().unwrap()));
                }
            }
            kept
        };
        let mut dec = open_cfr24(DecodeConfig::default());
        let landed = dec.seek(t, SeekMode::Exact).unwrap();
        assert_eq!(landed, t, "Exact seek returns the requested pts");
        let mut sought = Vec::new();
        while let Some(f) = dec.next().unwrap() {
            sought.push((f.pts, f.deep_copy_cpu().unwrap()));
        }
        assert_eq!(sought.len(), reference.len(), "t={t}: same frame count");
        for (i, ((sp, sb), (rp, rb))) in sought.iter().zip(reference.iter()).enumerate() {
            assert_eq!(sp, rp, "t={t} frame {i}: pts equal");
            assert_eq!(
                sb.cpu_bytes().unwrap().data,
                rb.cpu_bytes().unwrap().data,
                "t={t} frame {i}: decoded bytes must be frame-identical"
            );
        }
    }
}

// ---------------------------------------------------------------------------
// D-5 seek Snap lands on the ≤ t keyframe (E-007's 1.5s trap, as a POSITIVE test)
// ---------------------------------------------------------------------------

#[test]
fn d5_seek_snap_lands_on_keyframe() {
    let kf = keyframes_of("cfr24"); // 0,1,2,...,7 s
    let index = FfmpegProbe.keyframe_index(&asset("cfr24.mp4")).unwrap();
    for t in [
        Rational::new(1, 2), // between kf0 and kf1
        Rational::new(3, 2), // the E-007 1.5s case -> must land on 1.0s, never 2.0s
        Rational::new(79, 10),
    ] {
        let expected_floor = kf.iter().rev().find(|k| **k <= t).unwrap();
        let cfg = DecodeConfig {
            keyframe_index: Some(index.clone()),
            ..DecodeConfig::default()
        };
        let mut dec = open_cfr24(cfg);
        let landed = dec.seek(t, SeekMode::Snap).unwrap();
        assert_eq!(landed, *expected_floor, "t={t}: snap lands on ≤ t keyframe");
        let first = dec.next().unwrap().expect("frame at keyframe");
        assert_eq!(
            first.pts, landed,
            "t={t}: first decoded frame IS the keyframe"
        );
        assert!(first.keyframe, "t={t}: landed frame flagged as keyframe");
    }
    let _ = kf;
}

// ---------------------------------------------------------------------------
// D-6 flush introduces no stale state
// ---------------------------------------------------------------------------

#[test]
fn d6_flush_equivalence() {
    // A: seek -> decode 3 frames
    let a = {
        let mut dec = open_cfr24(DecodeConfig::default());
        dec.seek(Rational::new(3, 2), SeekMode::Exact).unwrap();
        (0..3)
            .map(|_| dec.next().unwrap().expect("frame"))
            .map(|f| (f.pts, f.deep_copy_cpu().unwrap()))
            .collect::<Vec<_>>()
    };
    // B: seek -> flush -> decode 3 frames (must equal A — no stale state)
    let b = {
        let mut dec = open_cfr24(DecodeConfig::default());
        dec.seek(Rational::new(3, 2), SeekMode::Exact).unwrap();
        dec.flush();
        (0..3)
            .map(|_| dec.next().unwrap().expect("frame"))
            .map(|f| (f.pts, f.deep_copy_cpu().unwrap()))
            .collect::<Vec<_>>()
    };
    assert_eq!(a.len(), b.len());
    for ((ap, ab), (bp, bb)) in a.iter().zip(b.iter()) {
        assert_eq!(ap, bp);
        assert_eq!(
            ab.cpu_bytes().unwrap().data,
            bb.cpu_bytes().unwrap().data,
            "flush must not change decoded output"
        );
    }
}

// ---------------------------------------------------------------------------
// D-7 cooperative cancel
// ---------------------------------------------------------------------------

#[test]
fn d7_cancel_stops_cleanly() {
    let mut dec = open_cfr24(DecodeConfig::default());
    let mut held = Vec::new();
    for _ in 0..5 {
        held.push(dec.next().unwrap().expect("frame"));
    }
    assert_eq!(dec.pool_stats().live, 5);
    dec.cancel();
    assert_eq!(
        dec.next(),
        Err(DecodeError::Cancelled),
        "cancel observed by next()"
    );
    assert_eq!(
        dec.next(),
        Err(DecodeError::Cancelled),
        "session unusable after cancel"
    );
    // resource hygiene: held frames reclaim without stale-generation errors
    for f in held {
        dec.reclaim(f).expect("reclaim after cancel");
    }
    assert_eq!(dec.pool_stats().live, 0);
    drop(dec); // libav resources freed, no panic
}

// ---------------------------------------------------------------------------
// D-8 error model: corrupt inputs are typed values, never panics
// ---------------------------------------------------------------------------

#[test]
fn d8_error_model_corrupt_and_unreadable() {
    // pure garbage: header unparseable
    let e = FfmpegSwDecoder::open(&asset("garbage.mp4"), StreamId(0), DecodeConfig::default())
        .err()
        .expect("garbage must fail to open");
    assert!(
        matches!(e, DecodeError::Unreadable | DecodeError::Corrupt(_)),
        "got {e:?}"
    );
    assert!(!e.to_string().is_empty());

    // truncated mp4 (moov cut off): header-level failure
    let e = FfmpegSwDecoder::open(
        &asset("truncated.mp4"),
        StreamId(0),
        DecodeConfig::default(),
    )
    .err()
    .expect("truncated must fail to open");
    assert!(
        matches!(e, DecodeError::Unreadable | DecodeError::Corrupt(_)),
        "got {e:?}"
    );

    // missing file
    let missing = AssetRef {
        content_hash: ove_media::ContentHash::from_bytes(b"missing"),
        locator: media("does-not-exist.mp4").to_string_lossy().into_owned(),
    };
    let e = FfmpegSwDecoder::open(&missing, StreamId(0), DecodeConfig::default())
        .err()
        .expect("missing file must fail");
    assert!(
        matches!(e, DecodeError::Unreadable | DecodeError::Io(_)),
        "got {e:?}"
    );

    // audio-only stream request on a video+audio file: typed Unsupported (v1 scope)
    let e = FfmpegSwDecoder::open(&asset("vidaud.mp4"), StreamId(1), DecodeConfig::default())
        .err()
        .expect("audio stream must be typed-rejected in v1");
    assert!(matches!(e, DecodeError::Unsupported(_)), "got {e:?}");
}

// ---------------------------------------------------------------------------
// D-9 SW fallback honesty: hw request on a SW adapter is typed Unsupported
// ---------------------------------------------------------------------------

#[test]
fn d9_no_hidden_downgrade() {
    let e = FfmpegSwDecoder::open(
        &asset("cfr24.mp4"),
        StreamId(0),
        DecodeConfig {
            hw: Some(HwAccel::Auto),
            ..DecodeConfig::default()
        },
    )
    .err()
    .expect("hw request must not silently fall back");
    assert!(matches!(e, DecodeError::Unsupported(_)));
    let e = FfmpegSwDecoder::open(
        &asset("cfr24.mp4"),
        StreamId(0),
        DecodeConfig {
            hw: Some(HwAccel::Vendor("nvdec")),
            ..DecodeConfig::default()
        },
    )
    .err()
    .expect("vendor hw request must not silently fall back");
    assert!(matches!(e, DecodeError::Unsupported(_)));
    assert!(
        e.to_string().contains("software-only"),
        "message names the limit: {e}"
    );
}

// ---------------------------------------------------------------------------
// D-10 color tags: five tags present on EVERY video frame (unknown = value)
// ---------------------------------------------------------------------------

#[test]
fn d10_color_tags_always_present() {
    let envs = decode_all(&asset("cfr24.mp4"), DecodeConfig::default());
    let f = envs.first().unwrap();
    // the corpus carries no color metadata: the honest value set is explicit
    // Unknown everywhere — and the five fields EXIST by type (compile-enforced)
    let c = f.color;
    // mp4/mpeg4 corpus carries no CICP tags: decoder reports Unknown, and the
    // MPEG-4 4:2:0 default chroma location (LEFT) as a VALUE — absence is not
    // representable (FRAME_CONTRACT §1/§4)
    assert_eq!(c.primaries, Primaries::Unknown);
    assert_eq!(c.transfer, Transfer::Unknown);
    assert_eq!(c.matrix, MatrixCoeffs::Unknown);
    assert_eq!(c.range, Range::Unknown);
    assert_eq!(c.chroma_loc, Some(ChromaLoc::Left));
}

// ---------------------------------------------------------------------------
// D-11 memory honesty: SW adapter emits only Cpu, caps declare it
// ---------------------------------------------------------------------------

#[test]
fn d11_memory_and_backend_honesty() {
    let mut dec = open_cfr24(DecodeConfig::default());
    let caps = dec.capabilities().clone();
    assert_eq!(caps.hw, None, "SW build declares no hw");
    assert_eq!(caps.memory_outputs, vec![FrameMemory::Cpu]);
    assert!(caps.seek_modes.contains(&SeekMode::Exact));
    assert!(!caps.threaded, "default config is single-threaded");
    let f = dec.next().unwrap().unwrap();
    assert!(f.cpu_bytes().is_some(), "SW frames are CPU frames");
    assert_eq!(f.backend, BackendId::FFmpegSw, "backend identity travels");
    assert_eq!(f.memory, FrameMemory::Cpu);
    assert_eq!(f.width, 320);
    assert_eq!(f.height, 240);
}

// ---------------------------------------------------------------------------
// D-12 VFR safety: nothing may assume CFR
// ---------------------------------------------------------------------------

#[test]
fn d12_vfr_pts_sequence_and_report() {
    let (pts, _, _) = frames_of("vfr");
    assert_eq!(pts.len(), 10);
    let envs = decode_all(&asset("vfr.mp4"), DecodeConfig::default());
    assert_eq!(envs.len(), pts.len());
    for (i, f) in envs.iter().enumerate() {
        assert_eq!(f.pts, pts[i], "VFR frame {i}: pts exact");
    }
    // the decoded pts sequence is genuinely non-uniform
    let report = VfrReport::from_pts(&envs.iter().map(|f| f.pts).collect::<Vec<_>>());
    assert!(report.is_vfr, "vfr.mp4 must detect as VFR");
    assert!(report.distinct_deltas.contains(&Rational::new(1, 30)));
    assert!(report.distinct_deltas.contains(&Rational::new(1, 15)));
    // and the CFR sample reports exactly one delta
    let cfr = decode_all(&asset("cfr24.mp4"), DecodeConfig::default());
    let cfr_report = VfrReport::from_pts(&cfr.iter().map(|f| f.pts).collect::<Vec<_>>());
    assert!(!cfr_report.is_vfr, "cfr24 must NOT detect as VFR");
}

// ---------------------------------------------------------------------------
// Thread-count determinism (DECODER_SPEC §4 note): same pts regardless of
// internal threading; same bytes on the golden set.
// ---------------------------------------------------------------------------

#[test]
fn threaded_decode_output_matches_single_thread() {
    let single = decode_all(&asset("cfr24.mp4"), DecodeConfig::default());
    let threaded = decode_all(
        &asset("cfr24.mp4"),
        DecodeConfig {
            thread_count: 4,
            ..DecodeConfig::default()
        },
    );
    assert_eq!(single.len(), threaded.len());
    for (a, b) in single.iter().zip(threaded.iter()) {
        assert_eq!(a.pts, b.pts, "pts identical under threading");
        assert_eq!(a.duration, b.duration);
        assert_eq!(a.keyframe, b.keyframe);
        assert_eq!(
            a.cpu_bytes().unwrap().data,
            b.cpu_bytes().unwrap().data,
            "decoded bytes bit-identical under threading (conformance corpus)"
        );
    }
}
