//! Property suite — Wave 1 acceptance gate (ENGINE_BUILD_PLAN).
//!
//! Ports E-003/E-009 invariants to Rust with REAL hash checks (note: E-003's
//! Python-model P2b check contained an `or True` — a vacuous pass; every check
//! here is a hard equality). Seeded xorshift RNG, no deps (ove-time discipline).
//!
//! P1  per-verb inverse identity: apply → inverse → apply == identity
//! P2  batch atomicity under fuzz: failure restores state + journal exactly
//! P3  replay determinism: fold(journal) == live state, hashes verified per entry
//! P4  undo-all/redo-all hash exactness (E-003 P2b done RIGHT)
//! P5  journal-prefix replay == recorded hash at every prefix (snapshot+suffix class)
//! P6  structural invariant: per-track sorted by start, non-overlapping, after any stream
//! P7  split-continuity: source-range and timeline coverage preserved across split
//! P8  cross-run determinism: same seed → identical hashes at every step

use ove_time::Rational;
use ove_timeline::*;

struct Rng(u64);
impl Rng {
    fn new(seed: u64) -> Self {
        Rng(seed.max(1))
    }
    fn next(&mut self) -> u64 {
        // xorshift64*
        let mut x = self.0;
        x ^= x >> 12;
        x ^= x << 25;
        x ^= x >> 27;
        self.0 = x;
        x.wrapping_mul(0x2545_F491_4F6C_DD1D)
    }
    fn below(&mut self, n: u64) -> u64 {
        self.next() % n.max(1)
    }
}

const TICK: i64 = 24000;

fn rr(num: i64, den: i64) -> Rational {
    Rational::new(num, den)
}

fn tracks3() -> &'static [TrackKind] {
    &[TrackKind::Video, TrackKind::Video, TrackKind::Audio]
}

fn fresh() -> TimelineEngine {
    TimelineEngine::new(TimelineState::new(tracks3()))
}

/// One random command given the live state (smart fuzz: uses existing ids and
/// valid-ish ranges; overlap may still occur and is counted as a rejection).
fn random_command(e: &TimelineEngine, rng: &mut Rng, step: usize) -> Command {
    let den = 24000;
    let ids: Vec<ClipId> = (0..e.state.track_count() as u32)
        .flat_map(|t| {
            e.state
                .clips_on_track(TrackId(t))
                .map(|c| c.id)
                .collect::<Vec<_>>()
        })
        .collect();
    let roll = rng.below(100);
    if ids.is_empty() || roll < 35 {
        Command::AddClip {
            id: None,
            track: TrackId(rng.below(3) as u32),
            asset: AssetId(rng.below(4)),
            start: rr(rng.below(200_000) as i64, den),
            dur: rr(1001 * (rng.below(96) as i64 + 1), den),
            src_in: rr(rng.below(100_000) as i64, den),
            speed: rr(1, 1),
            label: format!("f{}", step),
        }
    } else if roll < 50 {
        Command::RemoveClip {
            id: ids[rng.below(ids.len() as u64) as usize],
        }
    } else if roll < 70 {
        Command::MoveClip {
            id: ids[rng.below(ids.len() as u64) as usize],
            track: TrackId(rng.below(3) as u32),
            start: rr(rng.below(200_000) as i64, den),
        }
    } else if roll < 82 {
        let id = ids[rng.below(ids.len() as u64) as usize];
        let cur = e.state.clip(id).unwrap().dur;
        // shrink or modest grow within same den; may still overlap → rejected, fine
        let new_ticks = 1001 * (rng.below(96) as i64 + 1);
        Command::ResizeClip {
            id,
            dur: rr(new_ticks, cur.den()),
        }
    } else if roll < 94 {
        let id = ids[rng.below(ids.len() as u64) as usize];
        let cur = e.state.clip(id).unwrap().dur;
        // valid-by-construction split: k/m of the clip, 0 < k < m
        let m: i64 = rng.below(7) as i64 + 2;
        let k: i64 = rng.below((m - 1) as u64) as i64 + 1;
        Command::SplitClip {
            id,
            offset: rr(cur.num() * k, cur.den() * m),
            new_id: None,
        }
    } else {
        let id = ids[rng.below(ids.len() as u64) as usize];
        let k = rng.below(8) as i64 + 1;
        let m = rng.below(8) as i64 + 1;
        Command::RetimeClip {
            id,
            speed: rr(k, m),
        }
    }
}

