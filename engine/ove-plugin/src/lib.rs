//! ove-plugin — process-tier plugin client (wave 17, ADR-021).
//!
//! DIRECTIVE: "the plugin-tier research becomes a typed plugin client at
//! the same command-bus boundary … no new semantics outside the command
//! grammar." This crate is exactly that boundary for out-of-process
//! plugins (28_PLUGINS §7 Tier C, §8 "findings enter the project as
//! proposed commands through the normal ladder"):
//!
//! - plugins NEVER touch frames; frames cross only into effect tiers
//!   (Tier A/B — future waves; 28_PLUGINS §8 keeps them orthogonal, and
//!   effects NEVER emit commands)
//! - a plugin's ONLY write path is a PROPOSAL — a command in the same
//!   typed grammar the CLI/batch/MCP/script clients use, applied via the
//!   SAME engine command bus, so every plugin edit is validated,
//!   journaled, undoable, and state-hashed like every other edit
//! - the plugin edge enforces ME-7 (exact rationals only): a rational
//!   arrives as an exact `"num/den"` string; anything else is rejected
//!   in-band and the session continues
//! - capability declarations are default-DENY (28_PLUGINS §9): a plugin
//!   may propose only what its manifest declared, and only capability
//!   names the host knows are accepted at all
//! - every proposal is receipted (id, verb, applied/error, state hash
//!   after) so evidence can attribute every mutation to a plugin
//!   name + version (28_PLUGINS §10); receipts are deterministic (no
//!   timestamps — same project + same plugin behavior → identical bytes)
//!
//! PROTOCOL v1 (line-delimited JSON over the plugin process's stdio):
//!
//!   host → plugin:
//!     {"type":"hello","protocol":1,"engine":"ove"}
//!     {"type":"context", …}             read-only document view
//!     {"type":"proposal_result", …}     per-proposal verdict
//!   plugin → host:
//!     {"type":"manifest", …}            REQUIRED first message
//!     {"type":"proposal", …}            a command proposal
//!     {"type":"log","line":"…"}         relayed to host stderr
//!     {"type":"done","summary":"…"}     final message
//!
//! REJECTION TAXONOMY (mirrors the MCP split — protocol errors abort,
//! tool errors are isError content and the session continues):
//! protocol-level violations ABORT the session as typed
//! [`PluginError::Protocol`] (non-JSON line, message without `type`,
//! unknown message type, manifest ordering/duplicates, protocol version
//! mismatch, unknown capability name, EOF before `done`); payload-level
//! problems (bad rational shape, unknown verb/property/interp, undeclared
//! capability, engine-level rejection) REJECT the individual proposal
//! in-band (`proposal_result` with `applied:false`) and the session
//! continues so the plugin can adapt.

use std::io::{BufRead, BufReader, Write};
use std::path::Path;
use std::process::{Child, ChildStdout, Command as Proc, Stdio};

use ove_engine::Engine;
use ove_time::Rational;
use ove_timeline::{Command, GapTrack, Interpolation, Keyframe, PropertyName, TrackKind};

/// Wire protocol version. A manifest declaring any other version is a
/// typed rejection — no compatibility shims exist between protocol
/// generations (28_PLUGINS §10: N-1 compatibility is a later policy).
pub const PROTOCOL_VERSION: u32 = 1;

/// The only capability host v1 knows. Default DENY (28_PLUGINS §9):
/// proposals from a manifest that did not declare it are rejected in-band.
pub const CAP_TIMELINE: &str = "propose.timeline";

/// ADR-022 (wave 18): a plugin is an UNTRUSTED peer. The session line
/// cap bounds the memory a hostile plugin can force the host to buffer
/// per stdout line (enforced at READ time in [`PluginHost::read_line`]
/// and again at the state machine — defense in depth). 1 MiB is ~1000×
/// any legitimate protocol line.
pub const PLUGIN_LINE_MAX_BYTES: usize = 1024 * 1024;

