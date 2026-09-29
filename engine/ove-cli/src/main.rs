//! ove-cli — headless CLI for the open-video-engine (WAVE 5).
//!
//! Subcommands (each opens the project, does one unit of work, exits):
//!   new <dir> <tick-num> <tick-den>
//!   add-track <dir> <id> gap
//!   import <dir> <file>
//!   add-clip <dir> <track> <asset-hash> <dur-num>/<dur-den> <in-num>/<in-den>
//!   split <dir> <track> <clip-id> <at-num>/<at-den>
//!   status <dir>
//!   undo <dir> / redo <dir>
//!   export-copy <dir> <out> <hash> <start-n/d> <end-n/d>
//!
//! Rationals on the command line are "num/den" (exact; no floats ever).

use std::process::ExitCode;

use ove_engine::Engine;
use ove_timeline::{GapTrack, TrackKind};

fn parse_rational(s: &str) -> Result<R, String> {
    let (n, d) = s
        .split_once('/')
        .ok_or_else(|| format!("expected num/den, got {s:?}"))?;
    Ok(R::new(
        n.parse::<i64>().map_err(|e| format!("num: {e}"))?,
        d.parse::<i64>().map_err(|e| format!("den: {e}"))?,
    ))
}

use ove_time::Rational as R;

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    match run(&args) {
        Ok(msg) => {
            println!("{msg}");
            ExitCode::SUCCESS
        }
        Err(e) => {
            eprintln!("error: {e}");
            ExitCode::FAILURE
        }
    }
}

fn run(args: &[String]) -> Result<String, String> {
    let (cmd, rest) = args.split_first().ok_or("usage: ove-cli <command> ...")?;
    match cmd.as_str() {
        "new" => {
            let [dir, num, den] = take(rest, 3)?;
            let tick = (
                num.parse::<i64>().map_err(|e| e.to_string())?,
                den.parse::<i64>().map_err(|e| e.to_string())?,
            );
            Engine::create(std::path::Path::new(dir), tick).map_err(|e| e.to_string())?;
            Ok(format!("created project at {dir} (tick axis {tick:?})"))
        }
        "add-track" => {
            let [dir, id] = take(rest, 2)?;
            let mut e = Engine::open(std::path::Path::new(dir)).map_err(|e| e.to_string())?;
            let tid: u64 = id.parse().map_err(|e| format!("track id: {e}"))?;
            e.add_track(tid, TrackKind::Gap(GapTrack::new()))
                .map_err(|e| e.to_string())?;
            Ok(format!("track {tid} added"))
        }
        "import" => {
            let [dir, file] = take(rest, 2)?;
            let mut e = Engine::open(std::path::Path::new(dir)).map_err(|e| e.to_string())?;
            let hex = e
                .import_media(std::path::Path::new(file))
                .map_err(|e| e.to_string())?;
            Ok(format!("imported {file} as {hex}"))
        }
        "add-clip" => {
            let [dir, track, hash, dur, src_in] = take(rest, 5)?;
            let mut e = Engine::open(std::path::Path::new(dir)).map_err(|e| e.to_string())?;
            let clip = e
                .add_clip(
                    track.parse().map_err(|e| format!("track: {e}"))?,
                    hash,
                    parse_rational(dur)?,
                    parse_rational(src_in)?,
                )
                .map_err(|e| e.to_string())?;
            Ok(format!("clip {clip} added to track {track}"))
        }
        "split" => {
            let [dir, track, clip, at] = take(rest, 4)?;
            let mut e = Engine::open(std::path::Path::new(dir)).map_err(|e| e.to_string())?;
            let new_id = e
                .split(
                    track.parse().map_err(|e| format!("track: {e}"))?,
                    clip.parse().map_err(|e| format!("clip id: {e}"))?,
                    parse_rational(at)?,
                )
                .map_err(|e| e.to_string())?;
            Ok(format!("split clip {clip} at {at} -> new clip {new_id}"))
        }
        "undo" => {
            let [dir] = take(rest, 1)?;
            let mut e = Engine::open(std::path::Path::new(dir)).map_err(|e| e.to_string())?;
            let did = e.undo().map_err(|e| e.to_string())?;
            Ok(if did {
                "undone".into()
            } else {
                "nothing to undo".into()
            })
        }
        "redo" => {
            let [dir] = take(rest, 1)?;
            let mut e = Engine::open(std::path::Path::new(dir)).map_err(|e| e.to_string())?;
            let did = e.redo().map_err(|e| e.to_string())?;
            Ok(if did {
                "redone".into()
            } else {
                "nothing to redo".into()
            })
        }
        "status" => {
            let [dir] = take(rest, 1)?;
            let e = Engine::open(std::path::Path::new(dir)).map_err(|e| e.to_string())?;
            let assets = e
                .project()
                .assets()
                .iter()
                .map(|a| a.id.clone())
                .collect::<Vec<_>>()
                .join(",");
            let tracks = e
                .project()
                .timeline()
                .track_ids()
                .map(|t| t.to_string())
                .collect::<Vec<_>>()
                .join(",");
            Ok(format!(
                "state_hash={} uuid={} assets=[{assets}] tracks=[{tracks}]",
                e.state_hash(),
                e.uuid()
            ))
        }
        "export-copy" => {
            let [dir, out, hash, start, end] = take(rest, 5)?;
            let mut e = Engine::open(std::path::Path::new(dir)).map_err(|e| e.to_string())?;
            let (info, snaps) = e
                .export_copy(
                    hash,
                    parse_rational(start)?,
                    parse_rational(end)?,
                    std::path::Path::new(out),
                )
                .map_err(|e| e.to_string())?;
            Ok(format!(
                "exported {out} ({} bytes, {} snaps reported)",
                info.file_size,
                snaps.len()
            ))
        }
        "batch" => {
            let [dir, script] = take(rest, 2)?;
            batch_run(std::path::Path::new(dir), std::path::Path::new(script))
        }
        other => Err(format!("unknown command {other:?} (see module docs)")),
    }
}

