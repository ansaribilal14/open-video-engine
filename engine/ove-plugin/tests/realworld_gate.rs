//! WAVE 17 REAL-WORLD GATE (docs/REALWORLD_VALIDATION.md §8 — the
//! certification loop): the plugin tier exercised on the REAL reference
//! media (the same NASA public-domain source, same independent-baseline
//! discipline as the permanent proof in ove-engine).
//!
//! The stub plugin proposes ONE content-neutral structural edit (split the
//! first clip of the lowest track at half its duration) through the SAME
//! command bus. The gate pins, against REAL media:
//!
//!   manifest handshake → context → proposal applied → receipt evidence →
//!   state-hash advance → RENDER-INVARIANCE of the structural edit
//!   (spot renders byte-identical pre/post) → sample-exact audio across
//!   the plugin edit → exact re-encode export (frame count, video
//!   duration, audio duration all EXACT) → deterministic machine record.
//!
//! Scope note (REALWORLD_VALIDATION §5 wording discipline): this gate
//! certifies a verified EDITED SEGMENT of the real source — not a
//! full-source end-to-end pass-through.
//!
//! Env-gated exactly like the permanent proof (real media never enters CI):
//!
//!   OVE_REALWORLD_SOURCE  path to the acquired source mp4
//!   OVE_REALWORLD_EXPECT  INDEPENDENT baseline facts JSON
//!   OVE_REALWORLD_OUT     artifact directory (record + receipt + export)

use std::path::{Path, PathBuf};

use ove_engine::Engine;
use ove_media::{ContentHash, PixelFormat};
use ove_plugin::PluginHost;
use ove_render::OutputSpec;
use ove_time::Rational;
use ove_timeline::{GapTrack, TrackKind};

const AXIS: i64 = 24_000;
const OUT_RATE_NUM: i64 = 24_000;
const OUT_RATE_DEN: i64 = 1_001;
const N_FRAMES: i64 = 80; // span = 80080 ticks = 3.336667 s; 80080*147/80 = 147147 audio samples (exact)

fn ticks(n: i64) -> Rational {
    Rational::new(n, AXIS)
}

fn s(sec: i64, den: i64) -> Rational {
    Rational::new(sec * (AXIS / den), AXIS)
}

fn tmp(name: &str) -> PathBuf {
    let d = std::env::temp_dir().join("ove-realworld-plugin").join(name);
    let _ = std::fs::remove_dir_all(&d);
    std::fs::create_dir_all(d.parent().unwrap()).unwrap();
    d
}

struct Expect {
    width: u32,
    height: u32,
    fps_num: i64,
    fps_den: i64,
    audio_rate: u32,
    audio_channels: u32,
}

fn load_expect(path: &Path) -> Expect {
    let raw = std::fs::read_to_string(path).expect("baseline facts JSON readable");
    let v: serde_json::Value = serde_json::from_str(&raw).expect("baseline facts JSON valid");
    let fps: Vec<&str> = v["video"]["avg_frame_rate"]
        .as_str()
        .unwrap()
        .split('/')
        .collect();
    Expect {
        width: v["video"]["width"].as_u64().unwrap() as u32,
        height: v["video"]["height"].as_u64().unwrap() as u32,
        fps_num: fps[0].parse().unwrap(),
        fps_den: if fps.len() > 1 {
            fps[1].parse().unwrap()
        } else {
            1
        },
        audio_rate: v["audio"]["sample_rate"].as_u64().unwrap() as u32,
        audio_channels: v["audio"]["channels"].as_u64().unwrap() as u32,
    }
}

fn blake3_frame(f: &ove_media::FrameEnvelope) -> String {
    let bytes = f.cpu_bytes().expect("frame payload");
    ContentHash::from_bytes(&bytes.data).hex()
}

fn bt709() -> ove_media::ColorTags {
    ove_media::ColorTags {
        primaries: ove_media::Primaries::Bt709,
        transfer: ove_media::Transfer::Bt709,
        matrix: ove_media::MatrixCoeffs::Bt709,
        range: ove_media::Range::Limited,
        chroma_loc: Some(ove_media::ChromaLoc::Left),
    }
}

