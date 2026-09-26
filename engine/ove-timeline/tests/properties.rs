//! ove-timeline property tests (E-012 / doc 45 layer L2).
//!
//! Master invariants, enforced per ADR-009's own risk mitigation and
//! doc 45 rule 1 (no verb lands without its property triple):
//!   P1  apply(cmd) → apply(inverse) == identity state (per verb, both impls)
//!   P2  oracle equivalence: Gap == Avl == Oracle under identical randomized
//!       op scripts (hash/count/total)
//!   P3  replay determinism: apply(log) == replay(fresh, log) (hash)
//!   P4  split conservation + unsplit exactness (Rational equality)
//!   P5  hit-test total coverage vs oracle linear scan (exact boundaries)
//!   P6  batch atomicity: failure rolls back to pre-batch state
//!   P7  cross-track move: exactly-one-id invariant, counts correct
//!   P8  threaded disjoint-track storms: per-track hash == sequential
//!
//! Seeded loops: fixed seeds for CI; on failure the seed and step are
//! printed so the case is reproducible (doc 45 L2 policy).

use ove_time::Rational;
use ove_timeline::{
    AvlTrack, Clip, Command, GapTrack, OracleTrack, Timeline, TimelineError, TrackKind, TrackOps,
    UndoStack, TICK_DEN,
};

fn ticks(n: i64) -> Rational {
    Rational::new(n, TICK_DEN)
}

struct Rng(u64);
impl Rng {
    fn new(seed: u64) -> Self {
        Rng(seed ^ 0x9E37_79B9_7F4A_7C15)
    }
    fn next(&mut self) -> u64 {
        self.0 ^= self.0 << 13;
        self.0 ^= self.0 >> 7;
        self.0 ^= self.0 << 17;
        self.0
    }
    fn below(&mut self, n: usize) -> usize {
        (self.next() % n as u64) as usize
    }
}

fn kind_gap() -> TrackKind {
    TrackKind::Gap(GapTrack::new())
}
fn kind_avl() -> TrackKind {
    TrackKind::Avl(AvlTrack::new())
}
fn kind_oracle() -> TrackKind {
    TrackKind::Oracle(OracleTrack::new())
}

