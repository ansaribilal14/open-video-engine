//! Cross-platform conformance runner (CROSS_PLATFORM_CONFORMANCE_PLAN §3).
//!
//! ONE canonical scenario, byte-stable output, run on every target the
//! engine claims: native (the L-0 reference), wasm32-wasip1 (the L-3 core
//! evidence), aarch64-linux-android (the L-2 compile evidence). The CI
//! script `run_platform_conformance.sh` executes the same binary natively
//! and under wasmtime and requires IDENTICAL stdout — the §3 invariant
//! "state_hash equal to L-0" made concrete at the WASM boundary.
//!
//! Scenario (all ids EXPLICIT per E-012; no randomness, no floats):
//!   1. project create (24000/1 tick axis), 2 tracks
//!   2. batch insert of 3 clips / split / move / resize / remove
//!   3. SetKeyframes (opacity + x) with Linear keys (W8 semantics)
//!   4. undo×2 → redo×2
//!   5. hash → drop → reopen (log replay) → hash
//!   6. undo → redo roundtrip on the REOPENED project
//!   7. exact keyframe evaluation + exact rational spot checks (P13)
//!
//! Output lines are `KEY=VALUE` — diffable across targets.

use std::path::PathBuf;

use ove_project::{Owner, Project};
use ove_time::Rational;
use ove_timeline::{Clip, Command, GapTrack, Interpolation, Keyframe, PropertyName, TrackKind};

const OWNER: Owner = Owner::Human;

fn ticks(n: i64) -> Rational {
    Rational::new(n, 24_000)
}

fn setup_tracks(p: &mut Project) {
    p.add_track(1, TrackKind::Gap(GapTrack::new()))
        .expect("track 1");
    p.add_track(2, TrackKind::Gap(GapTrack::new()))
        .expect("track 2");
}

fn run(project_dir: PathBuf) {
    // 1. create (Project::create owns folder creation and asserts absence)
    let mut p = Project::create(&project_dir, (24_000, 1)).expect("create");
    setup_tracks(&mut p);

    // 2. structural commands, all explicit ids
    p.execute(
        Command::Batch {
            cmds: vec![
                Command::Insert {
                    track: 1,
                    index: 0,
                    clip: Clip::new(1, ticks(48), ticks(0)),
                },
                Command::Insert {
                    track: 1,
                    index: 1,
                    clip: Clip::new(2, ticks(96), ticks(48)),
                },
                Command::Insert {
                    track: 1,
                    index: 2,
                    clip: Clip::new(3, ticks(24), ticks(0)),
                },
            ],
        },
        OWNER,
    )
    .expect("batch insert");
    p.execute(
        Command::Split {
            track: 1,
            id: 2,
            at: ticks(48),
            new_id: 100,
        },
        OWNER,
    )
    .expect("split");
    p.execute(
        Command::Move {
            id: 3,
            from_track: 1,
            to_track: 2,
            to_index: 0,
        },
        OWNER,
    )
    .expect("move");
    p.execute(
        Command::SetKeyframes {
            track: 1,
            id: 1,
            property: PropertyName::Opacity,
            keys: vec![
                Keyframe::new(ticks(0), Rational::new(0, 1), Interpolation::Linear),
                Keyframe::new(ticks(24), Rational::new(1, 1), Interpolation::Linear),
            ],
        },
        OWNER,
    )
    .expect("keyframes opacity");
    p.execute(
        Command::SetKeyframes {
            track: 1,
            id: 1,
            property: PropertyName::X,
            keys: vec![
                Keyframe::new(ticks(0), Rational::new(-10, 1), Interpolation::Hold),
                Keyframe::new(ticks(48), Rational::new(20, 1), Interpolation::Linear),
            ],
        },
        OWNER,
    )
    .expect("keyframes x");
    p.execute(
        Command::Resize {
            track: 1,
            id: 1,
            duration: ticks(60),
        },
        OWNER,
    )
    .expect("resize");
    p.execute(Command::Remove { track: 1, id: 100 }, OWNER)
        .expect("remove");

    // 3. undo×2 → redo×2 (exact inverses; hash must return)
    p.undo(OWNER).expect("undo 1");
    p.undo(OWNER).expect("undo 2");
    p.redo(OWNER).expect("redo 1");
    p.redo(OWNER).expect("redo 2");

    let hash_after_edits = p.state_hash();
    let undo_depth = p.undo_depth();
    println!("STATE_HASH={hash_after_edits}");
    println!("UNDO_DEPTH={undo_depth}");

    // keyframe spot evaluation: opacity at t=12 ticks (mid ramp, Linear)
    // and x at t=12 (Hold → still the first key's value)
    let t = p.timeline();
    let pos1 = t
        .track_ref(1)
        .expect("track 1")
        .index_of(1)
        .expect("clip 1 position");
    let clip1 = t
        .track_ref(1)
        .expect("track 1")
        .clip_at(pos1)
        .expect("clip 1");
    let op = clip1
        .properties
        .opacity
        .evaluate(ticks(12))
        .expect("opacity eval @12");
    let xv = clip1.properties.x.evaluate(ticks(12)).expect("x eval @12");
    println!("OPACITY_AT_12={}/{}", op.num(), op.den());
    println!("X_AT_12={}/{}", xv.num(), xv.den());
    drop(p);

    // 4. reopen → log replay → the SAME hash (P-2 discipline, no kill drill
    // inside WASI — the kill-9 drill stays a native L-0 suite)
    let mut p2 = Project::open(&project_dir).expect("reopen");
    let hash_reopen = p2.state_hash();
    println!("REOPEN_HASH={hash_reopen}");
    println!("HASH_PARITY={}", hash_after_edits == hash_reopen);

    // 5. undo/redo still exact on the replayed state
    p2.undo(OWNER).expect("reopen undo");
    p2.redo(OWNER).expect("reopen redo");
    println!("REOPEN_ROUNDTRIP_PARITY={}", p2.state_hash() == hash_reopen);
    drop(p2);
    let _ = std::fs::remove_dir_all(&project_dir);

    // 6. exact-rational spot checks (§3 time-arithmetic invariant; P13)
    let a = Rational::new(1, 3);
    let b = Rational::new(1, 6);
    let s = a.add(b); // 1/2
    let m = a.mul(b); // 1/18
    let half = m.half(); // 1/36
    println!(
        "TIME_SPOT={}/{}, {}/{}, {}/{}",
        s.num(),
        s.den(),
        m.num(),
        m.den(),
        half.num(),
        half.den()
    );
    // round-half-up totality spot (P13): 1/2 → 1, 3/2 → 2
    let r1 = Rational::new(1, 2).round_half_up();
    let r2 = Rational::new(3, 2).round_half_up();
    println!("ROUND_SPOT={r1},{r2}");
    // split-half spot (half() = floor num/2, same den — the split contract):
    // 3/18 → 1/18
    let h = Rational::new(3, 18).half();
    println!("HALF_SPOT={}/{}", h.num(), h.den());
}

fn main() {
    let dir = std::env::args()
        .nth(1)
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("ove-conf-tmp"));
    run(dir);
}
