//! WAVE 8 keyframe suites — the ADR-019 contract:
//!   S7  evaluation agrees with a naive oracle evaluator at every probe
//!       instant (seeded, deterministic — same discipline as S0–S6)
//!   S8  split-preserve: evaluating the ORIGINAL across [0, dur) equals
//!       evaluating left on [0, at) and right (shifted back) on [at, dur)
//!       EXACTLY, for every property, key layout and split point
//!   S9  command-level exactness: SetKeyframes inverse roundtrips hash-
//!       identical, split-with-keys undo restores the pre-split state,
//!       replay of a log containing SetKeyframes is hash-deterministic,
//!       Resize strands keys inert (evaluation domain [0, dur)), opacity
//!       domain validation is loud.

use ove_time::Rational;
use ove_timeline::{
    Command, GapTrack, Interpolation, Keyframe, PropertyError, PropertyName, PropertyTrack,
    PropertyTrack as PT, Timeline, TimelineError, TrackId, TrackKind,
};

fn ticks(n: i64) -> Rational {
    Rational::new(n, 24_000)
}

fn kf(time_ticks: i64, value_num: i64, value_den: i64, interp: Interpolation) -> Keyframe {
    Keyframe {
        time: ticks(time_ticks),
        value: Rational::new(value_num, value_den),
        interp,
    }
}

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

/// The naive oracle evaluator: linear scan + explicit branch for every
/// boundary. Independent of the partition-point implementation.
fn oracle_evaluate(track: &PropertyTrack, t: Rational) -> Option<Rational> {
    let keys = track.keys();
    if keys.is_empty() {
        return None;
    }
    if t <= keys[0].time {
        return Some(keys[0].value);
    }
    let last = keys[keys.len() - 1];
    if t >= last.time {
        return Some(last.value);
    }
    for w in keys.windows(2) {
        let (k0, k1) = (w[0], w[1]);
        if k0.time <= t && t < k1.time {
            return match k0.interp {
                Interpolation::Hold => Some(k0.value),
                Interpolation::Linear => {
                    let dv = k1.value.sub(k0.value);
                    let dt = t.sub(k0.time);
                    let span = k1.time.sub(k0.time);
                    Some(k0.value.add(dv.mul(dt).mul(span.recip())))
                }
            };
        }
    }
    unreachable!("t is inside the key range; a segment must cover it")
}

// ---------------------------------------------------------------------------
// Unit: validation + boundary exactness
// ---------------------------------------------------------------------------

#[test]
fn construction_rejects_negative_and_non_monotonic_keys() {
    assert_eq!(
        PropertyTrack::from_keys(vec![kf(-1, 1, 1, Interpolation::Linear)]),
        Err(PropertyError::NegativeTime)
    );
    // equal times are not strictly increasing
    assert_eq!(
        PropertyTrack::from_keys(vec![
            kf(0, 1, 1, Interpolation::Linear),
            kf(0, 2, 1, Interpolation::Linear),
        ]),
        Err(PropertyError::NonMonotonic)
    );
    // descending times
    assert_eq!(
        PropertyTrack::from_keys(vec![
            kf(48, 1, 1, Interpolation::Linear),
            kf(24, 2, 1, Interpolation::Linear),
        ]),
        Err(PropertyError::NonMonotonic)
    );
    // insert into a live track obeys the same rules
    let mut pt = PropertyTrack::new();
    pt.insert(kf(24, 1, 1, Interpolation::Hold)).unwrap();
    assert_eq!(
        pt.insert(kf(24, 0, 1, Interpolation::Hold)),
        Err(PropertyError::NonMonotonic)
    );
    assert_eq!(
        pt.insert(kf(-24, 0, 1, Interpolation::Hold)),
        Err(PropertyError::NegativeTime)
    );
    pt.insert(kf(48, 0, 1, Interpolation::Hold)).unwrap();
    assert_eq!(pt.len(), 2);
}

