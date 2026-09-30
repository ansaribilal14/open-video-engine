//! WAVE 18 hostile plugin stub (security conformance — ADR-022): models
//! the two resource attacks the plugin host must survive typed:
//!
//!   flood line    — one stdout line far beyond the host line cap
//!   flood props   — valid manifest, then a proposal flood past the budget
//!
//! Reply-draining is balanced with writes so the host can never deadlock
//! on a full stdin pipe mid-attack.

use std::io::{BufRead, Write};
use std::process::exit;

fn main() {
    let mode = std::env::args().nth(1).unwrap_or_default();
    let stdin = std::io::stdin();
    let mut rin = stdin.lock();
    let stdout = std::io::stdout();
    let mut out = stdout.lock();

    match mode.as_str() {
        "line" => {
            // 2 MiB single line before any manifest — the host must abort
            // typed at READ time (never buffers it fully into the session).
            let junk = "x".repeat(2 * 1024 * 1024);
            let _ = writeln!(out, "{junk}");
            let _ = out.flush();
            // give the host a moment, then exit
            std::thread::sleep(std::time::Duration::from_millis(200));
            exit(0);
        }
        "props" => {
            // valid manifest (no capabilities — every proposal default-DENY
            // receipt), then 11 000 proposals: past the host's budget.
            let _ = writeln!(
                out,
                r#"{{"type":"manifest","protocol":1,"name":"flood","version":"0.1.0","capabilities":[]}}"#
            );
            let _ = out.flush();
            let _ = rin.read_line(&mut String::new()); // hello
            for i in 0..11_000u64 {
                let _ = writeln!(
                    out,
                    r#"{{"type":"proposal","proposal":{i},"verb":"add_track"}}"#
                );
                let _ = out.flush();
                let mut reply = String::new();
                if rin.read_line(&mut reply).unwrap_or(0) == 0 {
                    // host stopped replying (budget abort) — exit cleanly
                    exit(0);
                }
            }
            let _ = writeln!(out, r#"{{"type":"done","summary":"flood"}}"#);
            let _ = out.flush();
            exit(0);
        }
        _ => {
            eprintln!("usage: ove-plugin-flood line|props");
            exit(2);
        }
    }
}
