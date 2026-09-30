//! ove-mcp stdio entry: line-delimited JSON-RPC 2.0 over stdin/stdout
//! (MCP stdio transport). The dispatch itself is the pure function in
//! the library — this binary only wires IO.
//!
//! ADR-022: stdin lines are bounded AT READ TIME through a `take` window
//! — a hostile oversized line costs the host at most cap+2 bytes of
//! buffering, gets exactly ONE typed JSON-RPC error response, its tail is
//! discarded, and the server keeps serving the next line.

use std::io::{self, BufRead, Read, Write};

fn main() {
    let stdin = io::stdin();
    let stdout = io::stdout();
    let mut out = stdout.lock();
    let mut reader = stdin.lock();
    // true while the tail of an oversized line is being discarded
    let mut discarding = false;
    loop {
        let mut buf = Vec::new();
        let n = match reader
            .by_ref()
            .take((ove_mcp::LINE_MAX_BYTES + 2) as u64)
            .read_until(b'\n', &mut buf)
        {
            Ok(n) => n,
            Err(_) => break,
        };
        if n == 0 {
            break; // EOF
        }
        let ends_line = buf.last() == Some(&b'\n');
        if discarding {
            if ends_line {
                discarding = false;
            }
            continue;
        }
        if n > ove_mcp::LINE_MAX_BYTES + 1 {
            let _ = writeln!(out, "{}", ove_mcp::oversized_line_response(n));
            let _ = out.flush();
            if !ends_line {
                discarding = true;
            }
            continue;
        }
        let line = String::from_utf8_lossy(&buf);
        let line = line.trim_end_matches(['\n', '\r']);
        if line.trim().is_empty() {
            continue;
        }
        if let Some(resp) = ove_mcp::handle_line(line) {
            let _ = writeln!(out, "{resp}");
            let _ = out.flush();
        }
    }
}