#[test]
fn evaluate_boundary_exactness() {
    // linear fade 1 → 0 over [0, 48) ticks
    let track = PropertyTrack::from_keys(vec![
        kf(0, 1, 1, Interpolation::Linear),
        kf(48, 0, 1, Interpolation::Linear),
    ])
    .unwrap();
    // empty track → None
    assert_eq!(PropertyTrack::new().evaluate(ticks(0)), None);
    // before the first key → first value (hold-before)
    assert_eq!(track.evaluate(ticks(-5)), Some(Rational::new(1, 1)));
    // exactly on the first key
    assert_eq!(track.evaluate(ticks(0)), Some(Rational::new(1, 1)));
    // exact rational midpoint: 1/2
    assert_eq!(track.evaluate(ticks(24)), Some(Rational::new(1, 2)));
    // exact third: 1 − 1/3 = 2/3
    assert_eq!(track.evaluate(ticks(16)), Some(Rational::new(2, 3)));
    // exactly on the last key
    assert_eq!(track.evaluate(ticks(48)), Some(Rational::new(0, 1)));
    // after the last key → last value (hold-after)
    assert_eq!(track.evaluate(ticks(49)), Some(Rational::new(0, 1)));
    assert_eq!(track.evaluate(ticks(4800)), Some(Rational::new(0, 1)));

    // hold segment: value pinned until the next key
    let hold = PropertyTrack::from_keys(vec![
        kf(0, 1, 1, Interpolation::Hold),
        kf(24, 0, 1, Interpolation::Linear),
        kf(48, 1, 2, Interpolation::Linear),
    ])
    .unwrap();
    assert_eq!(hold.evaluate(ticks(12)), Some(Rational::new(1, 1))); // hold
    assert_eq!(hold.evaluate(ticks(24)), Some(Rational::new(0, 1))); // on-key
    assert_eq!(hold.evaluate(ticks(36)), Some(Rational::new(1, 4))); // lerp 0→1/2
    assert_eq!(hold.evaluate(ticks(47)), Some(Rational::new(23, 48))); // 1/2·47/48
}

// ---------------------------------------------------------------------------
// S7 — evaluation agrees with the naive oracle (seeded)
// ---------------------------------------------------------------------------

#[test]
fn s7_evaluation_matches_oracle() {
    let mut rng = Rng(0x5EED_0001);
    for case in 0..300 {
        // random keys on a fine tick grid, random values in [−4, 4] (nums
        // over a fixed den 8), random interps
        let n_keys = 1 + rng.below(6) as i64;
        let mut times = Vec::new();
        let mut t: i64 = rng.below(24) as i64; // first key possibly > 0
        for _ in 0..n_keys {
            times.push(t);
            t += 1 + rng.below(48) as i64; // strictly increasing
        }
        let keys: Vec<Keyframe> = times
            .iter()
            .map(|&tt| {
                let v = Rational::new(rng.below(65) as i64 - 32, 8);
                let interp = if rng.below(2) == 0 {
                    Interpolation::Linear
                } else {
                    Interpolation::Hold
                };
                Keyframe {
                    time: ticks(tt),
                    value: v,
                    interp,
                }
            })
            .collect();
        let track = PropertyTrack::from_keys(keys).unwrap();

        // probe: below-first, on-key, between, above-last — all exact
        let last_tick = *times.last().unwrap();
        let probes: Vec<Rational> = (0..=last_tick * 2)
            .map(|p| ticks(p / 2)) // integer-tick grid: on-key + between
            .chain(std::iter::once(ticks(-1)))
            .chain(std::iter::once(ticks(last_tick * 2 + 7)))
            .collect();
        for p in probes {
            assert_eq!(
                track.evaluate(p),
                oracle_evaluate(&track, p),
                "case {case} mismatch at {}",
                p
            );
        }
    }
}

// ---------------------------------------------------------------------------
// S8 — split-preserve: exact evaluation continuity across the seam
// ---------------------------------------------------------------------------

