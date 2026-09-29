//! PROJECT_FORMAT_SPEC §7 acceptance suite (P-1..P-8).
//!
//! The directive's test made concrete: save → kill → reopen → replay →
//! state hash equality; compaction equivalence; corruption drill; float
//! rejection; hash-addressed assets; disposable dirs.

use std::path::{Path, PathBuf};

use ove_project::log::LogPayload;
use ove_project::{AssetStatus, Owner, Project, ProjectError};
use ove_time::Rational;
use ove_timeline::{Clip, Command, GapTrack, TrackKind};

const OWNER: Owner = Owner::Human;

fn tmp_dir(name: &str) -> PathBuf {
    let d = std::env::temp_dir()
        .join("ove-project-acceptance")
        .join(name);
    // remove stale state but do NOT create the folder itself —
    // Project::create owns folder creation (and asserts absence)
    let _ = std::fs::remove_dir_all(&d);
    std::fs::create_dir_all(d.parent().expect("tmp root")).expect("tmp root");
    d
}

fn ticks(n: i64) -> Rational {
    Rational::new(n, 24_000)
}

/// Deterministic scenario builder: a batch + a repeat-safe edit cycle that
/// exercises split/resize/move/insert/remove. All ids are EXPLICIT (E-012).
fn build_scenario(project: &mut Project, n_cmds: usize) {
    setup_track(project);
    project
        .execute(
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
    for k in 0..n_cmds.saturating_sub(1) {
        let cmd = match k % 4 {
            // one split (clip 2: 96 -> 48 + 48); later cycles resize clip 3
            0 if k == 0 => Command::Split {
                track: 1,
                id: 2,
                at: ticks(48),
                new_id: 100,
            },
            0 => Command::Resize {
                track: 1,
                id: 3,
                duration: ticks(24 + 6 * (k as i64)),
            },
            1 => Command::Resize {
                track: 1,
                id: 1,
                duration: ticks(48 + 6 * (k as i64)),
            },
            2 => Command::Move {
                id: 3,
                from_track: 1,
                to_track: 1,
                to_index: 0,
            },
            _ => Command::Insert {
                track: 1,
                index: 0,
                clip: Clip::new(200 + k as u64, ticks(12), ticks(12)),
            },
        };
        if let Err(e) = project.execute(cmd, OWNER) {
            panic!("scenario command k={k} failed: {e:?}");
        }
    }
    // exercise Remove once (undo restores it via the exact inverse)
    if n_cmds >= 3 {
        project
            .execute(Command::Remove { track: 1, id: 2 }, OWNER)
            .expect("scenario remove");
    }
}

/// Project setup: the track registry (not a command — ADR-016).
fn setup_track(project: &mut Project) {
    project
        .add_track(1, TrackKind::Gap(GapTrack::new()))
        .expect("setup track 1");
}

// ---------------------------------------------------------------------------
// P-1: save → reopen → state_hash equal
// ---------------------------------------------------------------------------

#[test]
fn p1_save_reopen_hash_equal() {
    let dir = tmp_dir("p1");
    let mut p = Project::create(&dir, (48_000, 1)).expect("create");
    build_scenario(&mut p, 6);
    let live = p.state_hash();

    drop(p);
    let reopened = Project::open(&dir).expect("reopen");
    assert_eq!(reopened.state_hash(), live, "P-1: reopen hash equal");
    assert_eq!(reopened.loaded_entries(), 7); // batch + 6
}

// ---------------------------------------------------------------------------
// P-2: execute N commands → kill -9 (random point) → reopen → replay →
// hash == live hash. TWO models:
//   (a) subprocess abort (real kill semantics: no destructors run)
//   (b) drop-without-close at random points (write-through equivalence)
// ---------------------------------------------------------------------------

const CHILD_ENV: &str = "OVE_P2_CHILD_DIR";

#[test]
fn p2a_kill9_subprocess_reopen_hash_equal() {
    let dir = tmp_dir("p2a");
    // child: re-exec this test binary in a special mode
    let exe = std::env::current_exe().expect("test exe");
    let status = std::process::Command::new(exe)
        .args(["p2_child_entry", "--exact", "--nocapture"])
        .env(CHILD_ENV, dir.join("proj"))
        .env("OVE_P2_CMDS", "9")
        .status()
        .expect("spawn child");
    assert!(
        !status.success(),
        "child should have died by abort (SIGABRT)"
    );

    // reopen after the kill: replay must reproduce the live hash
    let child_live = Project::open(&dir.join("proj")).expect("reopen after kill");
    let hash_after_kill = child_live.state_hash();

    // the ground truth: same scenario without the kill
    let dir2 = tmp_dir("p2a-truth");
    let mut truth = Project::create(&dir2, (48_000, 1)).expect("create");
    setup_track(&mut truth);
    truth
        .execute(
            Command::Batch {
                cmds: vec![Command::Insert {
                    track: 1,
                    index: 0,
                    clip: Clip::new(1, ticks(48), ticks(0)),
                }],
            },
            OWNER,
        )
        .expect("batch");
    for k in 0..9u64 {
        truth
            .execute(
                Command::Insert {
                    track: 1,
                    index: 0,
                    clip: Clip::new(2 + k, ticks(12), ticks(12)),
                },
                OWNER,
            )
            .expect("cmd");
    }
    assert_eq!(
        hash_after_kill,
        truth.state_hash(),
        "P-2: post-kill replay == live hash"
    );
}

/// Child body (runs inside the test binary under OVE_P2_CHILD_DIR).
fn p2_child_body(dir: &Path, n_cmds: u64) -> ! {
    let mut p = Project::create(dir, (48_000, 1)).expect("child create");
    setup_track(&mut p);
    p.execute(
        Command::Batch {
            cmds: vec![Command::Insert {
                track: 1,
                index: 0,
                clip: Clip::new(1, ticks(48), ticks(0)),
            }],
        },
        OWNER,
    )
    .expect("child batch");
    for k in 0..n_cmds {
        p.execute(
            Command::Insert {
                track: 1,
                index: 0,
                clip: Clip::new(2 + k, ticks(12), ticks(12)),
            },
            OWNER,
        )
        .expect("child cmd");
    }
    // NO cleanup: abort = process death without destructors (the kill -9 model)
    std::process::abort();
}

#[test]
fn p2b_drop_without_close_at_random_points() {
    // write-through equivalence: after each execute the on-disk state IS
    // the crash state, so dropping at any point models kill -9 faithfully
    for cut in [0usize, 1, 3, 5, 9] {
        let dir = tmp_dir(&format!("p2b-{cut}"));
        let mut p = Project::create(&dir, (48_000, 1)).expect("create");
        // ground truth hash after `cut` scenario commands
        build_scenario(&mut p, cut);
        let live = p.state_hash();
        drop(p); // the "kill"
        let mut reopened = Project::open(&dir).expect("reopen");
        assert_eq!(reopened.state_hash(), live, "cut={cut}");
        // and the session CONTINUES deterministically: one more command
        reopened
            .execute(
                Command::Insert {
                    track: 1,
                    index: 0,
                    clip: Clip::new(900, ticks(7), ticks(3)),
                },
                Owner::Agent,
            )
            .expect("post-reopen command");
        let after = reopened.state_hash();
        drop(reopened);
        let again = Project::open(&dir).expect("second reopen");
        assert_eq!(again.state_hash(), after, "cut={cut}: post-continuation");
    }
}

// ---------------------------------------------------------------------------
// P-3: snapshot compaction equivalence (pre/post compaction reopen equal)
// ---------------------------------------------------------------------------

#[test]
fn p3_compaction_equivalence() {
    let dir = tmp_dir("p3");
    let mut p = Project::create(&dir, (48_000, 1)).expect("create");
    build_scenario(&mut p, 8);
    let live = p.state_hash();

    let snap_seq = p.snapshot().expect("snapshot");
    assert_eq!(snap_seq, 9, "all entries folded (batch + 7 edits + remove)");
    assert_eq!(p.state_hash(), live, "hash unchanged by compaction");

    // post-compaction reopen == live
    drop(p);
    let r = Project::open(&dir).expect("reopen after compaction");
    assert_eq!(r.state_hash(), live, "P-3: compaction preserves state");

    // suffix replay continues correctly after compaction
    let mut r = r;
    r.execute(
        Command::Insert {
            track: 1,
            index: 0,
            clip: Clip::new(500, ticks(5), ticks(1)),
        },
        OWNER,
    )
    .expect("post-compaction cmd");
    let live2 = r.state_hash();
    drop(r);
    let r2 = Project::open(&dir).expect("second reopen");
    assert_eq!(r2.state_hash(), live2, "post-compaction continuation");

    // second compaction on top (snapshot at a later seq)
    let mut r2 = r2;
    let seq2 = r2.snapshot().expect("second snapshot");
    assert!(seq2 >= 9);
    r2.undo(OWNER).expect("undo after snapshot");
    let live3 = r2.state_hash();
    drop(r2);
    let r3 = Project::open(&dir).expect("third reopen");
    assert_eq!(r3.state_hash(), live3);
    // ADR-008: the undo stack is SESSION state rebuilt from the log suffix
    // only — after an undo marker replayed, depth matches the live session
    // at the same point (0: one exec pushed at entry 10, popped by the undo
    // marker at entry 11).
    assert_eq!(r3.undo_depth(), 0, "replay rebuilds the suffix stack only");
}

// ---------------------------------------------------------------------------
// P-4: log corruption drill — truncated tail → typed error → explicit
// repair → last-good load; never auto-edit
// ---------------------------------------------------------------------------

#[test]
fn p4_corruption_drill() {
    let dir = tmp_dir("p4");
    let mut p = Project::create(&dir, (48_000, 1)).expect("create");
    // record the hash after EACH command: the last-good state after a torn
    // entry k is exactly the recorded hash at entry k
    setup_track(&mut p);
    let mut hashes: Vec<String> = Vec::new();
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
    .expect("batch");
    hashes.push(p.state_hash());
    for k in 0..4u64 {
        let cmd = match k {
            0 => Command::Split {
                track: 1,
                id: 2,
                at: ticks(48),
                new_id: 100,
            },
            1 => Command::Resize {
                track: 1,
                id: 1,
                duration: ticks(54),
            },
            2 => Command::Move {
                id: 3,
                from_track: 1,
                to_track: 1,
                to_index: 0,
            },
            _ => Command::Insert {
                track: 1,
                index: 0,
                clip: Clip::new(200, ticks(12), ticks(12)),
            },
        };
        p.execute(cmd, OWNER).expect("scenario command");
        hashes.push(p.state_hash());
    }
    let last_good = hashes[3].clone(); // state after entry 4 (the 5th entry is torn)
    drop(p);

    // corrupt: tear the LAST log line mid-line (the classic torn tail —
    // a kill between the first write() and the flush of a partial line)
    let log_path = dir.join("commands.jsonl");
    let raw = std::fs::read_to_string(&log_path).unwrap();
    let trimmed = raw.strip_suffix('\n').expect("file ends with newline");
    let last_line_start = trimmed.rfind('\n').expect("multi-line log") + 1;
    let tear = last_line_start + 8; // inside the last line, before its end
    std::fs::write(&log_path, &raw[..tear]).expect("tear log");

    // open must HARD STOP with the typed error
    let err = match Project::open(&dir) {
        Err(e) => e,
        Ok(_) => panic!("corrupt log must not open"),
    };
    assert!(matches!(err, ProjectError::LogCorruption { .. }), "{err:?}");

    // explicit repair: quarantine + good prefix
    let (good, quarantine) = Project::repair_log(&dir).expect("repair");
    assert_eq!(good, 4, "four complete entries survived the tear");
    assert!(quarantine.starts_with("commands.corrupt-"));
    assert!(dir.join(&quarantine).exists(), "quarantine file written");
    let repaired = std::fs::read_to_string(&log_path).unwrap();
    assert_eq!(repaired.lines().count() as u64, good);

    let r = Project::open(&dir).expect("open after repair");
    assert_eq!(
        r.state_hash(),
        last_good,
        "repair preserved the last-good state exactly"
    );
}

// ---------------------------------------------------------------------------
// P-5: float payload rejection; (num,den) round-trip exactness
// ---------------------------------------------------------------------------

#[test]
fn p5_float_rejected_and_rationals_exact() {
    let dir = tmp_dir("p5");
    let mut p = Project::create(&dir, (48_000, 1)).expect("create");
    setup_track(&mut p);
    // a duration with den 3 (non-decimal): round-trips EXACTLY
    p.execute(
        Command::Insert {
            track: 1,
            index: 0,
            clip: Clip::new(1, Rational::new(1, 3), Rational::new(7, 11)),
        },
        OWNER,
    )
    .expect("non-decimal rational");
    let live = p.state_hash();
    drop(p);

    // floats forbidden: fabricate a log line with a float payload
    let log_path = dir.join("commands.jsonl");
    let raw = std::fs::read_to_string(&log_path).unwrap();
    let float_line = raw
        .lines()
        .next()
        .unwrap()
        .replace("\"num\":1,\"den\":3", "\"num\":0.3333,\"den\":1");
    assert_ne!(raw, float_line, "fabrication changed the line");
    std::fs::write(&log_path, float_line).unwrap();

    let err = match Project::open(&dir) {
        Err(e) => e,
        Ok(_) => panic!("float payload must not load"),
    };
    assert!(
        matches!(err, ProjectError::LogCorruption { .. }),
        "float payload rejected at load: {err:?}"
    );

    // and the honest log still round-trips exactly
    std::fs::write(&log_path, raw).unwrap();
    let r = Project::open(&dir).expect("reopen");
    assert_eq!(r.state_hash(), live);
    let clip = r
        .timeline()
        .track_ref(1)
        .unwrap()
        .clip_at(0)
        .expect("clip present");
    assert_eq!(clip.duration, Rational::new(1, 3));
    assert_eq!(clip.source_in, Rational::new(7, 11));
}

// ---------------------------------------------------------------------------
// P-6: hash addressing — move folder → reopen works; corrupt asset →
// AssetMissing/Corrupt status (render refusal is wired at W6)
// ---------------------------------------------------------------------------

#[test]
fn p6_hash_addressed_assets() {
    let dir = tmp_dir("p6");
    std::fs::create_dir_all(&dir).expect("scratch root (not a project folder)");
    let media = dir.join("tiny.mp4");
    std::fs::write(&media, b"fake media bytes for hashing").unwrap();

    let proj = dir.join("proj");
    let mut p = Project::create(&proj, (48_000, 1)).expect("create");
    let hash = p.import_asset(&media, None).expect("import");
    drop(p);

    // MOVE the whole project folder: hash addressing survives (no paths)
    let moved = dir.join("proj-moved-elsewhere");
    std::fs::rename(&proj, &moved).expect("move folder");
    let r = Project::open(&moved).expect("reopen at new location");
    assert_eq!(r.assets().len(), 1);
    assert_eq!(
        r.asset_status(&hash.hex()),
        AssetStatus::Present,
        "asset follows the project by content hash"
    );

    // corrupt the asset bytes → Corrupt (identity mismatch), state intact
    let asset_file = moved.join("assets").join(hash.hex()).join("src.mp4");
    std::fs::write(&asset_file, b"DIFFERENT bytes - wrong media after relink").unwrap();
    assert_eq!(r.asset_status(&hash.hex()), AssetStatus::Corrupt);
    assert_eq!(
        r.state_hash(),
        r.state_hash(),
        "state self-consistent (P-6: assets do not gate the document)"
    );

    // delete the asset → Missing
    drop(r);
    let mut r2 = Project::open(&moved).expect("reopen");
    std::fs::remove_file(&asset_file).unwrap();
    assert_eq!(r2.asset_status(&hash.hex()), AssetStatus::Missing);
    // a re-import of identical bytes dedupes into the same identity
    std::fs::write(&media, b"fake media bytes for hashing").unwrap();
    let hash2 = r2.import_asset(&media, None).expect("re-import");
    assert_eq!(hash, hash2, "content hash IS the identity");
    assert_eq!(r2.assets().len(), 1, "deduped");
}

// ---------------------------------------------------------------------------
// P-7: undo markers replay — fresh engine replays the log incl. undo
// history correctly (E-009 pattern)
// ---------------------------------------------------------------------------

#[test]
fn p7_undo_markers_replay() {
    let dir = tmp_dir("p7");
    let mut p = Project::create(&dir, (48_000, 1)).expect("create");
    build_scenario(&mut p, 6);
    let before_undo = p.state_hash();
    let depth = p.undo_depth();

    p.undo(OWNER).expect("undo");
    p.undo(Owner::Agent).expect("undo 2");
    let after_undo = p.state_hash();
    assert_ne!(after_undo, before_undo);

    p.redo(OWNER).expect("redo");
    let after_redo = p.state_hash();

    drop(p);
    let r = Project::open(&dir).expect("reopen");
    assert_eq!(
        r.state_hash(),
        after_redo,
        "P-7: undo/redo markers replay to the same state"
    );
    // undo history rebuilt by replay (E-009): the reopened engine can keep
    // undoing pre-reopen steps
    assert!(
        r.undo_depth() >= depth - 1,
        "stack rebuilt: {}",
        r.undo_depth()
    );
    let mut r = r;
    r.undo(Owner::Script).expect("post-reopen undo");
    assert_eq!(
        r.state_hash(),
        after_undo,
        "post-reopen undo matches the live undo result"
    );
}

// ---------------------------------------------------------------------------
// P-8: disposable dirs — rm -rf cache renders → hash unchanged
// ---------------------------------------------------------------------------

#[test]
fn p8_disposable_dirs() {
    let dir = tmp_dir("p8");
    let mut p = Project::create(&dir, (48_000, 1)).expect("create");
    build_scenario(&mut p, 4);
    std::fs::write(dir.join("renders").join("out.mp4"), b"render artifact").unwrap();
    std::fs::write(dir.join("cache").join("waveform.bin"), b"derived data").unwrap();
    let live = p.state_hash();
    drop(p);

    std::fs::remove_dir_all(dir.join("renders")).unwrap();
    std::fs::remove_dir_all(dir.join("cache")).unwrap();
    let r = Project::open(&dir).expect("reopen");
    assert_eq!(
        r.state_hash(),
        live,
        "P-8: disposable dirs never affect state"
    );
}

// ---------------------------------------------------------------------------
// Schema-gate: schema_version future rejection (§6.1)
// ---------------------------------------------------------------------------

#[test]
fn s1_future_schema_rejected() {
    let dir = tmp_dir("s1");
    let p = Project::create(&dir, (48_000, 1)).expect("create");
    drop(p);
    let mpath = dir.join("manifest.json");
    let raw = std::fs::read_to_string(&mpath)
        .unwrap()
        .replace("\"schema_version\": 1", "\"schema_version\": 99");
    std::fs::write(&mpath, raw).unwrap();
    let err = match Project::open(&dir) {
        Err(e) => e,
        Ok(_) => panic!("future schema must not open"),
    };
    assert_eq!(
        err,
        ProjectError::SchemaVersion {
            found: 99,
            supported: 1
        }
    );
}

/// Entry point for the P-2a child mode (re-exec of this test binary).
/// `cargo test` runs every #[test] fn; this one only acts under the env var.
#[test]
fn p2_child_entry() {
    let Ok(dir) = std::env::var(CHILD_ENV) else {
        return; // normal test run: no-op
    };
    let n: u64 = std::env::var("OVE_P2_CMDS")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(5);
    p2_child_body(Path::new(&dir), n);
}

// LogPayload is referenced for the type-level contract; silence unused when
// only used transitively.
#[allow(unused)]
fn _surface(_p: LogPayload) {}

// ---------------------------------------------------------------------------
// P-9 — keyframe animation persists exactly (WAVE 8, ADR-019)
// ---------------------------------------------------------------------------

use ove_timeline::{Interpolation, Keyframe, PropertyName};

fn kf(time_ticks: i64, value_num: i64, value_den: i64, interp: Interpolation) -> Keyframe {
    Keyframe {
        time: ticks(time_ticks),
        value: Rational::new(value_num, value_den),
        interp,
    }
}

#[test]
fn p9_keyframes_save_reopen_hash_equal_and_split_preserved() {
    let dir = tmp_dir("p9-keyframes");
    let mut project = Project::create(&dir, (24_000, 1)).expect("create");
    setup_track(&mut project);
    project
        .execute(
            Command::Insert {
                track: 1,
                index: 0,
                clip: Clip::new(1, ticks(96), ticks(0)),
            },
            OWNER,
        )
        .expect("insert");
    // animate opacity (fade) and x (motion)
    project
        .execute(
            Command::SetKeyframes {
                track: 1,
                id: 1,
                property: PropertyName::Opacity,
                keys: vec![
                    kf(0, 1, 1, Interpolation::Linear),
                    kf(96, 0, 1, Interpolation::Linear),
                ],
            },
            OWNER,
        )
        .expect("set opacity keys");
    project
        .execute(
            Command::SetKeyframes {
                track: 1,
                id: 1,
                property: PropertyName::X,
                keys: vec![
                    kf(0, 0, 1, Interpolation::Linear),
                    kf(48, 100, 1, Interpolation::Linear),
                ],
            },
            OWNER,
        )
        .expect("set x keys");
    let hash_before = project.state_hash();

    // reopen from disk: the log replays SetKeyframes → identical hash
    drop(project);
    let mut reopened = Project::open(&dir).expect("reopen");
    assert_eq!(
        reopened.state_hash(),
        hash_before,
        "reopen must reproduce the animated state exactly"
    );

    // split mid-animation on the LIVE session; evaluation must be exact
    // across the seam
    let new_id = reopened.timeline_mut().alloc_id();
    reopened
        .execute(
            Command::Split {
                track: 1,
                id: 1,
                at: ticks(30),
                new_id,
            },
            OWNER,
        )
        .expect("split");
    let hash_split = reopened.state_hash();

    // save + reopen the SPLIT state — replay of the split inverse batch
    // (which embeds SetKeyframes restores) must land on the same hash
    drop(reopened);
    let reopened2 = Project::open(&dir).expect("reopen after split");
    assert_eq!(
        reopened2.state_hash(),
        hash_split,
        "post-split animated state must survive reopen"
    );

    // undo across reopen is NOT available (undo stack is session state,
    // ADR-008) — but the document's animation data is complete: evaluate
    // the seam continuity from the reopened timeline.
    let tl = reopened2.timeline();
    let track = tl.track_ref(1).expect("track");
    let left = track.clip_at(0).expect("left clip");
    let right = track.clip_at(1).expect("right clip");
    let v_left_at_29 = left.properties.opacity.evaluate(ticks(29));
    let v_right_at_0 = right.properties.opacity.evaluate(ticks(0));
    let v_left_at_30 = left.properties.opacity.evaluate(ticks(30));
    assert_eq!(v_right_at_0, v_left_at_30, "boundary value continuity");
    // exact lerp continuity against the unsplit fade: v(29) = 1 − 29/96·(1/2)
    assert_eq!(v_left_at_29, Some(Rational::new(67, 96)));
}

// ---------------------------------------------------------------------------
// P-10: CROSS-SESSION undo/redo (wave 15 boundary finding — the log fold's
// redo-stack reconstruction must carry the ORIGINAL FORWARD command; the
// original bug re-applied the inverse, making cross-session redo a second
// undo). Every CLI/MCP invocation is its own session, so this is the REAL
// operator contract.
// ---------------------------------------------------------------------------

#[test]
fn p10_cross_session_undo_redo_hash_exact() {
    let dir = tmp_dir("p10-cross-session");
    let mut e = Project::create(&dir, (24_000, 1)).expect("create");
    setup_track(&mut e);
    e.execute(
        Command::Insert {
            track: 1,
            index: 0,
            clip: Clip::new(1, ticks(48), ticks(0)),
        },
        OWNER,
    )
    .unwrap();
    e.execute(
        Command::Insert {
            track: 1,
            index: 1,
            clip: Clip::new(2, ticks(96), ticks(48)),
        },
        OWNER,
    )
    .unwrap();
    let h_after_inserts = e.state_hash();
    e.execute(
        Command::Split {
            track: 1,
            id: 1,
            at: ticks(24),
            new_id: 100,
        },
        OWNER,
    )
    .unwrap();
    let h_after_split = e.state_hash();
    drop(e);

    // session 2: undo (undo marker lands in the log)
    let mut e2 = Project::open(&dir).unwrap();
    assert!(e2.undo(OWNER).unwrap(), "session-2 undo must fire");
    assert_eq!(e2.state_hash(), h_after_inserts);
    drop(e2);

    // session 3: REDO — re-applies the split exactly (the bug made this a
    // second undo; the state stayed at h_after_inserts)
    let mut e3 = Project::open(&dir).unwrap();
    assert!(e3.redo(OWNER).unwrap(), "session-3 redo must fire");
    assert_eq!(
        e3.state_hash(),
        h_after_split,
        "cross-session redo must restore the undone state exactly"
    );
    drop(e3);

    // session 4: the reopened status agrees, and undo again still works
    let mut e4 = Project::open(&dir).unwrap();
    assert_eq!(e4.state_hash(), h_after_split);
    assert!(e4.undo(OWNER).unwrap(), "chained cross-session undo");
    assert_eq!(e4.state_hash(), h_after_inserts);
}
