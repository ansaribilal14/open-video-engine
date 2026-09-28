//! Mapping property suite — the timeline↔media seam contract (ADR-013,
//! MEDIA_ENGINE_SPEC §3). Deterministic seeded generation, same discipline as
//! the ove-time P-suite. These tests pin:
//!   S1 forward/backward mapping roundtrip is EXACT (identity)
//!   S2 mapping agrees with the ClipEntry primitive (src_at)
//!   S3 ingest validation accepts exactly the legal range (boundary closed)
//!   S4 snap seeks land on keyframes, ≤ target, monotone (E-007/D-5)
//!   S5 frame spans are exact at boundaries and match floor/ceil semantics
//!   S6 exact seeks report target == timeline_to_source (D-4 frame identity
//!      is the DECODER's conformance duty; the planner must only be exact)

use ove_time::Rational;
use ove_timeline::mapping::{
    frame_span, plan_seek, source_to_timeline, timeline_to_source, validate_ingest, ClipWindow,
    SeamError, SeekPlan, SourceClock,
};

struct Rng(u64);
impl Rng {
    fn next(&mut self) -> u64 {
        self.0 ^= self.0 << 13;
        self.0 ^= self.0 >> 7;
        self.0 ^= self.0 << 17;
        self.0
    }
    fn below(&mut self, n: u64) -> u64 {
        self.next() % n
    }
}

fn ticks(num: i64, den: i64) -> Rational {
    Rational::new(num, den)
}

fn clip(id: u64, start: Rational, dur: Rational, src_in: Rational, speed: Rational) -> ClipWindow {
    let _ = id; // ids live in the timeline; the seam sees placements only
    ClipWindow::retimed(start, dur, src_in, speed)
}

/// A CFR 24fps source, 10s long, keyframes every 2s (typical 2s GOP).
fn cfr_clock() -> SourceClock {
    SourceClock::new(ticks(10, 1), (0..5).map(|k| ticks(2 * k, 1)).collect()).unwrap()
}

const CASES: u64 = 20_000;

/// S1: source_to_timeline(timeline_to_source(t)) == t exactly, over random
/// clips (incl. retimes) and random times inside the clip (endpoints incl.).
#[test]
fn s1_mapping_roundtrip_exact() {
    let mut rng = Rng(0xB1);
    for _ in 0..CASES {
        let speed = [
            ticks(1, 1),
            ticks(1, 2),
            ticks(2, 1),
            ticks(1001, 1000),
            ticks(4, 5),
        ][rng.below(5) as usize];
        let dur = ticks(1 + rng.below(10_000) as i64, 24_000);
        let start = ticks(rng.below(100_000) as i64, 24_000);
        let src_in = ticks(rng.below(100_000) as i64, 48_000);
        let c = clip(1, start, dur, src_in, speed);
        // random fraction of the clip (0 ..= 1), plus both exact endpoints
        let frac = Rational::new(rng.below(24_001) as i64, 24_000);
        let ts = [
            c.timeline_start,
            c.end(),
            c.timeline_start.add(c.dur.mul(frac)),
        ];
        for t in ts {
            let src = timeline_to_source(&c, t).unwrap();
            let back = source_to_timeline(&c, src).unwrap();
            assert_eq!(back, t, "roundtrip {t} via {src} on {speed}");
        }
    }
}

/// S2: forward mapping agrees with the ClipEntry primitive (src_at) — the
/// seam wraps, never redefines, the clip's own semantics.
#[test]
fn s2_agrees_with_clip_primitive() {
    let mut rng = Rng(0xB2);
    for _ in 0..CASES {
        let c = clip(
            1,
            ticks(rng.below(1_000) as i64, 24_000),
            ticks(1 + rng.below(24_000) as i64, 24_000),
            ticks(rng.below(1_000) as i64, 48_000),
            ticks(1, 1),
        );
        let frac = Rational::new(rng.below(24_001) as i64, 24_000);
        let off = c.dur.mul(frac); // within [0, dur]
        let t = c.timeline_start.add(off);
        assert_eq!(timeline_to_source(&c, t).unwrap(), c.src_at(off));
    }
}

