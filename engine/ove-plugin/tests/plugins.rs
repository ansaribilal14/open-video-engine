//! WAVE 17 — plugin-tier conformance: a process plugin drives the SAME
//! engine command surface as the CLI/batch/MCP/script clients. Pins:
//! proposal flow (apply → receipt → undoable), deterministic receipts,
//! typed protocol aborts, in-band payload rejections with session
//! continuation, and default-DENY capabilities (ADR-021).

use std::path::{Path, PathBuf};

use ove_engine::Engine;
use ove_plugin::{PluginError, PluginHost, PluginSession};
use ove_time::Rational;
use ove_timeline::{GapTrack, TrackKind};

fn fixture(name: &str) -> PathBuf {
    let p = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../ove-encode/tests/media")
        .join(name);
    assert!(p.exists(), "corpus fixture missing: {}", p.display());
    p
}

fn tmp(name: &str) -> PathBuf {
    let d = std::env::temp_dir().join("ove-plugin-tests").join(name);
    let _ = std::fs::remove_dir_all(&d);
    std::fs::create_dir_all(d.parent().unwrap()).unwrap();
    d
}

/// A fresh single-track project with one 2 s clip of real corpus media.
fn fresh_project(dir: &Path) -> Engine {
    let mut e = Engine::create(dir, (24_000, 1)).expect("create project");
    e.add_track(1, TrackKind::Gap(GapTrack::new()))
        .expect("track 1");
    let hex = e.import_media(&fixture("copy24.mp4")).expect("import");
    e.add_clip(1, &hex, Rational::new(2, 1), Rational::new(0, 1))
        .expect("clip");
    e
}

fn run_stub(e: &mut Engine) -> ove_plugin::SessionReport {
    let mut host =
        PluginHost::spawn(Path::new(env!("CARGO_BIN_EXE_ove-plugin-stub"))).expect("spawn stub");
    host.handshake(e).expect("manifest handshake");
    host.send_context(e).expect("context");
    host.run(e).expect("plugin session")
}

#[test]
fn plugin_edits_flow_through_the_command_bus() {
    let dir = tmp("bus");
    let mut e = fresh_project(&dir);
    let pre = e.state_hash();

    let report = run_stub(&mut e);

    assert_eq!(report.plugin, "stub", "manifest identity");
    assert_eq!(report.version, "0.1.0");
    assert_eq!(
        report.capabilities,
        vec!["propose.timeline".to_string()],
        "declared capabilities recorded"
    );
    assert_eq!(report.applied, 1, "the split proposal applied");
    assert_eq!(report.rejected, 0, "nothing rejected");
    assert_eq!(report.proposals.len(), 1);
    assert_eq!(report.proposals[0].verb, "split");
    assert!(report.proposals[0].applied, "receipt records the apply");
    assert_ne!(
        report.final_state_hash, pre,
        "plugin edit advanced the document state"
    );
    assert_eq!(
        report.final_state_hash,
        e.state_hash(),
        "report hash == engine hash at done"
    );

    // The plugin's edit is an ORDINARY command: undoable, hashed, replayed.
    // ADR-016/E-012: id-state IS document state — undoing an
    // allocation-consuming command (Split allocated the right-half id)
    // restores the structure but never the pre-command hash; redo identity
    // is exact.
    assert!(e.undo().expect("undo"), "plugin edit undoes");
    assert_eq!(
        e.project().timeline().track_len(1).expect("track 1"),
        1,
        "undo restores the pre-split clip structure"
    );
    assert_ne!(
        e.state_hash(),
        pre,
        "id-state is document state: undo after Split keeps the advanced allocator"
    );
    assert!(e.redo().expect("redo"), "plugin edit redoes");
    assert_eq!(
        e.state_hash(),
        report.final_state_hash,
        "redo returns to the post-plugin hash"
    );
}

#[test]
fn plugin_receipts_are_deterministic() {
    let mut receipts = Vec::new();
    for i in 0..2 {
        let dir = tmp(&format!("det-{i}"));
        let mut e = fresh_project(&dir);
        let report = run_stub(&mut e);
        receipts.push(report.to_receipt_json());
    }
    assert_eq!(
        receipts[0], receipts[1],
        "same project + same plugin behavior → byte-identical receipt (no timestamps)"
    );
}

