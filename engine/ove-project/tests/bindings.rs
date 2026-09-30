//! RLW-7 — RW-NOTE-1 closure: Split's right-half clip must receive the SAME
//! `clip_assets` binding as the clip it was split from, in execute AND in
//! replay (the same `record_entry_bindings` function maintains both paths).
//!
//! Binding permanence (v1 rule since W6): removing a clip never removes its
//! binding; undo/redo therefore never mutates the map. Every test here
//! pins bindings only — timeline structure is asserted only where needed to
//! prove the split happened.
//!
//! Required proofs (RLW-7 directive):
//! 1. Split creates the right-half clip binding.
//! 2. Undo/redo preserves binding correctness.
//! 3. Replay/state reconstruction produces the same binding state.
//! 5. No existing binding is lost or reassigned incorrectly.
//!    (Proof 4 — realworld 5/6 → 6/6 — lives in ove-engine/tests/realworld.rs,
//!    env-gated on the real NASA media.)

use std::path::PathBuf;

use ove_project::{Owner, Project};
use ove_time::Rational;
use ove_timeline::{Clip, Command, GapTrack, TrackKind};

const OWNER: Owner = Owner::Human;

const ASSET_A: &str = "blake3-aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa";
const ASSET_B: &str = "blake3-bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb";

fn tmp_dir(name: &str) -> PathBuf {
    let d = std::env::temp_dir().join("ove-project-bindings").join(name);
    // remove stale state but do NOT create the folder itself —
    // Project::create owns folder creation (and asserts absence)
    let _ = std::fs::remove_dir_all(&d);
    std::fs::create_dir_all(d.parent().expect("tmp root")).expect("tmp root");
    d
}

fn ticks(n: i64) -> Rational {
    Rational::new(n, 24_000)
}

fn setup_track(project: &mut Project) {
    project
        .add_track(1, TrackKind::Gap(GapTrack::new()))
        .expect("setup track 1");
}

fn insert_bound(project: &mut Project, id: u64, asset: &str) {
    let index = project.timeline().track_len(1).expect("track 1");
    project
        .execute_insert_asset(
            1,
            index,
            Clip::new(id, ticks(96), ticks(0)),
            asset.to_string(),
            OWNER,
        )
        .expect("insert with asset");
}

fn insert_unbound(project: &mut Project, id: u64) {
    let index = project.timeline().track_len(1).expect("track 1");
    project
        .execute(
            Command::Insert {
                track: 1,
                index,
                clip: Clip::new(id, ticks(96), ticks(0)),
            },
            OWNER,
        )
        .expect("insert (asset-free)");
}

fn split(project: &mut Project, id: u64, new_id: u64, at_ticks: i64) {
    project
        .execute(
            Command::Split {
                track: 1,
                id,
                at: ticks(at_ticks),
                new_id,
            },
            OWNER,
        )
        .expect("split");
}

fn clip_ids(project: &Project) -> Vec<u64> {
    let mut ids = Vec::new();
    project
        .timeline()
        .track_ref(1)
        .expect("track 1")
        .walk(&mut |_pos, _start, clip: &Clip| {
            ids.push(clip.id);
        });
    ids
}

// ---------------------------------------------------------------------------
// Proof 1 + 5: Split binds the right half to the SAME asset; every other
// binding is untouched (no loss, no reassignment).
// ---------------------------------------------------------------------------

#[test]
fn split_binds_right_half_and_preserves_all_other_bindings() {
    let dir = tmp_dir("split-binds-right-half");
    let mut p = Project::create(&dir, (24_000, 1)).expect("create");
    setup_track(&mut p);
    insert_bound(&mut p, 1, ASSET_A);
    insert_bound(&mut p, 2, ASSET_B);
    insert_unbound(&mut p, 3);

    let before = p.clip_assets().clone();
    assert_eq!(
        before.get(&1).map(String::as_str),
        Some(ASSET_A),
        "precondition: clip 1 bound"
    );
    assert_eq!(
        before.get(&2).map(String::as_str),
        Some(ASSET_B),
        "precondition: clip 2 bound"
    );
    assert!(!before.contains_key(&3), "precondition: clip 3 asset-free");

    split(&mut p, 1, 10, 48); // split the ASSET_A clip
    split(&mut p, 3, 11, 24); // split the ASSET-FREE clip

    let after = p.clip_assets();
    assert_eq!(
        after.get(&10).map(String::as_str),
        Some(ASSET_A),
        "RW-NOTE-1: split's right half inherits the source clip's binding"
    );
    assert_eq!(
        after.get(&1).map(String::as_str),
        Some(ASSET_A),
        "left half keeps its binding unchanged"
    );
    assert_eq!(
        after.get(&2).map(String::as_str),
        Some(ASSET_B),
        "unrelated binding untouched (no reassignment)"
    );
    assert!(
        !after.contains_key(&11),
        "asset-free split must not invent a binding (mirrors Insert(asset: None))"
    );
    assert_eq!(
        after.len(),
        3,
        "exactly clips 1+2+10 bound: 2 originals + 1 new right half"
    );
    // structure sanity: the splits actually happened
    assert_eq!(
        clip_ids(&p),
        vec![1, 10, 2, 3, 11],
        "right halves inserted directly after their sources"
    );
}

