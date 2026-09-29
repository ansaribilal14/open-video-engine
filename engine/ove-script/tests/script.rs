//! WAVE 16 — scripting-client conformance: Rhai drives the SAME engine
//! semantics as the CLI/batch/MCP shells. Pins: no-float parse rejection
//! (ME-7), den==0 loud rejection, full edit cycle, determinism, and the
//! wave-15 cross-session undo/redo contract exercised FROM A SCRIPT.

use std::path::Path;

fn fixture(name: &str) -> String {
    let p = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../ove-encode/tests/media")
        .join(name);
    assert!(p.exists(), "corpus fixture missing: {}", p.display());
    p.display().to_string()
}

#[test]
fn script_full_cycle_and_boundary_rules() {
    let root = std::env::temp_dir().join("ove-script-test");
    let _ = std::fs::remove_dir_all(&root);
    std::fs::create_dir_all(&root).expect("root");
    let dir = root.join("proj");
    let media = fixture("copy24.mp4");

    let script = format!(
        r#"
        ove.emit(ove.create_project(24000, 1));
        ove.emit(ove.add_track(1));
        let imported = ove.import_media("{media}");
        let hash = imported.split(" as ")[1];
        ove.emit(ove.add_clip(1, hash, r(48, 24000), r(0, 1)));
        ove.emit(ove.split(1, 1, r(24, 24000)));
        ove.emit(ove.undo());
        ove.emit(ove.redo());
        ove.emit(ove.get_status());
        "#
    );

    let out1 = ove_script::run_script(&dir, &script).expect("script run 1");
    for frag in [
        "track 1 added",
        "clip 1 added to track 1",
        "split clip 1 at 1/1000 -> new clip 2",
        "undone",
        "redone",
        "state_hash=",
    ] {
        assert!(out1.contains(frag), "output missing {frag:?}:\n{out1}");
    }

    // determinism: a fresh project + same script → same state hash line
    let dir2 = root.join("proj2");
    let out2 = ove_script::run_script(&dir2, &script).expect("script run 2");
    let last = |s: &str| s.lines().last().unwrap().to_string();
    assert_eq!(
        last(&out1),
        last(&out2),
        "same script → same document state"
    );

    // ME-7 at the scripting edge: a float literal cannot even PARSE
    // (no_float turns `0.5` into a syntax error — imprecision is
    // inexpressible, which is the point)
    let err = ove_script::run_script(&root.join("pf"), "let x = 0.5;").unwrap_err();
    assert!(
        err.0.contains("Syntax error"),
        "float literal must be a parse error under no_float: {err}"
    );

    // den == 0 is a loud typed error, never a panic or silent coercion
    let err = ove_script::run_script(&root.join("pz"), "let bad = r(1, 0);").unwrap_err();
    assert!(err.0.contains("non-zero denominator"), "{err}");
}