#[test]
fn protocol_violations_are_typed_and_aborting() {
    // --- before any manifest: every message type is an abort ------------
    let dir = tmp("abuse-pre");
    let mut e = fresh_project(&dir);

    let mut s = PluginSession::new();
    let err = s
        .ingest(
            r#"{"type":"proposal","proposal":1,"verb":"add_track","track_id":9}"#,
            &mut e,
        )
        .unwrap_err();
    assert!(
        matches!(err, PluginError::Protocol(ref m) if m.contains("before manifest")),
        "proposal before manifest aborts: {err:?}"
    );

    let mut s = PluginSession::new();
    let err = s
        .ingest(r#"{"type":"log","line":"hi"}"#, &mut e)
        .unwrap_err();
    assert!(
        matches!(err, PluginError::Protocol(_)),
        "log before manifest aborts"
    );

    let mut s = PluginSession::new();
    let err = s.ingest("not json at all", &mut e).unwrap_err();
    assert!(
        matches!(err, PluginError::Protocol(ref m) if m.contains("non-JSON")),
        "non-JSON framing aborts: {err:?}"
    );

    let mut s = PluginSession::new();
    let err = s.ingest(r#"{"verb":"split"}"#, &mut e).unwrap_err();
    assert!(
        matches!(err, PluginError::Protocol(ref m) if m.contains("without \"type\"")),
        "message without type aborts: {err:?}"
    );

    // --- manifest-phase violations ---------------------------------------
    let dir = tmp("abuse-manifest");
    let mut e = fresh_project(&dir);

    let mut s = PluginSession::new();
    let err = s
        .ingest(
            r#"{"type":"manifest","name":"x","version":"0","protocol":2,"capabilities":[]}"#,
            &mut e,
        )
        .unwrap_err();
    assert!(
        matches!(err, PluginError::Protocol(ref m) if m.contains("protocol 2")),
        "wrong protocol version aborts: {err:?}"
    );

    let mut s = PluginSession::new();
    let err = s
        .ingest(
            r#"{"type":"manifest","name":"x","version":"0","protocol":1,"capabilities":["propose.frames"]}"#,
            &mut e,
        )
        .unwrap_err();
    assert!(
        matches!(err, PluginError::Protocol(ref m) if m.contains("unknown capability")),
        "unknown capability name aborts: {err:?}"
    );

    let mut s = PluginSession::new();
    let err = s
        .ingest(
            r#"{"type":"manifest","name":"x","version":"0","protocol":1}"#,
            &mut e,
        )
        .unwrap_err();
    assert!(
        matches!(err, PluginError::Protocol(ref m) if m.contains("capabilities")),
        "capabilities not an array aborts: {err:?}"
    );

    // --- running-phase violations ----------------------------------------
    let dir = tmp("abuse-running");
    let mut e = fresh_project(&dir);

    let mut s = PluginSession::new();
    s.ingest(
        r#"{"type":"manifest","name":"x","version":"0","protocol":1,"capabilities":["propose.timeline"]}"#,
        &mut e,
    )
    .expect("valid manifest");

    let err = s
        .ingest(
            r#"{"type":"manifest","name":"y","version":"0","protocol":1,"capabilities":[]}"#,
            &mut e,
        )
        .unwrap_err();
    assert!(
        matches!(err, PluginError::Protocol(ref m) if m.contains("exactly once")),
        "duplicate manifest aborts: {err:?}"
    );

    let err = s
        .ingest(r#"{"type":"render","pixels":[1,2,3]}"#, &mut e)
        .unwrap_err();
    assert!(
        matches!(err, PluginError::Protocol(ref m) if m.contains("unknown message type")),
        "unknown message type aborts — plugins never speak frames: {err:?}"
    );
}

#[test]
fn payload_rejections_reject_in_band_and_continue() {
    let dir = tmp("abuse-payload");
    let mut e = fresh_project(&dir);
    let pre = e.state_hash();

    let mut s = PluginSession::new();
    s.ingest(
        r#"{"type":"manifest","name":"x","version":"0","protocol":1,"capabilities":["propose.timeline"]}"#,
        &mut e,
    )
    .expect("valid manifest");

    // 1. undeclared-capability default-DENY is covered by the empty-capability
    //    manifest below; here: a JSON NUMBER in a rational field.
    let step = s
        .ingest(
            r#"{"type":"proposal","proposal":1,"verb":"split","track_id":1,"clip_id":1,"at":0.5}"#,
            &mut e,
        )
        .expect("payload problems never abort");
    match step {
        ove_plugin::Step::Reply(line) => {
            let v: serde_json::Value = serde_json::from_str(&line).unwrap();
            assert_eq!(v["applied"], false, "float `at` rejected");
            assert!(v["error"].is_string(), "rejection carries a reason");
        }
        other => panic!("expected a Reply step, got {other:?}"),
    }
    assert_eq!(e.state_hash(), pre, "rejected proposal mutated nothing");

    // 2. a NON-"num/den" string in a rational field.
    let step = s
        .ingest(
            r#"{"type":"proposal","proposal":2,"verb":"split","track_id":1,"clip_id":1,"at":"0.5"}"#,
            &mut e,
        )
        .expect("still not an abort");
    match step {
        ove_plugin::Step::Reply(line) => {
            let v: serde_json::Value = serde_json::from_str(&line).unwrap();
            assert_eq!(v["applied"], false);
            let err = v["error"].as_str().unwrap();
            assert!(
                err.contains("num/den"),
                "ME-7 rejection names the exact shape: {err}"
            );
        }
        other => panic!("expected a Reply step, got {other:?}"),
    }

    // 3. unknown verb → serde unknown-variant rejection, session continues.
    let step = s
        .ingest(
            r#"{"type":"proposal","proposal":3,"verb":"reformat_drive","track_id":1}"#,
            &mut e,
        )
        .expect("unknown verb is in-band");
    match step {
        ove_plugin::Step::Reply(line) => {
            let v: serde_json::Value = serde_json::from_str(&line).unwrap();
            assert_eq!(v["applied"], false, "unknown verb rejected");
        }
        other => panic!("expected a Reply step, got {other:?}"),
    }

    // 4. engine-level rejection (nonexistent clip) is in-band too.
    let step = s
        .ingest(
            r#"{"type":"proposal","proposal":4,"verb":"split","track_id":1,"clip_id":999,"at":"1/1"}"#,
            &mut e,
        )
        .expect("engine rejection is in-band");
    match step {
        ove_plugin::Step::Reply(line) => {
            let v: serde_json::Value = serde_json::from_str(&line).unwrap();
            assert_eq!(v["applied"], false);
        }
        other => panic!("expected a Reply step, got {other:?}"),
    }

    // 5. a VALID proposal after all that noise still applies.
    let step = s
        .ingest(
            r#"{"type":"proposal","proposal":5,"verb":"split","track_id":1,"clip_id":1,"at":"1/1"}"#,
            &mut e,
        )
        .expect("session continues");
    match step {
        ove_plugin::Step::Reply(line) => {
            let v: serde_json::Value = serde_json::from_str(&line).unwrap();
            assert_eq!(
                v["applied"], true,
                "valid proposal applies after rejections"
            );
            assert_ne!(e.state_hash(), pre, "document advanced");
        }
        other => panic!("expected a Reply step, got {other:?}"),
    }

    let report = s.report(e.state_hash());
    assert_eq!(report.applied, 1);
    assert_eq!(report.rejected, 4, "all four rejections receipted");
}

#[test]
fn default_deny_when_capability_not_declared() {
    let dir = tmp("deny");
    let mut e = fresh_project(&dir);
    let pre = e.state_hash();

    let mut s = PluginSession::new();
    s.ingest(
        r#"{"type":"manifest","name":"x","version":"0","protocol":1,"capabilities":[]}"#,
        &mut e,
    )
    .expect("manifest with no capabilities is valid");

    let step = s
        .ingest(
            r#"{"type":"proposal","proposal":1,"verb":"split","track_id":1,"clip_id":1,"at":"1/1"}"#,
            &mut e,
        )
        .expect("deny is in-band, not an abort");
    match step {
        ove_plugin::Step::Reply(line) => {
            let v: serde_json::Value = serde_json::from_str(&line).unwrap();
            assert_eq!(v["applied"], false, "default DENY");
            let err = v["error"].as_str().unwrap();
            assert!(
                err.contains("default DENY"),
                "rejection names the deny policy: {err}"
            );
        }
        other => panic!("expected a Reply step, got {other:?}"),
    }
    assert_eq!(e.state_hash(), pre, "denied proposal mutated nothing");
}
