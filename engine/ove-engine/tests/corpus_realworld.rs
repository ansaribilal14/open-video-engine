//! REAL-WORLD CORPUS CERTIFICATION — difficult real media (RLW-8 leg of
//! docs/REALWORLD_VALIDATION.md §10).
//!
//! The primary NASA scenario (realworld.rs) proves ordinary operation. THIS
//! file exercises deliberately difficult — but benign — real media classes:
//!
//!   portrait_true      true-portrait storage (406x720, no rotation needed)
//!   rotation_metadata  landscape storage + Display Matrix rotation=90
//!                      (the phone-portrait pattern; probe/rotation truth)
//!   vfr_constructed    genuinely uneven frame timing (3 distinct deltas),
//!                      silent (no audio stream)
//!   long_gop           5 keyframes / max GOP 25.025 s (seek + forward-decode)
//!   audio_48k/mono/51  audio geometry variants (48 kHz stereo / mono / 5.1)
//!
//! DISCIPLINE (binding for this wave):
//!   * media is NEVER committed to git; every item runs only when its env
//!     vars are set (CI stays corpus-only);
//!   * OVE is never the authority — expectations come from ffprobe-built
//!     baseline JSONs (scripts/realworld/corpus_baseline.py);
//!   * a corpus item that exposes a defect is RECORDED with its typed/untyped
//!     classification, not silently normalized. The engine must never die
//!     untyped on difficult-but-benign input: a panic/OOM/hang here is the
//!     finding, and the test run is the preserved evidence.
//!
//! Env vars per scenario:
//!   OVE_CORPUS_PORTRAIT / OVE_CORPUS_ROTATION / OVE_CORPUS_VFR /
//!   OVE_CORPUS_LONGGOP            path to the media file
//!   <same>_BASELINE               path to its corpus_baseline.py JSON
//!   OVE_CORPUS_AUDIO_DIR          dir with audio_48k/mono/51 .mp4 + baseline
//!   OVE_CORPUS_OUT                artifact/record output dir
//!   OVE_COMMIT                    commit bound written into every record

use std::path::{Path, PathBuf};

use ove_engine::{decoder_opens, reset_decoder_opens, Engine};
use ove_media::{ContentHash, PixelFormat};
use ove_render::OutputSpec;
use ove_time::Rational;
use serde_json::{json, Value};

// Exact project tick axis (v1 constant across waves).
const AXIS: i64 = 24_000;

struct Baseline {
    width: u32,
    height: u32,
    fps_num: i64,
    fps_den: i64,
    audio_rate: Option<u32>,
    audio_channels: Option<u32>,
    keyframes: usize,
    is_vfr: bool,
    is_vfr_jitter_tolerant: bool,
    vfr_exact_deltas: u64,
    rotation: Option<f64>,
    sha256: String,
}

fn load_baseline(path: &Path) -> Baseline {
    let raw = std::fs::read_to_string(path).expect("corpus baseline JSON readable");
    let v: Value = serde_json::from_str(&raw).expect("corpus baseline JSON valid");
    let fps: Vec<&str> = v["video"]["avg_frame_rate"]
        .as_str()
        .unwrap()
        .split('/')
        .collect();
    let num: i64 = fps[0].parse().unwrap();
    let den: i64 = if fps.len() > 1 {
        fps[1].parse().unwrap()
    } else {
        1
    };
    Baseline {
        width: v["video"]["width"].as_u64().unwrap() as u32,
        height: v["video"]["height"].as_u64().unwrap() as u32,
        fps_num: num,
        fps_den: den,
        audio_rate: v["audio"]["sample_rate"].as_u64().map(|x| x as u32),
        audio_channels: v["audio"]["channels"].as_u64().map(|x| x as u32),
        keyframes: v["keyframe_count"].as_u64().unwrap() as usize,
        // The OVE-comparable verdict is the EXACT integer-timebase analysis
        // (OVE's VfrReport computes deltas in exact rationals; nominal-CFR
        // media in a 1/16000 container carries ±1-tick jitter that IS
        // distinct at container level). The jitter-tolerant verdict is the
        // human "nominal cadence" truth — recorded alongside, never mixed.
        is_vfr: v["vfr"]["is_vfr_exact"].as_bool().unwrap_or(false),
        is_vfr_jitter_tolerant: v["vfr"]["is_vfr"].as_bool().unwrap_or(false),
        vfr_exact_deltas: v["vfr"]["distinct_delta_count_exact"].as_u64().unwrap_or(0),
        rotation: v["display_rotation_deg"].as_f64(),
        sha256: v["sha256"].as_str().unwrap().to_string(),
    }
}

