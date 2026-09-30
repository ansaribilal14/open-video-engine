//! WAVE 18 — project-file security conformance (ADR-022): a project
//! folder is UNTRUSTED input (projects are shared artifacts). Loading
//! must bound hostile file sizes (OOM) and reject registry paths that
//! escape the project folder — as TYPED errors, never silent acceptance.

use std::path::{Path, PathBuf};

use ove_project::{Project, ProjectError};

fn tmp(name: &str) -> PathBuf {
    let d = std::env::temp_dir().join("ove-project-security").join(name);
    let _ = std::fs::remove_dir_all(&d);
    std::fs::create_dir_all(&d).unwrap();
    d
}

/// A minimal schema-valid manifest body (schema_version 1) with an
/// attacker-controlled asset path.
fn manifest_with_asset_path(path: &str) -> String {
    format!(
        r#"{{
  "schema_version": 1,
  "format": "ove/project",
  "tick_axis": {{ "num": 24000, "den": 1 }},
  "created_by": {{ "engine": "ove", "version": "0.1.0" }},
  "state": {{ "snapshot_seq": 0, "log_len": 0, "state_hash": "" }},
  "assets": [
    {{
      "id": "a-1",
      "content_hash": "deadbeefdeadbeefdeadbeefdeadbeefdeadbeefdeadbeefdeadbeefdeadbeef",
      "path": "{path}"
    }}
  ],
  "tracks": [],
  "uuid": "00000000-0000-0000-0000-000000000000"
}}"#
    )
}

/// A minimal schema-valid manifest with an EMPTY asset registry.
fn manifest_no_assets() -> String {
    manifest_with_asset_path("").replace(
        r#""assets": [
    {
      "id": "a-1",
      "content_hash": "deadbeefdeadbeefdeadbeefdeadbeefdeadbeefdeadbeefdeadbeefdeadbeef",
      "path": ""
    }
  ]"#,
        r#""assets": []"#,
    )
}

fn write(dir: &Path, rel: &str, bytes: &[u8]) {
    let p = dir.join(rel);
    if let Some(parent) = p.parent() {
        std::fs::create_dir_all(parent).unwrap();
    }
    std::fs::write(p, bytes).unwrap();
}

/// An asset registry path that escapes the project folder (absolute or
/// traversal) must fail the open as a TYPED manifest error — the manifest
/// is a shared artifact and its paths are data, not instructions.
#[test]
fn hostile_asset_path_is_a_typed_manifest_error() {
    let hostiles = [
        "../../../../etc/passwd",
        "assets/ok/../../../../../etc/passwd",
        "/etc/passwd",
    ];
    for (i, hostile) in hostiles.iter().enumerate() {
        let dir = tmp(&format!("path-{i}"));
        write(
            &dir,
            "manifest.json",
            manifest_with_asset_path(hostile).as_bytes(),
        );
        write(&dir, "commands.jsonl", b"");
        let err = match Project::open(&dir) {
            Err(e) => e,
            Ok(_) => panic!("hostile path {hostile:?} must fail open"),
        };
        match &err {
            ProjectError::ManifestInvalid(msg) => assert!(
                msg.contains("asset path"),
                "typed asset-path message, got: {msg}"
            ),
            other => panic!("expected ManifestInvalid for {hostile:?}, got {other:?}"),
        }
    }
}

/// A single hostile log line far past the declared line cap must abort
/// the load typed — the log loader must not buffer unbounded lines.
#[test]
fn oversized_log_line_is_a_typed_corruption_error() {
    let dir = tmp("log-line");
    write(&dir, "manifest.json", manifest_no_assets().as_bytes());
    let hostile_line = format!(
        "{{\"seq\":1,\"junk\":\"{}\"}}",
        "x".repeat(16 * 1024 * 1024)
    );
    write(&dir, "commands.jsonl", hostile_line.as_bytes());
    let err = match Project::open(&dir) {
        Err(e) => e,
        Ok(_) => panic!("oversized log line must fail open"),
    };
    match &err {
        ProjectError::LogCorruption { reason, .. } => assert!(
            reason.contains("line too large"),
            "typed cap message, got: {reason}"
        ),
        other => panic!("expected LogCorruption, got {other:?}"),
    }
}

/// A manifest file far past the declared manifest cap fails typed BEFORE
/// parsing (never read wholesale into the parser).
#[test]
fn oversized_manifest_is_a_typed_manifest_error() {
    let dir = tmp("manifest-size");
    let giant = format!("{{\"pad\":\"{}\"}}", "x".repeat(20 * 1024 * 1024));
    write(&dir, "manifest.json", giant.as_bytes());
    write(&dir, "commands.jsonl", b"");
    let err = match Project::open(&dir) {
        Err(e) => e,
        Ok(_) => panic!("oversized manifest must fail open"),
    };
    match &err {
        ProjectError::ManifestInvalid(msg) => assert!(
            msg.contains("manifest too large"),
            "typed cap message, got: {msg}"
        ),
        other => panic!("expected ManifestInvalid, got {other:?}"),
    }
}

/// A snapshot state file far past the declared state cap fails typed
/// BEFORE the wholesale read + parse.
#[test]
fn oversized_snapshot_state_is_a_typed_snapshot_error() {
    let dir = tmp("state-size");
    let manifest = r#"
    {
      "schema_version": 1,
      "format": "ove/project",
      "tick_axis": { "num": 24000, "den": 1 },
      "created_by": { "engine": "ove", "version": "0.1.0" },
      "state": { "snapshot_seq": 1, "log_len": 1, "state_hash": "" },
      "assets": [],
      "tracks": [],
      "uuid": "00000000-0000-0000-0000-000000000000"
    }"#;
    write(&dir, "manifest.json", manifest.as_bytes());
    write(&dir, "commands.jsonl", b"");
    let meta = r#"
    {
      "snapshot_seq": 1,
      "state_hash": "deadbeef",
      "engine_version": "0.1.0"
    }"#;
    write(&dir, "snapshot/meta.json", meta.as_bytes());
    let giant = format!("{{\"pad\":\"{}\"}}", "x".repeat(65 * 1024 * 1024));
    write(&dir, "snapshot/state-1.json", giant.as_bytes());
    let err = match Project::open(&dir) {
        Err(e) => e,
        Ok(_) => panic!("oversized state must fail open"),
    };
    match &err {
        ProjectError::SnapshotInvalid(msg) => assert!(
            msg.contains("state file too large"),
            "typed cap message, got: {msg}"
        ),
        other => panic!("expected SnapshotInvalid, got {other:?}"),
    }
}
