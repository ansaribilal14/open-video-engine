//! WAVE 5 integration: MEDIA+TIMELINE+PROJECT+RENDER+EXPORT cooperating
//! (ENGINE_BUILD_PLAN wave 5 acceptance, via the engine session).
//!
//! The directive's gate, exercised end-to-end on real committed media:
//!   1. save → kill -9 at a random point → reopen → replay → hash == live
//!   2. exported MP4 duration-exact, ffprobe-clean timestamps
//!   3. decode → render → export chain reproducible in CI (software legs)

use std::path::{Path, PathBuf};

use ove_engine::Engine;
use ove_media::PixelFormat;
use ove_render::OutputSpec;
use ove_time::Rational;
use ove_timeline::GapTrack;

fn media(name: &str) -> PathBuf {
    // the committed wave-3 corpus doubles as the engine's fixture set
    // (single repo, stable relative path; existence is the contract)
    let p = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../ove-encode/tests/media")
        .join(name);
    assert!(p.exists(), "corpus fixture missing: {}", p.display());
    p
}

fn tmp(name: &str) -> PathBuf {
    let d = std::env::temp_dir()
        .join("ove-engine-integration")
        .join(name);
    let _ = std::fs::remove_dir_all(&d);
    std::fs::create_dir_all(d.parent().unwrap()).unwrap();
    d
}

fn ticks(n: i64) -> Rational {
    Rational::new(n, 24_000)
}

fn output_spec(w: u32, h: u32) -> OutputSpec {
    OutputSpec {
        width: w,
        height: h,
        rate_num: 24,
        rate_den: 1,
        working_space: color_tags_bt709(),
    }
}

fn color_tags_bt709() -> ove_media::ColorTags {
    ove_media::ColorTags {
        primaries: ove_media::Primaries::Bt709,
        transfer: ove_media::Transfer::Bt709,
        matrix: ove_media::MatrixCoeffs::Bt709,
        range: ove_media::Range::Limited,
        chroma_loc: None,
    }
}

/// Build a session: import copy24 (6 s @ 24 fps), two clips on track 1
/// (0-2 s and 1-3 s of source), split + resize + undo. Returns the engine
/// and the live state hash.
fn build_session(dir: &Path) -> (Engine, String) {
    let mut e = Engine::create(dir, (48_000, 1)).expect("create");
    e.add_track(1, ove_timeline::TrackKind::Gap(GapTrack::new()))
        .expect("track");
    let hex = e.import_media(&media("copy24.mp4")).expect("import");

    // clip A: source [0, 2 s) at timeline 0; clip B: source [1, 3 s) after it
    e.add_clip(1, &hex, ticks(48), ticks(0)).expect("clip A");
    e.add_clip(1, &hex, ticks(48), ticks(24)).expect("clip B");
    // split clip A at 1 s
    e.split(1, 1, ticks(24)).expect("split");
    // resize clip B
    e.resize(1, 2, ticks(40)).expect("resize");
    // one undo (drops the resize)
    e.undo().expect("undo");

    let hash = e.state_hash();
    (e, hash)
}

// ---------------------------------------------------------------------------
// 1: cooperating chain — import → timeline → save → reopen → hash equal
// ---------------------------------------------------------------------------

#[test]
fn w5_save_reopen_hash_equal() {
    let dir = tmp("w5-p1");
    let (_e, live) = build_session(&dir);
    let r = Engine::open(&dir).expect("reopen");
    assert_eq!(r.state_hash(), live, "engine session reopen: hash equal");
}

// ---------------------------------------------------------------------------
// 2: kill -9 (drop-without-close model; the subprocess drill lives at the
//    project layer) at a "random" point + session continuation
// ---------------------------------------------------------------------------

