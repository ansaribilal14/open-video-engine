//! RLW-9 decode-input budgets (ADR-024) + RLW-8-F3 zero-duration pin.
//!
//! Hostile geometry is SYNTHESIZED at test runtime by header-patching an
//! existing committed fixture (no hostile media in git, no corpus media in
//! CI). The v210/MOV vector is used because its codecpar dimensions come
//! straight from the sample entry (no codec-level config to spoof): the
//! patched container IS the pixel-bomb shape a header-driven allocator must
//! survive. The zero-duration fixture (vfr_zerodur.mp4) is a tiny synthetic
//! (testsrc2, 64×64) B-frame VFR stream whose container carries NO
//! per-frame durations — the real-media pattern the hostile corpus found
//! (RLW-8-F3), reproducible from public tooling alone.

use std::path::{Path, PathBuf};

use ove_decode::ffmpeg::{FfmpegProbe, FfmpegSwDecoder};
use ove_decode::{
    check_video_budget, DecodeConfig, DecodeError, Decoder, DECODE_MAX_DIM, DECODE_MAX_PIXELS,
};
use ove_media::{AssetRef, ProbeBackend, ProbeError};

fn media(name: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests/media")
        .join(name)
}

/// Patch the v210 VisualSampleEntry width/height (u16 BE at +28 from the
/// sample-entry 4cc) and write the result to a temp file. The bitstream is
/// untouched — the container now DECLARES hostile geometry.
fn patch_dims(src: &Path, width: u16, height: u16, tag: &str) -> PathBuf {
    let data = std::fs::read(src).expect("fixture readable");
    let needle = b"v210";
    let pos = data
        .windows(4)
        .position(|w| w == needle)
        .expect("v210 sample entry present");
    let mut out = data;
    out[pos + 28..pos + 32].copy_from_slice(&[
        width.to_be_bytes()[0],
        width.to_be_bytes()[1],
        height.to_be_bytes()[0],
        height.to_be_bytes()[1],
    ]);
    let path = std::env::temp_dir().join(format!("ove_limits_{tag}.mov"));
    std::fs::write(&path, &out).expect("patched fixture writable");
    path
}

// ---------------------------------------------------------------------------
// RLW-9: declared budget unit edges (pure, no media)
// ---------------------------------------------------------------------------

#[test]
fn budget_constants_are_self_consistent() {
    // 8K UHD must fit inside the pixel cap (the honest-scale claim of
    // ADR-024), and the cap must bind below MAX_DIM² (the bomb bind claim).
    let uhd = 7680u64 * 4320;
    assert!(uhd <= DECODE_MAX_PIXELS, "8K UHD must be legal");
    assert!(
        (DECODE_MAX_DIM as u64) * (DECODE_MAX_DIM as u64) > DECODE_MAX_PIXELS,
        "MAX_DIM squared must exceed the pixel cap (cap is the binding limit)"
    );
}

#[test]
fn budget_rejects_degenerate_and_extreme_geometry() {
    assert!(matches!(
        check_video_budget(0, 720),
        Err(DecodeError::BeyondDeclaredLimits(_))
    ));
    assert!(matches!(
        check_video_budget(1280, 0),
        Err(DecodeError::BeyondDeclaredLimits(_))
    ));
    // one dimension over the per-dimension cap (even with few total pixels)
    assert!(matches!(
        check_video_budget(DECODE_MAX_DIM + 1, 2),
        Err(DecodeError::BeyondDeclaredLimits(_))
    ));
    // dimensions under the cap but pixels over the per-frame cap
    assert!(matches!(
        check_video_budget(4000, 9000),
        Err(DecodeError::BeyondDeclaredLimits(_))
    ));
    // honest scale passes
    assert!(check_video_budget(1280, 720).is_ok());
    assert!(check_video_budget(406, 720).is_ok());
    assert!(check_video_budget(7680, 4320).is_ok());
}

// ---------------------------------------------------------------------------
// RLW-9: hostile headers are rejected TYPED at both boundaries
// ---------------------------------------------------------------------------

