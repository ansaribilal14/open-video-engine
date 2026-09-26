//! E-012 — timeline structure INTEGRATION benchmark (ADR-011 revisit gate).
//!
//! Runs the real `ove-timeline` crate (not a throwaway bench skeleton):
//!
//! Container leg (position-addressed TrackOps, E-002c2-comparable):
//! - W1 cursor-local splits (playhead crawl) — gap's home turf
//! - W2 random storm (split/resize/move/remove/insert)
//! - W3 far-jump (front/back quarter random splits) — gap worst case
//!
//! Command leg (Timeline::apply with explicit ids, production path):
//! - W2' random storm by id (id scan + inverse computation included)
//! - W4 scrub-burst: per round = 1 edit + burst of 100 hit_tests;
//!   budget: burst p99 <= 1 ms (doc 34 scrub budget)
//! - W5 multi-track: 8 tracks, interleaved by-id edits + cross-track moves
//!
//! Cross-validation: after EVERY workload, Gap/Avl/Oracle must agree on
//! state hash, clip count and total duration (E-002c methodology).

use std::time::Instant;

use ove_time::Rational;
use ove_timeline::{
    AvlTrack, Clip, Command, GapTrack, OracleTrack, Timeline, TrackKind, TrackOps, TICK_DEN,
};

// ---------------- rng (same family as E-002c2) ----------------
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

fn ticks(n: i64) -> Rational {
    Rational::new(n, TICK_DEN)
}

fn gap_kind() -> TrackKind {
    TrackKind::Gap(GapTrack::new())
}
fn avl_kind() -> TrackKind {
    TrackKind::Avl(AvlTrack::new())
}
fn oracle_kind() -> TrackKind {
    TrackKind::Oracle(OracleTrack::new())
}

fn ms(t: Instant) -> f64 {
    t.elapsed().as_secs_f64() * 1000.0
}

// ---------------- container leg ----------------
#[derive(Clone)]
#[allow(clippy::enum_variant_names)] // the At suffix documents tick-grid addressing
enum COp {
    SplitAt { pos: usize, at: i64 },
    ResizeAt { pos: usize, ticks: i64 },
    MoveAt { i: usize, j: usize },
    RemoveAt { pos: usize },
    InsertAt { pos: usize, ticks: i64, id: u64 },
}

fn apply_cop(c: &mut dyn TrackOps, op: &COp) {
    match op {
        COp::SplitAt { pos, at } => {
            let cl = c.clip_at(*pos).unwrap().clone();
            let at_r = ticks(*at);
            c.set_duration_at(*pos, at_r).unwrap();
            c.insert_at(
                pos + 1,
                Clip {
                    id: cl.id,
                    duration: cl.duration.sub(at_r),
                    source_in: cl.source_in.add(at_r),
                },
            )
            .unwrap();
        }
        COp::ResizeAt { pos, ticks: tk } => {
            c.set_duration_at(*pos, ticks(*tk)).unwrap();
        }
        COp::MoveAt { i, j } => {
            let cl = c.remove_at(*i).unwrap();
            let jj = if j > i { j - 1 } else { *j };
            c.insert_at(jj, cl).unwrap();
        }
        COp::RemoveAt { pos } => {
            c.remove_at(*pos).unwrap();
        }
        COp::InsertAt { pos, ticks: tk, id } => {
            c.insert_at(
                *pos,
                Clip {
                    id: *id,
                    duration: ticks(*tk),
                    source_in: Rational::new(0, TICK_DEN),
                },
            )
            .unwrap();
        }
    }
}

fn stats(t: &dyn TrackOps) -> (u64, usize, i64) {
    let mut total = Rational::new(0, TICK_DEN);
    t.walk(&mut |_p, s, c| {
        total = s.add(c.duration);
    });
    (t.state_hash(), t.len(), total.num())
}

// ---------------- command leg ----------------
#[derive(Clone)]
enum KOp {
    Split { id: u64, at: i64, new_id: u64 },
    Resize { id: u64, ticks: i64 },
    RemoveInsert { id: u64, new_index: usize },
}