#[test]
fn w5_kill_reopen_continue() {
    for cut in [1usize, 3, 4] {
        let dir = tmp(&format!("w5-p2-{cut}"));
        let mut e = Engine::create(&dir, (48_000, 1)).expect("create");
        e.add_track(1, ove_timeline::TrackKind::Gap(GapTrack::new()))
            .expect("track");
        let hex = e.import_media(&media("copy24.mp4")).expect("import");
        for k in 0..cut {
            e.add_clip(1, &hex, ticks(24), ticks(12 * k as i64))
                .expect("clip");
        }
        let live = e.state_hash();
        drop(e); // the "kill" (write-through: disk state == live state)

        let mut r = Engine::open(&dir).expect("reopen");
        assert_eq!(r.state_hash(), live, "cut={cut}");
        // the session continues deterministically
        r.add_clip(1, &hex, ticks(6), ticks(0))
            .expect("post-reopen clip");
        let after = r.state_hash();
        drop(r);
        let r2 = Engine::open(&dir).expect("second reopen");
        assert_eq!(r2.state_hash(), after, "cut={cut}: post-continuation");
    }
}

// ---------------------------------------------------------------------------
// 3: decode → render (real decode through the seam) → rendered frame facts
// ---------------------------------------------------------------------------

#[test]
fn w5_render_frame_from_real_decode() {
    let dir = tmp("w5-render");
    let (mut e, _hash) = build_session(&dir);
    let output = output_spec(320, 240);

    // frame 0: clip A covers timeline 0; source pts 0 (a keyframe in copy24)
    let frame = e
        .render_frame(&output, Rational::new(0, 1))
        .expect("render frame 0");
    assert_eq!(frame.width, 320);
    assert_eq!(frame.height, 240);
    assert_eq!(frame.pixel_format, PixelFormat::Rgba);
    assert_eq!(frame.pts, Rational::new(0, 1));
    // not a black frame: the corpus testsrc2 pattern has non-zero chroma/luma
    let fb = frame.cpu_bytes().expect("cpu rgba");
    assert!(
        fb.data
            .chunks(4)
            .any(|px| px[0] > 8 || px[1] > 8 || px[2] > 8),
        "rendered frame carries real content"
    );

    // frame 96 = t 4 s: inside clip B (timeline [2 s, 2 s + 40/24 s))
    // source pts = 1 s + (4 s - 2 s) = 3 s — inside copy24's 6 s
    let frame = e
        .render_frame(&output, Rational::new(4, 1))
        .expect("render frame at t=4s");
    assert_eq!(frame.pts, Rational::new(4, 1), "output pts == frame_pts(k)");

    // determinism: the same time renders byte-identical frames
    let a = e
        .render_frame(&output, Rational::new(12, 1))
        .expect("render a");
    let b = e
        .render_frame(&output, Rational::new(12, 1))
        .expect("render b");
    let ab = a.cpu_bytes().expect("a").data.clone();
    let bb = b.cpu_bytes().expect("b").data.clone();
    assert_eq!(ab, bb, "render determinism (same session)");
}

// ---------------------------------------------------------------------------
// 4: decode → render → export re-encode → ffprobe-clean MP4, duration exact
// ---------------------------------------------------------------------------

#[test]
fn w5_export_reencode_duration_exact() {
    let dir = tmp("w5-export");
    let (mut e, _hash) = build_session(&dir);
    let output = output_spec(320, 240);

    // total timeline: clip A (48) + split tail (24) + clip B resized (40)
    // = 112 ticks = 112/24000 s... but the RENDER span is clip coverage at
    // 24 fps output: frames 0..111 land inside clips; export exactly the
    // clip-covered span: 112 ticks = 4.667 s → 112 frames at 1/24? No:
    // ticks(48) = 2 s, ticks(24) = 1 s, ticks(40) = 1.6667 s → total 4.6667 s
    // → 112 frames at 24 fps exactly.
    const N: i64 = 112;
    let out = dir.join("export.mp4");
    let info = e
        .export_reencode(&out, &output, N)
        .expect("export re-encode");
    assert_eq!(info.tracks.len(), 1);
    assert_eq!(
        info.tracks[0].nb_frames,
        Some(N as u64),
        "frame count exact"
    );
    assert_eq!(
        info.tracks[0].duration,
        Some(Rational::new(N * output.rate_den, output.rate_num)),
        "duration exact (± 0 frames)"
    );

    // reopen eye: libav probe of the produced file
    let asset = ove_media::AssetRef::from_path(&out).expect("asset");
    use ove_media::ProbeBackend;
    let probed = ove_decode::ffmpeg::FfmpegProbe
        .probe(&asset)
        .expect("probe output");
    assert_eq!(probed.container, ove_media::ContainerKind::Mp4);
    let v = &probed.streams[0];
    assert_eq!(v.codec, "mpeg4");
    assert_eq!(v.nb_frames_hint, Some(N as u64));
    assert_eq!(v.duration, Some(Rational::new(N, 24)));
}

