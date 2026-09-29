//! WAVE 14 — headless batch mode: the CLI as a scripted, deterministic
//! operator. ONE engine session executes the script; output lines carry
//! state hashes; the same script + same inputs → byte-identical stdout
//! (the determinism contract), and a bad verb fails fast with a typed
//! line number (fail-fast contract).

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

fn write_script(dir: &Path, media_path: &Path, run: usize) -> PathBuf {
    let script = dir.join(format!("batch{run}.ove"));
    std::fs::write(
        &script,
        format!(
            "# deterministic headless batch (wave 14)\n\
             new 24000 1\n\
             add-track 1\n\
             import {}\n\
             add-clip 1 HASH_PLACEHOLDER 48/24000 0/1\n\
             split 1 1 24/24000\n\
             resize 1 1 36/24000\n\
             undo\n\
             redo\n\
             status\n\
             export-copy out.mp4 HASH_PLACEHOLDER 0/1 1/1\n",
            media_path.display()
        ),
    )
    .expect("write script");
    script
}

fn run_batch(dir: &Path, script: &Path) -> (String, String, bool) {
    let out = Command::new(exe())
        .args(["batch", dir.to_str().unwrap(), script.to_str().unwrap()])
        .output()
        .expect("spawn ove-cli batch");
    (
        String::from_utf8_lossy(&out.stdout).to_string(),
        String::from_utf8_lossy(&out.stderr).to_string(),
        out.status.success(),
    )
}

#[test]
fn batch_flow_is_deterministic_and_fail_fast() {
    let media_path = media("copy24.mp4");
    let root = std::env::temp_dir().join("ove-cli-batch");
    let _ = std::fs::remove_dir_all(&root);
    std::fs::create_dir_all(&root).expect("root");

    // the script embeds the content hash — pre-compute it via a first
    // import-only run, then run the full script twice for the parity check
    let hash = {
        let d1 = root.join("probe");
        let _ = std::fs::remove_dir_all(&d1);
        let (_o, e, ok) = {
            let out = Command::new(exe())
                .args(["new", d1.to_str().unwrap(), "24000", "1"])
                .output()
                .unwrap();
            (
                String::new(),
                String::from_utf8_lossy(&out.stderr).to_string(),
                out.status.success(),
            )
        };
        assert!(ok, "probe new failed: {e}");
        let out = Command::new(exe())
            .args(["import", d1.to_str().unwrap(), media_path.to_str().unwrap()])
            .output()
            .unwrap();
        assert!(out.status.success());
        String::from_utf8_lossy(&out.stdout)
            .trim()
            .split(" as ")
            .nth(1)
            .expect("hash in stdout")
            .to_string()
    };

    let mut outs = Vec::new();
    for run in 0..2 {
        // the script lives OUTSIDE the project dir — `new` requires absence
        let dir = root.join(format!("proj{run}"));
        let script = write_script(&root, &media_path, run);
        let text = std::fs::read_to_string(&script)
            .unwrap()
            .replace("HASH_PLACEHOLDER", &hash);
        std::fs::write(&script, text).expect("script with hash");
        let (o, e, ok) = run_batch(&dir, &script);
        assert!(ok, "batch run{run} failed: {o}{e}");
        for verb in [
            "OK 2 new",
            "OK 3 add-track",
            "OK 4 import",
            "OK 5 add-clip",
            "OK 6 split",
            "OK 7 resize",
            "OK 8 undo",
            "OK 9 redo",
            "OK 10 status state_hash=",
            "OK 11 export-copy",
        ] {
            assert!(o.contains(verb), "run{run} output missing {verb:?}:\n{o}");
        }
        assert!(dir.join("out.mp4").exists(), "exported file must exist");
        outs.push(o);
    }

    // fail-fast: an unknown verb aborts at its line with the typed marker
    let dir = root.join("projfail");
    let script = root.join("bad.ove");
    std::fs::write(&script, "new 24000 1\nadd-track 1\nbogus-verb x\nstatus\n")
        .expect("write bad script");
    let (o, e, ok) = run_batch(&dir, &script);
    assert!(!ok, "bad script must fail");
    assert!(e.contains("ERR 3 bogus-verb"), "typed fail-fast line: {e}");
    assert!(
        !o.contains("OK 4 status"),
        "must abort at line 3, not continue"
    );

    // determinism: the two full runs must have produced byte-identical output
    assert_eq!(
        outs[0], outs[1],
        "same script + same inputs must produce byte-identical output"
    );
}