/// Structural invariant P6: per-track sorted by start, touching-or-gap, no overlap.
fn assert_sorted_non_overlapping(e: &TimelineEngine) {
    for t in 0..e.state.track_count() as u32 {
        let mut prev_end: Option<Rational> = None;
        for c in e.state.clips_on_track(TrackId(t)) {
            assert!(
                c.dur > Rational::zero(1),
                "zero/negative duration escaped: {:?}",
                c
            );
            if let Some(pe) = prev_end {
                assert!(
                    c.start >= pe,
                    "overlap/order violation on track {}: prev_end {} > start {}",
                    t,
                    pe,
                    c.start
                );
            }
            prev_end = Some(c.end());
        }
    }
}

fn run_stream(seed: u64, steps: usize) -> (TimelineEngine, Vec<u64>, Vec<(usize, u64)>) {
    let mut e = fresh();
    let mut rng = Rng::new(seed);
    let mut hashes = vec![e.state.state_hash()];
    let mut checkpoints = vec![(0usize, e.state.state_hash())];
    for step in 0..steps {
        let cmd = if rng.below(10) == 0 {
            // random batch of 2–4
            let n = rng.below(3) as usize + 2;
            Command::Batch {
                cmds: (0..n).map(|_| random_command(&e, &mut rng, step)).collect(),
                label: format!("b{}", step),
            }
        } else {
            random_command(&e, &mut rng, step)
        };
        let _ = e.apply(cmd); // rejections are part of the stream; state stays valid
        hashes.push(e.state.state_hash());
        checkpoints.push((e.journal().len(), e.state.state_hash()));
    }
    (e, hashes, checkpoints)
}

#[test]
fn p1_per_verb_inverse_identity() {
    for seed in 1..=8 {
        let mut e = fresh();
        let mut rng = Rng::new(seed * 7919);
        for step in 0..300 {
            let cmd = random_command(&e, &mut rng, step);
            let h_before = e.state.state_hash();
            if let Ok(rec) = e.apply(cmd.clone()) {
                let h_applied = e.state.state_hash();
                // apply the engine-computed exact inverse → must equal identity
                e.apply(rec.inverse.clone())
                    .unwrap_or_else(|err| panic!("inverse of {:?} rejected: {}", cmd.verb(), err));
                assert_eq!(
                    e.state.state_hash(),
                    h_before,
                    "seed {} step {}: inverse of {} not identity",
                    seed,
                    step,
                    cmd.verb()
                );
                // re-apply the RESOLVED command to continue the stream (undo+redo
                // sandwich); the resolved form carries the assigned ids, so the
                // state (including ids) is reproduced exactly
                e.apply(rec.resolved.clone())
                    .expect("re-apply after inverse sandwich must succeed");
                assert_eq!(e.state.state_hash(), h_applied);
            }
        }
    }
}

#[test]
fn p2_batch_atomicity_fuzz() {
    for seed in 1..=8 {
        let mut e = fresh();
        let mut rng = Rng::new(seed * 104_729);
        // seed some clips
        for i in 0..6 {
            let _ = e.apply(Command::AddClip {
                id: None,
                track: TrackId((i % 3) as u32),
                asset: AssetId(1),
                start: rr(i * 48_000, 24000),
                dur: rr(24_000, 24000),
                src_in: Rational::zero(1),
                speed: rr(1, 1),
                label: format!("s{}", i),
            });
        }
        for _ in 0..40 {
            let h = e.state.state_hash();
            let j = e.journal().len();
            let mut subs: Vec<Command> = (0..4).map(|_| random_command(&e, &mut rng, 0)).collect();
            // force a guaranteed-invalid tail: remove a random id TWICE
            if let Some(id) = e.state.clips_on_track(TrackId(0)).next().map(|c| c.id) {
                subs.push(Command::RemoveClip { id });
                subs.push(Command::RemoveClip { id });
            }
            let batch = Command::Batch {
                cmds: subs,
                label: "fuzz".into(),
            };
            match e.apply(batch) {
                Ok(_) => {}
                Err(TimelineError::BatchRolledBack { .. }) => {
                    assert_eq!(e.state.state_hash(), h, "batch rollback not exact");
                    assert_eq!(e.journal().len(), j, "failed batch touched journal");
                }
                Err(other) => panic!("unexpected error class: {}", other),
            }
        }
    }
}

#[test]
fn p3_replay_determinism() {
    for seed in 1..=6 {
        let (e, _, _) = run_stream(seed, 400);
        let journal = e.journal().to_vec();
        let e2 = TimelineEngine::replay(tracks3(), &journal)
            .unwrap_or_else(|err| panic!("seed {}: replay failed: {}", seed, err));
        assert_eq!(
            e2.state.state_hash(),
            e.state.state_hash(),
            "seed {}: replay != apply",
            seed
        );
        assert_eq!(e2.state.clip_count(), e.state.clip_count());
        assert_eq!(e2.journal().len(), e.journal().len());
    }
}