#[test]
fn realworld_plugin_tier_gate() {
    let Some(src) = std::env::var("OVE_REALWORLD_SOURCE").ok() else {
        eprintln!("SKIP: OVE_REALWORLD_SOURCE not set (real media stays out of CI)");
        return;
    };
    let expect_path = std::env::var("OVE_REALWORLD_EXPECT")
        .expect("OVE_REALWORLD_EXPECT must accompany OVE_REALWORLD_SOURCE");
    let exp = load_expect(Path::new(&expect_path));
    assert_eq!(
        (exp.fps_num, exp.fps_den),
        (OUT_RATE_NUM, OUT_RATE_DEN),
        "baseline cadence == output cadence (CFR 1:1, no retime)"
    );
    let src = PathBuf::from(&src);
    assert!(src.exists(), "real-world source missing: {}", src.display());

    let out_dir = std::env::var("OVE_REALWORLD_OUT")
        .map(PathBuf::from)
        .unwrap_or_else(|_| tmp("artifacts"));
    std::fs::create_dir_all(&out_dir).unwrap();

    // -- project on the real source: track 1 = contiguous audio lane ---------
    // clip1 [0, 1.0) src 0-1  (44,100 samples — the stub's half-split stays
    //                          sample-exact: 0.5 s = 22,050)
    // clip2 [1.0, span) src 1-… (56080 ticks = 103,047 samples)
    let dir = tmp("project");
    let mut e = Engine::create(&dir, (AXIS, 1)).expect("create project");
    e.add_track(1, TrackKind::Gap(GapTrack::new()))
        .expect("track 1 (video bottom layer = v1 audio lane)");
    let hex = e.import_media(&src).expect("import real-world media");
    let media = e.source(&hex).expect("source registered").clone();
    let vstream = media
        .probe
        .streams
        .iter()
        .find(|s| s.kind == ove_media::StreamKind::Video)
        .expect("video stream probed")
        .clone();
    let vd = vstream.video.clone().expect("video details");
    assert_eq!((vd.width, vd.height), (exp.width, exp.height));
    let astream = media
        .probe
        .streams
        .iter()
        .find(|s| s.kind == ove_media::StreamKind::Audio)
        .expect("audio stream probed")
        .clone();
    let ad = astream.audio.clone().expect("audio details");
    assert_eq!(
        ad.sample_rate, exp.audio_rate,
        "audio rate == INDEPENDENT baseline"
    );
    assert_eq!(
        ad.channels, exp.audio_channels,
        "channels == INDEPENDENT baseline"
    );
    let span = Rational::new(N_FRAMES * OUT_RATE_DEN, OUT_RATE_NUM);
    assert!(
        vstream.duration.expect("video duration probed") > span,
        "source covers the plugin-edit span"
    );
    let _clip1 = e
        .add_clip(1, &hex, s(1, 1), Rational::new(0, 1))
        .expect("clip1 [0,1)");
    let _clip2 = e
        .add_clip(1, &hex, ticks(N_FRAMES * OUT_RATE_DEN - AXIS), s(1, 1))
        .expect("clip2 [1,span)");
    let pre_hash = e.state_hash();

    // -- pre-plugin renders (the invariance witnesses) ------------------------
    let output = OutputSpec {
        width: exp.width,
        height: exp.height,
        rate_num: OUT_RATE_NUM,
        rate_den: OUT_RATE_DEN,
        working_space: bt709(),
    };
    let pre_inside_clip1 = e
        .render_frame(&output, s(1, 4))
        .expect("render t=0.25s (inside clip1)");
    let pre_inside_clip2 = e
        .render_frame(&output, s(2, 1))
        .expect("render t=2s (inside clip2)");
    assert_eq!(pre_inside_clip1.pixel_format, PixelFormat::Rgba);
    assert_eq!(pre_inside_clip2.pixel_format, PixelFormat::Rgba);
    let w1 = blake3_frame(&pre_inside_clip1);
    let w2 = blake3_frame(&pre_inside_clip2);

    // -- W17: the plugin tier on REAL media (ADR-021) -------------------------
    let mut host =
        PluginHost::spawn(Path::new(env!("CARGO_BIN_EXE_ove-plugin-stub"))).expect("spawn stub");
    host.handshake(&mut e).expect("manifest handshake");
    host.send_context(&e).expect("document context");
    let report = host.run(&mut e).expect("plugin session");

    assert_eq!(report.plugin, "stub");
    assert_eq!(
        report.applied, 1,
        "the split proposal applied on real media"
    );
    assert_eq!(report.rejected, 0);
    assert_eq!(report.proposals[0].verb, "split");
    assert_eq!(
        report.proposals[0].proposal, 1,
        "receipt ties the mutation to the proposal id"
    );
    assert_ne!(
        report.final_state_hash, pre_hash,
        "plugin edit advanced the real-media document state"
    );
    assert_eq!(report.final_state_hash, e.state_hash());

    // -- render invariance: a structural split must not move a pixel ---------
    let post_inside_clip1 = e
        .render_frame(&output, s(1, 4))
        .expect("render t=0.25s after plugin edit");
    let post_inside_clip2 = e
        .render_frame(&output, s(2, 1))
        .expect("render t=2s after plugin edit");
    assert_eq!(
        blake3_frame(&post_inside_clip1),
        w1,
        "split of clip1 is render-invariant inside the left half"
    );
    assert_eq!(
        blake3_frame(&post_inside_clip2),
        w2,
        "split of clip1 is render-invariant inside clip2"
    );

    // -- exact export of the plugin-edited real-media project -----------------
    let out_av = out_dir.join("realworld_plugin_av.mp4");
    let info = e
        .export_reencode(&out_av, &output, N_FRAMES)
        .expect("A/V re-encode export after plugin edit");
    assert_eq!(
        info.tracks[0].nb_frames,
        Some(N_FRAMES as u64),
        "video frame count exact"
    );
    assert_eq!(
        info.tracks[0].duration,
        Some(span),
        "video duration == exact span"
    );
    let audio_samples: u64 = (span * Rational::new(exp.audio_rate as i64, 1)).num() as u64;
    assert_eq!(
        audio_samples,
        (N_FRAMES as u64) * (OUT_RATE_DEN as u64) * (exp.audio_rate as u64) / (OUT_RATE_NUM as u64),
        "guard: span→samples conversion is integral for this span"
    );
    assert_eq!(
        info.tracks[1].duration,
        Some(Rational::new(audio_samples as i64, exp.audio_rate as i64)),
        "audio duration == exact sample count / rate (sample-exact across the plugin split)"
    );

    // -- deterministic machine record ------------------------------------------
    let record = serde_json::json!({
        "commit": option_env!("OVE_COMMIT").unwrap_or("unspecified"),
        "wave": "W17-plugin-tier",
        "source_asset_blake3": hex,
        "state_hash_pre_plugin": pre_hash,
        "state_hash_post_plugin": report.final_state_hash,
        "plugin_receipt": serde_json::from_str::<serde_json::Value>(&report.to_receipt_json())
            .expect("receipt is valid JSON"),
        "render_invariance": {
            "t_0_25s": w1,
            "t_2s": w2,
        },
        "export": {
            "path": out_av.display().to_string(),
            "sha256": info.file_sha256,
            "video_frames": info.tracks[0].nb_frames,
            "video_duration": format!("{}/{}", span.num(), span.den()),
            "audio_samples": audio_samples,
        },
    });
    std::fs::write(
        out_dir.join("realworld_record_plugin.json"),
        serde_json::to_string_pretty(&record).unwrap(),
    )
    .unwrap();
    std::fs::write(
        out_dir.join("realworld_plugin_receipt.json"),
        report.to_receipt_json(),
    )
    .unwrap();
    eprintln!(
        "RW17: record written to {}",
        out_dir.join("realworld_record_plugin.json").display()
    );
}
