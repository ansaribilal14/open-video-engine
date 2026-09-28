//! Planner property/unit tests (pure logic, no libav, no I/O).
//!
//! The planner IS ENCODER_SPEC §3 as executable rules; these tests pin the
//! rules: keyframe-aligned snapping that is always REPORTED, keyframe-start
//! validity, exact rational boundaries (23.976 included), audio grid
//! alignment, mixed segmentation exactness.

use ove_encode::planner::{
    audio_aligns, kf_ceil, kf_floor, plan_export, plan_track, plan_video_copy, plan_video_mixed,
    AudioGrid, Boundary, CopyPolicy, PlanError, SnapDirection, TimeRange, TrackInput, TrackKindTag,
    TrackRoute,
};
use ove_media::StreamId;
use ove_time::Rational;

fn r(num: i64, den: i64) -> Rational {
    Rational::new(num, den)
}

/// 6 s @ 24 fps, keyframes every second (the copy24 corpus shape).
fn kf_24fps() -> Vec<Rational> {
    (0..=6).map(|s| r(s, 1)).collect()
}

fn video_input(kfs: Vec<Rational>, media_end: Rational) -> TrackInput {
    TrackInput {
        stream_id: StreamId(0),
        kind: TrackKindTag::Video,
        keyframes: kfs,
        media_end,
        audio_grid: None,
        reencode_available: true,
    }
}

// ---------------------------------------------------------------------------
// kf floor/ceil binary search
// ---------------------------------------------------------------------------

#[test]
fn p01_floor_ceil_exact_and_edges() {
    let kfs = kf_24fps();
    assert_eq!(kf_floor(&kfs, r(5, 2)), Some(r(2, 1))); // 2.5 → 2
    assert_eq!(kf_ceil(&kfs, r(5, 2)), Some(r(3, 1))); // 2.5 → 3
    assert_eq!(kf_floor(&kfs, r(0, 1)), Some(r(0, 1)));
    assert_eq!(kf_ceil(&kfs, r(0, 1)), Some(r(0, 1)));
    // before first / after last
    let late: Vec<Rational> = vec![r(1, 1), r(2, 1)];
    assert_eq!(kf_floor(&late, r(0, 1)), None);
    assert_eq!(kf_ceil(&late, r(3, 1)), None);
}

// ---------------------------------------------------------------------------
// KeyframeAlignedOnly: exact boundaries → no snaps, exact span
// ---------------------------------------------------------------------------

#[test]
fn p02_exact_keyframe_cut_has_no_snaps() {
    let route = plan_video_copy(
        &kf_24fps(),
        r(6, 1),
        TimeRange::new(r(1, 1), r(3, 1)).unwrap(),
        CopyPolicy::KeyframeAlignedOnly,
    )
    .expect("plannable");
    let TrackRoute::StreamCopy { span, snaps } = route else {
        panic!("expected copy");
    };
    assert_eq!(span.start, r(1, 1));
    assert_eq!(span.end, r(3, 1));
    assert!(snaps.is_empty(), "aligned cut must not snap");
}

// ---------------------------------------------------------------------------
// Snapping: round-out at both ends, always reported (§3.1)
// ---------------------------------------------------------------------------

#[test]
fn p03_mid_gop_cut_snaps_and_reports() {
    let route = plan_video_copy(
        &kf_24fps(),
        r(6, 1),
        TimeRange::new(r(3, 2), r(9, 2)).unwrap(), // [1.5, 4.5)
        CopyPolicy::KeyframeAlignedOnly,
    )
    .expect("plannable");
    let TrackRoute::StreamCopy { span, snaps } = route else {
        panic!("expected copy");
    };
    assert_eq!(span.start, r(1, 1), "start snaps out to kf floor");
    assert_eq!(span.end, r(5, 1), "end snaps out to kf ceil");
    assert_eq!(snaps.len(), 2, "both snaps reported");
    let (s, e) = (&snaps[0], &snaps[1]);
    assert_eq!(s.boundary, Boundary::Start);
    assert_eq!(s.direction, SnapDirection::RoundOut);
    assert_eq!(s.requested, r(3, 2));
    assert_eq!(s.resolved, r(1, 1));
    assert_eq!(e.boundary, Boundary::End);
    assert_eq!(e.direction, SnapDirection::RoundOut);
    assert_eq!(e.requested, r(9, 2));
    assert_eq!(e.resolved, r(5, 1));
}

// ---------------------------------------------------------------------------
// Validity: start before first keyframe → RoundIn to the first keyframe
// (a copy must open on a keyframe) — reported, never silent
// ---------------------------------------------------------------------------