// ---------------------------------------------------------------------------
// 5: stream-copy export through the engine planner path
// ---------------------------------------------------------------------------

#[test]
fn w5_export_copy_keyframe_aligned() {
    let dir = tmp("w5-copy");
    let mut e = Engine::create(&dir, (48_000, 1)).expect("create");
    e.add_track(1, ove_timeline::TrackKind::Gap(GapTrack::new()))
        .expect("track");
    let hex = e.import_media(&media("copy24.mp4")).expect("import");

    // copy [1 s, 3 s) — exact keyframe boundaries on this corpus
    let out = dir.join("copy.mp4");
    let (info, snaps) = e
        .export_copy(&hex, Rational::new(1, 1), Rational::new(3, 1), &out)
        .expect("export copy");
    assert!(snaps.is_empty(), "aligned cut: no snaps");
    assert_eq!(info.tracks[0].nb_frames, Some(48));
    assert_eq!(info.tracks[0].duration, Some(Rational::new(2, 1)));
}

// ---------------------------------------------------------------------------
// 6: undo/redo through the engine session survives a reopen
// ---------------------------------------------------------------------------

#[test]
fn w5_undo_redo_reopen() {
    let dir = tmp("w5-undo");
    let (mut e, before_undo) = build_session(&dir);
    e.undo().expect("undo");
    let after_undo = e.state_hash();
    assert_ne!(before_undo, after_undo);
    e.redo().expect("redo");
    let after_redo = e.state_hash();

    drop(e);
    let r = Engine::open(&dir).expect("reopen");
    assert_eq!(r.state_hash(), after_redo, "redo marker replayed");
    assert!(
        r.project().undo_depth() >= 1,
        "undo history rebuilt by replay"
    );
}

// ---------------------------------------------------------------------------
// W6 milestone: real video → probe → registry → hash → commands → mapping →
// decode → FrameEnvelope → RenderPlan → software render → encode/mux →
// valid MP4 → ffprobe verify → save → kill → reopen → SAME hash → re-export
// semantically identical. TWO sources (multi-rate: 24 fps + ntsc 29.97).
// ---------------------------------------------------------------------------

fn w6_copy_fixture(name: &str) -> PathBuf {
    media(name)
}

// ---------------------------------------------------------------------------
// WAVE 8: keyframe animation — exact evaluation into the render path
// (ADR-019): opacity/x keys evaluated at the frame's local time, carried
// into the compiled plan, hash-stable, reopen-stable, render-deterministic.
// ---------------------------------------------------------------------------

