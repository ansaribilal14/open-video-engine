//! WAVE 15 — MCP boundary conformance: the AI-client edge.
//!
//! Drives the REAL stdio binary: initialize → tools/list → a full edit
//! cycle (create/track/clip/split/keyframes/undo/redo/status). Pins the
//! ME-7 float-rejection rule AT THE BOUNDARY and the determinism of the
//! tool-driven document state (two identical tool cycles → one hash).

use std::io::{BufRead, BufReader, Write};
use std::path::Path;
use std::process::{Child, ChildStdin, ChildStdout, Command, Stdio};

struct Server {
    child: Child,
    stdin: ChildStdin,
    stdout: BufReader<ChildStdout>,
    next_id: u64,
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
            next_id: 1,
        }
    }

    fn request(&mut self, method: &str, params: serde_json::Value) -> serde_json::Value {
        let id = self.next_id;
        self.next_id += 1;
        let msg =
            serde_json::json!({"jsonrpc": "2.0", "id": id, "method": method, "params": params});
        writeln!(self.stdin, "{msg}").expect("write request");
        self.stdin.flush().expect("flush request");
        let mut line = String::new();
        self.stdout.read_line(&mut line).expect("read response");
        serde_json::from_str(&line).expect("valid JSON-RPC response")
    }

    fn notify(&mut self, method: &str) {
        let msg = serde_json::json!({"jsonrpc": "2.0", "method": method});
        writeln!(self.stdin, "{msg}").expect("write notify");
        self.stdin.flush().expect("flush notify");
    }

    fn call_tool(&mut self, name: &str, args: serde_json::Value) -> (String, bool) {
        let resp = self.request(
            "tools/call",
            serde_json::json!({"name": name, "arguments": args}),
        );
        let result = resp.get("result").expect("result present");
        let is_error = result
            .get("isError")
            .and_then(|v| v.as_bool())
            .unwrap_or(false);
        let text = result["content"][0]["text"]
            .as_str()
            .unwrap_or_default()
            .to_string();
        (text, is_error)
    }
}

fn fixture(name: &str) -> String {
    let p = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../ove-encode/tests/media")
        .join(name);
    assert!(p.exists(), "corpus fixture missing: {}", p.display());
    p.display().to_string()
}

