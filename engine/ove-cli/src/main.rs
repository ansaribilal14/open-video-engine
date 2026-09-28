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
        other => Err(format!("unknown command {other:?} (see module docs)")),
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
