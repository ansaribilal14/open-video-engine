//! WAVE 18 REAL-WORLD SECURITY GATE (docs/REALWORLD_VALIDATION.md §8 —
//! the certification loop; docs/ADR-022): the security hardening
//! exercised ON the REAL reference media (the same NASA public-domain
//! source, same independent-baseline discipline as the permanent proof).
//!
//! Pins, against REAL media:
//!   hostile plugin proposal-flood → TYPED protocol abort at the budget,
//!     engine state hash UNCHANGED (the denied flood is inert) →
//!   the LEGIT plugin still applies its real-media edit through the same
//!     command bus (split applied, receipt, hash advance) →
//!   render invariance of the structural edit →
//!   exact re-encode export (frame count + durations EXACT) →
//!   deterministic machine record (wave + commit + hash).
//!
//! Scope note (REALWORLD_VALIDATION §5 wording discipline): this gate
//! certifies a verified EDITED SEGMENT of the real source — not a
//! full-source end-to-end pass-through.
//!
//! Env-gated exactly like the other real-world legs (real media never
//! enters CI):
//!   OVE_REALWORLD_SOURCE  path to the acquired source mp4
//!   OVE_REALWORLD_EXPECT  INDEPENDENT baseline facts JSON
//!   OVE_REALWORLD_OUT     artifact directory (record + export)

use std::path::{Path, PathBuf};

use ove_engine::Engine;
use ove_media::ContentHash;
use ove_plugin::{PluginError, PluginHost};
use ove_render::OutputSpec;
use ove_time::Rational;
use ove_timeline::{GapTrack, TrackKind};

const AXIS: i64 = 24_000;
const OUT_RATE_NUM: i64 = 24_000;
const OUT_RATE_DEN: i64 = 1_001;
const N_FRAMES: i64 = 80; // span = 80080 ticks = 3.336667 s

fn ticks(n: i64) -> Rational {
    Rational::new(n, AXIS)
}

fn s(sec: i64, den: i64) -> Rational {
    Rational::new(sec * (AXIS / den), AXIS)
}

fn tmp(name: &str) -> PathBuf {
    let d = std::env::temp_dir()
        .join("ove-realworld-security")
        .join(name);
    let _ = std::fs::remove_dir_all(&d);
    std::fs::create_dir_all(d.parent().unwrap()).unwrap();
    d
}

struct Expect {
    width: u32,
    height: u32,
    audio_rate: u32,
}

