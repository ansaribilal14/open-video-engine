//! ove-script — Rhai scripting client (wave 16, directive §1: "scripting is
//! a client; edit semantics live ONLY in the engine core").
//!
//! FLOAT DISCIPLINE (ME-7 at the scripting edge): Rhai is built with
//! `no_float` — a script containing a float literal fails to PARSE, so
//! imprecise time cannot even be expressed. Rationals come from integer
//! pairs via `r(num, den)` with `den != 0` validated loudly; there is no
//! float path into the engine.
//!
//! SESSION MODEL: one engine session per script run, bound as the `ove`
//! variable (a shared handle). The registered API mirrors the MCP tool
//! set so all three clients (human CLI, AI agent, script) drive the SAME
//! semantics. Determinism: no clock, no randomness — the project uuid is
//! intentionally NOT exposed (scripts see document state only, exactly
//! like batch `status`).

use rhai::{Engine as RhaiEngine, EvalAltResult, Position};

use ove_engine::Engine;
use ove_time::Rational;
use ove_timeline::{GapTrack, TrackKind};

/// Errors surfacing from a script run (stringly at the Rhai boundary —
/// the Rhai error type owns the context; we add no information loss).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ScriptError(pub String);

impl std::fmt::Display for ScriptError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "script error: {}", self.0)
    }
}

impl std::error::Error for ScriptError {}

impl From<String> for ScriptError {
    fn from(s: String) -> Self {
        ScriptError(s)
    }
}

/// Rhai-native error wrapper (registered functions return NativeCallResult).
fn rerr(e: impl std::fmt::Display) -> Box<EvalAltResult> {
    Box::new(EvalAltResult::ErrorRuntime(
        e.to_string().into(),
        Position::NONE,
    ))
}

/// The `ove` handle the script drives. Interior mutability via RefCell so
/// Rhai method calls (`ove.add_track(1)`) work on a shared clone.
#[derive(Clone)]
pub struct Session {
    inner: std::rc::Rc<std::cell::RefCell<Inner>>,
}

struct Inner {
    engine: Option<Engine>,
    dir: std::path::PathBuf,
    out: Vec<String>,
}

impl Session {
    /// Bind to a project directory (opened lazily by the first engine verb).
    pub fn new(dir: &std::path::Path) -> Self {
        Session {
            inner: std::rc::Rc::new(std::cell::RefCell::new(Inner {
                engine: None,
                dir: dir.to_path_buf(),
                out: Vec::new(),
            })),
        }
    }

    /// Run `f` with the open engine session (opening lazily).
    fn with_engine<T>(
        &mut self,
        f: impl FnOnce(&mut Engine) -> Result<T, String>,
    ) -> Result<T, ScriptError> {
        if self.inner.borrow().engine.is_none() {
            let e = Engine::open(&self.inner.borrow().dir)
                .map_err(|e| ScriptError(format!("open project: {e}")))?;
            self.inner.borrow_mut().engine = Some(e);
        }
        let mut inner = self.inner.borrow_mut();
        f(inner.engine.as_mut().unwrap()).map_err(ScriptError)
    }

    /// Deterministic output collected via `emit` during the run.
    pub fn output(&self) -> String {
        self.inner.borrow().out.join("\n")
    }