fn tmp(name: &str) -> PathBuf {
    let d = std::env::temp_dir().join("ove-corpus").join(name);
    let _ = std::fs::remove_dir_all(&d);
    std::fs::create_dir_all(d.parent().unwrap()).unwrap();
    d
}

fn sec(sec: i64) -> Rational {
    Rational::new(sec * AXIS, AXIS)
}

/// One recorded step: Ok(payload) or a TYPED engine error (stringified).
/// Nothing in this file may panic on an engine error path — a typed error is
/// evidence, recorded verbatim into the machine record.
fn record_step<T>(rec: &mut Value, key: &str, r: Result<T, ove_engine::EngineError>) -> Option<T> {
    match r {
        Ok(v) => {
            rec[key] = json!({ "outcome": "ok" });
            Some(v)
        }
        Err(e) => {
            rec[key] = json!({ "outcome": "typed_error", "error": e.to_string() });
            None
        }
    }
}

/// The shared corpus scenario. Returns the machine record.
/// Ops: import → probe-vs-baseline → clip1 [0,2) src t0 → split @1.0 →
/// resize right 1.25 → clip2 [2.25, span) src t0+4 → spot renders ×2 +
/// determinism → export n_frames → WAV (if audio) → save/reopen (hash +
/// bindings) → re-export byte-identical.
fn corpus_scenario(
    id: &str,
    media: &Path,
    baseline: &Baseline,
    out_rate_num: i64,
    out_rate_den: i64,
    n_frames: i64,
    src_t0_sec: i64,
) -> Value {
    let out_dir = std::env::var("OVE_CORPUS_OUT")
        .map(PathBuf::from)
        .unwrap_or_else(|_| tmp("artifacts"));
    std::fs::create_dir_all(&out_dir).unwrap();
    let mut rec = json!({
        "corpus_id": id,
        "media": media.display().to_string(),
        "media_sha256": baseline.sha256,
        "commit": option_env!("OVE_COMMIT").unwrap_or("unspecified"),
        "out_rate": format!("{}/{}", out_rate_num, out_rate_den),
        "out_frames": n_frames,
    });

    // -- import ---------------------------------------------------------------
    let dir = tmp(&format!("project-{}", id));
    let mut e = Engine::create(&dir, (AXIS, 1)).expect("create corpus project");
    e.add_track(
        1,
        ove_timeline::TrackKind::Gap(ove_timeline::GapTrack::new()),
    )
    .expect("track 1");
    let import = e.import_media(media);
    let hex = match import {
        Ok(h) => {
            rec["import"] = json!({ "outcome": "ok", "asset_blake3": h });
            h
        }
        Err(e) => {
            // A typed import rejection on difficult-but-benign media is a
            // first-class finding: record it and stop the scenario here.
            rec["import"] = json!({ "outcome": "typed_error", "error": e.to_string() });
            std::fs::write(
                out_dir.join(format!("corpus_record_{}.json", id)),
                serde_json::to_string_pretty(&rec).unwrap(),
            )
            .unwrap();
            return rec;
        }
    };
    let media_info = e.source(&hex).expect("source registered").clone();

    // -- probe vs INDEPENDENT baseline (identity of OVE probe and ffprobe) -----
    let vstream = media_info
        .probe
        .streams
        .iter()
        .find(|s| s.kind == ove_media::StreamKind::Video)
        .expect("video stream probed")
        .clone();
    let vd = vstream.video.clone().expect("video details");
    assert_eq!(vd.width, baseline.width, "probe width == ffprobe width");
    assert_eq!(vd.height, baseline.height, "probe height == ffprobe height");
    let kf_index = media_info
        .probe
        .keyframe_index
        .as_ref()
        .expect("keyframe index");
    assert_eq!(
        kf_index.entries.len(),
        baseline.keyframes,
        "probe keyframe count == ffprobe keyframe count"
    );
    // D-12: nothing assumes CFR. VFR truth must agree with the ffprobe
    // frame-delta analysis.
    let probe_vfr = media_info.probe.vfr.as_ref().expect("VFR report built");
    // VFR verdict alignment — RLW-8 finding **RLW-8-F1 FIXED** in the F1/F3
    // fix wave: the detector now sorts packet-order pts into PRESENTATION
    // order before the delta pass (VFR is a property of presentation
    // timing), so B-frame CFR media no longer false-positives and the probe
    // verdict must EQUAL the ffprobe exact verdict in BOTH directions on
    // every corpus item (pre-fix: all 5 B-frame CFR items reported
    // is_vfr=true with reordering-shaped deltas).
    assert_eq!(
        probe_vfr.is_vfr, baseline.is_vfr,
        "probe VFR verdict must equal the ffprobe frame-order verdict (F1 fixed)"
    );
    rec["probe"] = json!({
        "width": vd.width, "height": vd.height,
        "keyframes": kf_index.entries.len(),
        "vfr_is_vfr_exact": probe_vfr.is_vfr,
        "vfr_distinct_deltas": probe_vfr.distinct_deltas.len(),
        "vfr_probe_delta_sample": probe_vfr
            .distinct_deltas
            .iter()
            .take(8)
            .map(|d| format!("{}/{}", d.num(), d.den()))
            .collect::<Vec<_>>(),
        "vfr_is_vfr_nominal_jitter_tolerant": baseline.is_vfr_jitter_tolerant,
        "vfr_is_vfr_exact_ffprobe": baseline.is_vfr,
        "vfr_exact_deltas_ffprobe": baseline.vfr_exact_deltas,
        // v1 probe schema has NO rotation field: display-matrix metadata is
        // invisible at the probe surface (recorded, not normalized away).
        "engine_probe_rotation_field": "ABSENT (v1 probe schema; source display_rotation = \
                                        the baseline's display_rotation_deg)",
        "source_display_rotation_deg": baseline.rotation,
    });

    // -- audio geometry (U-4 audio-geometry question, per item) ---------------
    let astream = media_info
        .probe
        .streams
        .iter()
        .find(|s| s.kind == ove_media::StreamKind::Audio)
        .map(|s| s.audio.clone().expect("audio details"));
    rec["audio_geometry"] = match (&astream, baseline.audio_channels) {
        (Some(ad), Some(ch)) => {
            assert_eq!(
                ad.channels, ch,
                "probe audio channels == INDEPENDENT baseline (mono/5.1/stereo truth)"
            );
            json!({ "channels": ad.channels, "sample_rate": ad.sample_rate })
        }
        (None, None) => json!({ "stream": "absent (silent media)" }),
        (Some(ad), None) => json!({ "anomaly": "engine sees audio where ffprobe sees none",
                                    "engine_channels": ad.channels }),
        (None, Some(ch)) => json!({ "anomaly": "ffprobe sees audio where engine sees none",
                                    "baseline_channels": ch }),
    };
    let has_audio = astream.is_some();

    // -- timeline edits on THIS media ------------------------------------------
    // Output cadence must equal the source's nominal cadence on CFR media
    // (CFR 1:1, no retime); genuinely VFR media picks the declared r_frame_rate.
    if !baseline.is_vfr_jitter_tolerant {
        assert_eq!(
            (out_rate_num, out_rate_den),
            (baseline.fps_num, baseline.fps_den),
            "output cadence == source nominal cadence (CFR 1:1, no retime)"
        );
    }
    let clip1 = e.add_clip(1, &hex, sec(2), sec(src_t0_sec)).expect("clip1");
    let h_after_clip1 = e.state_hash();
    let clip1b = e.split(1, clip1, sec(1)).expect("split clip1 at 1.0s");
    let h_after_split = e.state_hash();
    e.resize(1, clip1b, Rational::new(5 * AXIS / 4, AXIS))
        .expect("resize clip1b to 1.25s");
    let h_after_resize = e.state_hash();
    let span_ticks = n_frames * Rational::new(out_rate_den * AXIS, out_rate_num).num()
        / Rational::new(out_rate_den * AXIS, out_rate_num).den();
    let clip2_start = Rational::new(9 * AXIS / 4, AXIS); // 2.25 s
                                                         // span_ticks are PROJECT-axis ticks (AXIS per second) — NOT seconds.
    let clip2_dur = Rational::new(span_ticks, AXIS) - clip2_start;
    let _clip2 = e
        .add_clip(1, &hex, clip2_dur, sec(src_t0_sec + 4))
        .expect("clip2 (ends exactly at span end)");
    let h_final = e.state_hash();
    assert_ne!(h_final, h_after_clip1, "state hash tracks edits");
    rec["edits"] = json!({ "clips": 3, "split": "1.0s", "resize_right": "1.25s",
                           "span_ticks": span_ticks });

    // -- undo/redo over the real edit graph ------------------------------------
    // DOCUMENTED CONTRACT (E-012/ADR-016 §5 + RLW-2 finding): the state hash
    // COVERS id-allocation state (next_id + used_ids are document state), so
    // hash-exact undo holds ONLY for id-neutral inverses (Resize); undoing
    // any allocation-consuming command (Insert, Split) restores structure
    // but not the pre-command hash. Redo identity is exact end-to-end.
    // The corpus harness pins each truth at its own granularity.
    //
    // (a) allocation-consuming dive on the CLEAN stack (split/resize/clip2):
    //     structure restored at every step, hash divergence recorded (the
    //     DOCUMENTED id-state behavior, not a regression), redo identity
    //     exact end-to-end.
    assert!(e.undo().expect("undo clip2"), "undo clip2");
    let h_u1 = e.state_hash();
    assert!(e.undo().expect("undo resize"), "undo resize");
    let h_u2 = e.state_hash();
    assert!(e.undo().expect("undo split"), "undo split");
    let h_u3 = e.state_hash();
    {
        let track = e.project().timeline().track_ref(1).unwrap();
        let mut placements = 0usize;
        track.walk(&mut |_pos, _start, _clip: &ove_timeline::Clip| {
            placements += 1;
        });
        assert_eq!(
            placements, 1,
            "undo through split restores structure (1 placement)"
        );
    }
    rec["undo_alloc_consuming"] = json!({
        "hashes": { "undo_clip2": h_u1, "undo_resize": h_u2, "undo_split": h_u3,
                    "after_split": h_after_split, "after_clip1": h_after_clip1 },
        "equals_pre_command_hash": {
            "undo_clip2": h_u1 == h_after_resize,
            "undo_split": h_u3 == h_after_clip1
        },
        "contract": "E-012/ADR-016 §5: id-allocation state is document state — \
                     hash divergence on allocation-consuming undo is the \
                     documented behavior (RLW-2 finding carries for corpus media)"
    });
    assert!(e.redo().expect("redo split"), "redo split");
    assert!(e.redo().expect("redo resize"), "redo resize");
    assert!(e.redo().expect("redo clip2"), "redo clip2");
    assert_eq!(
        e.state_hash(),
        h_final,
        "redo identity exact across the split inverse"
    );

    // (b) id-neutral window (Resize 1.25s -> 1.5s -> back): hash-exact both ways
    e.resize(1, clip1b, Rational::new(3 * AXIS / 2, AXIS))
        .expect("resize clip1b to 1.5s (undo window probe)");
    let h_resize15 = e.state_hash();
    assert!(e.undo().expect("undo id-neutral resize"), "undo executed");
    assert_eq!(
        e.state_hash(),
        h_final,
        "undo(id-neutral resize) is hash-exact"
    );
    assert!(e.redo().expect("redo id-neutral resize"), "redo executed");
    assert_eq!(
        e.state_hash(),
        h_resize15,
        "redo(id-neutral resize) is hash-exact"
    );
    e.resize(1, clip1b, Rational::new(5 * AXIS / 4, AXIS))
        .expect("restore the certified 1.25s shape");
    assert_eq!(e.state_hash(), h_final, "shape restored hash-exactly");
    rec["undo_redo"] = json!({ "outcome": "ok", "windows": ["alloc-consuming dive (structure + redo-exact)", "id-neutral resize (hash-exact both ways)"] });

    // -- spot renders + determinism ---------------------------------------------
    let output = OutputSpec {
        width: baseline.width,
        height: baseline.height,
        rate_num: out_rate_num,
        rate_den: out_rate_den,
        working_space: bt709(),
    };
    let span = Rational::new(n_frames * out_rate_den, out_rate_num);
    let t_mid = Rational::new(span.num() / 2, span.den());
    let r0 = record_step(
        &mut rec,
        "render_t0",
        e.render_frame(&output, Rational::new(0, 1)),
    );
    let rm = record_step(&mut rec, "render_mid", e.render_frame(&output, t_mid));
    // RLW-8 finding **RLW-8-F3 FIXED** in the F1/F3 fix wave: genuinely VFR
    // media (container-declared zero/no durations) used to typed-fail every
    // render/export with SourceFrameMissing at verifiably-existing frames
    // (decoder typed the frames Corrupt("frame without duration")). The fix
    // delivers a container-declared absence as duration 0/1 — corruption
    // typing stays for NEGATIVE durations only. The fix is load-bearing
    // here: the VFR item must now render AND export (no normalization — the
    // pre-fix typed-failure evidence is preserved in the 10-01/10-02
    // records and in docs/REALWORLD_VALIDATION.md §10).
    if baseline.is_vfr_jitter_tolerant {
        assert!(
            r0.is_some(),
            "VFR source must render at t0 (RLW-8-F3 fixed — reorder-safe zero-duration decode)"
        );
        assert!(
            rm.is_some(),
            "VFR source must render mid-span (RLW-8-F3 fixed)"
        );
    }
    if let (Some(f0), Some(fm)) = (&r0, &rm) {
        for (name, f) in [("t0", f0), ("mid", fm)] {
            assert_eq!(f.pixel_format, PixelFormat::Rgba, "{name} is RGBA");
            assert_eq!(
                (f.width, f.height),
                (baseline.width, baseline.height),
                "{name} geometry == probe geometry"
            );
            assert!(
                f.cpu_bytes().expect("payload").data.iter().any(|&b| b != 0),
                "{name} is not black"
            );
        }
        let fm2 = e
            .render_frame(&output, t_mid)
            .expect("repeat render for determinism");
        assert_eq!(
            blake3_frame(fm),
            blake3_frame(&fm2),
            "spot render is deterministic"
        );
    }

    // -- export (A/V re-encode at the corpus cadence) ---------------------------
    reset_decoder_opens();
    let out_av = out_dir.join(format!("corpus_{}_av.mp4", id));
    let export = record_step(
        &mut rec,
        "export_reencode",
        e.export_reencode(&out_av, &output, n_frames),
    );
    let export_opens = decoder_opens();
    let export_sha = export.as_ref().map(|i| i.file_sha256.clone());
    if baseline.is_vfr_jitter_tolerant {
        assert!(
            export.is_some(),
            "VFR source must export (RLW-8-F3 fixed — the VFR class upgrades from \
             render-blocked to certified; pre-fix typed-failure evidence preserved)"
        );
    }
    if let Some(info) = &export {
        assert_eq!(
            info.tracks[0].nb_frames,
            Some(n_frames as u64),
            "export frame count exact (VFR sources included — output cadence is exact)"
        );
        rec["export"] = json!({
            "sha256": info.file_sha256,
            "size": info.file_size,
            "video_frames": info.tracks[0].nb_frames,
            "decoder_opens": export_opens,
        });
    } else {
        rec["export"] = json!({ "decoder_opens": export_opens });
    }

    // -- WAV (only where the source carries audio; silent media is expected to
    //    be typed-rejected — that IS the v1 contract row) -----------------------
    if has_audio {
        let out_wav = out_dir.join(format!("corpus_{}.wav", id));
        match e.export_wav(&out_wav) {
            Ok(samples) => {
                let expect = (span * Rational::new(baseline.audio_rate.unwrap() as i64, 1)).num();
                rec["wav"] = json!({ "outcome": "ok", "samples": samples, "expected": expect });
                assert_eq!(samples, expect as u64, "WAV samples == span × source rate");
            }
            Err(err) => {
                rec["wav"] = json!({ "outcome": "typed_error", "error": err.to_string() });
            }
        }
    } else {
        let out_wav = out_dir.join(format!("corpus_{}.wav", id));
        rec["wav_silent_media"] = match e.export_wav(&out_wav) {
            Ok(n) => json!({ "outcome": "ok", "samples": n }),
            Err(err) => json!({ "outcome": "typed_error", "error": err.to_string() }),
        };
    }

    // -- persistence: reopen → same hash → bindings alive → re-export identical
    let live_hash = e.state_hash();
    let uuid = e.uuid().to_string();
    let t_mid_bytes = rm.as_ref().map(|f| f.cpu_bytes().unwrap().data.clone());
    drop(e);
    let mut r = Engine::open(&dir).expect("reopen corpus project");
    assert_eq!(r.uuid(), uuid, "project identity survives reopen");
    assert_eq!(r.state_hash(), live_hash, "reopen hash == live hash");
    assert_eq!(
        r.project().clip_assets().len(),
        3,
        "RLW-7 binding invariant on corpus media: clip1 + split right half + clip2 all bound (3/3)"
    );
    if let Some(bytes) = t_mid_bytes {
        let re = r.render_frame(&output, t_mid).expect("render after reopen");
        assert_eq!(
            blake3_frame(&re),
            ContentHash::from_bytes(&bytes).hex(),
            "reopen re-render byte-identical"
        );
    }
    if export_sha.is_some() {
        let out_av2 = out_dir.join(format!("corpus_{}_av_reopen.mp4", id));
        let info2 = r
            .export_reencode(&out_av2, &output, n_frames)
            .expect("re-export after reopen");
        assert_eq!(
            export_sha.as_deref().unwrap(),
            info2.file_sha256,
            "re-export byte-identical (deterministic software pipeline on corpus media)"
        );
        rec["reopen_export_sha256"] = json!(info2.file_sha256);
    }

    rec["scenario"] = json!({ "outcome": "complete" });
    std::fs::write(
        out_dir.join(format!("corpus_record_{}.json", id)),
        serde_json::to_string_pretty(&rec).unwrap(),
    )
    .unwrap();
    rec
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

fn run_scen(
    env_media: &str,
    id: &str,
    out_rate_num: i64,
    out_rate_den: i64,
    n_frames: i64,
    src_t0_sec: i64,
) {
    let Some(src) = std::env::var(env_media).ok() else {
        eprintln!("SKIP: {} not set (corpus media stays out of CI)", env_media);
        return;
    };
    let base_path =
        std::env::var(format!("{}_BASELINE", env_media)).expect("baseline must accompany media");
    let baseline = load_baseline(Path::new(&base_path));
    corpus_scenario(
        id,
        Path::new(&src),
        &baseline,
        out_rate_num,
        out_rate_den,
        n_frames,
        src_t0_sec,
    );
}

#[test]
fn corpus_portrait_true() {
    run_scen("OVE_CORPUS_PORTRAIT", "portrait_true", 30, 1, 96, 10);
}

#[test]
fn corpus_rotation_metadata() {
    run_scen(
        "OVE_CORPUS_ROTATION",
        "rotation_metadata",
        24_000,
        1_001,
        80,
        10,
    );
}

#[test]
fn corpus_vfr_constructed() {
    run_scen("OVE_CORPUS_VFR", "vfr_constructed", 30, 1, 96, 1);
}

#[test]
fn corpus_long_gop() {
    run_scen("OVE_CORPUS_LONGGOP", "long_gop", 24_000, 1_001, 80, 30);
}

#[test]
fn corpus_audio_variants() {
    let Some(dir) = std::env::var("OVE_CORPUS_AUDIO_DIR").ok() else {
        eprintln!("SKIP: OVE_CORPUS_AUDIO_DIR not set (corpus media stays out of CI)");
        return;
    };
    let dir = PathBuf::from(&dir);
    for (stem, label) in [
        ("audio_48k", "48 kHz stereo"),
        ("audio_mono", "48 kHz mono"),
        ("audio_51", "48 kHz 5.1"),
    ] {
        eprintln!("corpus audio variant: {} ({})", stem, label);
        let baseline = load_baseline(&dir.join(format!("{}.baseline.json", stem)));
        corpus_scenario(
            stem,
            &dir.join(format!("{}.mp4", stem)),
            &baseline,
            24_000,
            1_001,
            80,
            10,
        );
    }
}