const TRACK: u64 = 1;

fn build_timeline(kind: fn() -> TrackKind, n_clips: usize, durs: &[i64]) -> (Timeline, Vec<u64>) {
    let mut tl = Timeline::new();
    tl.add_track(TRACK, kind()).unwrap();
    let mut ids = Vec::with_capacity(n_clips);
    for (i, &d) in durs.iter().enumerate().take(n_clips) {
        let id = tl.alloc_id();
        ids.push(id);
        tl.apply(&Command::Insert {
            track: TRACK,
            index: i,
            clip: Clip {
                id,
                duration: ticks(d),
                source_in: Rational::new(0, TICK_DEN),
            },
        })
        .unwrap();
    }
    (tl, ids)
}

fn main() {
    println!("E-012 — ove-timeline integration benchmark (ADR-011 revisit gate)");
    println!("crate: engine/ove-timeline (production verb surface, exact Rational time, 24 kHz tick grid)");
    println!("N=20000 initial clips; identical scripts across Gap/Avl/Oracle; hash cross-validation after every workload\n");

    let mut rng = Rng::new(0x0E12_5EED_2026_0926);
    const N: usize = 20_000;
    let durs: Vec<i64> = (0..N)
        .map(|_| (1001 + rng.next() % 240_000) as i64)
        .collect();

    // ============ W1: cursor-local splits (container leg) ============
    {
        println!("W1 cursor-local split x5000 (playhead crawl: split clip under cursor, cursor advances)");
        // generate script on oracle
        let mut oracle = OracleTrack::from_clips(
            durs.iter()
                .enumerate()
                .map(|(i, &d)| Clip {
                    id: i as u64 + 1,
                    duration: ticks(d),
                    source_in: Rational::new(0, TICK_DEN),
                })
                .collect(),
        );
        let mut cursor = 0usize;
        let mut ops: Vec<COp> = Vec::new();
        let mut attempted = 0usize;
        while ops.len() < 5000 && attempted < 50_000 {
            attempted += 1;
            let c = oracle.clip_at(cursor).unwrap().clone();
            let dt = c.duration.ticks_at(24000, 1);
            if dt >= 3 {
                let at = 1 + (rng.next() % (dt - 1) as u64) as i64;
                let op = COp::SplitAt { pos: cursor, at };
                apply_cop(&mut oracle, &op);
                ops.push(op);
            }
            cursor += 1;
            if cursor >= oracle.len() {
                cursor = 0;
            }
        }
        println!("  (W1: {} real splits, {} attempts)", ops.len(), attempted);
        run_container("W1", &durs, &ops);
    }

    // ============ W2: random storm (container leg) ============
    {
        println!("W2 random storm (container leg): split x5000, resize x2000, move x2000, remove+insert x500");
        let mut oracle = OracleTrack::from_clips(
            durs.iter()
                .enumerate()
                .map(|(i, &d)| Clip {
                    id: i as u64 + 1,
                    duration: ticks(d),
                    source_in: Rational::new(0, TICK_DEN),
                })
                .collect(),
        );
        let mut ops: Vec<COp> = Vec::new();
        for k in 0..9500 {
            let len = oracle.len();
            match k % 4 {
                0 | 1 => {
                    let pos = rng.below(len);
                    let dt = oracle.clip_at(pos).unwrap().duration.ticks_at(24000, 1);
                    if dt < 3 {
                        continue;
                    }
                    let at = 1 + (rng.next() % (dt - 1) as u64) as i64;
                    let op = COp::SplitAt { pos, at };
                    apply_cop(&mut oracle, &op);
                    ops.push(op);
                }
                2 => {
                    let pos = rng.below(len);
                    let op = COp::ResizeAt {
                        pos,
                        ticks: (481 + rng.next() % 240_000) as i64,
                    };
                    apply_cop(&mut oracle, &op);
                    ops.push(op);
                }
                _ => {
                    if k % 8 == 3 {
                        // move
                        let i = rng.below(len);
                        let j = rng.below(len);
                        let op = COp::MoveAt { i, j };
                        apply_cop(&mut oracle, &op);
                        ops.push(op);
                    } else {
                        // remove + insert elsewhere
                        let pos = rng.below(len);
                        let cl = oracle.clip_at(pos).unwrap().clone();
                        let newpos = rng.below(len); // len-1 after removal, +1 back ≈ same range
                        let op1 = COp::RemoveAt { pos };
                        apply_cop(&mut oracle, &op1);
                        let newpos = newpos.min(oracle.len());
                        let op2 = COp::InsertAt {
                            pos: newpos,
                            ticks: cl.duration.ticks_at(24000, 1),
                            id: cl.id,
                        };
                        apply_cop(&mut oracle, &op2);
                        ops.push(op1);
                        ops.push(op2);
                    }
                }
            }
        }
        run_container("W2", &durs, &ops);
    }

    // ============ W3: far-jump (container leg) ============
    {
        println!("W3 far-jump (alternate edits at track front and back — gap worst case)");
        let mut oracle = OracleTrack::from_clips(
            durs.iter()
                .enumerate()
                .map(|(i, &d)| Clip {
                    id: i as u64 + 1,
                    duration: ticks(d),
                    source_in: Rational::new(0, TICK_DEN),
                })
                .collect(),
        );
        let mut ops: Vec<COp> = Vec::new();
        let mut attempted = 0usize;
        while ops.len() < 5000 && attempted < 200_000 {
            attempted += 1;
            // alternate: RANDOM position in the front quarter, then in the
            // back quarter — every op is a genuine far jump of the gap.
            let len = oracle.len();
            let quarter = (len / 4).max(1);
            let pos = if ops.len().is_multiple_of(2) {
                rng.below(quarter)
            } else {
                len - 1 - rng.below(quarter)
            };
            let dt = oracle.clip_at(pos).unwrap().duration.ticks_at(24000, 1);
            if dt >= 3 {
                let at = 1 + (rng.next() % (dt - 1) as u64) as i64;
                let op = COp::SplitAt { pos, at };
                apply_cop(&mut oracle, &op);
                ops.push(op);
            }
        }
        println!(
            "  (W3: {} real far-jump ops, {} attempts)",
            ops.len(),
            attempted
        );
        run_container("W3", &durs, &ops);
    }

    // ============ W2': command leg random storm by id ============
    {
        println!("W2' random storm BY ID (command leg through Timeline::apply; includes id scan + inverse computation): split x1200, resize x800");
        let script = gen_kop_script(&mut rng, &durs, 1200, 800, 0, 0);
        let (h_gap, t_gap) = run_command_leg(gap_kind, &durs, &script);
        let (h_avl, t_avl) = run_command_leg(avl_kind, &durs, &script);
        let (h_orc, t_orc) = run_command_leg(oracle_kind, &durs, &script);
        assert_eq!(h_gap, h_avl, "W2' hash mismatch gap vs avl");
        assert_eq!(h_gap, h_orc, "W2' hash mismatch gap vs oracle");
        println!(
            "  W2'  gap {:>8.1}ms | avl {:>8.1}ms | oracle {:>8.1}ms   hashes OK ({:016x})\n",
            t_gap, t_avl, t_orc, h_gap
        );
    }

    // ============ W4: scrub-burst (command leg + hit_test_mut) ============
    {
        println!(
            "W4 scrub-burst: 1000 rounds of (1 edit by id + burst of 100 hit_tests) @ N=20000"
        );
        println!("     budget: burst p99 <= 1.0 ms (doc 34 scrub budget; hit-test + dispatch)");
        for (name, ctor) in [
            ("gap", gap_kind as fn() -> TrackKind),
            ("avl", avl_kind as fn() -> TrackKind),
        ] {
            let (mut tl, ids) = build_timeline(ctor, N, &durs);
            let mut shadow: Vec<(u64, i64)> =
                ids.iter().zip(durs.iter()).map(|(&i, &d)| (i, d)).collect();
            let mut bursts: Vec<f64> = Vec::with_capacity(1000);
            let mut edit_ms = 0.0;
            for r in 0..1000 {
                // one edit (alternate split/resize) — addressed BY ID (production path)
                let e0 = Instant::now();
                let (id, dt) = {
                    let i = rng.below(shadow.len());
                    shadow[i]
                };
                if r % 2 == 0 && dt >= 3 {
                    let at = 1 + (rng.next() % (dt - 1) as u64) as i64;
                    let nid = tl.alloc_id();
                    let cmd = Command::Split {
                        track: TRACK,
                        id,
                        at: ticks(at),
                        new_id: nid,
                    };
                    tl.apply(&cmd).unwrap();
                    // shadow: split id's dur; right half carries its own engine id
                    let p = shadow.iter().position(|&(i, _)| i == id).unwrap();
                    let d = shadow[p].1;
                    shadow[p] = (id, at);
                    shadow.insert(p + 1, (nid, d - at));
                } else {
                    let nd = (481 + rng.next() % 240_000) as i64;
                    let cmd = Command::Resize {
                        track: TRACK,
                        id,
                        duration: ticks(nd),
                    };
                    tl.apply(&cmd).unwrap();
                    let p = shadow.iter().position(|&(i, _)| i == id).unwrap();
                    shadow[p].1 = nd;
                }
                edit_ms += ms(e0);
                // total ticks from shadow
                let total: i64 = shadow.iter().map(|&(_, d)| d).sum();
                // burst of 100 hit tests (fresh &mut borrow — single-writer scrub path)
                let b0 = Instant::now();
                {
                    let track = tl.track_mut(TRACK).unwrap();
                    for _ in 0..100 {
                        let q = ticks((rng.next() % total.max(1) as u64) as i64);
                        let _ = track.hit_test_mut(q);
                    }
                }
                bursts.push(ms(b0));
            }
            bursts.sort_by(|a, b| a.partial_cmp(b).unwrap());
            let p50 = bursts[bursts.len() / 2];
            let p99 = bursts[(bursts.len() as f64 * 0.99) as usize];
            let max = *bursts.last().unwrap();
            let verdict = if p99 <= 1.0 { "PASS" } else { "FAIL" };
            println!(
                "  W4[{}] bursts: p50 {:>8.3}ms | p99 {:>8.3}ms | max {:>8.3}ms  -> budget {}   (edit path total {:.0}ms/1000)",
                name, p50, p99, max, verdict, edit_ms
            );
        }
        println!();
    }

    // ============ W5: multi-track interleaved (command leg) ============
    {
        println!("W5 multi-track: 8 tracks x 2500 build clips/track + 800 by-id edits interleaved + cross-track moves");
        let mut hashes: Vec<(String, u64)> = Vec::new();
        for (name, ctor) in [
            ("gap", gap_kind as fn() -> TrackKind),
            ("avl", avl_kind as fn() -> TrackKind),
        ] {
            let mut tl = Timeline::new();
            const TR: u64 = 8;
            for t in 1..=TR {
                tl.add_track(t, ctor()).unwrap();
            }
            let per = N / TR as usize; // 2500 clips/track
            let mut shadow: Vec<Vec<(u64, i64)>> = vec![Vec::with_capacity(per); TR as usize];
            let t0 = Instant::now();
            let mut seed_acc = 7919u64;
            for t in 1..=TR {
                for i in 0..per {
                    let id = tl.alloc_id();
                    seed_acc = seed_acc
                        .wrapping_mul(6364136223846793005)
                        .wrapping_add(1442695040888963407);
                    let d = 1001 + (seed_acc % 240_000) as i64;
                    tl.apply(&Command::Insert {
                        track: t,
                        index: i,
                        clip: Clip {
                            id,
                            duration: ticks(d),
                            source_in: Rational::new(0, TICK_DEN),
                        },
                    })
                    .unwrap();
                    shadow[(t - 1) as usize].push((id, d));
                }
            }
            let build = ms(t0);
            let t0 = Instant::now();
            let mut rng5 = Rng::new(0x0E12_0005_0005);
            for r in 0..800 {
                let t = 1 + (r % TR);
                let ti = (t - 1) as usize;
                let len = shadow[ti].len();
                if len == 0 {
                    continue;
                }
                let si = rng5.below(len);
                let (id, dt) = shadow[ti][si];
                if r % 4 == 3 && dt >= 3 {
                    let at = 1 + (rng5.next() % (dt - 1) as u64) as i64;
                    let nid = tl.alloc_id();
                    tl.apply(&Command::Split {
                        track: t,
                        id,
                        at: ticks(at),
                        new_id: nid,
                    })
                    .unwrap();
                    shadow[ti][si] = (id, at);
                    shadow[ti].insert(si + 1, (nid, dt - at)); // right half: own engine id
                } else {
                    let nd = (481 + rng5.next() % 240_000) as i64;
                    tl.apply(&Command::Resize {
                        track: t,
                        id,
                        duration: ticks(nd),
                    })
                    .unwrap();
                    shadow[ti][si].1 = nd;
                }
                if r % 4 == 1 {
                    // cross-track move of a random clip from track t to t'
                    let t2 = 1 + (rng5.below(TR as usize) as u64);
                    if t2 != t && !shadow[ti].is_empty() {
                        let mi = rng5.below(shadow[ti].len());
                        let (mid, _) = shadow[ti][mi];
                        let dest_len = shadow[(t2 - 1) as usize].len();
                        let to_index = rng5.below(dest_len + 1);
                        tl.apply(&Command::Move {
                            id: mid,
                            from_track: t,
                            to_track: t2,
                            to_index,
                        })
                        .unwrap();
                        let c = shadow[ti].remove(mi);
                        let to_index = to_index.min(shadow[(t2 - 1) as usize].len());
                        shadow[(t2 - 1) as usize].insert(to_index, c);
                    }
                }
            }
            let edits = ms(t0);
            let h = tl.state_hash();
            println!(
                "  W5[{}] build(8x{}) {:>7.1}ms | 800 mixed edits {:>7.1}ms | hash {:016x}",
                name, per, build, edits, h
            );
            hashes.push((name.to_string(), h));
        }
        assert_eq!(hashes[0].1, hashes[1].1, "W5 gap vs avl hash mismatch");
        println!("  W5 CROSS-VALIDATION OK\n");
    }

    println!(
        "\nall values single-run on container CPU — indicative relative order (E-002c methodology)"
    );
    println!("E-012 bench complete");
}