/// ADR-022: the proposal budget bounds the receipt memory and the
/// mutation-flood rate of a hostile session. Receipts are kept for
/// evidence (ADR-021 §10); 10 000 proposals is far past any legitimate
/// scripted edit run.
pub const PLUGIN_MAX_PROPOSALS: usize = 10_000;

// ---------------------------------------------------------------------------
// Errors
// ---------------------------------------------------------------------------

/// Errors from the host side of the plugin boundary. Protocol violations
/// abort the session; engine-level proposal rejections are NOT errors —
/// they ride `proposal_result` receipts.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PluginError {
    /// The plugin process could not be started.
    Spawn(String),
    /// Protocol-level violation — the session is aborted.
    Protocol(String),
    /// The plugin process ended before `done` (or before the manifest).
    PluginExited(String),
}

impl std::fmt::Display for PluginError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            PluginError::Spawn(s) => write!(f, "plugin spawn: {s}"),
            PluginError::Protocol(s) => write!(f, "plugin protocol violation: {s}"),
            PluginError::PluginExited(s) => write!(f, "plugin exited early: {s}"),
        }
    }
}

impl std::error::Error for PluginError {}

// ---------------------------------------------------------------------------
// Proposals — the ONLY write path across the plugin boundary
// ---------------------------------------------------------------------------

/// One key of a `set_keyframes` proposal. Time and value are exact
/// `"num/den"` strings; interp is `linear|hold` (ADR-019 v1 curve set).
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct KeySpec {
    pub time: String,
    pub value: String,
    pub interp: String,
}

/// A command proposal from a plugin — one-to-one with the typed grammar of
/// the other clients (CLI/batch/MCP/script), minus the verbs that would
/// grant the plugin host capabilities it does not have (import_media is a
/// filesystem grant — a later capability, default-DENY). Rationals are
/// exact `"num/den"` strings; a JSON number in a rational field fails
/// payload validation and is rejected in-band (ME-7 at the plugin edge).
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(tag = "verb", rename_all = "snake_case")]
pub enum Proposal {
    AddTrack {
        track_id: u64,
    },
    AddClip {
        track_id: u64,
        hash: String,
        duration: String,
        source_in: String,
    },
    Split {
        track_id: u64,
        clip_id: u64,
        at: String,
    },
    Resize {
        track_id: u64,
        clip_id: u64,
        duration: String,
    },
    MoveClip {
        clip_id: u64,
        from_track: u64,
        to_track: u64,
        to_index: u64,
    },
    RemoveClip {
        track_id: u64,
        clip_id: u64,
    },
    SetKeyframes {
        track_id: u64,
        clip_id: u64,
        property: String,
        keys: Vec<KeySpec>,
    },
}

