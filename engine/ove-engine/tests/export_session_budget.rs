//! WAVE 19 perf conformance — export-lifetime decoder session budget
//! (ADR-023). Deterministic open-count pinning, never wall-clock.
//!
//! Pre-W19 defect shape: `export_reencode` rebuilt the DecodeSource map
//! for EVERY output frame (inside `render_frame`), so every exported frame
//! re-OPENED the demuxer + decoder, re-seeked to the keyframe floor, and
//! re-decoded the keyframe→target span — for every placement. The declared
//! budget: exactly ONE decoder open per distinct source per export (the
//! session lives for the export; sequential same-GOP fetches then reuse
//! the decoder cursor — the REALWORLD-BUG-3 discipline finally engages in
//! the export path). A count that grows with OUTPUT FRAMES is the
//! regression signature; the counter is the W19 conformance instrument
//! (`ove_engine::decoder_opens`).
//!
//! Time convention follows the certified realworld scenario
//! (`tests/realworld.rs`): project tick axis 24 000; clip windows are
//! rationals on that axis (`s(sec, den)` builds true seconds).

use std::path::{Path, PathBuf};
use std::sync::Mutex;

use ove_engine::{decoder_opens, reset_decoder_opens, Engine};
use ove_render::OutputSpec;
use ove_time::Rational;
use ove_timeline::GapTrack;

const AXIS: i64 = 24_000;

/// The open counter is a PROCESS-GLOBAL instrument; cargo runs this binary's
/// tests on parallel threads. Every budget measurement therefore holds this
/// lock across reset → export → assert, so each count is exactly its own
/// export's (deterministic under any test-thread schedule).
static BUDGET_LOCK: Mutex<()> = Mutex::new(());

fn s(sec: i64, den: i64) -> Rational {
    Rational::new(sec * (AXIS / den), AXIS)
}

fn media(name: &str) -> PathBuf {
    let p = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../ove-encode/tests/media")
        .join(name);
    assert!(p.exists(), "corpus fixture missing: {}", p.display());
    p
}

fn tmp(name: &str) -> PathBuf {
    let d = std::env::temp_dir()
        .join("ove-engine-export-budget")
        .join(name);
    let _ = std::fs::remove_dir_all(&d);
    std::fs::create_dir_all(d.parent().unwrap()).unwrap();
    d
}

fn output_spec() -> OutputSpec {
    OutputSpec {
        width: 320,
        height: 240,
        rate_num: 24,
        rate_den: 1,
        working_space: ove_media::ColorTags {
            primaries: ove_media::Primaries::Bt709,
            transfer: ove_media::Transfer::Bt709,
            matrix: ove_media::MatrixCoeffs::Bt709,
            range: ove_media::Range::Limited,
            chroma_loc: None,
        },
    }
}

/// One source, one 2 s clip: 8 exported frames must open the decoder ONCE.
/// Pre-W19 this opened it once per frame — the regression signature this
/// test exists to kill. Every frame must also be non-trivially covered by
/// the clip (a zero-fetch frame renders empty-plan black and would make the
/// budget vacuous) — the exporter's frame count is asserted exactly.
#[test]
fn export_opens_decoder_once_per_source() {
    let dir = tmp("single-source");
    let mut e = Engine::create(&dir, (AXIS, 1)).expect("create");
    e.add_track(1, ove_timeline::TrackKind::Gap(GapTrack::new()))
        .expect("track");
    let hex = e.import_media(&media("copy24.mp4")).expect("import");
    e.add_clip(1, &hex, s(2, 1), Rational::new(0, 1))
        .expect("clip");

    let output = output_spec();
    let _guard = BUDGET_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    reset_decoder_opens();
    const N: i64 = 8;
    let out = dir.join("export.mp4");
    let info = e
        .export_reencode(&out, &output, N)
        .expect("export re-encode");
    assert_eq!(info.tracks[0].nb_frames, Some(N as u64), "frames exact");

    let opens = decoder_opens();
    assert_eq!(
        opens, 1,
        "decoder must open ONCE per source per export (got {opens} for {N} frames — \
         per-frame session rebuild regression)"
    );
}

/// Two tracks sharing ONE source (two placements per output frame — the
/// realworld overlay shape): still exactly one open for the shared source.
#[test]
fn export_two_placements_same_source_open_once() {
    let dir = tmp("shared-source");
    let mut e = Engine::create(&dir, (AXIS, 1)).expect("create");
    e.add_track(1, ove_timeline::TrackKind::Gap(GapTrack::new()))
        .expect("track 1");
    e.add_track(2, ove_timeline::TrackKind::Gap(GapTrack::new()))
        .expect("track 2");
    let hex = e.import_media(&media("copy24.mp4")).expect("import");
    e.add_clip(1, &hex, s(2, 1), Rational::new(0, 1))
        .expect("base clip");
    e.add_clip(2, &hex, s(2, 1), s(0, 1)).expect("overlay clip");

    let output = output_spec();
    let _guard = BUDGET_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    reset_decoder_opens();
    const N: i64 = 8;
    let out = dir.join("export.mp4");
    e.export_reencode(&out, &output, N)
        .expect("export re-encode");

    let opens = decoder_opens();
    assert_eq!(
        opens, 1,
        "shared source = one session for the whole export (got {opens})"
    );
}

/// Multi-source project under the DECLARED v1 render contract (ADR-017
/// note): every clip maps to the session's FIRST imported source — the
/// second source is never fetched, so the budget stays at one open. This
/// pins the v1 single-source render binding at the decode level (the
/// assertion upgrades to one-open-per-BOUND-source when multi-source
/// render-by-binding lands and closes RW-NOTE-1).
#[test]
fn export_multi_source_binds_first_source_only() {
    let dir = tmp("two-sources");
    let mut e = Engine::create(&dir, (AXIS, 1)).expect("create");
    e.add_track(1, ove_timeline::TrackKind::Gap(GapTrack::new()))
        .expect("track 1");
    e.add_track(2, ove_timeline::TrackKind::Gap(GapTrack::new()))
        .expect("track 2");
    // video-only fixtures: v1 audio assembly is single-source (declared
    // capability-matrix gap) and copyav's audio stream would ride an
    // unrelated edge, not the session budget under test here.
    let hex1 = e.import_media(&media("copy24.mp4")).expect("import 1");
    let hex2 = e.import_media(&media("copyntsc.mp4")).expect("import 2");
    e.add_clip(1, &hex1, s(2, 1), Rational::new(0, 1))
        .expect("base clip");
    e.add_clip(2, &hex2, s(2, 1), Rational::new(0, 1))
        .expect("overlay clip");

    let output = output_spec();
    let _guard = BUDGET_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    reset_decoder_opens();
    const N: i64 = 8;
    let out = dir.join("export.mp4");
    let info = e
        .export_reencode(&out, &output, N)
        .expect("export re-encode");
    assert_eq!(info.tracks[0].nb_frames, Some(N as u64), "frames exact");

    let opens = decoder_opens();
    assert_eq!(
        opens, 1,
        "v1 binds the FIRST imported source; exactly one open (got {opens})"
    );
}
