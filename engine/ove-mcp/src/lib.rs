//! ove-mcp — MCP tools for the open-video-engine (wave 15, directive §1:
//! "AI agents are clients; edit semantics live ONLY in the engine core").
//!
//! SCOPE (honest): a minimal MCP 2024-11-05 stdio subset — `initialize`,
//! `tools/list`, `tools/call`, notifications ignored, JSON-RPC 2.0 errors.
//! No resources/prompts/sampling — the engine's command surface is the
//! product here, not a full MCP SDK.
//!
//! BOUNDARY DISCIPLINE (ME-7 at the AI edge):
//!   * every duration/position is a STRING "num/den" — a JSON number in a
//!     rational field is a TYPED error, never coerced (floats are the
//!     enemy of exact time);
//!   * the server is STATELESS across calls: every tool carries `dir`
//!     (the project folder); no hidden sessions, no server-side undo
//!     history beyond what the project log itself records;
//!   * tool errors surface as `isError: true` text content; protocol
//!     errors as JSON-RPC error objects — never a silent downgrade.
//!
//! The tool set mirrors the W14 batch grammar so AI clients and human
//! operators drive the SAME semantics through the SAME engine.

use ove_engine::Engine;
use ove_time::Rational;
use serde_json::{json, Value};

pub const PROTOCOL_VERSION: &str = "2024-11-05";
pub const SERVER_NAME: &str = "ove-mcp";
pub const SERVER_VERSION: &str = "0.1.0";

/// The tool catalogue (name, description, JSON-schema input).
pub fn tool_catalogue() -> Vec<Value> {
    let rat = || {
        json!({
            "type": "string",
            "pattern": "^-?[0-9]+/-?[1-9][0-9]*$",
            "description": "exact rational \"num/den\" — JSON numbers are REJECTED (ME-7)"
        })
    };
    let dir = || json!({"type": "string", "description": "project folder"});
    vec![
        tool(
            "create_project",
            "Create a project (fails if the folder exists)",
            json!({
                "type": "object", "required": ["dir", "tick_num", "tick_den"],
                "properties": {"dir": dir(), "tick_num": {"type": "integer"}, "tick_den": {"type": "integer"}}
            }),
        ),
        tool(
            "add_track",
            "Add a gap track",
            json!({
                "type": "object", "required": ["dir", "track_id"],
                "properties": {"dir": dir(), "track_id": {"type": "integer"}}
            }),
        ),
        tool(
            "import_media",
            "Import a media file (content-hash addressed)",
            json!({
                "type": "object", "required": ["dir", "path"],
                "properties": {"dir": dir(), "path": {"type": "string"}}
            }),
        ),
        tool(
            "add_clip",
            "Append a clip (explicit id allocation by the engine)",
            json!({
                "type": "object", "required": ["dir", "track_id", "asset_hash", "duration", "source_in"],
                "properties": {"dir": dir(), "track_id": {"type": "integer"}, "asset_hash": {"type": "string"}, "duration": rat(), "source_in": rat()}
            }),
        ),
        tool(
            "split",
            "Split a clip at an exact offset",
            json!({
                "type": "object", "required": ["dir", "track_id", "clip_id", "at"],
                "properties": {"dir": dir(), "track_id": {"type": "integer"}, "clip_id": {"type": "integer"}, "at": rat()}
            }),
        ),
        tool(
            "resize",
            "Resize a clip to an exact duration",
            json!({
                "type": "object", "required": ["dir", "track_id", "clip_id", "duration"],
                "properties": {"dir": dir(), "track_id": {"type": "integer"}, "clip_id": {"type": "integer"}, "duration": rat()}
            }),
        ),
        tool(
            "move_clip",
            "Move a clip (post-state index semantics)",
            json!({
                "type": "object", "required": ["dir", "clip_id", "from_track", "to_track", "to_index"],
                "properties": {"dir": dir(), "clip_id": {"type": "integer"}, "from_track": {"type": "integer"}, "to_track": {"type": "integer"}, "to_index": {"type": "integer"}}
            }),
        ),
        tool(
            "remove_clip",
            "Remove a clip (exact inverse on undo)",
            json!({
                "type": "object", "required": ["dir", "track_id", "clip_id"],
                "properties": {"dir": dir(), "track_id": {"type": "integer"}, "clip_id": {"type": "integer"}}
            }),
        ),
        tool(
            "set_keyframes",
            "Replace ONE property's keyframe list (W8)",
            json!({
                "type": "object", "required": ["dir", "track_id", "clip_id", "property", "keys"],
                "properties": {
                    "dir": dir(), "track_id": {"type": "integer"}, "clip_id": {"type": "integer"},
                    "property": {"type": "string", "enum": ["opacity", "x", "y"]},
                    "keys": {"type": "array", "items": {"type": "object",
                        "required": ["time", "value", "interp"],
                        "properties": {"time": rat(), "value": rat(), "interp": {"type": "string", "enum": ["linear", "hold"]}}}}
                }
            }),
        ),
        tool(
            "undo",
            "Undo the last command (exact inverse)",
            json!({
                "type": "object", "required": ["dir"], "properties": {"dir": dir()}
            }),
        ),
        tool(
            "redo",
            "Redo",
            json!({
                "type": "object", "required": ["dir"], "properties": {"dir": dir()}
            }),
        ),
        tool(
            "get_status",
            "Deterministic document status (state hash + shape)",
            json!({
                "type": "object", "required": ["dir"], "properties": {"dir": dir()}
            }),
        ),
    ]
}