#[test]
fn p4_undo_all_redo_all_hash_exact() {
    for seed in 1..=6 {
        let (mut e, _, _) = run_stream(seed, 200);
        let h_full = e.state.state_hash();
        // true initial hash: replay empty journal
        let h_init = TimelineEngine::replay(tracks3(), &[])
            .unwrap()
            .state
            .state_hash();
        let depth = e.undo_depth();
        for _ in 0..depth {
            e.undo().unwrap();
        }
        assert_eq!(
            e.state.state_hash(),
            h_init,
            "seed {}: undo-all not hash-exact",
            seed
        );
        for _ in 0..depth {
            e.redo().unwrap();
        }
        assert_eq!(
            e.state.state_hash(),
            h_full,
            "seed {}: redo-all not hash-exact",
            seed
        );
    }
}

#[test]
fn p5_journal_prefix_replay_matches_live_checkpoints() {
    for seed in 1..=4 {
        let (e, _, checkpoints) = run_stream(seed, 150);
        let journal = e.journal();
        // replay every 10th checkpoint prefix; state must equal the live hash there
        for (cut, live_hash) in checkpoints.iter().step_by(10) {
            let prefix = &journal[..*cut];
            let e2 = TimelineEngine::replay(tracks3(), prefix).unwrap();
            assert_eq!(
                e2.state.state_hash(),
                *live_hash,
                "seed {}: prefix {} mismatch",
                seed,
                cut
            );
        }
    }
}

#[test]
fn p6_structural_invariant_after_fuzz() {
    for seed in 1..=8 {
        let (e, _, _) = run_stream(seed, 300);
        assert_sorted_non_overlapping(&e);
        // every clip is on exactly the track it claims
        for t in 0..e.state.track_count() as u32 {
            for c in e.state.clips_on_track(TrackId(t)) {
                assert_eq!(c.track.0, t, "clip {} on wrong track", c.id.0);
            }
        }
    }
}

#[test]
fn p7_split_continuity_source_and_coverage() {
    for seed in 1..=4 {
        let mut e = fresh();
        let mut rng = Rng::new(seed * 31);
        // build one long clip
        let dur_ticks: i64 = 240_000;
        let rec = e
            .apply(Command::AddClip {
                id: None,
                track: TrackId(0),
                asset: AssetId(3),
                start: Rational::zero(1),
                dur: rr(dur_ticks, TICK),
                src_in: rr(1000, 1),
                speed: rr(1, 1),
                label: "long".into(),
            })
            .unwrap();
        let id = rec.assigned[0];
        let coverage_before = e.state.track_duration(TrackId(0));
        let h_before = e.state.state_hash();

        // 20 sequential valid splits at random fractions
        let mut live = vec![id];
        for _ in 0..20 {
            let victim = live[rng.below(live.len() as u64) as usize];
            let d = e.state.clip(victim).unwrap().dur;
            let m: i64 = rng.below(5) as i64 + 2;
            let k: i64 = rng.below((m - 1) as u64) as i64 + 1;
            if let Ok(r2) = e.apply(Command::SplitClip {
                id: victim,
                offset: rr(d.num() * k, d.den() * m),
                new_id: None,
            }) {
                live.push(r2.assigned[0]);
            }
        }
        // coverage identical; source span preserved: sum of src ranges == original
        let src_total = live
            .iter()
            .filter_map(|cid| e.state.clip(*cid))
            .fold(Rational::zero(1), |acc, c| acc.add(c.src_dur()));
        assert_eq!(
            src_total,
            rr(dur_ticks, TICK),
            "source span not preserved across splits"
        );
        assert_eq!(
            e.state.track_duration(TrackId(0)),
            coverage_before,
            "timeline coverage changed"
        );

        // undo all splits → hash-exact original clip
        for _ in 0..20 {
            e.undo().unwrap();
        }
        assert_eq!(
            e.state.state_hash(),
            h_before,
            "undo of split chain not hash-exact"
        );
        assert_eq!(e.state.clip_count(), 1);
    }
}

#[test]
fn p8_cross_run_determinism_same_seed() {
    for seed in 1..=4 {
        let (e1, h1, _) = run_stream(seed, 250);
        let (e2, h2, _) = run_stream(seed, 250);
        assert_eq!(h1, h2, "seed {}: per-step hashes diverged", seed);
        assert_eq!(e1.state.state_hash(), e2.state.state_hash());
        assert_eq!(e1.journal().len(), e2.journal().len());
    }
}