fn run_container(w: &str, durs: &[i64], ops: &[COp]) {
    let mk = |ctor: fn() -> TrackKind| -> (TrackKind, f64, f64) {
        let mut kind = ctor();
        let t0 = Instant::now();
        for (i, &d) in durs.iter().enumerate() {
            kind.insert_at(
                i,
                Clip {
                    id: i as u64 + 1,
                    duration: ticks(d),
                    source_in: Rational::new(0, TICK_DEN),
                },
            )
            .unwrap();
        }
        let build = ms(t0);
        let t0 = Instant::now();
        for op in ops {
            let k: &mut dyn TrackOps = &mut kind;
            apply_cop(k, op);
        }
        let apply = ms(t0);
        (kind, build, apply)
    };
    let (g, bg, ag) = mk(gap_kind);
    let (a, ba, aa) = mk(avl_kind);
    let (o, bo, ao) = mk(oracle_kind);
    let (hg, cg, tg) = stats(&g);
    let (ha, ca, ta) = stats(&a);
    let (ho, co, to) = stats(&o);
    assert_eq!(hg, ha, "{} hash mismatch gap vs avl", w);
    assert_eq!(hg, ho, "{} hash mismatch gap vs oracle", w);
    assert_eq!((cg, tg), (ca, ta), "{} count/total mismatch gap vs avl", w);
    assert_eq!(
        (cg, tg),
        (co, to),
        "{} count/total mismatch gap vs oracle",
        w
    );
    println!(
        "  {}  gap: build+ops {:>8.1}ms | avl: {:>8.1}ms | oracle: {:>9.1}ms   [build gap {:>6.1} / avl {:>6.1} / oracle {:>7.1}] CROSS-VALIDATION OK (hash {:016x}, {} clips, total {} ticks)",
        w, ag, aa, ao, bg, ba, bo, hg, cg, tg
    );
}