#[test]
fn w8_keyframes_exact_eval_and_stability() {
    let dir = tmp("w8-keyframes");
    let mut e = Engine::create(&dir, (24_000, 1)).expect("create");
    e.add_track(1, ove_timeline::TrackKind::Gap(GapTrack::new()))
        .expect("track");
    let hex = e.import_media(&media("copy24.mp4")).expect("import");
    // clip: 2 s at timeline 0 (48_000 ticks @ 24 kHz); 24 fps → 48 frames
    e.add_clip(1, &hex, ticks(48_000), ticks(0)).expect("clip");

    let out = output_spec(64, 64);

    // animate: opacity 1 → 0 linear over the clip; x 0 → 100 linear
    e.execute(ove_timeline::Command::SetKeyframes {
        track: 1,
        id: 1,
        property: ove_timeline::PropertyName::Opacity,
        keys: vec![
            ove_timeline::Keyframe::new(
                ticks(0),
                Rational::new(1, 1),
                ove_timeline::Interpolation::Linear,
            ),
            ove_timeline::Keyframe::new(
                ticks(48_000),
                Rational::new(0, 1),
                ove_timeline::Interpolation::Linear,
            ),
        ],
    })
    .expect("opacity keys");
    e.execute(ove_timeline::Command::SetKeyframes {
        track: 1,
        id: 1,
        property: ove_timeline::PropertyName::X,
        keys: vec![
            ove_timeline::Keyframe::new(
                ticks(0),
                Rational::new(0, 1),
                ove_timeline::Interpolation::Linear,
            ),
            ove_timeline::Keyframe::new(
                ticks(48_000),
                Rational::new(100, 1),
                ove_timeline::Interpolation::Linear,
            ),
        ],
    })
    .expect("x keys");
    let animated_hash = e.state_hash();

    // exact evaluation at frame pts: frame k covers t = k/24 s (frame 24
    // == 1 s == 24_000 ticks). local == t (clip starts at 0).
    let input = e
        .build_render_input(&out, Rational::new(1, 1))
        .expect("input at 1s");
    let p = &input.tracks[0].placements[0];
    assert_eq!(p.alpha, Rational::new(1, 2), "alpha at 1 s is exactly 1/2");
    assert_eq!(p.offset, (50, 0), "x at 1 s is exactly 50");
    // the compiled plan carries the evaluated values verbatim
    let plan = ove_render::compile_frame(&input, 24).expect("plan");
    let alpha_pass = plan
        .passes
        .iter()
        .find(|pass| matches!(pass.kind, ove_render::PassKind::BlendOver { .. }))
        .expect("blend pass");
    match alpha_pass.kind {
        ove_render::PassKind::BlendOver { alpha } => assert_eq!(alpha, Rational::new(1, 2)),
        _ => unreachable!(),
    }
    let tx_pass = plan
        .passes
        .iter()
        .find(|pass| matches!(pass.kind, ove_render::PassKind::Transform { .. }))
        .expect("transform pass");
    match tx_pass.kind {
        ove_render::PassKind::Transform { dx, dy } => {
            assert_eq!((dx, dy), (50, 0));
        }
        _ => unreachable!(),
    }

    // a half-second frame (local 12_000 ticks): alpha 3/4, x 25
    let input_q = e
        .build_render_input(&out, Rational::new(1, 2))
        .expect("input at 0.5s");
    let pq = &input_q.tracks[0].placements[0];
    assert_eq!(pq.alpha, Rational::new(3, 4));
    assert_eq!(pq.offset, (25, 0));

    // undo/redo roundtrip is hash-exact (two sets = two undo steps)
    e.undo().expect("undo x");
    e.undo().expect("undo opacity");
    let static_hash = e.state_hash();
    {
        let input_s = e
            .build_render_input(&out, Rational::new(1, 2))
            .expect("static input");
        let ps = &input_s.tracks[0].placements[0];
        assert_eq!(ps.alpha, Rational::new(1, 1), "static fallback after undo");
        assert_eq!(ps.offset, (0, 0));
    }
    e.redo().expect("redo opacity");
    e.redo().expect("redo x");
    assert_eq!(
        e.state_hash(),
        animated_hash,
        "redo restores the animated state"
    );

    // reopen: the animated state survives persistence
    drop(e);
    let mut r = Engine::open(&dir).expect("reopen");
    assert_eq!(
        r.state_hash(),
        animated_hash,
        "animated state survives reopen"
    );

    // render determinism WITH animation: same frame twice → byte-identical
    let f1 = r.render_frame(&out, Rational::new(1, 1)).expect("render 1");
    let f2 = r.render_frame(&out, Rational::new(1, 1)).expect("render 2");
    assert_eq!(
        f1.cpu_bytes().expect("payload").data,
        f2.cpu_bytes().expect("payload").data,
        "animated render must be deterministic"
    );
    assert_eq!(f1.pts, f2.pts);
    let _ = static_hash;
}

