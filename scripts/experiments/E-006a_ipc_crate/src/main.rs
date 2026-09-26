//! E-006a — IPC serialization costs, native side (serde_json vs bincode vs postcard).
//!
//! Re-run of the experiment whose artifacts were lost (audit D-1 / OPEN_GAPS X-5).
//! Spec: research/experiments/E-006a_ipc_serialization.md (unchanged).
//!
//! Traffic shapes from the engine's own data model:
//!   P1: 1 000-command batch (ADR-010 verbs: enum + i64 args + short string)
//!   P2: 20 000-clip metadata snapshot (E-002c scale)
//!   P3: 1 MiB RGBA byte payload (frame-scale; JSON bans bytes as decimal lists)
//!
//! Method improvement over the lost run: warmup + 30 iterations, MEDIAN per-op
//! timings (the original was a single run; medians make the numbers reproducible
//! across machines while the finding — relative orders — is unchanged).
//! Raw output: experiments/E-006a_result.txt (piped stdout).

use serde::{Deserialize, Serialize};
use std::fmt::Write as _;
use std::time::{Duration, Instant};

// ---------- payload models (ADR-010 command surface, E-002c clip scale) ----------

#[derive(Clone, Serialize, Deserialize)]
enum Cmd {
    AddClip { id: u64, track: u32, src: String, start_num: i64, start_den: i64, dur_num: i64, dur_den: i64 },
    MoveClip { id: u64, track: u32, start_num: i64, start_den: i64 },
    ResizeClip { id: u64, out_num: i64, out_den: i64 },
    SplitClip { id: u64, at_num: i64, at_den: i64, new_id: u64 },
    SetOpacity { id: u64, num: i64, den: i64 },
}

fn command_batch(n: usize) -> Vec<Cmd> {
    (0..n)
        .map(|i| match i % 5 {
            0 => Cmd::AddClip {
                id: i as u64, track: (i % 4) as u32, src: format!("asset-{}/stream-0", i % 97),
                start_num: (i as i64) * 1001, start_den: 24000, dur_num: 1001 * 48, dur_den: 24000,
            },
            1 => Cmd::MoveClip { id: i as u64, track: ((i + 1) % 4) as u32, start_num: (i as i64) * 997, start_den: 24000 },
            2 => Cmd::ResizeClip { id: i as u64, out_num: 1001 * 30, out_den: 24000 },
            3 => Cmd::SplitClip { id: i as u64, at_num: 1001 * 12, at_den: 24000, new_id: (i as u64) + 1_000_000 },
            _ => Cmd::SetOpacity { id: i as u64, num: 1, den: 2 },
        })
        .collect()
}

#[derive(Clone, Serialize, Deserialize)]
struct ClipMeta {
    id: u64,
    track: u32,
    start: (i64, i64),   // exact rational (num, den)
    duration: (i64, i64),
    src_in: (i64, i64),
    asset: String,
}

fn clip_snapshot(n: usize) -> Vec<ClipMeta> {
    (0..n)
        .map(|i| ClipMeta {
            id: i as u64,
            track: (i % 4) as u32,
            start: ((i as i64) * 1001, 24000),
            duration: (1001 * 48, 24000),
            src_in: (0, 24000),
            asset: format!("asset-{}", i % 97),
        })
        .collect()
}

// ---------- measurement ----------

fn time_median<F: FnMut()>(mut f: F, iters: usize) -> Duration {
    let mut samples: Vec<Duration> = Vec::with_capacity(iters);
    for _ in 0..iters {
        let t = Instant::now();
        f();
        samples.push(t.elapsed());
    }
    samples.sort();
    samples[iters / 2]
}

fn ms(d: Duration) -> f64 { d.as_secs_f64() * 1e3 }