impl Proposal {
    pub fn verb(&self) -> &'static str {
        match self {
            Proposal::AddTrack { .. } => "add_track",
            Proposal::AddClip { .. } => "add_clip",
            Proposal::Split { .. } => "split",
            Proposal::Resize { .. } => "resize",
            Proposal::MoveClip { .. } => "move_clip",
            Proposal::RemoveClip { .. } => "remove_clip",
            Proposal::SetKeyframes { .. } => "set_keyframes",
        }
    }

    /// Exact-rational parse at the plugin edge: `"num/den"` strings only.
    fn r(field: &str, s: &str) -> Result<Rational, String> {
        let (n, d) = s
            .split_once('/')
            .ok_or_else(|| format!("{field}: expected \"num/den\", got {s:?}"))?;
        let num = n
            .trim()
            .parse::<i64>()
            .map_err(|er| format!("{field}.num: {er}"))?;
        let den = d
            .trim()
            .parse::<i64>()
            .map_err(|er| format!("{field}.den: {er}"))?;
        if den == 0 {
            return Err(format!(
                "{field}: den == 0 — an exact rational needs a non-zero denominator"
            ));
        }
        Ok(Rational::new(num, den))
    }

    /// Apply through the SAME engine surface every other client uses.
    /// `Err` = the proposal is rejected in-band (payload or engine);
    /// the session continues either way.
    pub fn apply(self, e: &mut Engine) -> Result<String, String> {
        match self {
            Proposal::AddTrack { track_id } => e
                .add_track(track_id, TrackKind::Gap(GapTrack::new()))
                .map(|_| format!("track {track_id} added"))
                .map_err(|er| er.to_string()),
            Proposal::AddClip {
                track_id,
                hash,
                duration,
                source_in,
            } => {
                let d = Self::r("duration", &duration)?;
                let si = Self::r("source_in", &source_in)?;
                e.add_clip(track_id, &hash, d, si)
                    .map(|id| format!("clip {id} added"))
                    .map_err(|er| er.to_string())
            }
            Proposal::Split {
                track_id,
                clip_id,
                at,
            } => {
                let at = Self::r("at", &at)?;
                e.split(track_id, clip_id, at)
                    .map(|new| format!("split -> new clip {new}"))
                    .map_err(|er| er.to_string())
            }
            Proposal::Resize {
                track_id,
                clip_id,
                duration,
            } => {
                let d = Self::r("duration", &duration)?;
                e.resize(track_id, clip_id, d)
                    .map(|_| "resized".to_string())
                    .map_err(|er| er.to_string())
            }
            Proposal::MoveClip {
                clip_id,
                from_track,
                to_track,
                to_index,
            } => e
                .move_clip(clip_id, from_track, to_track, to_index as usize)
                .map(|_| "moved".to_string())
                .map_err(|er| er.to_string()),
            Proposal::RemoveClip { track_id, clip_id } => e
                .remove(track_id, clip_id)
                .map(|_| "removed".to_string())
                .map_err(|er| er.to_string()),
            Proposal::SetKeyframes {
                track_id,
                clip_id,
                property,
                keys,
            } => {
                let prop = match property.as_str() {
                    "opacity" => PropertyName::Opacity,
                    "x" => PropertyName::X,
                    "y" => PropertyName::Y,
                    other => return Err(format!("property: unknown {other:?}")),
                };
                let mut ks = Vec::new();
                for (i, k) in keys.iter().enumerate() {
                    let t = Self::r(&format!("keys[{i}].time"), &k.time)?;
                    let v = Self::r(&format!("keys[{i}].value"), &k.value)?;
                    let interp = match k.interp.as_str() {
                        "linear" => Interpolation::Linear,
                        "hold" => Interpolation::Hold,
                        other => {
                            return Err(format!(
                                "keys[{i}].interp: expected linear|hold, got {other:?}"
                            ))
                        }
                    };
                    ks.push(Keyframe::new(t, v, interp));
                }
                e.execute(Command::SetKeyframes {
                    track: track_id,
                    id: clip_id,
                    property: prop,
                    keys: ks,
                })
                .map(|_| "keyframes set".to_string())
                .map_err(|er| er.to_string())
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Wire helpers
// ---------------------------------------------------------------------------

fn hello_line() -> String {
    serde_json::json!({
        "type": "hello",
        "protocol": PROTOCOL_VERSION,
        "engine": "ove",
    })
    .to_string()
}

/// The read-only document view a plugin receives: state hash, tick axis,
/// and per-track clip windows with exact-rational durations. No project
/// uuid, no filesystem paths — document state only, exactly like batch
/// `status` (determinism: no clock, no randomness, no ambient paths).
pub fn document_context(e: &Engine) -> String {
    let tl = e.project().timeline();
    let (axis_num, axis_den) = e.project().tick_axis();
    let tracks: Vec<serde_json::Value> = tl
        .track_ids()
        .map(|tid| {
            let len = tl.track_len(tid).unwrap_or(0);
            let clips: Vec<serde_json::Value> = (0..len)
                .filter_map(|i| {
                    let c = tl.track_ref(tid).ok()?.clip_at(i)?.clone();
                    let animated: Vec<serde_json::Value> = [
                        ("opacity", &c.properties.opacity),
                        ("x", &c.properties.x),
                        ("y", &c.properties.y),
                    ]
                    .into_iter()
                    .filter(|(_, t)| !t.is_empty())
                    .map(|(n, _)| serde_json::Value::from(n))
                    .collect();
                    Some(serde_json::json!({
                        "id": c.id,
                        "duration": format!("{}/{}", c.duration.num(), c.duration.den()),
                        "source_in": format!("{}/{}", c.source_in.num(), c.source_in.den()),
                        "animated": animated,
                    }))
                })
                .collect();
            serde_json::json!({ "id": tid, "clips": clips })
        })
        .collect();
    serde_json::json!({
        "type": "context",
        "state_hash": e.state_hash(),
        "tick_axis": format!("{axis_num}/{axis_den}"),
        "tracks": tracks,
    })
    .to_string()
}

fn result_line(id: u64, verb: &str, applied: bool, error: Option<&str>, hash: &str) -> String {
    let mut v = serde_json::json!({
        "type": "proposal_result",
        "proposal": id,
        "verb": verb,
        "applied": applied,
        "state_hash": hash,
    });
    if let Some(err) = error {
        v["error"] = serde_json::Value::String(err.to_string());
    }
    v.to_string()
}

// ---------------------------------------------------------------------------
// Receipts + report (28_PLUGINS §10 evidence discipline)
// ---------------------------------------------------------------------------

/// Per-proposal evidence.
#[derive(Debug, Clone, serde::Serialize)]
pub struct ProposalReceipt {
    pub proposal: u64,
    pub verb: String,
    pub applied: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
    pub state_hash_after: String,
}

/// Deterministic session evidence: plugin identity, every receipt, final
/// hash. NO timestamps — the same project plus the same plugin behavior
/// yields byte-identical receipts (E-003 determinism discipline at the
/// plugin edge).
#[derive(Debug, Clone, serde::Serialize)]
pub struct SessionReport {
    pub plugin: String,
    pub version: String,
    pub capabilities: Vec<String>,
    pub applied: usize,
    pub rejected: usize,
    pub proposals: Vec<ProposalReceipt>,
    pub summary: String,
    pub final_state_hash: String,
}

impl SessionReport {
    /// Deterministic receipt bytes.
    pub fn to_receipt_json(&self) -> String {
        serde_json::to_string_pretty(self).expect("session report serializes")
    }
}

// ---------------------------------------------------------------------------
// Session state machine (pure — unit-testable without a process)
// ---------------------------------------------------------------------------

/// One processed plugin line.
#[derive(Debug, Clone)]
pub enum Step {
    /// A serialized host→plugin line to write back.
    Reply(String),
    /// A plugin log line — relay to stderr.
    Log(String),
    /// The plugin sent `done` — session complete.
    Complete,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Phase {
    AwaitManifest,
    Running,
    Complete,
}

/// The protocol state machine for ONE plugin session. Process-free: the
/// same [`PluginSession`] drives a real child ([`PluginHost`]) or a test
/// harness feeding lines directly.
pub struct PluginSession {
    name: String,
    version: String,
    capabilities: Vec<String>,
    phase: Phase,
    receipts: Vec<ProposalReceipt>,
    applied: usize,
    rejected: usize,
    summary: String,
}

impl Default for PluginSession {
    fn default() -> Self {
        Self::new()
    }
}

impl PluginSession {
    pub fn new() -> Self {
        PluginSession {
            name: String::new(),
            version: String::new(),
            capabilities: Vec::new(),
            phase: Phase::AwaitManifest,
            receipts: Vec::new(),
            applied: 0,
            rejected: 0,
            summary: String::new(),
        }
    }

    pub fn name(&self) -> &str {
        &self.name
    }

    pub fn version(&self) -> &str {
        &self.version
    }

    pub fn capabilities(&self) -> &[String] {
        &self.capabilities
    }

    /// Process one plugin line against the live engine. Protocol
    /// violations return [`PluginError::Protocol`] (abort); payload
    /// problems reject the proposal in-band and the session continues.
    pub fn ingest(&mut self, line: &str, e: &mut Engine) -> Result<Step, PluginError> {
        // ADR-022: hostile-line bound at the state machine too (the wire
        // reader enforces the same cap — defense in depth).
        if line.len() > PLUGIN_LINE_MAX_BYTES {
            return Err(PluginError::Protocol(format!(
                "line too large: {} > {PLUGIN_LINE_MAX_BYTES} bytes",
                line.len()
            )));
        }
        let v: serde_json::Value = serde_json::from_str(line)
            .map_err(|er| PluginError::Protocol(format!("non-JSON line: {er}")))?;
        let ty = v
            .get("type")
            .and_then(|t| t.as_str())
            .ok_or_else(|| PluginError::Protocol("message without \"type\"".to_string()))?;
        match ty {
            "manifest" => {
                if self.phase != Phase::AwaitManifest {
                    return Err(PluginError::Protocol(
                        "manifest must be the FIRST message and sent exactly once".to_string(),
                    ));
                }
                let protocol = v.get("protocol").and_then(|p| p.as_u64()).ok_or_else(|| {
                    PluginError::Protocol("manifest: missing protocol version".to_string())
                })? as u32;
                if protocol != PROTOCOL_VERSION {
                    return Err(PluginError::Protocol(format!(
                        "manifest protocol {protocol} != host protocol {PROTOCOL_VERSION} — no compatibility shim exists"
                    )));
                }
                let name = v
                    .get("name")
                    .and_then(|n| n.as_str())
                    .ok_or_else(|| PluginError::Protocol("manifest: missing name".to_string()))?
                    .to_string();
                let version = v
                    .get("version")
                    .and_then(|n| n.as_str())
                    .ok_or_else(|| PluginError::Protocol("manifest: missing version".to_string()))?
                    .to_string();
                let caps_raw = v.get("capabilities").cloned().unwrap_or_default();
                let caps = caps_raw
                    .as_array()
                    .ok_or_else(|| {
                        PluginError::Protocol("manifest: capabilities must be an array".to_string())
                    })?
                    .iter()
                    .map(|c| {
                        c.as_str().ok_or_else(|| {
                            PluginError::Protocol("manifest: capability not a string".to_string())
                        })
                    })
                    .collect::<Result<Vec<_>, _>>()?;
                for cap in &caps {
                    if *cap != CAP_TIMELINE {
                        return Err(PluginError::Protocol(format!(
                            "manifest: unknown capability {cap:?} (host v1 knows only {CAP_TIMELINE:?})"
                        )));
                    }
                }
                self.name = name;
                self.version = version;
                self.capabilities = caps.iter().map(|c| c.to_string()).collect();
                self.phase = Phase::Running;
                Ok(Step::Reply(hello_line()))
            }
            "proposal" => {
                if self.phase != Phase::Running {
                    return Err(PluginError::Protocol(
                        "proposal before manifest (or after done)".to_string(),
                    ));
                }
                // ADR-022: proposal budget — a hostile session must abort
                // typed instead of accumulating receipts unboundedly.
                if self.receipts.len() >= PLUGIN_MAX_PROPOSALS {
                    return Err(PluginError::Protocol(format!(
                        "proposal budget exhausted: more than {PLUGIN_MAX_PROPOSALS} proposals in one session"
                    )));
                }
                let id = v.get("proposal").and_then(|p| p.as_u64()).ok_or_else(|| {
                    PluginError::Protocol("proposal: missing numeric \"proposal\" id".to_string())
                })?;
                let hash = e.state_hash();
                // Default DENY (28_PLUGINS §9): undeclared capability →
                // in-band rejection, session continues.
                if !self.capabilities.iter().any(|c| c.as_str() == CAP_TIMELINE) {
                    self.rejected += 1;
                    let err = format!(
                        "capability {CAP_TIMELINE:?} not declared in manifest — default DENY"
                    );
                    self.receipts.push(ProposalReceipt {
                        proposal: id,
                        verb: "(undeclared)".to_string(),
                        applied: false,
                        error: Some(err.clone()),
                        state_hash_after: hash.clone(),
                    });
                    return Ok(Step::Reply(result_line(
                        id,
                        "(undeclared)",
                        false,
                        Some(&err),
                        &hash,
                    )));
                }
                let verb = v
                    .get("verb")
                    .and_then(|x| x.as_str())
                    .unwrap_or("(none)")
                    .to_string();
                let receipt = match serde_json::from_value::<Proposal>(v.clone()) {
                    Ok(p) => {
                        let verb = p.verb().to_string();
                        match p.apply(e) {
                            Ok(_) => {
                                self.applied += 1;
                                ProposalReceipt {
                                    proposal: id,
                                    verb,
                                    applied: true,
                                    error: None,
                                    state_hash_after: e.state_hash(),
                                }
                            }
                            Err(reason) => {
                                self.rejected += 1;
                                ProposalReceipt {
                                    proposal: id,
                                    verb,
                                    applied: false,
                                    error: Some(reason),
                                    state_hash_after: e.state_hash(),
                                }
                            }
                        }
                    }
                    Err(reason) => {
                        self.rejected += 1;
                        ProposalReceipt {
                            proposal: id,
                            verb,
                            applied: false,
                            error: Some(format!("proposal payload: {reason}")),
                            state_hash_after: e.state_hash(),
                        }
                    }
                };
                let line = result_line(
                    receipt.proposal,
                    &receipt.verb,
                    receipt.applied,
                    receipt.error.as_deref(),
                    &receipt.state_hash_after,
                );
                self.receipts.push(receipt);
                Ok(Step::Reply(line))
            }
            "log" => {
                if self.phase != Phase::Running {
                    return Err(PluginError::Protocol(
                        "log outside the running phase".to_string(),
                    ));
                }
                Ok(Step::Log(
                    v.get("line")
                        .and_then(|l| l.as_str())
                        .unwrap_or_default()
                        .to_string(),
                ))
            }
            "done" => {
                if self.phase != Phase::Running {
                    return Err(PluginError::Protocol(
                        "done before manifest (or duplicate done)".to_string(),
                    ));
                }
                self.summary = v
                    .get("summary")
                    .and_then(|s| s.as_str())
                    .unwrap_or_default()
                    .to_string();
                self.phase = Phase::Complete;
                Ok(Step::Complete)
            }
            other => Err(PluginError::Protocol(format!(
                "unknown message type {other:?}"
            ))),
        }
    }

    /// The final report (only meaningful once [`Step::Complete`] fired).
    pub fn report(&self, final_state_hash: String) -> SessionReport {
        SessionReport {
            plugin: self.name.clone(),
            version: self.version.clone(),
            capabilities: self.capabilities.clone(),
            applied: self.applied,
            rejected: self.rejected,
            proposals: self.receipts.clone(),
            summary: self.summary.clone(),
            final_state_hash,
        }
    }
}

// ---------------------------------------------------------------------------
// Process host
// ---------------------------------------------------------------------------

/// Drives ONE plugin process through ONE session against ONE engine.
/// The child's stderr is inherited (its own diagnostics stay its own);
/// protocol traffic is the child's stdin/stdout, one JSON value per line.
pub struct PluginHost {
    child: Child,
    stdout: BufReader<ChildStdout>,
    session: PluginSession,
}

impl PluginHost {
    /// Launch rules (ADR-022): the plugin process is exec'd DIRECTLY —
    /// arguments are passed verbatim to the executable, never through a
    /// shell, so no plugin name or argument can ever gain shell
    /// semantics. `spawn` is the no-argument form.
    pub fn spawn(exe: &Path) -> Result<Self, PluginError> {
        Self::spawn_with_args(exe, &[])
    }

    /// Direct-exec launch with verbatim arguments (no shell anywhere).
    pub fn spawn_with_args(exe: &Path, args: &[&str]) -> Result<Self, PluginError> {
        let mut child = Proc::new(exe)
            .args(args)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::inherit())
            .spawn()
            .map_err(|er| PluginError::Spawn(format!("{}: {er}", exe.display())))?;
        let stdout = BufReader::new(child.stdout.take().expect("stdout piped"));
        Ok(PluginHost {
            child,
            stdout,
            session: PluginSession::new(),
        })
    }

    fn read_line(&mut self) -> Result<Option<String>, PluginError> {
        // ADR-022: bounded read — the host never buffers a hostile line
        // past the cap; the abort happens BEFORE the full line is read.
        use std::io::Read;
        let mut buf = Vec::new();
        let n = self
            .stdout
            .by_ref()
            .take((PLUGIN_LINE_MAX_BYTES + 2) as u64)
            .read_until(b'\n', &mut buf)
            .map_err(|er| PluginError::Protocol(format!("read from plugin: {er}")))?;
        if n == 0 {
            return Ok(None);
        }
        if n > PLUGIN_LINE_MAX_BYTES + 1 {
            return Err(PluginError::Protocol(format!(
                "line too large: {n} > {PLUGIN_LINE_MAX_BYTES} bytes — aborted at read time"
            )));
        }
        let s = String::from_utf8_lossy(&buf).to_string();
        Ok(Some(s.trim_end_matches(['\n', '\r']).to_string()))
    }

    fn write_line(&mut self, line: &str) -> Result<(), PluginError> {
        let stdin = self.child.stdin.as_mut().expect("stdin piped");
        writeln!(stdin, "{line}")
            .map_err(|er| PluginError::Protocol(format!("write to plugin: {er}")))
    }

    /// Manifest handshake: the plugin's first line must be a valid
    /// manifest; the host answers `hello`.
    pub fn handshake(&mut self, e: &mut Engine) -> Result<(), PluginError> {
        match self.read_line()? {
            Some(line) => match self.session.ingest(&line, e)? {
                Step::Reply(r) => self.write_line(&r),
                _ => Err(PluginError::Protocol(
                    "manifest phase produced a non-reply".to_string(),
                )),
            },
            None => Err(PluginError::PluginExited(
                "plugin closed stdout before manifest".to_string(),
            )),
        }
    }

    /// Push the read-only document context.
    pub fn send_context(&mut self, e: &Engine) -> Result<(), PluginError> {
        self.write_line(&document_context(e))
    }

    /// Run to `done`, applying proposals to the engine as they arrive.
    /// Reaps the child; protocol completeness governs success (the exit
    /// status is advisory — a plugin that said `done` may exit non-zero
    /// only through its own failure after its last mutation was receipted).
    pub fn run(&mut self, e: &mut Engine) -> Result<SessionReport, PluginError> {
        loop {
            match self.read_line()? {
                Some(line) => match self.session.ingest(&line, e)? {
                    Step::Reply(r) => self.write_line(&r)?,
                    Step::Log(l) => eprintln!("[plugin {}] {}", self.session.name(), l),
                    Step::Complete => break,
                },
                None => {
                    return Err(PluginError::PluginExited(
                        "plugin closed stdout before done".to_string(),
                    ))
                }
            }
        }
        let final_hash = e.state_hash();
        let report = self.session.report(final_hash);
        let _ = self.child.wait();
        Ok(report)
    }
}