/// S3: ingest validation accepts exactly src range [0, duration] (closed at
/// the duration end: a clip may consume UP TO the last tick) and rejects
/// everything beyond, with typed errors — no panics for range bugs.
#[test]
fn s3_ingest_gate() {
    let clock = cfr_clock(); // 10s source
                             // legal: consumes exactly the whole source
    let whole = clip(1, ticks(0, 1), ticks(10, 1), ticks(0, 1), ticks(1, 1));
    assert_eq!(validate_ingest(&whole, &clock), Ok(()));
    // legal: 2x retime of half the source
    let half_2x = clip(2, ticks(0, 1), ticks(5, 1), ticks(0, 1), ticks(2, 1));
    assert_eq!(validate_ingest(&half_2x, &clock), Ok(()));
    // illegal: one tick beyond the source
    let over = clip(3, ticks(0, 1), ticks(10, 1), ticks(1, 24000), ticks(1, 1));
    assert_eq!(
        validate_ingest(&over, &clock),
        Err(SeamError::SourceBeyondDuration {
            requested_end: ticks(240_001, 24_000),
            duration: ticks(10, 1),
        })
    );
    // illegal: negative src_in
    let neg = clip(4, ticks(0, 1), ticks(1, 1), ticks(-1, 24000), ticks(1, 1));
    assert!(matches!(
        validate_ingest(&neg, &clock),
        Err(SeamError::SourceNegative { .. })
    ));
    // illegal: zero speed
    let frozen = clip(5, ticks(0, 1), ticks(1, 1), ticks(0, 1), ticks(0, 1));
    assert!(matches!(
        validate_ingest(&frozen, &clock),
        Err(SeamError::SpeedNotPositive { .. })
    ));
    // illegal: zero duration
    let empty = clip(6, ticks(0, 1), ticks(0, 1), ticks(0, 1), ticks(1, 1));
    assert!(matches!(
        validate_ingest(&empty, &clock),
        Err(SeamError::DurationNotPositive { .. })
    ));
}

/// S4: Snap seeks land ON a keyframe, ≤ target; landing is monotone in the
/// target (checked on ordered target pairs). E-007/D-5 golden trap: with a
/// 2s GOP, target 1.5s lands 0s; target 2.0s lands 2s exactly.
#[test]
fn s4_snap_lands_on_keyframes_monotone() {
    let clock = cfr_clock();
    let mut rng = Rng(0xB4);
    let c = clip(1, ticks(0, 1), ticks(10, 1), ticks(0, 1), ticks(1, 1));
    for _ in 0..CASES {
        let (t1, t2) = {
            let a = ticks(rng.below(240_000) as i64, 24_000);
            let b = ticks(rng.below(240_000) as i64, 24_000);
            if a <= b {
                (a, b)
            } else {
                (b, a)
            }
        };
        let p1: SeekPlan = plan_seek(&c, &clock, t1, true).unwrap();
        let p2: SeekPlan = plan_seek(&c, &clock, t2, true).unwrap();
        // each landing IS a member of the keyframe set, ≤ its own target
        assert!(
            clock.keyframes.contains(&p1.land_pts),
            "land {} not a keyframe for t={t1}",
            p1.land_pts
        );
        assert!(p1.land_pts <= p1.target_pts, "snap overshoots");
        // monotone: ordered targets produce ordered landings
        assert!(p1.land_pts <= p2.land_pts, "snap landing not monotone");
    }
    // D-5 golden trap: target 1.5s lands 0s, target 2.0s lands 2s exactly
    let p15 = plan_seek(&c, &clock, ticks(3, 2), true).unwrap();
    assert_eq!(p15.land_pts, ticks(0, 1));
    let p20 = plan_seek(&c, &clock, ticks(2, 1), true).unwrap();
    assert_eq!(p20.land_pts, ticks(2, 1));
    // before the first keyframe: typed error, no panic
    let before = plan_seek(
        &c,
        &SourceClock::new(ticks(10, 1), vec![ticks(2, 1), ticks(4, 1)]).unwrap(),
        ticks(1, 1),
        true,
    );
    assert!(matches!(
        before,
        Err(SeamError::NoKeyframeAtOrBefore { .. })
    ));
}