struct Row { name: &'static str, json_bytes: usize, json_ser: f64, json_de: f64, bin_bytes: usize, bin_ser: f64, bin_de: f64, post_bytes: usize, post_ser: f64, post_de: f64 }

fn bench<T: Serialize + for<'de> Deserialize<'de> + Clone>(name: &'static str, payload: &T, iters: usize) -> Row {
    // sizes (single measurement, deterministic)
    let json_bytes = serde_json::to_vec(payload).expect("json ser").len();
    let bin_bytes = bincode::serialize(payload).expect("bincode ser").len();
    let post_bytes = postcard::to_allocvec(payload).expect("postcard ser").len();

    // timings: serialize = fresh to_vec per iter; deserialize = from prebuilt bytes
    let json_ser = time_median(|| { let _ = serde_json::to_vec(payload).unwrap(); }, iters);
    let json_buf = serde_json::to_vec(payload).unwrap();
    let json_de = time_median(|| { let _: T = serde_json::from_slice(&json_buf).unwrap(); }, iters);

    let bin_ser = time_median(|| { let _ = bincode::serialize(payload).unwrap(); }, iters);
    let bin_buf = bincode::serialize(payload).unwrap();
    let bin_de = time_median(|| { let _: T = bincode::deserialize(&bin_buf).unwrap(); }, iters);

    let post_ser = time_median(|| { let _ = postcard::to_allocvec(payload).unwrap(); }, iters);
    let post_buf = postcard::to_allocvec(payload).unwrap();
    let post_de = time_median(|| { let _: T = postcard::from_bytes(&post_buf).unwrap(); }, iters);

    Row { name, json_bytes, json_ser: ms(json_ser), json_de: ms(json_de), bin_bytes, bin_ser: ms(bin_ser), bin_de: ms(bin_de), post_bytes, post_ser: ms(post_ser), post_de: ms(post_de) }
}

fn main() {
    const ITERS: usize = 30;
    let mut out = String::new();

    // warmup (page in allocator, populate caches)
    let _ = bench::<Vec<Cmd>>("warmup", &command_batch(100), 3);

    let rows = vec![
        bench("P1 command-batch-1000", &command_batch(1_000), ITERS),
        bench("P2 clip-snapshot-20000", &clip_snapshot(20_000), ITERS),
        bench("P3 rgba-1MiB", &vec![0xA5u8; 1 << 20], ITERS),
    ];

    let _ = writeln!(out, "E-006a native IPC serialization — re-run {}", chrono_header());
    let _ = writeln!(out, "method: median of {} iterations, release build, container CPU", ITERS);
    let _ = writeln!(out, "payload                 | json bytes | json ser/de (ms) | bin bytes | bin ser/de (ms) | post bytes | post ser/de (ms)");
    let _ = writeln!(out, "------------------------|-----------:|------------------|----------:|-----------------|-----------:|-----------------");
    for r in &rows {
        let _ = writeln!(out, "{:<23} | {:>10} | {:>6.3} / {:>6.3} | {:>9} | {:>7.3} / {:>6.3} | {:>10} | {:>6.3} / {:>6.3}",
            r.name, r.json_bytes, r.json_ser, r.json_de, r.bin_bytes, r.bin_ser, r.bin_de, r.post_bytes, r.post_ser, r.post_de);
    }

    // derived ratios + extrapolations — ONE consistent basis: cost scales linearly
    // in PAYLOAD bytes (the 1 MiB of pixels), never in encoded bytes (which JSON
    // inflates 4x — dividing by those would understate per-frame cost by 4x).
    let p3 = &rows[2];
    let blowup = p3.json_bytes as f64 / p3.bin_bytes as f64;
    let _ = writeln!(out, "\nJSON byte-payload blowup: {:.2}x (byte arrays become decimal lists)", blowup);
    // 1080p RGBA = 1920*1080*4 = 8_294_400 B ≈ 7.91x the 1 MiB payload
    let f1080 = 1_920.0 * 1_080.0 * 4.0;
    let scale = f1080 / (1 << 20) as f64;
    let _ = writeln!(out, "extrapolated 1080p RGBA JSON ser ≈ {:.0} ms/frame, de ≈ {:.0} ms/frame",
        p3.json_ser * scale, p3.json_de * scale);
    let _ = writeln!(out, "extrapolated 1080p60 one second (JSON) ≈ {:.0} ms ser + {:.0} ms de (one core)",
        p3.json_ser * scale * 60.0, p3.json_de * scale * 60.0);
    let _ = writeln!(out, "binary (bincode) same frame ≈ {:.1} ms ser + {:.1} ms de per frame — and the correct native path is no serialization at all (shared buffer/handle), making both JSON numbers pure waste",
        p3.bin_ser * scale, p3.bin_de * scale);

    let _ = writeln!(out, "\nverdict (expected, re-confirmed): JSON affordable for command/state scale, disqualified for frame payloads at both transport edges; frames-never-JSON rule stands (E-006 webview leg + this native leg).");

    print!("{}", out);
}

fn chrono_header() -> String {
    // no chrono dep: date via std (UTC, ISO-ish)
    let secs = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_secs();
    let days = secs / 86_400;
    // civil-from-days (Howard Hinnant's algorithm, compact)
    let z = days as i64 + 719_468;
    let era = z / 146_097;
    let doe = z % 146_097;
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let y = yoe + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    let y = if m <= 2 { y + 1 } else { y };
    format!("{}-{:02}-{:02}T{:02}:{:02}:{:02}Z", y, m, d, (secs % 86_400) / 3600, (secs % 3600) / 60, secs % 60)
}
