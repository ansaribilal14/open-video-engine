//! WAVE 18 — scripting-edge security conformance (ADR-022): a script is
//! UNTRUSTED input. Rhai's default engine leaves string/array/map sizes
//! effectively unbounded, so a memory-bomb script can abort the host
//! process (allocator OOM). The engine must run with DECLARED resource
//! budgets and surface them as typed script errors. The `emit` output
//! buffer (host memory) is budgeted the same way.

use ove_script::{run_script, ScriptError};

fn tmp(name: &str) -> std::path::PathBuf {
    let d = std::env::temp_dir().join("ove-script-security").join(name);
    let _ = std::fs::remove_dir_all(&d);
    std::fs::create_dir_all(&d).unwrap();
    d
}

/// A string-doubling memory bomb must fail as a TYPED script error once
/// it passes the declared string budget — never an allocator abort, never
/// a hang. (Doubling is cheap to 32 MiB, so the pre-fix engine would
/// happily complete this script; the budget makes it a typed failure.)
#[test]
fn string_memory_bomb_is_a_typed_error() {
    let dir = tmp("bomb");
    let src = r#"
        let s = "x";
        let i = 0;
        while i < 40 {
            s = s + s;
            i += 1;
        }
        ove.emit(s);
    "#;
    let err: ScriptError = run_script(&dir, src).expect_err("memory bomb must fail typed");
    let msg = format!("{err}");
    assert!(
        msg.contains("too big") || msg.contains("too large") || msg.contains("limit"),
        "resource-limit wording, got: {msg}"
    );
}

/// Emit output within the declared budget is fine (the cap is a host
/// memory bound, not a script censorship rule).
#[test]
fn emit_within_budget_is_accepted() {
    let dir = tmp("emit-ok");
    let src = r#"
        for i in 0..10000 {
            ove.emit("0123456789012345678901234567890123456789");
        }
    "#; // 10 000 × 40 B = 400 KiB < 1 MiB budget
    let out = run_script(&dir, src).expect("within-budget emit must succeed");
    assert_eq!(out.lines().count(), 10_000, "every emit collected");
}

/// An emit flood past the output budget fails typed — the host must not
/// buffer unbounded script output.
#[test]
fn emit_flood_is_a_typed_error() {
    let dir = tmp("emit-flood");
    let src = r#"
        for i in 0..60000 {
            ove.emit("0123456789012345678901234567890123456789");
        }
    "#; // 60 000 × 40 B = 2.4 MiB > 1 MiB budget
    let err: ScriptError = run_script(&dir, src).expect_err("emit flood must fail typed");
    let msg = format!("{err}");
    assert!(
        msg.contains("emit budget") || msg.contains("budget") || msg.contains("limit"),
        "budget wording, got: {msg}"
    );
}

/// Determinism pin: the same within-budget script run is reproducible
/// (security hardening must not break the E-003 discipline).
#[test]
fn scripted_run_is_still_deterministic() {
    let dir = tmp("determinism");
    let src = r#"
        let a = 3 * 4 + 11;
        let b = 0;
        for i in 0..100 { b += i; }
        ove.emit("sum-check " + a + " " + b);
    "#;
    let a = run_script(&dir, src).expect("run a");
    let b = run_script(&dir, src).expect("run b");
    assert_eq!(a, b, "same script → same output");
    assert!(a.contains("sum-check 23 4950"), "exact integer output: {a}");
}