/// S5: frame_span bounds are exact floor/ceil of the clip ends; half-open
/// span contains exactly ceil(end·r) - floor(start·r) frames.
#[test]
fn s5_frame_span_exact() {
    let mut rng = Rng(0xB5);
    for _ in 0..2_000 {
        let start_f = rng.below(1_000) as i64; // frame-aligned starts
        let n_f = 1 + rng.below(1_000) as i64;
        let frame = ticks(1001, 24000); // 23.976 fps frame duration
        let c = clip(
            1,
            frame.mul(ticks(start_f, 1)),
            frame.mul(ticks(n_f, 1)),
            ticks(0, 1),
            ticks(1, 1),
        );
        let (first, last) = frame_span(&c, (24000, 1001)).unwrap();
        assert_eq!(first, start_f, "floor start");
        assert_eq!(last, start_f + n_f, "ceil end");
        assert_eq!(last - first, n_f, "span length is the frame count");
        // timeline_to_source at each frame boundary stays inside the clip
        for k in [first, last - 1] {
            let t = frame.mul(ticks(k, 1));
            let src = timeline_to_source(&c, t).unwrap();
            assert!(src >= c.src_in && src <= c.src_in.add(c.src_dur()));
        }
    }
    // unaligned start floors DOWN, unaligned end ceils UP: clip runs a tick
    // after the frame-0 boundary and a tick BEFORE the frame-24 boundary
    let c2 = clip(
        7,
        ticks(1, 24000),       // a tick after frame 0
        ticks(47_997, 48_000), // ends a tick before the frame-24 boundary
        ticks(0, 1),
        ticks(1, 1),
    );
    assert_eq!(frame_span(&c2, (24, 1)).unwrap(), (0, 24));
    // one tick LATER end crosses the boundary: frame 24 joins the span
    let c3 = clip(
        8,
        ticks(1, 24000),
        ticks(47_999, 48_000), // end = 48001/48000 s — 1/48000 past frame 24
        ticks(0, 1),
        ticks(1, 1),
    );
    assert_eq!(frame_span(&c3, (24, 1)).unwrap(), (0, 25));
}

/// S6: Exact plans carry the exact target (the decoder owns frame-dropping);
/// index-less clocks plan Exact with land == target.
#[test]
fn s6_exact_plan_carries_target() {
    let clock = cfr_clock();
    let c = clip(1, ticks(0, 1), ticks(10, 1), ticks(0, 1), ticks(1, 1));
    let p = plan_seek(&c, &clock, ticks(3, 2), false).unwrap();
    assert_eq!(p.target_pts, ticks(3, 2));
    assert!(!p.snap);
    assert_eq!(p.land_pts, ticks(0, 1)); // decoder decodes forward from kf 0
                                         // index-less (audio): land == target, not a snap
    let bare = SourceClock::new(ticks(10, 1), vec![]).unwrap();
    let pa = plan_seek(&c, &bare, ticks(3, 2), true).unwrap();
    assert_eq!(pa.land_pts, pa.target_pts);
    assert!(!pa.snap);
}

/// S0: SourceClock re-validates its boundary (sortedness + range) — data
/// crossing crate boundaries is checked, not trusted.
#[test]
fn s0_clock_validates_boundary() {
    assert_eq!(
        SourceClock::new(ticks(10, 1), vec![ticks(2, 1), ticks(2, 1)]),
        Err(SeamError::KeyframesNotSorted)
    );
    assert_eq!(
        SourceClock::new(ticks(10, 1), vec![ticks(10, 1)]),
        Err(SeamError::KeyframeBeyondDuration {
            pts: ticks(10, 1),
            duration: ticks(10, 1),
        })
    );
    // time outside a clip is a typed error, never a panic
    let c = clip(1, ticks(0, 1), ticks(1, 1), ticks(0, 1), ticks(1, 1));
    assert!(matches!(
        timeline_to_source(&c, ticks(5, 1)),
        Err(SeamError::TimeOutsideClip { .. })
    ));
    assert!(matches!(
        source_to_timeline(&c, ticks(5, 1)),
        Err(SeamError::SourceOutsideClip { .. })
    ));
}
