//! Performance smoke test — 20k-clip editor budgets (ENGINE_BUILD_PLAN Wave 1
//! acceptance, budgets from E-002c: split ≤5ms, resize ≤2ms, move ≤3ms at 20k,
//! container CPU).
//!
//! These are SMOKE budgets (catch accidental O(n²) / Vec-insert pathologies /
//! O(n) validation walks) with ≥10× headroom over E-002c medians — they are NOT
//! performance claims. Perf claims live in E-012 benches, correctness here
//! (directive: correctness and performance are separated).

use ove_time::Rational;
use ove_timeline::*;
use std::time::Instant;

fn rr(num: i64, den: i64) -> Rational {
    Rational::new(num, den)
}

const TICK: i64 = 24000;
const N: usize = 20_000;

/// E-002c budgets are RELEASE-mode numbers (CI runs `cargo test --release`).
/// Debug builds are ~10× slower (no inlining, overflow checks on); scale budgets
/// there so the test is runnable in both modes without lying about either.
fn budget_scale() -> u32 {
    if cfg!(debug_assertions) {
        10
    } else {
        1
    }
}

fn build_20k() -> (TimelineEngine, Vec<ClipId>) {
    let mut e = TimelineEngine::new(TimelineState::new(&[TrackKind::Video]));
    let mut ids = Vec::with_capacity(N);
    let clip_dur = rr(1001, TICK);
    // contiguous layout: clip i at start i*1001/24000
    for i in 0..N {
        let start = clip_dur.mul(rr(i as i64, 1));
        let rec = e
            .apply(Command::AddClip {
                id: None,
                track: TrackId(0),
                asset: AssetId(1),
                start,
                dur: clip_dur,
                src_in: start,
                speed: rr(1, 1),
                label: String::new(),
            })
            .expect("contiguous adds must never overlap-reject");
        ids.push(rec.assigned[0]);
    }
    (e, ids)
}

struct Rng(u64);
impl Rng {
    fn new(seed: u64) -> Self {
        Rng(seed.max(1))
    }
    fn below(&mut self, n: u64) -> u64 {
        let mut x = self.0;
        x ^= x >> 12;
        x ^= x << 25;
        x ^= x >> 27;
        self.0 = x;
        ((x.wrapping_mul(0x2545_F491_4F6C_DD1D)) >> 16) % n.max(1)
    }
}

#[test]
fn smoke_20k_editor_budgets() {
    let t_build = Instant::now();
    let (mut e, ids) = build_20k();
    let build = t_build.elapsed();
    assert_eq!(e.state.clip_count(), N);

    // walk budget: exact duration fold over 20k clips (E-002c: bounded by
    // rational add ~140ns/op → 20k ≈ 2.8ms; allow 10ms smoke)
    let t = Instant::now();
    let d = e.state.track_duration(TrackId(0));
    let walk = t.elapsed();
    assert_eq!(
        d,
        rr(1001 * N as i64, TICK),
        "contiguous fold must be exact"
    );
    assert!(
        walk.as_millis() <= 10 * budget_scale() as u128,
        "walk over 20k took {:?}",
        walk
    );

    // hit-test budget: 1000 binary searches ≤ 20ms
    let mut rng = Rng::new(42);
    let t = Instant::now();
    for _ in 0..1000 {
        let tick = rng.below((1001 * N as i64) as u64) as i64;
        let _ = e.state.hit_test(TrackId(0), rr(tick, TICK));
    }
    let hits = t.elapsed();
    assert!(
        hits.as_millis() <= 20 * budget_scale() as u128,
        "1000 hit-tests took {:?}",
        hits
    );

    // split budget: 1000 random in-clip splits ≤ 50ms (E-002c cursor median
    // 0.22µs/op; random positions include gap-moves — still far under budget)
    let mut rng = Rng::new(7);
    let t = Instant::now();
    let mut splits_ok = 0u32;
    for _ in 0..1000 {
        let victim = ids[rng.below(ids.len() as u64) as usize];
        if let Some(clip) = e.state.clip(victim) {
            let d = clip.dur;
            if d.num() > 2 {
                let half = rr(d.num() / 2, d.den());
                if e.apply(Command::SplitClip {
                    id: victim,
                    offset: half,
                    new_id: None,
                })
                .is_ok()
                {
                    splits_ok += 1;
                }
            }
        }
    }
    let splits = t.elapsed();
    assert!(
        splits_ok > 900,
        "splits unexpectedly rejected: {}",
        splits_ok
    );
    assert!(
        splits.as_millis() <= 50 * budget_scale() as u128,
        "1000 splits took {:?}",
        splits
    );

    // resize budget: 1000 shrinks ≤ 20ms (E-002c budget 2ms/op at 20k is the
    // per-op ceiling; our smoke asserts aggregate, ~20µs/op headroom ≥ 100×).
    // Halving is always a valid shrink (no overlap possible on contiguous layout).
    let mut rng = Rng::new(11);
    let t = Instant::now();
    let mut resizes_ok = 0u32;
    for _ in 0..1000 {
        let victim = ids[rng.below(ids.len() as u64) as usize];
        let half = e.state.clip(victim).unwrap().dur.mul(rr(1, 2));
        if e.apply(Command::ResizeClip {
            id: victim,
            dur: half,
        })
        .is_ok()
        {
            resizes_ok += 1;
        }
    }
    let resizes = t.elapsed();
    assert!(
        resizes_ok > 900,
        "resizes unexpectedly rejected: {}",
        resizes_ok
    );
    assert!(
        resizes.as_millis() <= 20 * budget_scale() as u128,
        "1000 resizes took {:?}",
        resizes
    );

    // move budget: 1000 random same-track moves ≤ 100ms (includes rejected
    // overlap attempts; each attempt is O(log n) validate + O(dist) gap move)
    let mut rng = Rng::new(13);
    let t = Instant::now();
    let total_ticks = 1001 * N as i64;
    for _ in 0..1000 {
        let victim = ids[rng.below(ids.len() as u64) as usize];
        let dest = rr(rng.below(total_ticks as u64) as i64, TICK);
        let _ = e.apply(Command::MoveClip {
            id: victim,
            track: TrackId(0),
            start: dest,
        });
    }
    let moves = t.elapsed();
    assert!(
        moves.as_millis() <= 100 * budget_scale() as u128,
        "1000 random moves took {:?}",
        moves
    );

    // structural sanity after all edits
    let mut prev_end = Rational::zero(1);
    for c in e.state.clips_on_track(TrackId(0)) {
        assert!(c.start >= prev_end);
        prev_end = c.end();
    }

    // report (visible with --nocapture)
    eprintln!(
        "smoke 20k: build {:?} | walk {:?} | 1000 hits {:?} | 1000 splits {:?} | 1000 resizes {:?} | 1000 moves {:?}",
        build, walk, hits, splits, resizes, moves
    );
}
