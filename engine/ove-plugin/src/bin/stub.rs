//! ove-plugin-stub — the reference Tier-C process plugin (wave 17,
//! ADR-021). Deliberately boring: deterministic (no clock, no randomness,
//! no filesystem, no network), declares `propose.timeline`, derives ONE
//! content-neutral structural proposal from the host-provided context —
//! split the FIRST clip of the LOWEST track id at exactly half its
//! duration — logs its decision, and reports the host's verdict before
//! `done`. For any single-source v1 project the split is render-invariant:
//! the union of timeline windows is unchanged, so every rendered frame and
//! every audio sample is unchanged (pinned by the real-world gate).

use std::io::{BufRead, Write};

fn send(out: &mut impl Write, v: serde_json::Value) {
    writeln!(out, "{v}").expect("plugin stdout write");
}

fn main() {
    let stdout = std::io::stdout();
    let mut out = stdout.lock();

    send(
        &mut out,
        serde_json::json!({
            "type": "manifest",
            "name": "stub",
            "version": "0.1.0",
            "protocol": 1,
            "capabilities": ["propose.timeline"],
        }),
    );

    let stdin = std::io::stdin();
    for line in stdin.lock().lines() {
        let line = match line {
            Ok(l) => l,
            Err(_) => return,
        };
        let v: serde_json::Value = match serde_json::from_str(&line) {
            Ok(v) => v,
            Err(_) => return,
        };
        match v.get("type").and_then(|t| t.as_str()) {
            Some("hello") => {}
            Some("context") => {
                // Deterministic target: lowest track id, first clip.
                let mut tracks: Vec<(u64, &serde_json::Value)> = v
                    .get("tracks")
                    .and_then(|t| t.as_array())
                    .map(|a| {
                        a.iter()
                            .filter_map(|t| t.get("id").and_then(|i| i.as_u64()).map(|id| (id, t)))
                            .collect()
                    })
                    .unwrap_or_default();
                tracks.sort_by_key(|(id, _)| *id);
                let mut target = None;
                for (tid, t) in tracks {
                    if let Some(clip) = t
                        .get("clips")
                        .and_then(|c| c.as_array())
                        .and_then(|c| c.first())
                    {
                        let cid = clip.get("id").and_then(|i| i.as_u64());
                        let dur = clip.get("duration").and_then(|d| d.as_str());
                        if let (Some(cid), Some(dur)) = (cid, dur) {
                            target = Some((tid, cid, dur.to_string()));
                            break;
                        }
                    }
                }
                let Some((tid, cid, dur)) = target else {
                    send(
                        &mut out,
                        serde_json::json!({
                            "type": "log",
                            "line": "stub: no clip found in context — nothing to propose",
                        }),
                    );
                    send(
                        &mut out,
                        serde_json::json!({"type": "done", "summary": "no clip; no proposal"}),
                    );
                    return;
                };
                // Exact half by integer arithmetic: num / (den * 2).
                let (n, d) = match dur.split_once('/') {
                    Some((n, d)) => (n, d),
                    None => return,
                };
                let num: i64 = n.trim().parse().unwrap_or(0);
                let den: i64 = d.trim().parse().unwrap_or(1);
                if num <= 0 || den <= 0 {
                    send(
                        &mut out,
                        serde_json::json!({
                            "type": "log",
                            "line": format!("stub: clip {cid} duration {dur} is not splittable — no proposal"),
                        }),
                    );
                    send(
                        &mut out,
                        serde_json::json!({"type": "done", "summary": "duration not splittable; no proposal"}),
                    );
                    return;
                }
                let half = format!("{}/{}", num, den.saturating_mul(2));
                send(
                    &mut out,
                    serde_json::json!({
                        "type": "log",
                        "line": format!("stub: proposing split of clip {cid} on track {tid} at {half} (half of {dur})"),
                    }),
                );
                send(
                    &mut out,
                    serde_json::json!({
                        "type": "proposal",
                        "proposal": 1,
                        "verb": "split",
                        "track_id": tid,
                        "clip_id": cid,
                        "at": half,
                    }),
                );
            }
            Some("proposal_result") => {
                let applied = v.get("applied").and_then(|a| a.as_bool()).unwrap_or(false);
                let err = v.get("error").and_then(|e| e.as_str()).unwrap_or("");
                send(
                    &mut out,
                    serde_json::json!({
                        "type": "log",
                        "line": format!("stub: verdict applied={applied}{err}"),
                    }),
                );
                send(
                    &mut out,
                    serde_json::json!({
                        "type": "done",
                        "summary": format!("split proposal applied={applied}"),
                    }),
                );
                return;
            }
            _ => {}
        }
    }
}