// ---------------------------------------------------------------------------
// WAVE 14 — headless batch mode: ONE engine session, MANY script verbs.
//
// Grammar (one verb per line; `#` comments; blank lines ignored; all
// rationals `num/den` — floats are rejected by the parser, never coerced):
//   new <tick-num> <tick-den>          (create the project; first verb)
//   add-track <id> gap
//   import <file>
//   add-clip <track> <hash> <dur> <src-in>
//   split <track> <clip> <at>
//   resize <track> <clip> <dur>
//   move <clip> <from-track> <to-track> <index>
//   remove <track> <clip>
//   undo | redo
//   status
//   export-copy <out> <hash> <start> <end>
//   export-wav <out>
//
// Path contract: RELATIVE paths in file-taking verbs (import/export-*)
// resolve against the PROJECT dir — the project is the batch's workspace
// root, independent of the process cwd.
// Determinism contract: same script → same output lines (state hashes
// included); fail-fast at the first error (`ERR <lineno> <verb>: <e>` on
// stderr, exit 1) — never a silent skip (D-9 honesty rule at the shell).
// ---------------------------------------------------------------------------
fn batch_run(dir: &std::path::Path, script: &std::path::Path) -> Result<String, String> {
    let text = std::fs::read_to_string(script)
        .map_err(|e| format!("read script {}: {e}", script.display()))?;
    let mut session: Option<Engine> = None;
    let mut out_lines: Vec<String> = Vec::new();
    for (idx, raw) in text.lines().enumerate() {
        let lineno = idx + 1;
        let line = raw.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        let toks: Vec<String> = line.split_whitespace().map(String::from).collect();
        let verb = toks[0].as_str();
        let rest: Vec<String> = toks[1..].to_vec();
        let detail = batch_exec(dir, &mut session, verb, &rest)
            .map_err(|e| format!("ERR {lineno} {verb}: {e}"))?;
        out_lines.push(format!("OK {lineno} {verb} {detail}"));
    }
    Ok(out_lines.join("\n"))
}