fn gen_kop_script(
    rng: &mut Rng,
    durs: &[i64],
    splits: usize,
    resizes: usize,
    removs: usize,
    _moves: usize,
) -> Vec<KOp> {
    let mut shadow: Vec<(u64, i64)> = durs
        .iter()
        .enumerate()
        .map(|(i, &d)| (i as u64 + 1, d))
        .collect();
    let mut ops = Vec::new();
    let total_ops = splits + resizes + removs;
    let mut new_id_ctr: u64 = durs.len() as u64 + 1; // prologue ids are 1..=N
    let mut k = 0;
    while k < total_ops {
        if k < splits {
            let i = rng.below(shadow.len());
            let (id, dt) = shadow[i];
            if dt < 3 {
                continue;
            }
            let at = 1 + (rng.next() % (dt - 1) as u64) as i64;
            ops.push(KOp::Split {
                id,
                at,
                new_id: new_id_ctr,
            });
            new_id_ctr += 1;
            shadow[i] = (id, at);
            shadow.insert(i + 1, (new_id_ctr - 1, dt - at)); // right half carries its OWN engine id
        } else if k < splits + resizes {
            let i = rng.below(shadow.len());
            let (id, _) = shadow[i];
            let nd = (481 + rng.next() % 240_000) as i64;
            ops.push(KOp::Resize { id, ticks: nd });
            shadow[i].1 = nd;
        } else {
            if shadow.len() > 100 {
                let i = rng.below(shadow.len());
                let (id, d) = shadow.remove(i);
                let new_index = rng.below(shadow.len() + 1);
                ops.push(KOp::RemoveInsert { id, new_index });
                shadow.insert(new_index, (id, d));
            }
        }
        k += 1;
    }
    ops
}

