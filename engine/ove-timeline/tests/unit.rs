//! Unit tests: verb semantics, exact inverses, error model (ENGINE_BUILD_PLAN
//! Wave 1 acceptance, plus DECODER-spec-style honesty: errors are typed, never
//! panics, never silent).

use ove_time::Rational;
use ove_timeline::*;

const R: (i64, i64) = (24000, 1); // NTSC-family tick axis for tests

fn r(num: i64, den: i64) -> Rational {
    Rational::new(num, den)
}

fn engine2() -> TimelineEngine {
    TimelineEngine::new(TimelineState::new(&[
        TrackKind::Video,
        TrackKind::Video,
        TrackKind::Audio,
    ]))
}

fn add(id: Option<ClipId>, track: u32, start_num: i64, dur: i64) -> Command {
    Command::AddClip {
        id,
        track: TrackId(track),
        asset: AssetId(1),
        start: r(start_num, R.0),
        dur: r(dur, R.0),
        src_in: Rational::zero(R.0),
        speed: r(1, 1),
        label: format!("c{}", start_num),
    }
}

#[test]
fn add_remove_move_resize_exact() {
    let mut e = engine2();
    let rec = e.apply(add(None, 0, 0, 48000)).unwrap(); // [0s..2s]
    let c1 = rec.assigned[0];
    e.apply(add(None, 0, 48000, 24000)).unwrap(); // [2s..3s]
    assert_eq!(e.state.clip_count(), 2);
    assert_eq!(e.state.track_duration(TrackId(0)), r(72000, R.0));

    // move first clip into the gap after the second: [72000..120000)
    e.apply(Command::MoveClip {
        id: c1,
        track: TrackId(0),
        start: r(72000, R.0),
    })
    .unwrap();
    assert_eq!(e.state.clip(c1).unwrap().start, r(72000, R.0));
    assert_eq!(e.state.track_duration(TrackId(0)), r(120000, R.0));

    // resize to half duration: c1 end 96000, c2 end 72000 → track 96000
    e.apply(Command::ResizeClip {
        id: c1,
        dur: r(24000, R.0),
    })
    .unwrap();
    assert_eq!(e.state.clip(c1).unwrap().dur, r(24000, R.0));
    assert_eq!(e.state.track_duration(TrackId(0)), r(96000, R.0));

    // remove both
    e.apply(Command::RemoveClip { id: c1 }).unwrap();
    assert_eq!(e.state.clip_count(), 1);
}

#[test]
fn cross_track_move() {
    let mut e = engine2();
    let rec = e.apply(add(None, 0, 0, 24000)).unwrap();
    let c = rec.assigned[0];
    assert_eq!(e.state.track_kind(TrackId(2)), Some(TrackKind::Audio));
    e.apply(Command::MoveClip {
        id: c,
        track: TrackId(2),
        start: r(0, R.0),
    })
    .unwrap();
    assert_eq!(e.state.clip(c).unwrap().track, TrackId(2));
    assert_eq!(e.state.clips_on_track(TrackId(0)).count(), 0);
    assert_eq!(e.state.clips_on_track(TrackId(2)).count(), 1);
    // undo: back to track 0
    e.undo().unwrap();
    assert_eq!(e.state.clip(c).unwrap().track, TrackId(0));
}

#[test]
fn split_semantics_exact_source_mapping() {
    let mut e = engine2();
    // 4s clip, speed 1/2 → consumes 8s of source starting at 10s
    let rec = e
        .apply(Command::AddClip {
            id: None,
            track: TrackId(0),
            asset: AssetId(7),
            start: r(0, R.0),
            dur: r(96000, R.0),
            src_in: r(10 * R.0, R.0),
            speed: r(1, 2),
            label: "src".into(),
        })
        .unwrap();
    let c = rec.assigned[0];

    // split at timeline 1s (offset 24000 ticks = 1/4 of clip)
    let rec2 = e
        .apply(Command::SplitClip {
            id: c,
            offset: r(24000, R.0),
            new_id: None,
        })
        .unwrap();
    let c2 = rec2.assigned[0];

    let left = e.state.clip(c).unwrap();
    let right = e.state.clip(c2).unwrap();
    // left: [0..1s) at speed 1/2 → src span 0.5s: [10s..10.5s)
    assert_eq!(left.dur, r(24000, R.0));
    assert_eq!(left.src_dur(), r(12000, R.0));
    assert_eq!(left.src_at(left.dur), r(252000, R.0)); // 10s + 0.5s
                                                       // right: starts 1s, dur 3s, src_in = 10.5s (exact: src_in + offset*speed)
    assert_eq!(right.start, r(24000, R.0));
    assert_eq!(right.dur, r(72000, R.0));
    assert_eq!(right.src_in, r(252000, R.0));
    assert_eq!(right.src_dur(), r(36000, R.0)); // 1.5s source
    assert_eq!(right.speed, r(1, 2));
    // timeline coverage unchanged
    assert_eq!(e.state.track_duration(TrackId(0)), r(96000, R.0));

    // undo split → hash-exact merge
    let h_split = e.state.state_hash();
    e.undo().unwrap();
    let merged = e.state.clip(c).unwrap();
    assert_eq!(merged.dur, r(96000, R.0));
    assert_eq!(merged.src_in, r(10 * R.0, R.0));
    assert_eq!(e.state.clip_count(), 1);
    // redo → same partition (deterministic id assignment)
    let h_before_redo = e.state.state_hash();
    e.redo().unwrap();
    assert_eq!(e.state.state_hash(), h_split);
    assert_eq!(e.state.clip_count(), 2);
    let _ = h_before_redo;
}