fn batch_exec(
    dir: &std::path::Path,
    session: &mut Option<Engine>,
    verb: &str,
    a: &[String],
) -> Result<String, String> {
    // project-rooted path resolution (see the path contract above)
    let rooted = |p: &str| -> std::path::PathBuf {
        let pb = std::path::Path::new(p);
        if pb.is_absolute() {
            pb.to_path_buf()
        } else {
            dir.join(pb)
        }
    };
    // lazy open — every verb except `new` reuses the ONE session
    fn opened<'a>(
        dir: &std::path::Path,
        s: &'a mut Option<Engine>,
    ) -> Result<&'a mut Engine, String> {
        if s.is_none() {
            *s = Some(Engine::open(dir).map_err(|e| e.to_string())?);
        }
        Ok(s.as_mut().unwrap())
    }
    match verb {
        "new" => {
            let [num, den] = take(a, 2)?;
            if session.is_some() {
                return Err("project already created in this batch".into());
            }
            let tick = (
                num.parse::<i64>().map_err(|e| e.to_string())?,
                den.parse::<i64>().map_err(|e| e.to_string())?,
            );
            Engine::create(dir, tick).map_err(|e| e.to_string())?;
            *session = Some(Engine::open(dir).map_err(|e| e.to_string())?);
            Ok(format!("created (tick axis {tick:?})"))
        }
        "add-track" => {
            let [id] = take(a, 1)?;
            let e = opened(dir, session)?;
            e.add_track(
                id.parse().map_err(|er| format!("track id: {er}"))?,
                TrackKind::Gap(GapTrack::new()),
            )
            .map_err(|er| er.to_string())?;
            Ok(format!("track {id} added"))
        }
        "import" => {
            let [file] = take(a, 1)?;
            let e = opened(dir, session)?;
            let hex = e.import_media(&rooted(file)).map_err(|er| er.to_string())?;
            Ok(format!("imported {file} as {hex}"))
        }
        "add-clip" => {
            let [track, hash, dur, src_in] = take(a, 4)?;
            let e = opened(dir, session)?;
            let clip = e
                .add_clip(
                    track.parse().map_err(|er| format!("track: {er}"))?,
                    hash,
                    parse_rational(dur)?,
                    parse_rational(src_in)?,
                )
                .map_err(|er| er.to_string())?;
            Ok(format!("clip {clip} added to track {track}"))
        }
        "split" => {
            let [track, clip, at] = take(a, 3)?;
            let e = opened(dir, session)?;
            let new_id = e
                .split(
                    track.parse().map_err(|er| format!("track: {er}"))?,
                    clip.parse().map_err(|er| format!("clip id: {er}"))?,
                    parse_rational(at)?,
                )
                .map_err(|er| er.to_string())?;
            Ok(format!("split clip {clip} at {at} -> new clip {new_id}"))
        }
        "resize" => {
            let [track, clip, dur] = take(a, 3)?;
            let e = opened(dir, session)?;
            e.resize(
                track.parse().map_err(|er| format!("track: {er}"))?,
                clip.parse().map_err(|er| format!("clip id: {er}"))?,
                parse_rational(dur)?,
            )
            .map_err(|er| er.to_string())?;
            Ok(format!("clip {clip} resized to {dur}"))
        }
        "move" => {
            let [clip, from, to, index] = take(a, 4)?;
            let e = opened(dir, session)?;
            e.move_clip(
                clip.parse().map_err(|er| format!("clip id: {er}"))?,
                from.parse().map_err(|er| format!("track: {er}"))?,
                to.parse().map_err(|er| format!("track: {er}"))?,
                index.parse().map_err(|er| format!("index: {er}"))?,
            )
            .map_err(|er| er.to_string())?;
            Ok(format!("clip {clip} moved to track {to} index {index}"))
        }
        "remove" => {
            let [track, clip] = take(a, 2)?;
            let e = opened(dir, session)?;
            e.remove(
                track.parse().map_err(|er| format!("track: {er}"))?,
                clip.parse().map_err(|er| format!("clip id: {er}"))?,
            )
            .map_err(|er| er.to_string())?;
            Ok(format!("clip {clip} removed"))
        }
        "undo" => {
            let [] = take(a, 0)?;
            let e = opened(dir, session)?;
            let did = e.undo().map_err(|er| er.to_string())?;
            Ok(if did {
                "undone".into()
            } else {
                "nothing to undo".into()
            })
        }
        "redo" => {
            let [] = take(a, 0)?;
            let e = opened(dir, session)?;
            let did = e.redo().map_err(|er| er.to_string())?;
            Ok(if did {
                "redone".into()
            } else {
                "nothing to redo".into()
            })
        }
        "status" => {
            let [] = take(a, 0)?;
            let e = opened(dir, session)?;
            // batch output carries DOCUMENT state only: the project uuid is
            // random-by-design (fresh identity per create) and would break
            // the determinism contract; the interactive `status` prints it.
            Ok(format!("state_hash={}", e.state_hash()))
        }
        "export-copy" => {
            let [out, hash, start, end] = take(a, 4)?;
            let e = opened(dir, session)?;
            let (info, snaps) = e
                .export_copy(
                    hash,
                    parse_rational(start)?,
                    parse_rational(end)?,
                    &rooted(out),
                )
                .map_err(|er| er.to_string())?;
            Ok(format!(
                "exported {out} ({} bytes, {} snaps reported)",
                info.file_size,
                snaps.len()
            ))
        }
        "export-wav" => {
            let [out] = take(a, 1)?;
            let e = opened(dir, session)?;
            let bytes = e.export_wav(&rooted(out)).map_err(|er| er.to_string())?;
            Ok(format!("wrote {out} ({bytes} samples)"))
        }
        other => Err(format!("unknown batch verb {other:?}")),
    }
}

fn take<const N: usize>(args: &[String], n: usize) -> Result<[&str; N], String> {
    if args.len() < n {
        return Err(format!("missing arguments (expected {n})"));
    }
    let mut out = [""; N];
    for (i, slot) in out.iter_mut().enumerate() {
        *slot = &args[i];
    }
    Ok(out)
}