#[test]
fn p04_start_before_first_kf_rounds_in() {
    // media keyframes start at 1 s (open start)
    let kfs: Vec<Rational> = (1..=5).map(|s| r(s, 1)).collect();
    let route = plan_video_copy(
        &kfs,
        r(5, 1),
        TimeRange::new(r(0, 1), r(3, 1)).unwrap(),
        CopyPolicy::KeyframeAlignedOnly,
    )
    .expect("plannable");
    let TrackRoute::StreamCopy { span, snaps } = route else {
        panic!("expected copy");
    };
    assert_eq!(span.start, r(1, 1));
    assert_eq!(snaps.len(), 1);
    assert_eq!(snaps[0].direction, SnapDirection::RoundIn);
}

// ---------------------------------------------------------------------------
// End past the last keyframe → ToEof snap (reported); exactness held
// ---------------------------------------------------------------------------

#[test]
fn p05_end_beyond_last_kf_snaps_to_eof() {
    // kf grid 0..=4 (5 kfs), media runs to 6 s
    let kfs: Vec<Rational> = (0..=4).map(|s| r(s, 1)).collect();
    let route = plan_video_copy(
        &kfs,
        r(6, 1),
        TimeRange::new(r(2, 1), r(6, 1)).unwrap(),
        CopyPolicy::KeyframeAlignedOnly,
    )
    .expect("plannable");
    let TrackRoute::StreamCopy { span, snaps } = route else {
        panic!("expected copy");
    };
    assert_eq!(span.start, r(2, 1));
    assert_eq!(span.end, r(6, 1));
    assert_eq!(snaps.len(), 1);
    assert_eq!(snaps[0].direction, SnapDirection::ToEof);
    assert_eq!(snaps[0].resolved, r(6, 1));
}

// ---------------------------------------------------------------------------
// 23.976 (ntsc) exactness: boundaries on the 30000/1001 grid never round
// ---------------------------------------------------------------------------

#[test]
fn p06_ntsc_grid_exact() {
    // 4 s @ 30000/1001; kfs at every second: 1001k/30000
    let kf = |s: i64| r(1001 * s, 30000);
    let kfs: Vec<Rational> = (0..=4).map(kf).collect();
    let route = plan_video_copy(
        &kfs,
        r(4004, 1000), // media end 4.004 s
        TimeRange::new(kf(1), kf(3)).unwrap(),
        CopyPolicy::KeyframeAlignedOnly,
    )
    .expect("plannable");
    let TrackRoute::StreamCopy { span, snaps } = route else {
        panic!("expected copy");
    };
    assert_eq!(span.start, r(1001, 30000));
    assert_eq!(span.end, r(3003, 30000));
    assert!(snaps.is_empty());
    // 1-frame-in boundary: ceil lands exactly on the next kf
    let half = r(1001 * 3 + 500, 30000);
    assert_eq!(kf_ceil(&kfs, half), Some(kf(4)));
    assert_eq!(kf_floor(&kfs, half), Some(kf(3)));
}

// ---------------------------------------------------------------------------
// Error paths: inverted ranges, beyond-media, no keyframes
// ---------------------------------------------------------------------------

#[test]
fn p07b_inverted_range_is_typed_error() {
    assert!(TimeRange::new(r(3, 1), r(2, 1)).is_err());
}

#[test]
fn p07c_end_beyond_media() {
    let err = plan_video_copy(
        &kf_24fps(),
        r(6, 1),
        TimeRange::new(r(1, 1), r(7, 1)).unwrap(),
        CopyPolicy::KeyframeAlignedOnly,
    )
    .unwrap_err();
    assert_eq!(
        err,
        PlanError::EndBeyondMedia {
            end: r(7, 1),
            media_end: r(6, 1)
        }
    );
}

#[test]
fn p07d_no_keyframes() {
    let err = plan_video_copy(
        &[],
        r(6, 1),
        TimeRange::new(r(1, 1), r(3, 1)).unwrap(),
        CopyPolicy::KeyframeAlignedOnly,
    )
    .unwrap_err();
    assert_eq!(err, PlanError::NoKeyframes);
}

// ---------------------------------------------------------------------------
// Mixed segmentation (AllowReencodeBoundaries): exact total coverage
// ---------------------------------------------------------------------------