    /// Register the engine API onto a rhai engine (exact rationals only).
    fn register(self, rh: &mut RhaiEngine) {
        // the ONLY rational constructor: integer pairs, den != 0, loud
        rh.register_fn(
            "r",
            |num: i64, den: i64| -> Result<Rational, Box<EvalAltResult>> {
                if den == 0 {
                    Err(rerr(
                        "r(num, den): den == 0 — an exact rational needs a non-zero denominator",
                    ))
                } else {
                    Ok(Rational::new(num, den))
                }
            },
        );
        rh.register_fn("emit", |s: &mut Session, line: String| {
            s.inner.borrow_mut().out.push(line);
        });
        rh.register_fn(
            "create_project",
            |s: &mut Session, num: i64, den: i64| -> Result<String, Box<EvalAltResult>> {
                if den == 0 {
                    return Err(rerr("create_project: tick_den == 0"));
                }
                let dir = s.inner.borrow().dir.clone();
                Engine::create(&dir, (num, den)).map_err(rerr)?;
                s.inner.borrow_mut().engine = Some(Engine::open(&dir).map_err(rerr)?);
                Ok(format!(
                    "created project at {} (tick axis {num}/{den})",
                    dir.display()
                ))
            },
        );
        rh.register_fn(
            "add_track",
            |s: &mut Session, id: i64| -> Result<String, Box<EvalAltResult>> {
                s.with_engine(|e| {
                    let id = u64::try_from(id).map_err(|er| format!("track id: {er}"))?;
                    e.add_track(id, TrackKind::Gap(GapTrack::new()))
                        .map_err(|er| er.to_string())?;
                    Ok(format!("track {id} added"))
                })
                .map_err(rerr)
            },
        );
        rh.register_fn(
            "import_media",
            |s: &mut Session, path: String| -> Result<String, Box<EvalAltResult>> {
                s.with_engine(|e| {
                    let hex = e
                        .import_media(std::path::Path::new(&path))
                        .map_err(|er| er.to_string())?;
                    Ok(format!("imported {path} as {hex}"))
                })
                .map_err(rerr)
            },
        );
        rh.register_fn(
            "add_clip",
            |s: &mut Session,
             track: i64,
             hash: String,
             dur: Rational,
             src_in: Rational|
             -> Result<String, Box<EvalAltResult>> {
                s.with_engine(move |e| {
                    let track = u64::try_from(track).map_err(|er| format!("track: {er}"))?;
                    let clip = e
                        .add_clip(track, &hash, dur, src_in)
                        .map_err(|er| er.to_string())?;
                    Ok(format!("clip {clip} added to track {track}"))
                })
                .map_err(rerr)
            },
        );
        rh.register_fn(
            "split",
            |s: &mut Session,
             track: i64,
             clip: i64,
             at: Rational|
             -> Result<String, Box<EvalAltResult>> {
                s.with_engine(move |e| {
                    let (track, clip) = (
                        u64::try_from(track).map_err(|er| format!("track: {er}"))?,
                        u64::try_from(clip).map_err(|er| format!("clip: {er}"))?,
                    );
                    let new_id = e.split(track, clip, at).map_err(|er| er.to_string())?;
                    Ok(format!("split clip {clip} at {at} -> new clip {new_id}"))
                })
                .map_err(rerr)
            },
        );
        rh.register_fn(
            "undo",
            |s: &mut Session| -> Result<String, Box<EvalAltResult>> {
                s.with_engine(|e| {
                    let did = e.undo().map_err(|er| er.to_string())?;
                    Ok(if did { "undone" } else { "nothing to undo" }.to_string())
                })
                .map_err(rerr)
            },
        );
        rh.register_fn(
            "redo",
            |s: &mut Session| -> Result<String, Box<EvalAltResult>> {
                s.with_engine(|e| {
                    let did = e.redo().map_err(|er| er.to_string())?;
                    Ok(if did { "redone" } else { "nothing to redo" }.to_string())
                })
                .map_err(rerr)
            },
        );
        rh.register_fn(
            "get_status",
            |s: &mut Session| -> Result<String, Box<EvalAltResult>> {
                s.with_engine(|e| {
                    let tracks = e
                        .project()
                        .timeline()
                        .track_ids()
                        .map(|t| t.to_string())
                        .collect::<Vec<_>>()
                        .join(",");
                    Ok(format!("state_hash={} tracks=[{tracks}]", e.state_hash()))
                })
                .map_err(rerr)
            },
        );
    }
}

/// Run `source` against the project at `dir` with the standard API exposed
/// as the `ove` variable. Same script + same project state → same output.
pub fn run_script(dir: &std::path::Path, source: &str) -> Result<String, ScriptError> {
    let session = Session::new(dir);
    let mut rh = RhaiEngine::new();
    session.clone().register(&mut rh);
    let mut scope = rhai::Scope::new();
    scope.push("ove", session.clone());
    rh.run_with_scope(&mut scope, source)
        .map_err(|e| ScriptError(format!("{e}")))?;
    Ok(session.output())
}