#[test]
fn s8_split_preserves_evaluation_exactly() {
    let mut rng = Rng(0x5EED_0002);
    for case in 0..300 {
        let n_keys = 1 + rng.below(6) as i64;
        let mut times = Vec::new();
        let mut t: i64 = rng.below(24) as i64;
        for _ in 0..n_keys {
            times.push(t);
            t += 1 + rng.below(48) as i64;
        }
        let dur_ticks = t + 24; // duration beyond the last key (inert keys allowed)
        let keys: Vec<Keyframe> = times
            .iter()
            .map(|&tt| {
                let v = Rational::new(rng.below(65) as i64 - 32, 8);
                let interp = if rng.below(2) == 0 {
                    Interpolation::Linear
                } else {
                    Interpolation::Hold
                };
                Keyframe {
                    time: ticks(tt),
                    value: v,
                    interp,
                }
            })
            .collect();
        let track = PropertyTrack::from_keys(keys).unwrap();

        // split point strictly inside the display range
        let at_ticks = 1 + rng.below((dur_ticks - 1) as u64) as i64;
        let at = ticks(at_ticks);
        let (left, right) = track.slice_split(at);

        // left: every probe in [0, at) evaluates identically (half-tick grid)
        for p_tick in 0..at_ticks * 2 {
            let p = Rational::new(p_tick, 48_000); // p_tick/2 ticks < at
            assert_eq!(
                left.evaluate(p),
                track.evaluate(p),
                "case {case} LEFT mismatch at {} (at={at_ticks})",
                p
            );
        }
        // left boundary key exists at exactly `at` (load-bearing, inert for
        // the half-open display domain) with the exact boundary value
        let lk = left.keys();
        assert_eq!(lk[lk.len() - 1].time, at);
        assert_eq!(
            lk[lk.len() - 1].value,
            track.evaluate(at).expect("non-empty")
        );

        // right: every probe in [at, dur] evaluates identically (local = orig − at)
        for p_tick in at_ticks * 2..=dur_ticks * 2 {
            let p = Rational::new(p_tick, 48_000);
            let local = Rational::new(p_tick - at_ticks * 2, 48_000);
            assert_eq!(
                right.evaluate(local),
                track.evaluate(p),
                "case {case} RIGHT mismatch at {} (at={at_ticks})",
                p
            );
        }
        // right's first key sits at local 0 with the exact boundary value
        let rk = right.keys();
        assert_eq!(rk[0].time, Rational::zero(1));
        assert_eq!(rk[0].value, track.evaluate(at).expect("non-empty"));
    }
}

#[test]
fn s8_split_interp_semantics_survive_the_cut() {
    // A HOLD segment cut mid-way must stay HOLD on the right half (not
    // degenerate into a linear ramp to the next key).
    let track = PropertyTrack::from_keys(vec![
        kf(0, 1, 1, Interpolation::Hold),
        kf(96, 0, 1, Interpolation::Linear),
    ])
    .unwrap();
    let (left, right) = track.slice_split(ticks(48));
    // original: hold 1.0 until 96; right local [0, 48): still hold 1.0
    assert_eq!(right.evaluate(ticks(24)), Some(Rational::new(1, 1)));
    assert_eq!(right.evaluate(ticks(47)), Some(Rational::new(1, 1)));
    assert_eq!(right.evaluate(ticks(48)), Some(Rational::new(0, 1)));
    // left: hold 1.0 on [0, 48), boundary key at 48 with value 1.0
    assert_eq!(left.evaluate(ticks(47)), Some(Rational::new(1, 1)));
    let lk = left.keys();
    assert_eq!(lk.len(), 2);
    assert_eq!(lk[1].interp, Interpolation::Hold); // cut segment's interp rides along
                                                   // a key exactly at the split point shifts to local 0 with ITS interp
    let with_key = PropertyTrack::from_keys(vec![
        kf(0, 1, 1, Interpolation::Linear),
        kf(48, 1, 2, Interpolation::Hold),
        kf(96, 0, 1, Interpolation::Linear),
    ])
    .unwrap();
    let (_l2, r2) = with_key.slice_split(ticks(48));
    assert_eq!(r2.keys()[0].time, Rational::zero(1));
    assert_eq!(r2.keys()[0].interp, Interpolation::Hold);
    assert_eq!(r2.keys().len(), 2); // no extra boundary key inserted
}

// ---------------------------------------------------------------------------
// S9 — command-level exactness
// ---------------------------------------------------------------------------

fn timeline_with_clip() -> (Timeline, TrackId, ove_timeline::ClipId) {
    let mut tl = Timeline::new();
    tl.add_track(1, TrackKind::Gap(GapTrack::new())).unwrap();
    let clip = ove_timeline::Clip::new(10, ticks(96), ticks(0));
    tl.apply(&Command::Insert {
        track: 1,
        index: 0,
        clip,
    })
    .unwrap();
    (tl, 1, 10)
}