#[test]
fn retime_preserves_source_range_derives_duration() {
    let mut e = engine2();
    // 2s at speed 1 → src range 2s
    let rec = e.apply(add(None, 0, 0, 48000)).unwrap();
    let c = rec.assigned[0];
    // retime to 2× → duration halves exactly, src range preserved
    e.apply(Command::RetimeClip {
        id: c,
        speed: r(2, 1),
    })
    .unwrap();
    let cl = e.state.clip(c).unwrap();
    assert_eq!(cl.dur, r(24000, R.0));
    assert_eq!(cl.src_dur(), r(48000, R.0)); // 24000 * 2
                                             // retime to 1/2× → duration doubles, src range still 2s
    e.apply(Command::RetimeClip {
        id: c,
        speed: r(1, 2),
    })
    .unwrap();
    let cl = e.state.clip(c).unwrap();
    assert_eq!(cl.dur, r(96000, R.0));
    assert_eq!(cl.src_dur(), r(48000, R.0));
}

#[test]
fn error_model_typed_no_partial_state() {
    let mut e = engine2();
    let rec = e.apply(add(None, 0, 0, 48000)).unwrap();
    let c = rec.assigned[0];
    let h0 = e.state.state_hash();
    let j0 = e.journal().len();

    // overlap rejection
    match e.apply(add(None, 0, 24000, 48000)) {
        Err(TimelineError::Overlap { existing, .. }) => assert_eq!(existing, c),
        other => panic!("expected Overlap, got {:?}", other),
    }
    // zero duration
    assert!(matches!(
        e.apply(Command::ResizeClip {
            id: c,
            dur: Rational::zero(1)
        }),
        Err(TimelineError::InvalidDuration(_))
    ));
    // unknown clip/track
    assert!(matches!(
        e.apply(Command::RemoveClip { id: ClipId(999) }),
        Err(TimelineError::UnknownClip(_))
    ));
    assert!(matches!(
        e.apply(add(None, 9, 0, 1000)),
        Err(TimelineError::UnknownTrack(_))
    ));
    // split out of range (at 0, at dur, beyond)
    assert!(matches!(
        e.apply(Command::SplitClip {
            id: c,
            offset: Rational::zero(1),
            new_id: None
        }),
        Err(TimelineError::InvalidSplit { .. })
    ));
    assert!(matches!(
        e.apply(Command::SplitClip {
            id: c,
            offset: r(48000, R.0),
            new_id: None
        }),
        Err(TimelineError::InvalidSplit { .. })
    ));
    // zero/negative speed
    assert!(matches!(
        e.apply(Command::RetimeClip {
            id: c,
            speed: Rational::zero(1)
        }),
        Err(TimelineError::InvalidSpeed(_))
    ));
    // explicit id reuse
    assert!(matches!(
        e.apply(add(Some(c), 1, 0, 1000)),
        Err(TimelineError::IdInUse(_))
    ));

    // NOTHING changed: state hash and journal length untouched
    assert_eq!(e.state.state_hash(), h0);
    assert_eq!(e.journal().len(), j0);

    // undo/redo unavailable on a fresh engine (typed errors, not panics)
    let mut fresh = engine2();
    assert!(matches!(fresh.undo(), Err(TimelineError::UndoUnavailable)));
    assert!(matches!(fresh.redo(), Err(TimelineError::RedoUnavailable)));
}

#[test]
fn batch_atomic_all_or_nothing() {
    let mut e = engine2();
    let h0 = e.state.state_hash();
    let j0 = e.journal().len();

    // valid + valid + INVALID: whole batch must roll back
    let batch = Command::Batch {
        cmds: vec![
            add(None, 0, 0, 24000),
            add(None, 0, 24000, 24000),
            add(None, 0, 0, 48000),
        ],
        label: "setup".into(),
    };
    match e.apply(batch) {
        Err(TimelineError::BatchRolledBack { applied, .. }) => assert_eq!(applied, 2),
        other => panic!("expected BatchRolledBack, got {:?}", other),
    }
    assert_eq!(
        e.state.state_hash(),
        h0,
        "batch rollback must restore state exactly"
    );
    assert_eq!(e.state.clip_count(), 0);
    assert_eq!(
        e.journal().len(),
        j0,
        "failed batch must not touch the journal"
    );

    // valid batch applies as ONE unit; its inverse is the composite
    let batch2 = Command::Batch {
        cmds: vec![add(None, 0, 0, 24000), add(None, 0, 24000, 24000)],
        label: "pair".into(),
    };
    let h_before = e.state.state_hash();
    e.apply(batch2).unwrap();
    assert_eq!(e.state.clip_count(), 2);
    e.undo().unwrap(); // composite inverse removes BOTH
    assert_eq!(e.state.clip_count(), 0);
    assert_eq!(e.state.state_hash(), h_before);
}

