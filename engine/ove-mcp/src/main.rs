//! ove-mcp stdio entry: line-delimited JSON-RPC 2.0 over stdin/stdout
//! (MCP stdio transport). The dispatch itself is the pure function in
//! the library — this binary only wires IO.

use std::io::{self, BufRead, Write};

fn main() {
    let stdin = io::stdin();
    let stdout = io::stdout();
    let mut out = stdout.lock();
    for line in stdin.lock().lines() {
        let line = match line {
            Ok(l) => l,
            Err(_) => break,
        };
        if line.trim().is_empty() {
            continue;
        }
        if let Some(resp) = ove_mcp::handle_line(&line) {
            let _ = writeln!(out, "{resp}");
            let _ = out.flush();
        }
    }
}