#[test]
fn probe_rejects_beyond_dimension_cap() {
    // 16400 > DECODE_MAX_DIM (16384) with trivial total pixels — the
    // DIMENSION cap binds independently of the pixel cap.
    let path = patch_dims(&media("v210_base.mov"), 16_400, 64, "dim");
    let asset = AssetRef::from_path(&path).expect("asset");
    let err = FfmpegProbe.probe(&asset).expect_err("must reject");
    assert!(
        matches!(err, ProbeError::BeyondDeclaredLimits(ref d) if d.contains("DECODE_MAX_DIM")),
        "typed dimension-cap rejection, got: {err:?}"
    );
    let _ = std::fs::remove_file(&path);
}

#[test]
fn probe_rejects_beyond_pixel_cap() {
    // 4000×9000 = 36 MP > DECODE_MAX_PIXELS, both dims under DECODE_MAX_DIM —
    // proves the PIXEL cap binds independently of the dimension cap.
    let path = patch_dims(&media("v210_base.mov"), 4000, 9000, "px");
    let asset = AssetRef::from_path(&path).expect("asset");
    let err = FfmpegProbe.probe(&asset).expect_err("must reject");
    assert!(
        matches!(err, ProbeError::BeyondDeclaredLimits(ref d) if d.contains("DECODE_MAX_PIXELS")),
        "typed pixel-cap rejection, got: {err:?}"
    );
    let _ = std::fs::remove_file(&path);
}

#[test]
fn decoder_open_rejects_beyond_pixel_cap() {
    // Defense-in-depth boundary: a caller that bypasses the probe must still
    // never reach libav's allocator with hostile geometry.
    let path = patch_dims(&media("v210_base.mov"), 4000, 9000, "open");
    let asset = AssetRef::from_path(&path).expect("asset");
    let err = match FfmpegSwDecoder::open(&asset, ove_media::StreamId(0), DecodeConfig::default()) {
        Err(e) => e,
        Ok(_) => panic!("must reject hostile geometry at the open boundary"),
    };
    assert!(
        matches!(err, DecodeError::BeyondDeclaredLimits(ref d) if d.contains("DECODE_MAX_PIXELS")),
        "typed pixel-cap rejection at open, got: {err:?}"
    );
    let _ = std::fs::remove_file(&path);
}

#[test]
fn unpatched_fixture_still_opens_through_both_boundaries() {
    // negative control: the budgets are invisible to honest media
    let asset = AssetRef::from_path(media("v210_base.mov").as_path()).expect("asset");
    assert!(FfmpegProbe.probe(&asset).is_ok(), "probe ok");
    assert!(
        FfmpegSwDecoder::open(&asset, ove_media::StreamId(0), DecodeConfig::default()).is_ok(),
        "open ok"
    );
}

// ---------------------------------------------------------------------------
// RLW-8-F3 pin: zero-duration frames are DELIVERED, never typed corrupt
// ---------------------------------------------------------------------------

#[test]
fn zero_duration_frames_are_delivered_not_corrupt() {
    // vfr_zerodur.mp4: synthetic B-frame VFR media whose container declares
    // NO per-frame duration (23/23 packets duration N/A in ffprobe; decode
    // clean under -xerror). Pre-fix, the FIRST frame was typed
    // Corrupt("frame without duration") — the RLW-8-F3 render-block. The
    // fix: a container-declared absence is delivered as duration 0/1.
    let asset = AssetRef::from_path(media("vfr_zerodur.mp4").as_path()).expect("asset");
    let mut dec = FfmpegSwDecoder::open(&asset, ove_media::StreamId(0), DecodeConfig::default())
        .expect("open");
    let mut frames = 0u32;
    let mut zero_duration = 0u32;
    let mut last_pts = None;
    while let Some(f) = dec
        .next()
        .expect("every frame decodes typed-clean (F3 fixed)")
    {
        if let Some(p) = last_pts {
            assert!(f.pts > p, "decoded pts must be strictly increasing");
        }
        last_pts = Some(f.pts);
        if f.duration == ove_time::Rational::zero(1) {
            zero_duration += 1;
        }
        frames += 1;
    }
    assert!(frames >= 20, "whole stream decoded, got {frames} frames");
    assert!(
        zero_duration >= 20,
        "the zero-duration contract is load-bearing here: {zero_duration}/{frames} frames carry duration 0"
    );
}