fn tool(name: &str, description: &str, input_schema: Value) -> Value {
    json!({
        "name": name,
        "description": description,
        "inputSchema": input_schema,
    })
}

// ---------------------------------------------------------------------------
// parameter plumbing — the typed boundary
// ---------------------------------------------------------------------------

fn parse_rational_param(v: &Value, field: &str) -> Result<Rational, String> {
    match v {
        Value::String(s) => {
            let (n, d) = s
                .split_once('/')
                .ok_or_else(|| format!("{field}: expected \"num/den\", got {s:?}"))?;
            Ok(Rational::new(
                n.parse::<i64>().map_err(|e| format!("{field}.num: {e}"))?,
                d.parse::<i64>().map_err(|e| format!("{field}.den: {e}"))?,
            ))
        }
        // a JSON NUMBER in a rational field is REJECTED — never coerced
        Value::Number(_) => Err(format!(
            "{field}: JSON number rejected — pass an exact \"num/den\" string (ME-7)"
        )),
        other => Err(format!("{field}: expected \"num/den\" string, got {other}")),
    }
}

fn u64_param(args: &Value, field: &str) -> Result<u64, String> {
    args.get(field)
        .and_then(|v| v.as_u64())
        .ok_or_else(|| format!("{field}: missing or not an unsigned integer"))
}

fn str_param<'a>(args: &'a Value, field: &str) -> Result<&'a str, String> {
    args.get(field)
        .and_then(|v| v.as_str())
        .ok_or_else(|| format!("{field}: missing or not a string"))
}

fn open_engine(args: &Value) -> Result<Engine, String> {
    let dir = str_param(args, "dir")?;
    Engine::open(std::path::Path::new(dir)).map_err(|e| e.to_string())
}

// ---------------------------------------------------------------------------
// tool execution — same engine semantics as the CLI/batch shells
// ---------------------------------------------------------------------------

