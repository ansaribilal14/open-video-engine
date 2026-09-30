//! REAL-WORLD REFERENCE MEDIA validation — the permanent proof workflow
//! (docs/REALWORLD_VALIDATION.md).
//!
//! One canonical scenario runs the CURRENT engine against ONE real-world
//! YouTube source (acquired via ytagent — see the doc for provenance):
//!
//!   import → content-hash identity → timeline commands (explicit ids) →
//!   split/resize with sample-exact audio boundaries → keyframed overlay →
//!   undo/redo hash identity → spot renders → re-encode export (A/V) →
//!   WAV export → stream-copy export → save → reopen → SAME state hash →
//!   re-export byte-identical (deterministic software pipeline).
//!
//! The media itself is NEVER committed to git (huge binary). The test
//! runs only when BOTH env vars are set:
//!
//!   OVE_REALWORLD_SOURCE  path to the acquired source mp4
//!   OVE_REALWORLD_EXPECT  path to the INDEPENDENT baseline facts JSON
//!                         (produced by scripts/realworld/baseline_facts.py
//!                         from ffprobe — OVE is never the authority)
//!
//! CI runs the full workspace WITHOUT these vars; the real-media leg runs
//! on demand and locally per the doc. Wave discipline (REALWORLD REGRESSION
//! RULE): a wave that breaks this scenario is not complete.

use std::path::{Path, PathBuf};

use ove_engine::{decoder_opens, reset_decoder_opens, Engine};
use ove_media::{ContentHash, PixelFormat};
use ove_render::OutputSpec;
use ove_time::Rational;
use ove_timeline::{Clip, GapTrack, Keyframe, PropertyName};

const AXIS: i64 = 24_000;
const OUT_RATE_NUM: i64 = 24_000;
const OUT_RATE_DEN: i64 = 1_001; // 23.976 NTSC — the source's real cadence
const N_FRAMES: i64 = 160; // span = 160 x 1001/24000 s = 160160 ticks

fn ticks(n: i64) -> Rational {
    Rational::new(n, AXIS)
}

fn s(sec: i64, den: i64) -> Rational {
    Rational::new(sec * (AXIS / den), AXIS)
}

