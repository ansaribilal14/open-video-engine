//! WAVE 18 — plugin-boundary security conformance (ADR-022): a hostile
//! plugin is an UNTRUSTED peer (its manifest identity is self-claimed).
//! The host must survive resource attacks as TYPED aborts — never OOM,
//! never a hang. Pins the two host budgets (session line cap, proposal
//! budget) at both the pure state-machine level and the real wire level,
//! plus the host launch rule (direct exec, arguments passed verbatim,
//! never through a shell).

use std::path::{Path, PathBuf};

use ove_engine::Engine;
use ove_plugin::{PluginError, PluginHost, PluginSession};
use ove_timeline::{GapTrack, TrackKind};

fn tmp(name: &str) -> PathBuf {
    let d = std::env::temp_dir().join("ove-plugin-security").join(name);
    let _ = std::fs::remove_dir_all(&d);
    std::fs::create_dir_all(d.parent().unwrap()).unwrap();
    d
}

fn fresh_engine(dir: &Path) -> Engine {
    let mut e = Engine::create(dir, (24_000, 1)).expect("create project");
    e.add_track(1, TrackKind::Gap(GapTrack::new()))
        .expect("track 1");
    e
}

fn manifest_line() -> String {
    r#"{"type":"manifest","protocol":1,"name":"flood","version":"0.1.0","capabilities":[]}"#
        .to_string()
}

fn proposal_line(i: u64) -> String {
    format!(r#"{{"type":"proposal","proposal":{i},"verb":"add_track"}}"#)
}

/// A single plugin stdout line far past the session line cap is a TYPED
/// protocol abort — the hostile line must never reach the session state.
#[test]
fn oversized_line_is_a_typed_protocol_abort() {
    let dir = tmp("line-pure");
    let mut e = fresh_engine(&dir);
    let hash_before = e.state_hash();

    let mut s = PluginSession::new();
    let oversized = "x".repeat(2 * 1024 * 1024); // 2 MiB > 1 MiB cap
    let err = s
        .ingest(&oversized, &mut e)
        .expect_err("oversized line must abort typed");
    match &err {
        PluginError::Protocol(msg) => {
            assert!(
                msg.contains("line too large"),
                "typed cap message, got: {msg}"
            );
        }
        other => panic!("expected Protocol, got {other:?}"),
    }
    assert_eq!(e.state_hash(), hash_before, "engine state untouched");
}

/// A proposal flood past the session budget is a TYPED protocol abort.
/// Default-DENY receipts accumulate per proposal — the budget bounds the
/// receipt memory and the mutation-flood rate. The engine stays untouched
/// by a denied flood.
#[test]
fn proposal_flood_is_a_typed_protocol_abort() {
    let dir = tmp("flood-pure");
    let mut e = fresh_engine(&dir);
    let hash_before = e.state_hash();

    let mut s = PluginSession::new();
    s.ingest(&manifest_line(), &mut e)
        .expect("valid manifest accepted");

    let mut aborted = false;
    for i in 0..11_000u64 {
        match s.ingest(&proposal_line(i), &mut e) {
            Ok(_) => {}
            Err(PluginError::Protocol(msg)) => {
                assert!(msg.contains("proposal budget"), "cap message, got: {msg}");
                aborted = true;
                break;
            }
            Err(other) => panic!("unexpected error at {i}: {other:?}"),
        }
    }
    assert!(
        aborted,
        "the flood must be aborted typed before 11 000 receipts"
    );
    assert_eq!(e.state_hash(), hash_before, "default-DENY flood is inert");
}

/// WIRE level: a hostile process emitting ONE oversized line before any
/// manifest is aborted typed at READ time — the host never buffers the
/// hostile line fully into session memory before the cap applies.
#[test]
fn wire_oversized_line_aborts_at_read_time() {
    let dir = tmp("line-wire");
    let mut e = fresh_engine(&dir);

    let flood = Path::new(env!("CARGO_BIN_EXE_ove-plugin-flood"));
    let mut host = PluginHost::spawn_with_args(flood, &["line"]).expect("spawn hostile plugin");
    let err = host
        .handshake(&mut e)
        .expect_err("oversized first line must abort the session");
    match &err {
        PluginError::Protocol(msg) => assert!(
            msg.contains("line too large"),
            "typed wire cap message, got: {msg}"
        ),
        other => panic!("expected Protocol, got {other:?}"),
    }
}

/// WIRE level: a VALID manifest handshake followed by a proposal flood
/// past the budget aborts the session typed inside run() — never a
/// report, never a hang.
#[test]
fn wire_proposal_flood_aborts_typed() {
    let dir = tmp("flood-wire");
    let mut e = fresh_engine(&dir);
    let hash_before = e.state_hash();

    let flood = Path::new(env!("CARGO_BIN_EXE_ove-plugin-flood"));
    let mut host = PluginHost::spawn_with_args(flood, &["props"]).expect("spawn hostile plugin");
    host.handshake(&mut e)
        .expect("the flood's manifest itself is valid");
    let err = host.run(&mut e).expect_err("flood must abort the session");
    match &err {
        PluginError::Protocol(msg) => assert!(
            msg.contains("proposal budget"),
            "typed wire cap message, got: {msg}"
        ),
        other => panic!("expected Protocol, got {other:?}"),
    }
    assert_eq!(e.state_hash(), hash_before, "denied flood is inert");
}