fn load_expect(path: &Path) -> Expect {
    let raw = std::fs::read_to_string(path).expect("baseline facts JSON readable");
    let v: serde_json::Value = serde_json::from_str(&raw).expect("baseline facts JSON valid");
    Expect {
        width: v["video"]["width"].as_u64().unwrap() as u32,
        height: v["video"]["height"].as_u64().unwrap() as u32,
        audio_rate: v["audio"]["sample_rate"].as_u64().unwrap() as u32,
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
fn realworld_security_gate() {
    let Some(src) = std::env::var("OVE_REALWORLD_SOURCE").ok() else {
        eprintln!("SKIP: OVE_REALWORLD_SOURCE not set (real media stays out of CI)");
        return;
    };
    let expect_path = std::env::var("OVE_REALWORLD_EXPECT")
        .expect("OVE_REALWORLD_EXPECT must accompany OVE_REALWORLD_SOURCE");
    let exp = load_expect(Path::new(&expect_path));
    let src = PathBuf::from(&src);
    assert!(src.exists(), "real-world source missing: {}", src.display());

    let out_dir = std::env::var("OVE_REALWORLD_OUT")
        .map(PathBuf::from)
        .unwrap_or_else(|_| tmp("artifacts"));
    std::fs::create_dir_all(&out_dir).unwrap();

    // -- project on the real source (same shape as the W17 plugin gate) ------
    let dir = tmp("project");
    let mut e = Engine::create(&dir, (AXIS, 1)).expect("create project");
    e.add_track(1, TrackKind::Gap(GapTrack::new()))
        .expect("track 1");
    let hex = e.import_media(&src).expect("import real-world media");
    let span = Rational::new(N_FRAMES * OUT_RATE_DEN, OUT_RATE_NUM);
    let _clip1 = e
        .add_clip(1, &hex, s(1, 1), Rational::new(0, 1))
        .expect("clip1 [0,1)");
    let _clip2 = e
        .add_clip(1, &hex, ticks(N_FRAMES * OUT_RATE_DEN - AXIS), s(1, 1))
        .expect("clip2 [1,span)");
    let pre_hash = e.state_hash();

    // -- pre-attack render witnesses (real decoded frames) --------------------
    let output = OutputSpec {
        width: exp.width,
        height: exp.height,
        rate_num: OUT_RATE_NUM,
        rate_den: OUT_RATE_DEN,
        working_space: bt709(),
    };
    let w1 = blake3_frame(
        &e.render_frame(&output, s(1, 4))
            .expect("render t=0.25s pre-attack"),
    );
    let w2 = blake3_frame(
        &e.render_frame(&output, s(2, 1))
            .expect("render t=2s pre-attack"),
    );

    // -- W18: hostile plugin flood against the REAL-media session -------------
    // A valid manifest (no capabilities), then 11 000 default-DENY
    // proposals. The host must abort TYPED at the budget with the engine
    // state EXACTLY unchanged — a resource attack leaves no document mark.
    let flood = Path::new(env!("CARGO_BIN_EXE_ove-plugin-flood"));
    let mut host = PluginHost::spawn_with_args(flood, &["props"]).expect("spawn hostile plugin");
    host.handshake(&mut e)
        .expect("the flood's manifest itself is valid");
    let err = host.run(&mut e).expect_err("flood must abort the session");
    match &err {
        PluginError::Protocol(msg) => assert!(
            msg.contains("proposal budget"),
            "typed budget abort on real media, got: {msg}"
        ),
        other => panic!("expected Protocol, got {other:?}"),
    }
    assert_eq!(
        e.state_hash(),
        pre_hash,
        "the denied flood is INERT on real media — document state untouched"
    );
    // capture the post-attack hash NOW (before the legit edit advances it) —
    // the record field must mean exactly what it says
    let hash_after_attack = e.state_hash();
    assert_eq!(hash_after_attack, pre_hash);
    // render identity also survives the attack
    assert_eq!(
        blake3_frame(
            &e.render_frame(&output, s(1, 4))
                .expect("render t=0.25s post-attack")
        ),
        w1,
        "post-attack render byte-identical"
    );

    // -- the LEGIT plugin still works through the same boundary ---------------
    let mut host =
        PluginHost::spawn(Path::new(env!("CARGO_BIN_EXE_ove-plugin-stub"))).expect("spawn stub");
    host.handshake(&mut e).expect("manifest handshake");
    host.send_context(&e).expect("document context");
    let report = host.run(&mut e).expect("legit plugin session");
    assert_eq!(report.plugin, "stub");
    assert_eq!(report.applied, 1, "split applied after the hostile abort");
    assert_eq!(report.proposals[0].verb, "split");
    assert_ne!(
        report.final_state_hash, pre_hash,
        "legit plugin edit advanced the real-media document state"
    );

    // -- render invariance of the structural edit -----------------------------
    assert_eq!(
        blake3_frame(
            &e.render_frame(&output, s(1, 4))
                .expect("render t=0.25s post-split")
        ),
        w1,
        "split is render-invariant inside the left half"
    );
    assert_eq!(
        blake3_frame(
            &e.render_frame(&output, s(2, 1))
                .expect("render t=2s post-split")
        ),
        w2,
        "split is render-invariant inside clip2"
    );

    // -- exact export of the post-attack, post-edit real-media project --------
    let out_av = out_dir.join("realworld_security_av.mp4");
    let info = e
        .export_reencode(&out_av, &output, N_FRAMES)
        .expect("A/V re-encode export after hostile abort + legit edit");
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

    // -- deterministic machine record (wave + commit + hash) ------------------
    let record = serde_json::json!({
        "commit": option_env!("OVE_COMMIT").unwrap_or("unspecified"),
        "wave": "W18-security-hardening",
        "source_asset_blake3": hex,
        "state_hash_pre_attack": pre_hash,
        "hostile_flood": {
            "kind": "proposal flood (11k default-DENY proposals)",
            "abort": format!("{err}"),
            "state_hash_after_attack": hash_after_attack,
            "engine_unchanged": hash_after_attack == pre_hash,
        },
        "legit_plugin": {
            "applied": report.applied,
            "state_hash_after": report.final_state_hash,
        },
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
        out_dir.join("realworld_record_security.json"),
        serde_json::to_string_pretty(&record).unwrap(),
    )
    .unwrap();
    eprintln!(
        "RW18: record written to {}",
        out_dir.join("realworld_record_security.json").display()
    );
}