fn tmp(name: &str) -> PathBuf {
    let d = std::env::temp_dir().join("ove-realworld").join(name);
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

#[test]
fn realworld_reference_media_proof() {
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

    // -- project + tracks -----------------------------------------------------
    let dir = tmp("project");
    let mut e = Engine::create(&dir, (AXIS, 1)).expect("create project");
    e.add_track(1, ove_timeline::TrackKind::Gap(GapTrack::new()))
        .expect("track 1 (video bottom layer = v1 audio lane)");
    e.add_track(2, ove_timeline::TrackKind::Gap(GapTrack::new()))
        .expect("track 2 (overlay layer)");

    // -- import: content identity + probe vs INDEPENDENT baseline --------------
    let hex = e.import_media(&src).expect("import real-world media");
    let media = e.source(&hex).expect("source registered").clone();
    assert_eq!(media.hash.hex(), hex, "content-hash identity");
    let vstream = media
        .probe
        .streams
        .iter()
        .find(|s| s.kind == ove_media::StreamKind::Video)
        .expect("video stream probed")
        .clone();
    let vd = vstream.video.clone().expect("video details");
    assert_eq!(
        vd.width, exp.width,
        "probe width == INDEPENDENT ffprobe width"
    );
    assert_eq!(
        vd.height, exp.height,
        "probe height == INDEPENDENT ffprobe height"
    );
    let span = Rational::new(N_FRAMES * OUT_RATE_DEN, OUT_RATE_NUM);
    assert!(
        vstream.duration.expect("video duration probed") > span,
        "source covers the whole edit span"
    );
    let kf_index = media.probe.keyframe_index.as_ref().expect("keyframe index");
    assert!(!kf_index.entries.is_empty(), "real keyframe index present");
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
    eprintln!(
        "RW: imported {} ({} keyframes indexed)",
        hex,
        kf_index.entries.len()
    );

    // -- timeline: contiguous audio-lane edit sequence (sample-exact cuts) ----
    // track 1: clip1 [0,2.0) src 0-2 | clip2 [2,3.5) src 8-9.5
    //   -> split @2.75 -> clip2b [2.75,3.5) src 8.75-9.5
    //   -> resize clip2b to 1.0s -> [2.75,3.75) src 8.75-9.75
    //   -> clip4 [3.75, 160160 ticks) src 20-22.90667 (ends at span end)
    // track 2: filler [0,1) src 40-41 (opacity 0) | overlay [1,3.5) src 30-32.5
    //   keyframed: opacity fade-in/hold/fade-out, x pan, y drift
    e.add_clip(1, &hex, s(2, 1), Rational::new(0, 1))
        .expect("clip1");
    let h_after_clip2 = e.state_hash();
    let clip2 = e.add_clip(1, &hex, s(3, 2), s(8, 1)).expect("clip2");
    let clip2b = e.split(1, clip2, s(3, 4)).expect("split clip2 at 2.75s");
    e.resize(1, clip2b, s(1, 1)).expect("resize clip2b to 1.0s");
    let clip4 = e
        .add_clip(1, &hex, ticks(160_160 - 90_000), s(20, 1))
        .expect("clip4");
    let filler = e.add_clip(2, &hex, s(1, 1), s(40, 1)).expect("filler");
    let overlay = e.add_clip(2, &hex, s(5, 2), s(30, 1)).expect("overlay");
    let h_mid = e.state_hash(); // after all placements, before keyframes
    assert_ne!(h_mid, h_after_clip2, "state hash tracks edits");

    // exact source-mapping spot checks (ADR-013 seam): timeline 2.0s is
    // clip1's end and clip2's source 8.0s; timeline 2.75s -> source 8.75s.
    {
        use ove_timeline::mapping::ClipWindow;
        let track = e.project().timeline().track_ref(1).unwrap();
        let mut windows = Vec::new();
        track.walk(&mut |_pos, start, clip: &Clip| {
            windows.push(ClipWindow::from_clip(clip, start));
        });
        assert_eq!(
            windows.len(),
            4,
            "track 1 has 4 placements after split+clip4"
        );
        assert_eq!(windows[0].src_in, Rational::new(0, 1));
        assert_eq!(windows[1].timeline_start, s(2, 1), "clip2 starts at 2.0s");
        assert_eq!(windows[1].src_in, s(8, 1), "clip2 maps to source 8.0s");
        assert_eq!(
            windows[2].timeline_start,
            s(11, 4),
            "clip2b starts at 2.75s"
        );
        assert_eq!(windows[2].src_in, s(35, 4), "clip2b maps to source 8.75s");
        assert_eq!(
            windows[3].timeline_start,
            s(15, 4),
            "clip4 starts at 3.75s (after resized clip2b)"
        );
        assert_eq!(windows[3].src_in, s(20, 1), "clip4 maps to source 20.0s");
        assert_eq!(
            windows[3].dur,
            ticks(160_160 - 90_000),
            "clip4 ends exactly at the output span end"
        );
    }

    // -- keyframes (W8/ADR-019): wholesale SetKeyframes via commands -----------
    use ove_timeline::Command;
    let _ = &filler;
    let _ = &clip4;
    e.execute(Command::SetKeyframes {
        track: 2,
        id: filler,
        property: PropertyName::Opacity,
        keys: vec![Keyframe {
            time: Rational::new(0, 1),
            value: Rational::new(0, 1),
            interp: ove_timeline::Interpolation::Hold,
        }],
    })
    .expect("filler opacity 0 (hold)");
    e.execute(Command::SetKeyframes {
        track: 2,
        id: overlay,
        property: PropertyName::Opacity,
        keys: vec![
            Keyframe {
                time: Rational::new(0, 1),
                value: Rational::new(0, 1),
                interp: ove_timeline::Interpolation::Linear,
            },
            Keyframe {
                time: s(1, 4),
                value: Rational::new(1, 1),
                interp: ove_timeline::Interpolation::Linear,
            },
            Keyframe {
                time: s(9, 4),
                value: Rational::new(1, 1),
                interp: ove_timeline::Interpolation::Linear,
            },
            Keyframe {
                time: s(12, 5),
                value: Rational::new(0, 1),
                interp: ove_timeline::Interpolation::Hold,
            },
        ],
    })
    .expect("overlay opacity fade in/hold/out");
    e.execute(Command::SetKeyframes {
        track: 2,
        id: overlay,
        property: PropertyName::X,
        keys: vec![
            Keyframe {
                time: Rational::new(0, 1),
                value: Rational::new(-640, 1),
                interp: ove_timeline::Interpolation::Linear,
            },
            Keyframe {
                time: s(19, 10),
                value: Rational::new(640, 1),
                interp: ove_timeline::Interpolation::Linear,
            },
        ],
    })
    .expect("overlay x pan -640 -> 640");
    e.execute(Command::SetKeyframes {
        track: 2,
        id: overlay,
        property: PropertyName::Y,
        keys: vec![
            Keyframe {
                time: Rational::new(0, 1),
                value: Rational::new(30, 1),
                interp: ove_timeline::Interpolation::Linear,
            },
            Keyframe {
                time: s(19, 10),
                value: Rational::new(90, 1),
                interp: ove_timeline::Interpolation::Linear,
            },
        ],
    })
    .expect("overlay y drift 30 -> 90");
    let h_final = e.state_hash();
    assert_ne!(h_final, h_mid, "keyframes are document state");

    // -- undo/redo hash identity over the real edit graph -----------------------
    assert!(e.undo().expect("undo y"), "undo executed");
    assert!(e.undo().expect("undo x"), "undo executed");
    assert!(e.undo().expect("undo overlay opacity"), "undo executed");
    assert!(e.undo().expect("undo filler opacity"), "undo executed");
    assert_eq!(e.state_hash(), h_mid, "4 undos return to pre-keyframe hash");
    assert!(e.redo().expect("redo filler opacity"), "redo executed");
    assert!(e.redo().expect("redo overlay opacity"), "redo executed");
    assert!(e.redo().expect("redo x"), "redo executed");
    assert!(e.redo().expect("redo y"), "redo executed");
    assert_eq!(e.state_hash(), h_final, "4 redos return to final hash");

    // -- spot renders through decode -> envelope -> plan -> software exec -------
    let output = OutputSpec {
        width: exp.width,
        height: exp.height,
        rate_num: OUT_RATE_NUM,
        rate_den: OUT_RATE_DEN,
        working_space: bt709(),
    };
    let t0 = e
        .render_frame(&output, Rational::new(0, 1))
        .expect("render t=0");
    let t_mid = e
        .render_frame(&output, s(5, 2))
        .expect("render t=2.5s (overlay mid-fade over clip2a)");
    let t_tail = e
        .render_frame(&output, ticks(120_000))
        .expect("render t=5s (clip4 alone)");
    for (name, f) in [("t0", &t0), ("tmid", &t_mid), ("ttail", &t_tail)] {
        assert_eq!(f.pixel_format, PixelFormat::Rgba, "{name} is RGBA");
        assert_eq!(
            (f.width, f.height),
            (exp.width, exp.height),
            "{name} geometry == source geometry"
        );
        let bytes = f.cpu_bytes().expect("{name} has CPU payload");
        assert!(bytes.data.iter().any(|&b| b != 0), "{name} is not black");
    }
    let t_mid_again = e
        .render_frame(&output, s(5, 2))
        .expect("render t=2.5s again");
    assert_eq!(
        blake3_frame(&t_mid),
        blake3_frame(&t_mid_again),
        "spot render is deterministic"
    );

    // -- exports -----------------------------------------------------------------
    let span = Rational::new(N_FRAMES * OUT_RATE_DEN, OUT_RATE_NUM);
    let out_av = out_dir.join("realworld_proof_av.mp4");
    // W19 perf instrument (ADR-023): the A/V export leg is the perf-critical
    // path (decode → convert → render → encode per output frame). The wall
    // time and the decoder-open count land in the machine record; the open
    // budget is pinned by export_session_budget.rs (ONE open per source per
    // export — a per-frame count is the pre-W19 regression signature).
    reset_decoder_opens();
    let export_start = std::time::Instant::now();
    let info = e
        .export_reencode(&out_av, &output, N_FRAMES)
        .expect("A/V re-encode export");
    let export_av_seconds = export_start.elapsed().as_secs_f64();
    let export_av_decoder_opens = decoder_opens();
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
        info.tracks[1].duration,
        Some(Rational::new(audio_samples as i64, exp.audio_rate as i64)),
        "audio duration == exact sample count / rate"
    );

    let out_wav = out_dir.join("realworld_proof.wav");
    let wav_start = std::time::Instant::now();
    let wav_samples = e.export_wav(&out_wav).expect("WAV export");
    let export_wav_seconds = wav_start.elapsed().as_secs_f64();
    assert_eq!(wav_samples, audio_samples, "WAV sample count == assembly");

    // -- stream-copy export on real H.264 source --------------------------------
    // v1 contract (ADR-015): the copy route supports mpeg4/aac only. A real
    // H.264 source is TYPED-rejected by the muxer — honest unsupported, not a
    // crash. This pin documents the real-world v1 surface; h264 copy rides a
    // later wave (codec-tag/avcC propagation).
    let out_copy = out_dir.join("realworld_copy.mp4");
    match e.export_copy(&hex, s(24, 1), s(29, 1), &out_copy) {
        Err(ove_engine::EngineError::Mux(ove_encode::MuxError::InvalidTrackSpec(msg))) => {
            assert!(
                msg.contains("mpeg4/aac only"),
                "typed copy rejection names the v1 codec surface: {msg}"
            );
            eprintln!("RW: copy export typed-unsupported for h264 (v1 contract) — recorded");
        }
        other => panic!(
            "copy export on h264 must be typed-rejected, got: {:?}",
            other.map(|(i, _)| i.file_sha256)
        ),
    }

    // -- persistence: save -> drop -> reopen -> SAME hash -> re-export -----------
    let live_hash = e.state_hash();
    let uuid = e.uuid().to_string();
    let t_mid_bytes = t_mid.cpu_bytes().unwrap().data.clone();
    drop(e);
    let mut r = Engine::open(&dir).expect("reopen project");
    assert_eq!(r.uuid(), uuid, "project identity survives reopen");
    assert_eq!(r.state_hash(), live_hash, "reopen hash == live hash");
    assert_eq!(
        r.project().clip_assets().len(),
        5,
        "clip bindings survive reopen (clip1, clip2, clip4, filler, overlay). \
         NOTE: the Split command's right half (clip2b) has no binding entry — \
         record_entry_bindings does not consume Split (RW-NOTE-1; inert under \
         the v1 single-source render, must be closed before multi-source \
         render-by-binding lands)"
    );
    let t_mid_re = r
        .render_frame(&output, s(5, 2))
        .expect("render after reopen");
    assert_eq!(
        blake3_frame(&t_mid_re),
        ContentHash::from_bytes(&t_mid_bytes).hex(),
        "keyframed overlay renders byte-identical after reopen"
    );
    let out_av2 = out_dir.join("realworld_proof_av_reopen.mp4");
    reset_decoder_opens();
    let reexport_start = std::time::Instant::now();
    let info2 = r
        .export_reencode(&out_av2, &output, N_FRAMES)
        .expect("re-export after reopen");
    let export_reopen_seconds = reexport_start.elapsed().as_secs_f64();
    let export_reopen_decoder_opens = decoder_opens();
    assert_eq!(
        info.file_sha256, info2.file_sha256,
        "re-export byte-identical (deterministic software pipeline)"
    );

    // -- machine-readable record ---------------------------------------------------
    let record = serde_json::json!({
        "commit": option_env!("OVE_COMMIT").unwrap_or("unspecified"),
        "source_sha256": std::fs::read_to_string(out_dir.join("../source_identity_sha256.txt"))
            .unwrap_or_default()
            .trim(),
        "asset_blake3": hex,
        "state_hash_live": live_hash,
        "state_hash_mid": h_mid,
        "export": {
            "path": out_av.display().to_string(),
            "sha256": info.file_sha256,
            "size": info.file_size,
            "video_frames": info.tracks[0].nb_frames,
            "video_duration": format!("{}/{}", span.num(), span.den()),
            "audio_samples": audio_samples,
        },
        "wav_samples": wav_samples,
        "copy_export": "TYPED-UNSUPPORTED: v1 copy route is mpeg4/aac only; h264 rejected by muxer",
        "reopen_export_sha256": info2.file_sha256,
        "perf": {
            "note": "W19 ADR-023 instrument — wall-clock is machine-relative; the \
                     decoder-open budget is the deterministic pin (ONE per \
                     source per export; per-frame opens = pre-W19 defect shape)",
            "export_av_seconds": export_av_seconds,
            "export_av_decoder_opens": export_av_decoder_opens,
            "export_wav_seconds": export_wav_seconds,
            "export_reopen_seconds": export_reopen_seconds,
            "export_reopen_decoder_opens": export_reopen_decoder_opens,
        },
    });
    std::fs::write(
        out_dir.join("realworld_record.json"),
        serde_json::to_string_pretty(&record).unwrap(),
    )
    .unwrap();
    eprintln!(
        "RW: record written to {}",
        out_dir.join("realworld_record.json").display()
    );
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