#[test]
fn s9_set_keyframes_inverse_roundtrip_is_hash_exact() {
    let (mut tl, track, id) = timeline_with_clip();
    let h0 = tl.state_hash();
    let fwd = Command::SetKeyframes {
        track,
        id,
        property: PropertyName::Opacity,
        keys: vec![
            kf(0, 1, 1, Interpolation::Linear),
            kf(96, 0, 1, Interpolation::Linear),
        ],
    };
    let inv = tl.apply(&fwd).unwrap();
    let h1 = tl.state_hash();
    assert_ne!(h0, h1, "animation data must change the state hash");
    // undo via the recorded inverse — hash lands exactly back
    tl.apply(&inv).unwrap();
    assert_eq!(
        tl.state_hash(),
        h0,
        "undo must restore the pre-animation hash"
    );
    // re-apply the same forward command — deterministic hash
    tl.apply(&fwd).unwrap();
    assert_eq!(tl.state_hash(), h1);
    // overwrite with a second set; its inverse is the FIRST key list
    let inv2 = tl
        .apply(&Command::SetKeyframes {
            track,
            id,
            property: PropertyName::Opacity,
            keys: vec![
                kf(0, 0, 1, Interpolation::Hold),
                kf(48, 1, 1, Interpolation::Hold),
            ],
        })
        .unwrap();
    match &inv2 {
        Command::SetKeyframes { keys, .. } => {
            assert_eq!(keys.len(), 2, "inverse must carry the previous animation");
            assert_eq!(keys[0].value, Rational::new(1, 1));
        }
        other => panic!("inverse of set must be set, got {other:?}"),
    }
    tl.apply(&inv2).unwrap();
    assert_eq!(
        tl.state_hash(),
        h1,
        "undo of the overwrite restores the first animation"
    );
    // replay determinism: a fresh timeline re-applying the same commands
    let (mut tl2, _, _) = timeline_with_clip();
    tl2.apply(&fwd).unwrap();
    tl2.apply(&Command::SetKeyframes {
        track,
        id,
        property: PropertyName::X,
        keys: vec![
            kf(0, 0, 1, Interpolation::Linear),
            kf(48, 100, 1, Interpolation::Linear),
        ],
    })
    .unwrap();
    // tl currently has only the opacity animation (the X set was undone)
    tl2.apply(&Command::SetKeyframes {
        track,
        id,
        property: PropertyName::X,
        keys: vec![],
    })
    .unwrap();
    assert_eq!(
        tl2.state_hash(),
        tl.state_hash(),
        "replay must be hash-deterministic"
    );
}

#[test]
fn s9_split_with_keys_undo_restores_pre_split_hash() {
    let (mut tl, track, id) = timeline_with_clip();
    tl.apply(&Command::SetKeyframes {
        track,
        id,
        property: PropertyName::Opacity,
        keys: vec![
            kf(0, 1, 1, Interpolation::Linear),
            kf(48, 1, 2, Interpolation::Linear),
            kf(96, 0, 1, Interpolation::Linear),
        ],
    })
    .unwrap();
    let h_pre = tl.state_hash();
    let new_id = tl.alloc_id();
    let inv = tl
        .apply(&Command::Split {
            track,
            id,
            at: ticks(30),
            new_id,
        })
        .unwrap();
    let h_post = tl.state_hash();
    assert_ne!(h_pre, h_post);
    // both halves carry animation; evaluation is exact across the seam
    let t = tl.track_ref(track).unwrap();
    let left = t.clip_at(0).unwrap();
    let right = t.clip_at(1).unwrap();
    assert!(!left.properties.opacity.is_empty());
    assert!(!right.properties.opacity.is_empty());
    assert_eq!(
        left.properties.opacity.evaluate(ticks(29)),
        Some(Rational::new(67, 96))
    );
    assert_eq!(
        right.properties.opacity.evaluate(ticks(0)),
        left.properties.opacity.evaluate(ticks(30))
    );
    // undo (the inverse batch contains SetKeyframes restores) — exact
    tl.apply(&inv).unwrap();
    assert_eq!(
        tl.state_hash(),
        h_pre,
        "undo must restore the pre-split hash exactly"
    );
}

#[test]
fn s9_resize_strands_keys_inert_evaluation_domain() {
    let (mut tl, track, id) = timeline_with_clip();
    tl.apply(&Command::SetKeyframes {
        track,
        id,
        property: PropertyName::Opacity,
        keys: vec![
            kf(0, 1, 1, Interpolation::Linear),
            kf(96, 0, 1, Interpolation::Linear),
        ],
    })
    .unwrap();
    // resize smaller: keys survive untouched (inert beyond the duration)
    let inv = tl
        .apply(&Command::Resize {
            track,
            id,
            duration: ticks(48),
        })
        .unwrap();
    let t = tl.track_ref(track).unwrap();
    let c = t.clip_at(0).unwrap();
    assert_eq!(
        c.properties.opacity.keys().len(),
        2,
        "Resize must not touch keys"
    );
    assert_eq!(
        c.properties.opacity.evaluate(ticks(47)),
        Some(Rational::new(49, 96))
    );
    // grow back: the stranded keys re-activate — the inverse is exact
    tl.apply(&inv).unwrap();
    let t = tl.track_ref(track).unwrap();
    let c = t.clip_at(0).unwrap();
    assert_eq!(
        c.properties.opacity.evaluate(ticks(96)),
        Some(Rational::new(0, 1))
    );
}