// ---------------------------------------------------------------------------
// Chain: splitting the right half again keeps the binding lineage exact.
// ---------------------------------------------------------------------------

#[test]
fn split_chain_right_half_resplit_inherits_binding() {
    let dir = tmp_dir("split-chain");
    let mut p = Project::create(&dir, (24_000, 1)).expect("create");
    setup_track(&mut p);
    insert_bound(&mut p, 1, ASSET_A);

    split(&mut p, 1, 2, 48); // 1 → 2 (clip 2 duration = 48)
    split(&mut p, 2, 3, 24); // 2 → 3 (right of the right, at < 48)

    let after = p.clip_assets();
    assert_eq!(after.get(&1).map(String::as_str), Some(ASSET_A));
    assert_eq!(after.get(&2).map(String::as_str), Some(ASSET_A));
    assert_eq!(
        after.get(&3).map(String::as_str),
        Some(ASSET_A),
        "grandchild inherits through the chain"
    );
    assert_eq!(after.len(), 3, "no extra or missing bindings in the chain");
}

// ---------------------------------------------------------------------------
// Proof 2: undo/redo never mutates the binding map (binding permanence, v1).
// ---------------------------------------------------------------------------

#[test]
fn split_binding_unchanged_by_undo_and_redo() {
    let dir = tmp_dir("split-undo-redo");
    let mut p = Project::create(&dir, (24_000, 1)).expect("create");
    setup_track(&mut p);
    insert_bound(&mut p, 1, ASSET_A);
    split(&mut p, 1, 2, 48);

    let after_split: Vec<(u64, String)> = p
        .clip_assets()
        .iter()
        .map(|(k, v)| (*k, v.clone()))
        .collect();
    assert_eq!(after_split.len(), 2, "right half bound after split");

    // UNDO: timeline structure restored; bindings are PERMANENT (v1 rule —
    // the same behavior an Insert undo already has). No loss, no reassign.
    assert!(p.undo(OWNER).expect("undo"), "undo executed");
    let after_undo: Vec<(u64, String)> = p
        .clip_assets()
        .iter()
        .map(|(k, v)| (*k, v.clone()))
        .collect();
    assert_eq!(
        after_undo, after_split,
        "undo must not mutate the binding map"
    );
    assert_eq!(
        clip_ids(&p),
        vec![1],
        "undo restored the pre-split structure"
    );

    // REDO: same map, structure split again.
    assert!(p.redo(OWNER).expect("redo"), "redo executed");
    let after_redo: Vec<(u64, String)> = p
        .clip_assets()
        .iter()
        .map(|(k, v)| (*k, v.clone()))
        .collect();
    assert_eq!(
        after_redo, after_split,
        "redo must not mutate the binding map"
    );
    assert_eq!(clip_ids(&p), vec![1, 2], "redo restored the split");

    // Undo past the split AND the insert (2 undos), then redo both: map is
    // still byte-identical — permanence holds across multi-step cycles.
    assert!(p.undo(OWNER).expect("undo split again"));
    assert!(p.undo(OWNER).expect("undo insert"));
    assert!(p.redo(OWNER).expect("redo insert"));
    assert!(p.redo(OWNER).expect("redo split"));
    let after_cycle: Vec<(u64, String)> = p
        .clip_assets()
        .iter()
        .map(|(k, v)| (*k, v.clone()))
        .collect();
    assert_eq!(
        after_cycle, after_split,
        "full undo/redo cycle leaves bindings untouched"
    );
}

// ---------------------------------------------------------------------------
// Proof 3: replay (reopen) rebuilds the IDENTICAL binding map and hash —
// execute and replay share record_entry_bindings, so both paths must agree.
// ---------------------------------------------------------------------------