/// Execute one tool call; Ok(text) becomes content, Err(text) becomes
/// isError content. NEVER panics across the boundary.
pub fn call_tool(name: &str, args: &Value) -> Result<String, String> {
    match name {
        "create_project" => {
            let dir = str_param(args, "dir")?;
            let num = args
                .get("tick_num")
                .and_then(|v| v.as_i64())
                .ok_or("tick_num")?;
            let den = args
                .get("tick_den")
                .and_then(|v| v.as_i64())
                .ok_or("tick_den")?;
            Engine::create(std::path::Path::new(dir), (num, den)).map_err(|e| e.to_string())?;
            Ok(format!("created project at {dir} (tick axis {num}/{den})"))
        }
        "add_track" => {
            let mut e = open_engine(args)?;
            let tid = u64_param(args, "track_id")?;
            e.add_track(
                tid,
                ove_timeline::TrackKind::Gap(ove_timeline::GapTrack::new()),
            )
            .map_err(|er| er.to_string())?;
            Ok(format!("track {tid} added"))
        }
        "import_media" => {
            let mut e = open_engine(args)?;
            let path = str_param(args, "path")?;
            let hex = e
                .import_media(std::path::Path::new(path))
                .map_err(|er| er.to_string())?;
            Ok(format!("imported {path} as {hex}"))
        }
        "add_clip" => {
            let mut e = open_engine(args)?;
            let track = u64_param(args, "track_id")?;
            let hash = str_param(args, "asset_hash")?;
            let dur =
                parse_rational_param(args.get("duration").unwrap_or(&Value::Null), "duration")?;
            let src =
                parse_rational_param(args.get("source_in").unwrap_or(&Value::Null), "source_in")?;
            let clip = e
                .add_clip(track, hash, dur, src)
                .map_err(|er| er.to_string())?;
            Ok(format!("clip {clip} added to track {track}"))
        }
        "split" => {
            let mut e = open_engine(args)?;
            let new_id = e
                .split(
                    u64_param(args, "track_id")?,
                    u64_param(args, "clip_id")?,
                    parse_rational_param(args.get("at").unwrap_or(&Value::Null), "at")?,
                )
                .map_err(|er| er.to_string())?;
            Ok(format!("split -> new clip {new_id}"))
        }
        "resize" => {
            let mut e = open_engine(args)?;
            e.resize(
                u64_param(args, "track_id")?,
                u64_param(args, "clip_id")?,
                parse_rational_param(args.get("duration").unwrap_or(&Value::Null), "duration")?,
            )
            .map_err(|er| er.to_string())?;
            Ok("resized".into())
        }
        "move_clip" => {
            let mut e = open_engine(args)?;
            e.move_clip(
                u64_param(args, "clip_id")?,
                u64_param(args, "from_track")?,
                u64_param(args, "to_track")?,
                usize::try_from(u64_param(args, "to_index")?).map_err(|e| e.to_string())?,
            )
            .map_err(|er| er.to_string())?;
            Ok("moved".into())
        }
        "remove_clip" => {
            let mut e = open_engine(args)?;
            e.remove(u64_param(args, "track_id")?, u64_param(args, "clip_id")?)
                .map_err(|er| er.to_string())?;
            Ok("removed".into())
        }
        "set_keyframes" => {
            use ove_timeline::{Command, Interpolation, Keyframe, PropertyName};
            let mut e = open_engine(args)?;
            let prop = match str_param(args, "property")? {
                "opacity" => PropertyName::Opacity,
                "x" => PropertyName::X,
                "y" => PropertyName::Y,
                other => return Err(format!("property: unknown {other:?}")),
            };
            let keys_json = args
                .get("keys")
                .and_then(|v| v.as_array())
                .ok_or("keys: missing or not an array")?;
            let mut keys = Vec::new();
            for (i, k) in keys_json.iter().enumerate() {
                let t = parse_rational_param(
                    k.get("time").unwrap_or(&Value::Null),
                    &format!("keys[{i}].time"),
                )?;
                let v = parse_rational_param(
                    k.get("value").unwrap_or(&Value::Null),
                    &format!("keys[{i}].value"),
                )?;
                let interp = match k.get("interp").and_then(|x| x.as_str()) {
                    Some("linear") => Interpolation::Linear,
                    Some("hold") => Interpolation::Hold,
                    other => {
                        return Err(format!(
                            "keys[{i}].interp: expected linear|hold, got {other:?}"
                        ))
                    }
                };
                keys.push(Keyframe::new(t, v, interp));
            }
            e.execute(Command::SetKeyframes {
                track: u64_param(args, "track_id")?,
                id: u64_param(args, "clip_id")?,
                property: prop,
                keys,
            })
            .map_err(|er| er.to_string())?;
            Ok("keyframes set".into())
        }
        "undo" => {
            let mut e = open_engine(args)?;
            let did = e.undo().map_err(|er| er.to_string())?;
            Ok(if did { "undone" } else { "nothing to undo" }.into())
        }
        "redo" => {
            let mut e = open_engine(args)?;
            let did = e.redo().map_err(|er| er.to_string())?;
            Ok(if did { "redone" } else { "nothing to redo" }.into())
        }
        "get_status" => {
            let e = open_engine(args)?;
            let tracks = e
                .project()
                .timeline()
                .track_ids()
                .map(|t| t.to_string())
                .collect::<Vec<_>>()
                .join(",");
            let assets = e
                .project()
                .assets()
                .iter()
                .map(|a| a.id.clone())
                .collect::<Vec<_>>()
                .join(",");
            Ok(format!(
                "state_hash={} tracks=[{tracks}] assets=[{assets}]",
                e.state_hash()
            ))
        }
        other => Err(format!("unknown tool {other:?} (see tools/list)")),
    }
}