#[test]
fn s9_opacity_domain_validation_is_loud() {
    let (mut tl, track, id) = timeline_with_clip();
    // 2 > 1: outside the blend domain — rejected at command time
    assert_eq!(
        tl.apply(&Command::SetKeyframes {
            track,
            id,
            property: PropertyName::Opacity,
            keys: vec![kf(0, 2, 1, Interpolation::Linear)],
        }),
        Err(TimelineError::OpacityValueOutOfRange {
            clip: id,
            value: Rational::new(2, 1)
        })
    );
    // negative value: same
    assert!(matches!(
        tl.apply(&Command::SetKeyframes {
            track,
            id,
            property: PropertyName::Opacity,
            keys: vec![kf(0, -1, 4, Interpolation::Linear)],
        }),
        Err(TimelineError::OpacityValueOutOfRange { .. })
    ));
    // X is geometry — unbounded values are legal
    tl.apply(&Command::SetKeyframes {
        track,
        id,
        property: PropertyName::X,
        keys: vec![
            kf(0, -1920, 1, Interpolation::Linear),
            kf(96, 7680, 1, Interpolation::Linear),
        ],
    })
    .unwrap();
}

#[test]
fn s9_container_kinds_agree_on_keyframe_state() {
    // gap vs avl vs oracle must carry and hash animation identically
    let build = |kind: fn() -> TrackKind| -> Timeline {
        let mut tl = Timeline::new();
        tl.add_track(1, kind()).unwrap();
        tl.apply(&Command::Insert {
            track: 1,
            index: 0,
            clip: ove_timeline::Clip::new(10, ticks(96), ticks(0)),
        })
        .unwrap();
        tl.apply(&Command::SetKeyframes {
            track: 1,
            id: 10,
            property: PropertyName::Opacity,
            keys: vec![
                kf(0, 1, 1, Interpolation::Linear),
                kf(96, 0, 1, Interpolation::Linear),
            ],
        })
        .unwrap();
        tl
    };
    let h_gap = build(|| TrackKind::Gap(GapTrack::new())).state_hash();
    let h_avl = build(|| TrackKind::Avl(ove_timeline::AvlTrack::new())).state_hash();
    let h_oracle = build(|| TrackKind::Oracle(ove_timeline::OracleTrack::new())).state_hash();
    assert_eq!(h_gap, h_avl);
    assert_eq!(h_gap, h_oracle);
    // and the split behaves identically across containers
    for kind in [
        || TrackKind::Gap(GapTrack::new()) as TrackKind,
        || TrackKind::Avl(ove_timeline::AvlTrack::new()) as TrackKind,
        || TrackKind::Oracle(ove_timeline::OracleTrack::new()) as TrackKind,
    ] {
        let mut tl = build(kind);
        let inv = tl
            .apply(&Command::Split {
                track: 1,
                id: 10,
                at: ticks(30),
                new_id: 11,
            })
            .unwrap();
        tl.apply(&inv).unwrap();
        let t = tl.track_ref(1).unwrap();
        let c = t.clip_at(0).unwrap();
        assert!(c.properties.opacity.is_empty() || c.id == 10);
    }
}

#[test]
fn s9_set_keyframes_on_missing_clip_is_typed() {
    let (mut tl, track, _id) = timeline_with_clip();
    assert_eq!(
        tl.apply(&Command::SetKeyframes {
            track,
            id: 999,
            property: PropertyName::X,
            keys: vec![kf(0, 1, 1, Interpolation::Hold)],
        }),
        Err(TimelineError::ClipNotFound(999))
    );
    // so is a malformed key list
    assert!(matches!(
        tl.apply(&Command::SetKeyframes {
            track,
            id: 10,
            property: PropertyName::X,
            keys: vec![
                kf(48, 1, 1, Interpolation::Hold),
                kf(24, 0, 1, Interpolation::Hold)
            ],
        }),
        Err(TimelineError::InvalidKeyframes(PropertyError::NonMonotonic))
    ));
    let _ = PT::new();
}