#[test]
fn mcp_full_edit_cycle_and_boundary_rules() {
    let mut s = Server::spawn();

    // initialize handshake
    let resp = s.request(
        "initialize",
        serde_json::json!({"protocolVersion": "2024-11-05", "capabilities": {}, "clientInfo": {"name": "test", "version": "0"}}),
    );
    assert_eq!(resp["result"]["protocolVersion"], "2024-11-05");
    assert_eq!(resp["result"]["serverInfo"]["name"], "ove-mcp");
    s.notify("notifications/initialized");

    // tools/list: the catalogue is non-empty and carries input schemas
    let resp = s.request("tools/list", serde_json::json!({}));
    let tools = resp["result"]["tools"].as_array().expect("tools array");
    assert!(tools.len() >= 10, "tool catalogue present");
    assert!(tools.iter().any(|t| t["name"] == "set_keyframes"));

    // full edit cycle
    let root = std::env::temp_dir().join("ove-mcp-test");
    let _ = std::fs::remove_dir_all(&root);
    std::fs::create_dir_all(&root).expect("root");
    let dir = root.join("proj");
    let d = dir.display().to_string();

    let (text, err) = s.call_tool(
        "create_project",
        serde_json::json!({"dir": d, "tick_num": 24000, "tick_den": 1}),
    );
    assert!(!err, "{text}");
    let (text, err) = s.call_tool("add_track", serde_json::json!({"dir": d, "track_id": 1}));
    assert!(!err, "{text}");
    let (imp, err) = s.call_tool(
        "import_media",
        serde_json::json!({"dir": d, "path": fixture("copy24.mp4")}),
    );
    assert!(!err, "{imp}");
    let hash = imp.split(" as ").nth(1).expect("hash").to_string();
    let (text, err) = s.call_tool("add_clip", serde_json::json!({"dir": d, "track_id": 1, "asset_hash": hash, "duration": "48/24000", "source_in": "0/1"}));
    assert!(!err, "{text}");
    let (text, err) = s.call_tool(
        "split",
        serde_json::json!({"dir": d, "track_id": 1, "clip_id": 1, "at": "24/24000"}),
    );
    assert!(!err, "{text}");
    let (text, err) = s.call_tool(
        "set_keyframes",
        serde_json::json!({
            "dir": d, "track_id": 1, "clip_id": 1, "property": "opacity",
            "keys": [
                {"time": "0/1", "value": "0/1", "interp": "linear"},
                {"time": "24/24000", "value": "1/1", "interp": "linear"}
            ]
        }),
    );
    assert!(!err, "{text}");
    let (h1, err) = s.call_tool("get_status", serde_json::json!({"dir": d}));
    assert!(!err, "{h1}");
    let (text, err) = s.call_tool("undo", serde_json::json!({"dir": d}));
    assert!(!err, "{text}");
    let (text, err) = s.call_tool("redo", serde_json::json!({"dir": d}));
    assert!(!err, "{text}");
    let (h2, err) = s.call_tool("get_status", serde_json::json!({"dir": d}));
    assert!(!err, "{h2}");
    assert_eq!(h1, h2, "undo→redo must return to the same document state");

    // ME-7 at the AI boundary: a JSON NUMBER in a rational field is rejected
    let (text, err) = s.call_tool("add_clip", serde_json::json!({"dir": d, "track_id": 1, "asset_hash": hash, "duration": 0.5, "source_in": "0/1"}));
    assert!(err, "float must be a tool error");
    assert!(text.contains("JSON number rejected"), "{text}");

    // unknown method → JSON-RPC error object
    let resp = s.request("bogus/method", serde_json::json!({}));
    assert_eq!(resp["error"]["code"], -32601);

    // parse error → code -32700 with null id
    writeln!(s.stdin, "not json at all").expect("write garbage");
    s.stdin.flush().expect("flush");
    let mut line = String::new();
    s.stdout
        .read_line(&mut line)
        .expect("read parse-error response");
    let resp: serde_json::Value = serde_json::from_str(&line).expect("json");
    assert_eq!(resp["error"]["code"], -32700);
    assert!(resp["id"].is_null());

    // protocol-level determinism: fresh projects replaying the same tool
    // sequence produce the SAME state hash (uuid excluded from get_status)
    let mut cycle = |dir: &str| -> String {
        let (t, e) = s.call_tool(
            "create_project",
            serde_json::json!({"dir": dir, "tick_num": 24000, "tick_den": 1}),
        );
        assert!(!e, "{t}");
        let (_, e) = s.call_tool("add_track", serde_json::json!({"dir": dir, "track_id": 1}));
        assert!(!e);
        let (h, e) = s.call_tool(
            "import_media",
            serde_json::json!({"dir": dir, "path": fixture("copy24.mp4")}),
        );
        assert!(!e, "{h}");
        let h = h.split(" as ").nth(1).unwrap().to_string();
        let (t, e) = s.call_tool("add_clip", serde_json::json!({"dir": dir, "track_id": 1, "asset_hash": h, "duration": "48/24000", "source_in": "0/1"}));
        assert!(!e, "{t}");
        let (t, e) = s.call_tool(
            "split",
            serde_json::json!({"dir": dir, "track_id": 1, "clip_id": 1, "at": "24/24000"}),
        );
        assert!(!e, "{t}");
        let (t, e) = s.call_tool("get_status", serde_json::json!({"dir": dir}));
        assert!(!e, "{t}");
        t
    };
    let a = cycle(root.join("p2").display().to_string().as_str());
    let b = cycle(root.join("p3").display().to_string().as_str());
    assert_eq!(a, b, "same tool sequence → same document state");
}