#[test]
fn undo_redo_chain_hash_exact() {
    let mut e = engine2();
    let h_init = e.state.state_hash();
    let mut recs = vec![];
    for i in 0..8 {
        recs.push(
            e.apply(add(None, (i % 2) as u32, i * 24000, 24000))
                .unwrap()
                .assigned[0],
        );
    }
    let h_full = e.state.state_hash();
    // undo all → hash-exact initial state
    for _ in 0..8 {
        e.undo().unwrap();
    }
    assert_eq!(e.state.state_hash(), h_init);
    assert_eq!(e.state.clip_count(), 0);
    // redo all → hash-exact full state
    for _ in 0..8 {
        e.redo().unwrap();
    }
    assert_eq!(e.state.state_hash(), h_full);
    // partial undo then NEW command kills the redo stack
    e.undo().unwrap();
    assert_eq!(e.redo_depth(), 1);
    e.apply(add(None, 0, 999000, 24000)).unwrap();
    assert_eq!(e.redo_depth(), 0, "new command must clear redo stack");
    let _ = recs;
}

#[test]
fn replay_matches_apply_and_verifies_hashes() {
    let mut e = engine2();
    for i in 0..12 {
        e.apply(add(None, (i % 3) as u32, i * 12000, 24000))
            .unwrap();
        if i % 4 == 3 {
            e.undo().unwrap();
            e.redo().unwrap();
        }
    }
    let live_hash = e.state.state_hash();
    let journal = e.journal().to_vec();
    let mut e2 = TimelineEngine::replay(
        &[TrackKind::Video, TrackKind::Video, TrackKind::Audio],
        &journal,
    )
    .unwrap();
    assert_eq!(
        e2.state.state_hash(),
        live_hash,
        "replay must equal apply (E-003 P1)"
    );
    assert_eq!(e2.state.clip_count(), e.state.clip_count());
    // replayed engine keeps undo/redo working identically (E-009 marker replay)
    e2.undo().unwrap();
    let h2 = e2.state.state_hash();
    e.undo().unwrap();
    assert_eq!(
        e.state.state_hash(),
        h2,
        "undo after replay must match live undo"
    );
}

#[test]
fn hit_test_and_gaps() {
    let mut e = engine2();
    e.apply(add(None, 0, 0, 24000)).unwrap(); // [0..1s)
    e.apply(add(None, 0, 48000, 24000)).unwrap(); // [2..3s), gap [1..2s)
    assert_eq!(e.state.hit_test(TrackId(0), r(0, R.0)), Some(ClipId(1)));
    assert_eq!(e.state.hit_test(TrackId(0), r(23999, R.0)), Some(ClipId(1)));
    assert_eq!(
        e.state.hit_test(TrackId(0), r(24000, R.0)),
        None,
        "gap is not a hit"
    );
    assert_eq!(e.state.hit_test(TrackId(0), r(47999, R.0)), None);
    assert_eq!(e.state.hit_test(TrackId(0), r(48000, R.0)), Some(ClipId(2)));
    assert_eq!(e.state.hit_test(TrackId(0), r(71999, R.0)), Some(ClipId(2)));
    assert_eq!(
        e.state.hit_test(TrackId(0), r(72000, R.0)),
        None,
        "end is exclusive"
    );
    // touching clips are legal and hit-tested exactly
    e.apply(add(None, 0, 24000, 24000)).unwrap(); // fills the gap
    assert_eq!(e.state.hit_test(TrackId(0), r(24000, R.0)), Some(ClipId(3)));
    assert_eq!(e.state.hit_test(TrackId(0), r(23999, R.0)), Some(ClipId(1)));
}

#[test]
fn hash_excludes_id_counter_and_is_order_canonical() {
    // two engines; A assigns id 1 then removes it, then adds (id 2 assigned).
    // B adds one clip directly (id 1 assigned). Same CONTENT, different counters
    // and different id numerals — hash is content-canonical: A's clip has id 2,
    // B's has id 1, so hashes differ by id bytes... hence build B to also have
    // two clips with ids {1 removed, 2 present} via explicit ids to prove the
    // COUNTER is not hashed: A counter=2, B counter=1, same content+ids → equal.
    let mut a = engine2();
    let _ = a.apply(add(None, 0, 0, 24000)).unwrap(); // id 1, counter → 2
    a.apply(Command::RemoveClip { id: ClipId(1) }).unwrap();
    a.apply(add(Some(ClipId(7)), 0, 0, 24000)).unwrap(); // explicit, counter stays 2

    let mut b = engine2();
    b.apply(add(Some(ClipId(7)), 0, 0, 24000)).unwrap(); // explicit, counter → 1

    assert_eq!(a.state.clip_count(), 1);
    assert_eq!(b.state.clip_count(), 1);
    assert_eq!(
        a.state.state_hash(),
        b.state.state_hash(),
        "counter must not affect hash"
    );
}
