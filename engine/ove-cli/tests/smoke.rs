//! CLI smoke: drive the real binary end-to-end (project → import → clip →
//! status → export-copy). The binary is the engine's public face at W5.

use std::path::{Path, PathBuf};
use std::process::Command;

fn exe() -> &'static str {
    env!("CARGO_BIN_EXE_ove-cli")
}

fn media(name: &str) -> PathBuf {
    let p = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../ove-encode/tests/media")
        .join(name);
    assert!(p.exists(), "corpus fixture missing: {}", p.display());
    p
}

fn run(args: &[&str]) -> (String, String, bool) {
    let out = Command::new(exe())
        .args(args)
        .output()
        .expect("spawn ove-cli");
    (
        String::from_utf8_lossy(&out.stdout).to_string(),
        String::from_utf8_lossy(&out.stderr).to_string(),
        out.status.success(),
    )
}

#[test]
fn cli_full_flow_smoke() {
    let dir = std::env::temp_dir().join("ove-cli-smoke");
    let _ = std::fs::remove_dir_all(&dir);
    let dir_s = dir.to_str().unwrap();

    let (o, e, ok) = run(&["new", dir_s, "48000", "1"]);
    assert!(ok, "new failed: {o}{e}");

    let (o, e, ok) = run(&["add-track", dir_s, "1"]);
    assert!(ok, "add-track failed: {o}{e}");

    let (o, e, ok) = run(&["import", dir_s, media("copy24.mp4").to_str().unwrap()]);
    assert!(ok, "import failed: {o}{e}");
    let hash = o
        .trim()
        .split(" as ")
        .nth(1)
        .expect("hash in stdout")
        .to_string();

    let (o, e, ok) = run(&["add-clip", dir_s, "1", &hash, "48/24000", "0/1"]);
    assert!(ok, "add-clip failed: {o}{e}");

    let (o, e, ok) = run(&["split", dir_s, "1", "1", "24/24000"]);
    assert!(ok, "split failed: {o}{e}");

    // status: state hash present; deterministic across two opens
    let (o1, _, ok1) = run(&["status", dir_s]);
    let (o2, _, ok2) = run(&["status", dir_s]);
    assert!(ok1 && ok2);
    assert_eq!(o1, o2, "status deterministic");
    assert!(o1.contains("state_hash="));

    // export-copy [1s, 3s) → 48 frames, duration 2 s (keyframe-aligned)
    let out = dir.join("copy.mp4");
    let (o, e, ok) = run(&[
        "export-copy",
        dir_s,
        out.to_str().unwrap(),
        &hash,
        "1/1",
        "3/1",
    ]);
    assert!(ok, "export-copy failed: {o}{e}");
    assert!(out.exists());
}