#[test]
fn p08_mixed_segments_cover_exactly() {
    let segs = plan_video_mixed(
        &kf_24fps(),
        r(6, 1),
        TimeRange::new(r(3, 2), r(9, 2)).unwrap(), // [1.5, 4.5)
    )
    .expect("plannable");
    // expected: head re-encode [1.5, 2), copy [2, 4), tail re-encode [4, 4.5)
    assert_eq!(segs.len(), 3, "head + core + tail");
    let (h, hr) = &segs[0];
    let (c, cr) = &segs[1];
    let (t, tr) = &segs[2];
    assert_eq!((h.start, h.end), (r(3, 2), r(2, 1)));
    assert!(matches!(hr, TrackRoute::ReEncode { .. }));
    assert_eq!((c.start, c.end), (r(2, 1), r(4, 1)));
    assert!(matches!(cr, TrackRoute::StreamCopy { .. }));
    assert_eq!((t.start, t.end), (r(4, 1), r(9, 2)));
    assert!(matches!(tr, TrackRoute::ReEncode { .. }));
    // total coverage == requested, boundary-adjacent, no gaps/overlaps
    assert_eq!(h.start, r(3, 2));
    assert_eq!(h.end, c.start);
    assert_eq!(c.end, t.start);
    assert_eq!(t.end, r(9, 2));
}

#[test]
fn p08b_mixed_aligned_cut_collapses_to_copy() {
    let segs = plan_video_mixed(
        &kf_24fps(),
        r(6, 1),
        TimeRange::new(r(1, 1), r(3, 1)).unwrap(),
    )
    .expect("plannable");
    assert_eq!(segs.len(), 1);
    assert!(matches!(segs[0].1, TrackRoute::StreamCopy { .. }));
}

// ---------------------------------------------------------------------------
// Audio grid alignment
// ---------------------------------------------------------------------------

#[test]
fn p09_audio_grid_alignment() {
    let grid = AudioGrid {
        frame_samples: 1024,
        sample_rate: 48000,
    };
    // 1024/48000 s: aligned
    assert!(audio_aligns(r(1024, 48000), grid));
    // 1 s: 48000 % 1024 != 0 → NOT aligned
    assert!(!audio_aligns(r(1, 1), grid));
    // 0: aligned (start of stream)
    assert!(audio_aligns(r(0, 1), grid));
    // 2048/48000: aligned
    assert!(audio_aligns(r(2048, 48000), grid));
    // non-representable: 1/3 s × 48000 = 16000 → integer but 16000 % 1024 != 0
    assert!(!audio_aligns(r(1, 3), grid));
}

#[test]
fn p09b_audio_misaligned_without_reencode_is_deferred() {
    let input = TrackInput {
        stream_id: StreamId(1),
        kind: TrackKindTag::Audio,
        keyframes: vec![],
        media_end: r(6, 1),
        audio_grid: Some(AudioGrid {
            frame_samples: 1024,
            sample_rate: 48000,
        }),
        reencode_available: false,
    };
    let track = plan_track(
        &input,
        TimeRange::new(r(1, 1), r(3, 1)).unwrap(),
        CopyPolicy::KeyframeAlignedOnly,
    )
    .expect("audio plan never hard-fails on alignment");
    let TrackRoute::Deferred { reason } = track.route else {
        panic!("expected declared deferral");
    };
    assert!(reason.contains("W7"), "deferral names the wave: {reason}");
}

// ---------------------------------------------------------------------------
// Export shape classification
// ---------------------------------------------------------------------------

#[test]
fn p10_export_shapes() {
    let v = video_input(kf_24fps(), r(6, 1));
    // aligned cut → SingleStreamCopy
    let plan = plan_export(
        std::slice::from_ref(&v),
        TimeRange::new(r(1, 1), r(3, 1)).unwrap(),
        CopyPolicy::KeyframeAlignedOnly,
    )
    .unwrap();
    assert_eq!(
        plan.shape,
        ove_encode::planner::ExportShape::SingleStreamCopy
    );
    // unaligned cut → still copy (snapped, reported)
    let plan = plan_export(
        std::slice::from_ref(&v),
        TimeRange::new(r(3, 2), r(9, 2)).unwrap(),
        CopyPolicy::KeyframeAlignedOnly,
    )
    .unwrap();
    assert_eq!(
        plan.shape,
        ove_encode::planner::ExportShape::SingleStreamCopy
    );
    assert_eq!(plan.snaps().len(), 2, "receipt lists snaps");
    // mixed → Mixed
    let plan = plan_export(
        std::slice::from_ref(&v),
        TimeRange::new(r(3, 2), r(9, 2)).unwrap(),
        CopyPolicy::AllowReencodeBoundaries,
    )
    .unwrap();
    assert_eq!(plan.shape, ove_encode::planner::ExportShape::Mixed);
    // copy_span_of returns the copy core
    let span = plan.copy_span_of(StreamId(0)).expect("copy span");
    assert_eq!((span.start, span.end), (r(2, 1), r(4, 1)));
}