#[test]
fn w6_vertical_slice_milestone() {
    let dir = tmp("w6-milestone");
    let mut e = Engine::create(&dir, (24_000, 1)).expect("create");
    e.add_track(1, ove_timeline::TrackKind::Gap(GapTrack::new()))
        .expect("track 1");
    e.add_track(2, ove_timeline::TrackKind::Gap(GapTrack::new()))
        .expect("track 2");

    // import two multi-rate sources
    let h24 = e
        .import_media(&w6_copy_fixture("copy24.mp4"))
        .expect("import 24");
    let hntsc = e
        .import_media(&w6_copy_fixture("copyntsc.mp4"))
        .expect("import ntsc");
    assert_ne!(h24, hntsc, "content hashes are identities");

    // bindings ride the log: track 1 = copy24 [0,2s); track 2 = ntsc [2,3.5s)
    // (composited OVER track 1 by layer order), then copy24 [3,5s) again.
    // Timeline (24000 tick axis):
    //   t [0, 2s):       clip 1 (copy24 src [0,2s))      — track 1
    //   t [2s, 3.5s):    clip 2 (ntsc   src [0,1.5s))    — track 2 (top)
    //   t [3.5s, 5s):    clip 3 (copy24 src [3,4.5s))    — track 1
    e.add_clip(1, &h24, Rational::new(48_000, 24_000), Rational::new(0, 1))
        .expect("clip 1");
    e.add_clip(
        2,
        &hntsc,
        Rational::new(36_000, 24_000),
        Rational::new(0, 1),
    )
    .expect("clip 2");
    e.add_clip(
        1,
        &h24,
        Rational::new(36_000, 24_000),
        Rational::new(72_000, 24_000),
    )
    .expect("clip 3");

    // render from EACH source (multi-source proof):
    let output = output_spec(320, 240);
    let f0 = e
        .render_frame(&output, Rational::new(0, 1))
        .expect("render t=0 (copy24)");
    let f_mid = e
        .render_frame(&output, Rational::new(60_000, 24_000)) // t=2.5 s → ntsc
        .expect("render t=2.5s (ntsc)");
    let f_tail = e
        .render_frame(&output, Rational::new(96_000, 24_000)) // t=4 s → copy24
        .expect("render t=4s (copy24)");
    for f in [&f0, &f_mid, &f_tail] {
        assert_eq!(f.pixel_format, PixelFormat::Rgba);
        assert!(f.cpu_bytes().expect("payload").data.iter().any(|&b| b != 0));
    }

    // export the whole 5 s span: 120 frames @ 24 fps
    let out = dir.join("milestone.mp4");
    const N: i64 = 120;
    let info = e
        .export_reencode(&out, &output, N)
        .expect("export milestone");
    assert_eq!(info.tracks[0].nb_frames, Some(N as u64));
    assert_eq!(info.tracks[0].duration, Some(Rational::new(5, 1)));
    // save → kill → reopen → SAME state hash
    let live = e.state_hash();
    drop(e);
    let mut r = Engine::open(&dir).expect("reopen");
    assert_eq!(r.state_hash(), live, "W6: reopen hash == live hash");
    assert_eq!(r.project().clip_assets().len(), 3, "bindings survived");

    // re-export → semantically identical (deterministic SW path: byte-equal)
    let out2 = dir.join("milestone-2.mp4");
    let info2 = r.export_reencode(&out2, &output, N).expect("re-export");
    assert_eq!(info2.tracks[0].nb_frames, Some(N as u64));
    assert_eq!(
        info.file_sha256, info2.file_sha256,
        "W6: re-export byte-identical (deterministic SW pipeline)"
    );

    // ffprobe eye on the milestone output
    if ffprobe_available() {
        let j = ffprobe_json(&out).expect("ffprobe parses milestone output");
        assert_eq!(j["streams"][0]["codec_name"], "mpeg4");
        assert_eq!(
            j["streams"][0]["nb_frames"]
                .as_str()
                .unwrap()
                .parse::<u64>()
                .unwrap(),
            N as u64
        );
    }
}

fn ffprobe_available() -> bool {
    std::process::Command::new("ffprobe")
        .arg("-version")
        .output()
        .is_ok()
}

fn ffprobe_json(path: &Path) -> Option<serde_json::Value> {
    let out = std::process::Command::new("ffprobe")
        .args(["-v", "error", "-show_streams", "-of", "json"])
        .arg(path)
        .output()
        .ok()?;
    serde_json::from_str(&String::from_utf8(out.stdout).ok()?).ok()
}