type KindCtor = (&'static str, fn() -> TrackKind);
const KINDS: &[KindCtor] = &[
    ("gap", kind_gap),
    ("avl", kind_avl),
    ("oracle", kind_oracle),
];

/// Build a timeline with one track of n seeded clips; ids 1..=n.
fn seeded_timeline(kind: fn() -> TrackKind, seed: u64, n: usize) -> (Timeline, Vec<u64>) {
    let mut rng = Rng::new(seed);
    let mut tl = Timeline::new();
    tl.add_track(1, kind()).unwrap();
    let mut ids = Vec::with_capacity(n);
    for i in 0..n {
        let id = tl.alloc_id();
        ids.push(id);
        let d = 481 + (rng.next() % 240_000) as i64;
        tl.apply(&Command::Insert {
            track: 1,
            index: i,
            clip: Clip {
                id,
                duration: ticks(d),
                source_in: ticks(0),
            },
        })
        .unwrap();
    }
    (tl, ids)
}

/// Generate one valid random command against the given timeline state.
/// Returns the command; generation mirrors state in a tiny shadow so
/// commands are always valid (validation errors are tested separately).
fn random_command(rng: &mut Rng, tl: &mut Timeline, track: u64) -> Option<Command> {
    let tlen = tl.track_len(track).unwrap();
    if tlen == 0 {
        // only Insert possible
        let id = tl.alloc_id();
        return Some(Command::Insert {
            track,
            index: 0,
            clip: Clip {
                id,
                duration: ticks(481 + (rng.next() % 240_000) as i64),
                source_in: ticks(0),
            },
        });
    }
    match rng.below(5) {
        0 => {
            // Split a random clip at a random interior tick
            let t = tl.track_ref(track).unwrap();
            let pos = rng.below(tlen);
            let c = t.clip_at(pos).unwrap().clone();
            let dt = c.duration.ticks_at(24000, 1);
            if dt < 3 {
                return None;
            }
            let at = 1 + (rng.next() % (dt - 1) as u64) as i64;
            let new_id = tl.alloc_id();
            Some(Command::Split {
                track,
                id: c.id,
                at: ticks(at),
                new_id,
            })
        }
        1 => {
            let t = tl.track_ref(track).unwrap();
            let pos = rng.below(tlen);
            let c = t.clip_at(pos).unwrap().clone();
            Some(Command::Resize {
                track,
                id: c.id,
                duration: ticks(481 + (rng.next() % 240_000) as i64),
            })
        }
        2 => {
            // Remove a random clip
            let t = tl.track_ref(track).unwrap();
            let pos = rng.below(tlen);
            let c = t.clip_at(pos).unwrap().clone();
            Some(Command::Remove { track, id: c.id })
        }
        3 => {
            // Insert a fresh clip at a random index
            let id = tl.alloc_id();
            Some(Command::Insert {
                track,
                index: rng.below(tlen + 1),
                clip: Clip {
                    id,
                    duration: ticks(481 + (rng.next() % 240_000) as i64),
                    source_in: ticks(0),
                },
            })
        }
        _ => {
            // Move within / across tracks
            let from = track;
            let t = tl.track_ref(from).unwrap();
            let pos = rng.below(tlen);
            let c = t.clip_at(pos).unwrap().clone();
            let to = from; // same-track reorder keeps the single-track property loop simple
            let dest_len = tl.track_len(to).unwrap() - 1;
            Some(Command::Move {
                id: c.id,
                from_track: from,
                to_track: to,
                to_index: rng.below(dest_len + 1),
            })
        }
    }
}

/// P1: apply → inverse → identity, for every verb, per implementation.
#[test]
fn p1_verb_inverse_identity() {
    for &(name, kind) in KINDS {
        let (mut tl, _ids) = seeded_timeline(kind, 0x0E12_0001, 64);
        let mut rng = Rng::new(0x0E12_0001);
        for step in 0..300 {
            let before = tl.state_hash();
            let cmd = match random_command(&mut rng, &mut tl, 1) {
                Some(c) => c,
                None => continue,
            };
            let inv = tl
                .apply(&cmd)
                .unwrap_or_else(|e| panic!("[{name} step {step}] apply failed: {e:?}"));
            let after = tl.state_hash();
            tl.apply(&inv)
                .unwrap_or_else(|e| panic!("[{name} step {step}] inverse failed: {e:?}"));
            assert_eq!(
                tl.state_hash(),
                before,
                "[{name} step {step}] inverse did not restore state\nop: {cmd:?}"
            );
            // and applying again reproduces the same post-state (determinism)
            tl.apply(&cmd).unwrap();
            assert_eq!(
                tl.state_hash(),
                after,
                "[{name} step {step}] re-apply diverged\nop: {cmd:?}"
            );
        }
    }
}

/// P1b: undo/redo stack round-trips (ADR-009 LIFO, redo branch).
#[test]
fn p1b_undo_redo_stack() {
    for &(name, kind) in KINDS {
        let (mut tl, _ids) = seeded_timeline(kind, 0x0E12_000B, 50);
        let mut rng = Rng::new(0x0E12_000B);
        let mut stack = UndoStack::new();
        let h0 = tl.state_hash();
        let mut hashes = vec![h0];
        for _ in 0..100 {
            if let Some(cmd) = random_command(&mut rng, &mut tl, 1) {
                let inv = tl.apply(&cmd).unwrap();
                stack.record(inv);
                hashes.push(tl.state_hash());
            }
        }
        // undo all the way back (hashes.len()-1 undos: last applied first)
        for i in (0..hashes.len() - 1).rev() {
            stack.undo(&mut tl).unwrap();
            assert_eq!(
                tl.state_hash(),
                hashes[i],
                "[{name}] undo step {i} hash mismatch"
            );
        }
        assert!(
            !stack.undo(&mut tl).unwrap(),
            "[{name}] undo stack should be empty"
        );
        // redo all the way forward
        for (i, want) in hashes.iter().enumerate().skip(1) {
            stack.redo(&mut tl).unwrap();
            assert_eq!(
                tl.state_hash(),
                *want,
                "[{name}] redo step {i} hash mismatch"
            );
        }
    }
}

/// P2: oracle equivalence under one shared randomized script (all verbs).
#[test]
fn p2_oracle_equivalence() {
    // generate + validate the script on an oracle timeline first
    let mut rng = Rng::new(0x0E12_0002);
    let mut gen_tl = {
        let mut tl = Timeline::new();
        tl.add_track(1, kind_oracle()).unwrap();
        for i in 0..80 {
            let id = tl.alloc_id();
            tl.apply(&Command::Insert {
                track: 1,
                index: i,
                clip: Clip {
                    id,
                    duration: ticks(481 + (rng.next() % 240_000) as i64),
                    source_in: ticks(0),
                },
            })
            .unwrap();
        }
        tl
    };
    let mut script: Vec<Command> = Vec::new();
    for _ in 0..400 {
        if let Some(c) = random_command(&mut rng, &mut gen_tl, 1) {
            // apply to the generator state so the next command is valid
            if gen_tl.apply(&c).is_ok() {
                script.push(c);
            }
        }
    }
    let want = gen_tl.state_hash();

    for &(name, kind) in KINDS {
        let mut tl = Timeline::new();
        tl.add_track(1, kind()).unwrap();
        // replay the same Insert prologue then the script
        let mut rng2 = Rng::new(0x0E12_0002);
        for i in 0..80 {
            let id = i as u64 + 1;
            tl.apply(&Command::Insert {
                track: 1,
                index: i,
                clip: Clip {
                    id,
                    duration: ticks(481 + (rng2.next() % 240_000) as i64),
                    source_in: ticks(0),
                },
            })
            .unwrap();
        }
        for (step, cmd) in script.iter().enumerate() {
            // ids inside script reference generator ids; skip Split (fresh ids
            // differ per run order) by re-running the SAME command — Split's
            // right-half id comes from the timeline counter which follows the
            // same sequence given the same command prefix.
            tl.apply(cmd).unwrap_or_else(|e| {
                panic!("[{name} step {step}] replay failed: {e:?}\ncmd: {cmd:?}")
            });
        }
        assert_eq!(
            tl.state_hash(),
            want,
            "[{name}] final state diverged from oracle"
        );
    }
}

/// P3: replay determinism — apply(log) == replay(fresh, log).
#[test]
fn p3_replay_determinism() {
    for &(name, kind) in KINDS {
        let (mut tl, _) = seeded_timeline(kind, 0x0E12_0003, 40);
        let mut rng = Rng::new(0x0E12_0003 + 7);
        let mut log = Vec::new();
        for _ in 0..250 {
            if let Some(c) = random_command(&mut rng, &mut tl, 1) {
                if tl.apply(&c).is_ok() {
                    log.push(c);
                }
            }
        }
        let h1 = tl.state_hash();
        // replay on a fresh timeline with the same kind AND the same prologue
        let (mut tl2, _) = seeded_timeline(kind, 0x0E12_0003, 40);
        for cmd in &log {
            tl2.apply(cmd)
                .unwrap_or_else(|e| panic!("[{name}] replay diverged: {e:?}"));
        }
        assert_eq!(tl2.state_hash(), h1, "[{name}] replay hash != applied hash");
    }
}

/// P4: split conservation — exact Rational arithmetic, exact restore.
#[test]
fn p4_split_conservation() {
    for &(name, kind) in KINDS {
        let (mut tl, ids) = seeded_timeline(kind, 0x0E12_0004, 20);
        let mut rng = Rng::new(0x0E12_0004);
        for step in 0..100 {
            let tlen = tl.track_len(1).unwrap();
            let pos = rng.below(tlen);
            let orig = tl.track_ref(1).unwrap().clip_at(pos).unwrap().clone();
            let dt = orig.duration.ticks_at(24000, 1);
            if dt < 3 {
                continue;
            }
            let at = 1 + (rng.next() % (dt - 1) as u64) as i64;
            let at_r = ticks(at);
            let new_id = tl.alloc_id();
            let inv = tl
                .apply(&Command::Split {
                    track: 1,
                    id: orig.id,
                    at: at_r,
                    new_id,
                })
                .unwrap();
            // left keeps id, duration == at
            let left_pos = tl.track_ref(1).unwrap().index_of(orig.id).unwrap();
            let left = tl.track_ref(1).unwrap().clip_at(left_pos).unwrap().clone();
            assert_eq!(
                left.duration, at_r,
                "[{name} step {step}] left duration != at"
            );
            // right: source_in advanced exactly by at; duration conserved
            let right = tl
                .track_ref(1)
                .unwrap()
                .clip_at(left_pos + 1)
                .unwrap()
                .clone();
            assert_eq!(
                left.duration.add(right.duration),
                orig.duration,
                "[{name} step {step}] split not conserved"
            );
            assert_eq!(
                right.source_in,
                orig.source_in.add(at_r),
                "[{name} step {step}] source_in not advanced exactly"
            );
            // inverse restores the exact original record
            tl.apply(&inv).unwrap();
            let restored_pos = tl.track_ref(1).unwrap().index_of(orig.id).unwrap();
            let restored = tl
                .track_ref(1)
                .unwrap()
                .clip_at(restored_pos)
                .unwrap()
                .clone();
            assert_eq!(restored, orig, "[{name} step {step}] unsplit not exact");
            let _ = ids;
        }
    }
}

/// P5: hit-test total coverage vs oracle linear scan; exact boundaries.
#[test]
fn p5_hit_test_coverage() {
    for &(name, kind) in KINDS {
        let (mut tl, _) = seeded_timeline(kind, 0x0E12_0005, 60);
        // make some edits so starts are non-trivial
        let mut rng = Rng::new(0x0E12_0005);
        for _ in 0..30 {
            if let Some(c) = random_command(&mut rng, &mut tl, 1) {
                let _ = tl.apply(&c);
            }
        }
        let t = tl.track_mut(1).unwrap();
        // oracle scan on the same content
        let mut oracle: Vec<(Rational, Rational, u64)> = Vec::new(); // (start, dur, id)
        t.walk(&mut |_p, s, c| oracle.push((s, c.duration, c.id)));
        let total = oracle
            .iter()
            .fold(Rational::new(0, TICK_DEN), |a, &(_, d, _)| a.add(d));
        // sample all clip starts + interiors + ends + beyond
        let mut samples: Vec<Rational> = Vec::new();
        for &(s, d, _) in &oracle {
            let dt = d.ticks_at(24000, 1);
            samples.push(s);
            if dt > 2 {
                samples.push(s.add(ticks(dt / 2)));
            }
            samples.push(s.add(d).sub(ticks(1)));
        }
        samples.push(total); // == end: must be None
        samples.push(total.add(ticks(1000)));
        samples.push(ticks(-5));
        for q in samples {
            let got = t.hit_test_mut(q).map(|p| t.clip_at(p).unwrap().id);
            // oracle: linear scan
            let mut want = None;
            let mut acc = Rational::new(0, TICK_DEN);
            for &(_s, d, id) in &oracle {
                let end = acc.add(d);
                if q < acc {
                    want = None;
                    break;
                }
                if q < end {
                    want = Some(id);
                    break;
                }
                acc = end;
            }
            assert_eq!(got, want, "[{name}] hit_test({q}) != oracle");
        }
    }
}

/// P6: batch atomicity — a failing sub-command rolls the batch back.
#[test]
fn p6_batch_atomicity() {
    for &(name, kind) in KINDS {
        let (mut tl, ids) = seeded_timeline(kind, 0x0E12_0006, 30);
        let before = tl.state_hash();
        let ghost = Clip {
            id: 999_999,
            duration: ticks(1000),
            source_in: ticks(0),
        };
        let batch = Command::Batch {
            cmds: vec![
                Command::Resize {
                    track: 1,
                    id: ids[0],
                    duration: ticks(123_456),
                },
                Command::Insert {
                    track: 1,
                    index: 0,
                    clip: ghost.clone(),
                },
                Command::Resize {
                    track: 1,
                    id: 424_242,
                    duration: ticks(999),
                }, // fails: no such clip
            ],
        };
        let err = tl.apply(&batch).unwrap_err();
        assert_eq!(
            err,
            TimelineError::ClipNotFound(424_242),
            "[{name}] wrong batch error"
        );
        assert_eq!(tl.state_hash(), before, "[{name}] batch rollback failed");
        // successful batch: inverse restores exactly
        let ok_batch = Command::Batch {
            cmds: vec![
                Command::Resize {
                    track: 1,
                    id: ids[0],
                    duration: ticks(123_456),
                },
                Command::Insert {
                    track: 1,
                    index: 0,
                    clip: ghost,
                },
            ],
        };
        let inv = tl.apply(&ok_batch).unwrap();
        let after = tl.state_hash();
        assert_ne!(after, before, "[{name}] batch had no effect?");
        tl.apply(&inv).unwrap();
        assert_eq!(tl.state_hash(), before, "[{name}] batch inverse failed");
    }
}

/// P7: cross-track move — exactly-one-id invariant and count balance.
#[test]
fn p7_cross_track_move() {
    for &(name, kind) in KINDS {
        let mut tl = Timeline::new();
        tl.add_track(1, kind()).unwrap();
        tl.add_track(2, kind()).unwrap();
        for i in 0..10 {
            let id = tl.alloc_id();
            tl.apply(&Command::Insert {
                track: 1,
                index: i,
                clip: Clip {
                    id,
                    duration: ticks(1000 + i as i64),
                    source_in: ticks(0),
                },
            })
            .unwrap();
        }
        for i in 0..5 {
            let id = tl.alloc_id();
            tl.apply(&Command::Insert {
                track: 2,
                index: i,
                clip: Clip {
                    id,
                    duration: ticks(2000 + i as i64),
                    source_in: ticks(0),
                },
            })
            .unwrap();
        }
        // move clip id 3 (track 1) to track 2 at index 2
        let inv = tl
            .apply(&Command::Move {
                id: 3,
                from_track: 1,
                to_track: 2,
                to_index: 2,
            })
            .unwrap();
        assert_eq!(tl.track_len(1).unwrap(), 9);
        assert_eq!(tl.track_len(2).unwrap(), 6);
        // exactly-once invariant
        let mut count = 0;
        for t in [1u64, 2] {
            tl.track_ref(t).unwrap().walk(&mut |_p, _s, c| {
                if c.id == 3 {
                    count += 1;
                }
            });
        }
        assert_eq!(count, 1, "[{name}] id 3 not exactly-once after move");
        // inverse restores
        tl.apply(&inv).unwrap();
        assert_eq!(
            tl.track_len(1).unwrap(),
            10,
            "[{name}] move inverse count track 1"
        );
        assert_eq!(
            tl.track_len(2).unwrap(),
            5,
            "[{name}] move inverse count track 2"
        );
        let mut pos1 = None;
        tl.track_ref(1).unwrap().walk(&mut |p, _s, c| {
            if c.id == 3 {
                pos1 = Some(p);
            }
        });
        assert_eq!(pos1, Some(2), "[{name}] move inverse position");
    }
}

/// P8: threaded disjoint-track storms — per-track hashes equal sequential runs.
/// (ADR-011 risk 3: multi-track concurrency; single-writer-per-track sharding.)
#[test]
fn p8_threaded_disjoint_tracks() {
    fn storm(seed: u64, n_edits: usize) -> (u64, Vec<(usize, i64)>) {
        // sequential reference: apply the script to a real track first,
        // then compare threaded per-track hashes against it
        let mut track = AvlTrack::new();
        let mut rng = Rng::new(seed);
        let mut v: Vec<i64> = (0..200)
            .map(|_| 481 + (rng.next() % 240_000) as i64)
            .collect();
        let mut script: Vec<(usize, i64)> = Vec::new();
        for _ in 0..n_edits {
            let i = rng.below(v.len());
            let nd = 481 + (rng.next() % 240_000) as i64;
            script.push((i, nd));
            v[i] = nd;
        }
        for (i, &d) in v.iter().enumerate() {
            track
                .insert_at(
                    i,
                    Clip {
                        id: i as u64 + 1,
                        duration: ticks(d),
                        source_in: ticks(0),
                    },
                )
                .unwrap();
        }
        for (i, nd) in &script {
            track.set_duration_at(*i, ticks(*nd)).unwrap();
        }
        (track.state_hash(), script)
    }

    let seeds = [0xA1, 0xB2, 0xC3, 0xD4];
    // sequential hashes
    let seq: Vec<u64> = seeds.iter().map(|&s| storm(s, 500).0).collect();
    // threaded: each thread owns its track (shard)
    let handles: Vec<_> = seeds
        .iter()
        .map(|&s| std::thread::spawn(move || storm(s, 500).0))
        .collect();
    for (i, h) in handles.into_iter().enumerate() {
        assert_eq!(
            h.join().unwrap(),
            seq[i],
            "threaded track {i} hash != sequential"
        );
    }
}

/// Validation errors are typed and leave state unchanged (E-004a discipline).
#[test]
fn p9_typed_errors_no_mutation() {
    for &(name, kind) in KINDS {
        let (mut tl, ids) = seeded_timeline(kind, 0x0E12_0009, 10);
        let before = tl.state_hash();
        let fresh = tl.alloc_id();
        assert_eq!(
            tl.apply(&Command::Split {
                track: 1,
                id: ids[0],
                at: ticks(0),
                new_id: fresh
            }),
            Err(TimelineError::InvalidSplitPoint)
        );
        // Split onto a used id is a typed duplicate error (explicit-allocation rule)
        assert_eq!(
            tl.apply(&Command::Split {
                track: 1,
                id: ids[0],
                at: ticks(1),
                new_id: ids[1]
            }),
            Err(TimelineError::DuplicateClipId(ids[1]))
        );
        assert_eq!(
            tl.apply(&Command::Resize {
                track: 1,
                id: ids[0],
                duration: ticks(0)
            }),
            Err(TimelineError::InvalidDuration)
        );
        assert_eq!(
            tl.apply(&Command::Remove { track: 1, id: 777 }),
            Err(TimelineError::ClipNotFound(777))
        );
        assert_eq!(
            tl.apply(&Command::Insert {
                track: 9,
                index: 0,
                clip: Clip {
                    id: 888,
                    duration: ticks(1),
                    source_in: ticks(0)
                }
            }),
            Err(TimelineError::TrackNotFound(9))
        );
        assert_eq!(
            tl.state_hash(),
            before,
            "[{name}] failed commands mutated state"
        );
    }
}