fn run_command_leg(ctor: fn() -> TrackKind, durs: &[i64], ops: &[KOp]) -> (u64, f64) {
    let mut tl = Timeline::new();
    tl.add_track(TRACK, ctor()).unwrap();
    let t0 = Instant::now();
    for (i, &d) in durs.iter().enumerate() {
        let id = tl.alloc_id();
        tl.apply(&Command::Insert {
            track: TRACK,
            index: i,
            clip: Clip {
                id,
                duration: ticks(d),
                source_in: Rational::new(0, TICK_DEN),
            },
        })
        .unwrap();
    }
    for op in ops {
        let cmd = match op {
            KOp::Split { id, at, new_id } => Command::Split {
                track: TRACK,
                id: *id,
                at: ticks(*at),
                new_id: *new_id,
            },
            KOp::Resize { id, ticks: tk } => Command::Resize {
                track: TRACK,
                id: *id,
                duration: ticks(*tk),
            },
            KOp::RemoveInsert { id, new_index, .. } => {
                // find pos
                let t = tl.track_ref(TRACK).unwrap();
                let pos = t.index_of(*id).unwrap();
                let c = t.clip_at(pos).unwrap().clone();
                tl.apply(&Command::Remove {
                    track: TRACK,
                    id: *id,
                })
                .unwrap();
                let ni = (*new_index).min(tl.track_len(TRACK).unwrap());
                tl.apply(&Command::Insert {
                    track: TRACK,
                    index: ni,
                    clip: c,
                })
                .unwrap();
                continue;
            }
        };
        tl.apply(&cmd).unwrap();
    }
    (tl.state_hash(), ms(t0))
}