// ---------------------------------------------------------------------------
// JSON-RPC 2.0 / MCP dispatch — pure function (testable without stdio)
// ---------------------------------------------------------------------------

/// Handle one incoming line → Optional outgoing line (None for notifications).
pub fn handle_line(line: &str) -> Option<String> {
    let msg: Value = match serde_json::from_str(line) {
        Ok(v) => v,
        Err(e) => {
            return Some(
                json!({"jsonrpc": "2.0", "id": Value::Null, "error": {"code": -32700, "message": format!("parse error: {e}")}})
                    .to_string(),
            )
        }
    };
    let id = msg.get("id").cloned();
    // notification = no id → process but never respond
    let is_notification = id.is_none();
    let method = msg.get("method").and_then(|m| m.as_str()).unwrap_or("");
    let params = msg.get("params").cloned().unwrap_or(Value::Null);

    let result: Result<Value, (i64, String)> = match method {
        "initialize" => Ok(json!({
            "protocolVersion": PROTOCOL_VERSION,
            "capabilities": {"tools": {}},
            "serverInfo": {"name": SERVER_NAME, "version": SERVER_VERSION}
        })),
        "ping" => Ok(json!({})),
        "tools/list" => Ok(json!({ "tools": tool_catalogue() })),
        "tools/call" => {
            let name = params.get("name").and_then(|n| n.as_str()).unwrap_or("");
            let empty = Value::Object(serde_json::Map::new());
            let args = params.get("arguments").unwrap_or(&empty);
            match call_tool(name, args) {
                Ok(text) => {
                    Ok(json!({"content": [{"type": "text", "text": text}], "isError": false}))
                }
                Err(text) => {
                    Ok(json!({"content": [{"type": "text", "text": text}], "isError": true}))
                }
            }
        }
        "notifications/initialized" | _ if is_notification => Ok(Value::Null),
        other => Err((-32601, format!("method not found: {other}"))),
    };

    if is_notification {
        return None;
    }
    let id = id.unwrap();
    Some(match result {
        Ok(v) => json!({"jsonrpc": "2.0", "id": id, "result": v}).to_string(),
        Err((code, message)) => {
            json!({"jsonrpc": "2.0", "id": id, "error": {"code": code, "message": message}})
                .to_string()
        }
    })
}