#[test]
fn replay_rebuilds_identical_binding_map_and_hash() {
    let dir = tmp_dir("split-replay");
    let mut p = Project::create(&dir, (24_000, 1)).expect("create");
    setup_track(&mut p);
    insert_bound(&mut p, 1, ASSET_A);
    insert_bound(&mut p, 2, ASSET_B);
    insert_unbound(&mut p, 3);
    split(&mut p, 1, 10, 48);
    split(&mut p, 3, 11, 24); // asset-free split
    split(&mut p, 10, 12, 24); // chain on the bound right half
    p.execute(
        Command::SetKeyframes {
            track: 1,
            id: 1,
            property: ove_timeline::PropertyName::Opacity,
            keys: vec![ove_timeline::Keyframe {
                time: ticks(0),
                value: Rational::new(1, 1),
                interp: ove_timeline::Interpolation::Linear,
            }],
        },
        OWNER,
    )
    .expect("keyframes");
    // undo + redo so the log carries markers too (the replay must survive
    // Undo/Redo entries between binding-relevant entries).
    assert!(p.undo(OWNER).expect("undo"));
    assert!(p.redo(OWNER).expect("redo"));

    let live_map: Vec<(u64, String)> = p
        .clip_assets()
        .iter()
        .map(|(k, v)| (*k, v.clone()))
        .collect();
    let live_hash = p.state_hash();
    assert_eq!(
        live_map.len(),
        4,
        "clips 1+2+10+12 bound; clips 3 and 11 asset-free"
    );

    drop(p);
    let reopened = Project::open(&dir).expect("reopen");
    let replayed_map: Vec<(u64, String)> = reopened
        .clip_assets()
        .iter()
        .map(|(k, v)| (*k, v.clone()))
        .collect();
    assert_eq!(
        replayed_map, live_map,
        "replay must rebuild the binding map identically (RW-NOTE-1 closure)"
    );
    assert_eq!(
        reopened.state_hash(),
        live_hash,
        "replay hash == live hash (bindings are document state)"
    );
}

// ---------------------------------------------------------------------------
// Compaction continuity: the snapshot mirror is the binding authority across
// compaction — entries folded into the snapshot (Inserts AND Splits) must
// reappear in the map after reopen, with suffix replay appending on top.
// ---------------------------------------------------------------------------

#[test]
fn compaction_preserves_split_bindings_across_reopen() {
    let dir = tmp_dir("split-compaction");
    let mut p = Project::create(&dir, (24_000, 1)).expect("create");
    setup_track(&mut p);
    insert_bound(&mut p, 1, ASSET_A);
    insert_bound(&mut p, 2, ASSET_B);
    split(&mut p, 1, 10, 48); // RW-NOTE-1 binding (folded into the snapshot)

    let live_map: Vec<(u64, String)> = p
        .clip_assets()
        .iter()
        .map(|(k, v)| (*k, v.clone()))
        .collect();
    let live_hash = p.state_hash();
    assert_eq!(live_map.len(), 3, "clips 1+2+10 bound pre-compaction");

    // fold EVERYTHING into the snapshot (suffix becomes empty)
    p.snapshot().expect("snapshot");
    drop(p);

    let mut r = Project::open(&dir).expect("reopen after compaction");
    let replayed_map: Vec<(u64, String)> = r
        .clip_assets()
        .iter()
        .map(|(k, v)| (*k, v.clone()))
        .collect();
    assert_eq!(
        replayed_map, live_map,
        "compaction must preserve split-derived bindings (snapshot mirror)"
    );
    assert_eq!(r.state_hash(), live_hash, "hash parity across compaction");

    // suffix replay APPENDS onto the snapshotted map
    let index = r.timeline().track_len(1).expect("track 1");
    r.execute_insert_asset(
        1,
        index,
        Clip::new(20, ticks(96), ticks(96)),
        ASSET_B.to_string(),
        OWNER,
    )
    .expect("post-compaction bound insert");
    let live2: Vec<(u64, String)> = r
        .clip_assets()
        .iter()
        .map(|(k, v)| (*k, v.clone()))
        .collect();
    let hash2 = r.state_hash();
    drop(r);
    let r2 = Project::open(&dir).expect("second reopen");
    let replayed2: Vec<(u64, String)> = r2
        .clip_assets()
        .iter()
        .map(|(k, v)| (*k, v.clone()))
        .collect();
    assert_eq!(replayed2, live2, "suffix binding appends onto the mirror");
    assert_eq!(replayed2.len(), 4, "clip 20 joins 1+2+10");
    assert_eq!(r2.state_hash(), hash2);
}

// ---------------------------------------------------------------------------
// Reopen after SPLIT-ONLY history (no undo markers): the replay path binds
// the right half from the Split entry alone.
// ---------------------------------------------------------------------------

#[test]
fn reopen_after_split_only_replay_binds_right_half() {
    let dir = tmp_dir("split-reopen-only");
    let mut p = Project::create(&dir, (24_000, 1)).expect("create");
    setup_track(&mut p);
    insert_bound(&mut p, 1, ASSET_A);
    split(&mut p, 1, 2, 48);

    let live_map: Vec<(u64, String)> = p
        .clip_assets()
        .iter()
        .map(|(k, v)| (*k, v.clone()))
        .collect();
    let live_hash = p.state_hash();
    drop(p);

    let r = Project::open(&dir).expect("reopen");
    let replayed_map: Vec<(u64, String)> = r
        .clip_assets()
        .iter()
        .map(|(k, v)| (*k, v.clone()))
        .collect();
    assert_eq!(replayed_map, live_map, "split-only replay binds right half");
    assert_eq!(replayed_map.len(), 2);
    assert_eq!(r.state_hash(), live_hash);
}
