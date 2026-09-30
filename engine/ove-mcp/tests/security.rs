//! WAVE 18 — MCP boundary security conformance (ADR-022): an MCP client
//! (or a compromised transport) can push arbitrarily large stdin lines.
//! The stdio server must answer oversized lines with a TYPED JSON-RPC
//! error and KEEP SERVING — never buffer unbounded input, never die.

use std::io::{BufRead, BufReader, Write};
use std::process::{Child, ChildStdin, ChildStdout, Command, Stdio};

struct Server {
    child: Child,
    stdin: ChildStdin,
    stdout: BufReader<ChildStdout>,
}

impl Drop for Server {
    fn drop(&mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}

impl Server {
    fn spawn() -> Server {
        let mut child = Command::new(env!("CARGO_BIN_EXE_ove-mcp"))
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .spawn()
            .expect("spawn ove-mcp");
        let stdin = child.stdin.take().expect("stdin");
        let stdout = BufReader::new(child.stdout.take().expect("stdout"));
        Server {
            child,
            stdin,
            stdout,
        }
    }

    fn read_line(&mut self) -> String {
        let mut buf = String::new();
        let n = self
            .stdout
            .read_line(&mut buf)
            .expect("server responds on stdout");
        assert!(n > 0, "server closed stdout (died on hostile input)");
        buf
    }
}

/// An oversized stdin line gets a typed JSON-RPC error response (NOT a
/// parse-error reply, NOT silence) and the server keeps serving the next
/// request — the hostile line must not kill or wedge the session.
#[test]
fn oversized_line_gets_typed_error_and_server_survives() {
    let mut s = Server::spawn();

    // 2 MiB junk line — far past the 1 MiB stdio line cap.
    let junk = "x".repeat(2 * 1024 * 1024);
    writeln!(s.stdin, "{junk}").expect("write hostile line");
    s.stdin.flush().expect("flush hostile line");
    let resp = s.read_line();
    assert!(
        resp.contains("line too large"),
        "typed cap response, got: {resp}"
    );
    let v: serde_json::Value =
        serde_json::from_str(resp.trim()).expect("cap response is valid JSON-RPC");
    assert_eq!(v["jsonrpc"], "2.0", "JSON-RPC envelope preserved");
    assert!(
        v["error"]["code"].is_i64(),
        "error carries a numeric code: {v}"
    );

    // The server must still be alive and serving.
    writeln!(
        s.stdin,
        r#"{{"jsonrpc":"2.0","id":1,"method":"initialize","params":{{}}}}"#
    )
    .expect("write follow-up request");
    s.stdin.flush().expect("flush follow-up");
    let resp = s.read_line();
    let v: serde_json::Value =
        serde_json::from_str(resp.trim()).expect("initialize response valid JSON");
    assert_eq!(v["id"], 1, "initialize answered after the hostile line");
    assert!(
        v.get("result").is_some(),
        "initialize succeeded post-attack: {v}"
    );
}

/// A line exactly within the cap is processed normally (the cap is a
/// resource bound, not a content filter).
#[test]
fn large_but_within_cap_line_is_processed() {
    // 64 KiB of spaces — valid JSON whitespace padding, under the cap.
    let padding = " ".repeat(64 * 1024);
    let mut s = Server::spawn();
    write!(s.stdin, "{padding}").expect("write padding");
    writeln!(
        s.stdin,
        r#"{{"jsonrpc":"2.0","id":7,"method":"initialize","params":{{}}}}"#
    )
    .expect("write request");
    s.stdin.flush().expect("flush");
    let resp = s.read_line();
    let v: serde_json::Value = serde_json::from_str(resp.trim()).expect("valid JSON");
    assert_eq!(v["id"], 7, "padded request processed normally");
    assert!(v.get("result").is_some(), "initialize succeeded: {v}");
}
