// E-006a — IPC serialization costs, native side (serde_json vs bincode vs postcard).
//
// RE-RUN 2026-09-27. The original 2026-09-23 run's code + raw output were lost
// (CURRENT_STATE_AUDIT D-1); only the record's result table survived. This file
// reconstructs the bench from the record's methodology section and re-measures on
// today's container CPU. Numbers from different hosts are NOT comparable absolutely;
// the finding is the relative order (JSON vs binary) which was stable across runs.
//
// Payloads model the engine's own traffic shapes:
//   1. 1 000-command batch  — ADR-010 verbs: enum + i64 args + short string
//   2. 20 000-clip metadata snapshot — E-002c scale (rational times as num/den pairs)
//   3. 1 MiB RGBA byte payload — per-frame pixel route, linearly extrapolated to
//      1080p (8 294 400 B) and 1080p60 one second (x60).
//
// Method: deterministic xorshift RNG (seeded, no external dep); per measurement:
// 3 warmup + 31 timed iterations, report min and median (ms). Serialized sizes in bytes.

use serde::{Deserialize, Serialize};
use std::fmt::Write as _;
use std::time::{Duration, Instant};

// ---------- payload models ----------

#[derive(Serialize, Deserialize, Clone)]
struct ClipMeta {
    id: u64,
    track: u32,
    start: [i64; 2], // exact rational (num, den) per ADR-007
    duration: [i64; 2],
    asset: u64,
    label: String,
}

#[derive(Serialize, Deserialize, Clone)]
// NOTE: externally-tagged (serde default) is REQUIRED for bincode/postcard:
// internally/adjacently-tagged reprs need self-describing formats and fail with
// "Bincode does not support Deserializer::deserialize_identifier" (found this re-run,
// 2026-09-27 — a real wire-format constraint: binary routes must use variant-index
// encoding, JSON-side tag/content sugar cannot be shared with the binary path).
enum Verb {
    AddClip {
        track: i64,
        pos: [i64; 2],
        dur: [i64; 2],
        asset: String,
    },
    MoveClip {
        id: i64,
        track: i64,
        pos: [i64; 2],
    },
    TrimClip {
        id: i64,
        in_pt: [i64; 2],
        out_pt: [i64; 2],
    },
    SplitClip {
        id: i64,
        at: [i64; 2],
    },
    RemoveClip {
        id: i64,
    },
    SetProperty {
        id: i64,
        key: String,
        val: [i64; 2],
    },
    ReorderTrack {
        from: i64,
        to: i64,
    },
    Batch {
        ids: Vec<i64>,
    },
}

// ---------- deterministic RNG (xorshift64*) ----------

struct Rng(u64);
impl Rng {
    fn new(seed: u64) -> Self {
        Rng(seed | 1)
    }
    fn next(&mut self) -> u64 {
        let mut x = self.0;
        x ^= x >> 12;
        x ^= x << 25;
        x ^= x >> 27;
        self.0 = x;
        x.wrapping_mul(0x2545F4914F6CDD1D)
    }
    fn below(&mut self, n: u64) -> u64 {
        self.next() % n
    }
}

fn command_batch(n: usize, rng: &mut Rng) -> Vec<Verb> {
    (0..n)
        .map(|_| match rng.below(8) {
            0 => Verb::AddClip {
                track: rng.below(8) as i64,
                pos: [rng.below(10_000) as i64, 1_000],
                dur: [rng.below(500_000) as i64, 1_000],
                asset: format!("asset-{}", rng.below(1_000)),
            },
            1 => Verb::MoveClip {
                id: rng.below(20_000) as i64,
                track: rng.below(8) as i64,
                pos: [rng.below(10_000) as i64, 1_000],
            },
            2 => Verb::TrimClip {
                id: rng.below(20_000) as i64,
                in_pt: [rng.below(1_000) as i64, 1_000],
                out_pt: [rng.below(500_000) as i64, 1_000],
            },
            3 => Verb::SplitClip {
                id: rng.below(20_000) as i64,
                at: [rng.below(100_000) as i64, 1_000],
            },
            4 => Verb::RemoveClip {
                id: rng.below(20_000) as i64,
            },
            5 => Verb::SetProperty {
                id: rng.below(20_000) as i64,
                key: format!("prop-{}", rng.below(64)),
                val: [rng.below(100) as i64, 100],
            },
            6 => Verb::ReorderTrack {
                from: rng.below(8) as i64,
                to: rng.below(8) as i64,
            },
            _ => Verb::Batch {
                ids: (0..4).map(|_| rng.below(20_000) as i64).collect(),
            },
        })
        .collect()
}

fn clip_snapshot(n: usize, rng: &mut Rng) -> Vec<ClipMeta> {
    (0..n)
        .map(|i| ClipMeta {
            id: i as u64,
            track: rng.below(8) as u32,
            start: [rng.below(10_000_000) as i64, 1_000],
            duration: [rng.below(500_000) as i64, 1_000],
            asset: rng.below(4_000),
            label: format!("clip-{}", i),
        })
        .collect()
}

// ---------- timing helpers ----------

fn measure_iters(mut f: impl FnMut()) -> Vec<Duration> {
    // 3 warmup
    for _ in 0..3 {
        f();
    }
    let mut out = Vec::with_capacity(31);
    for _ in 0..31 {
        let t = Instant::now();
        f();
        out.push(t.elapsed());
    }
    out
}

fn stats(v: &[Duration]) -> (f64, f64) {
    let mut ms: Vec<f64> = v.iter().map(|d| d.as_secs_f64() * 1e3).collect();
    ms.sort_by(|a, b| a.partial_cmp(b).unwrap());
    let min = ms[0];
    let median = ms[ms.len() / 2];
    (min, median)
}

fn fmt_pair(a: (f64, f64), b: (f64, f64)) -> String {
    // one column per format: "ser_min / de_min" (ms), matching the record's table shape
    format!("{:.3} / {:.3}", a.0, b.0)
}

fn bench_payload<T: Serialize + for<'de> Deserialize<'de> + Clone>(
    label: &str,
    payload: &T,
) -> (String, usize, usize, usize) {
    // JSON
    let json_bytes: Vec<u8> = serde_json::to_vec(payload).expect("json ser");
    let jser = stats(&measure_iters(|| {
        let _ = serde_json::to_vec(payload).unwrap();
    }));
    let jde = stats(&measure_iters(|| {
        let _: T = serde_json::from_slice(&json_bytes).unwrap();
    }));
    // bincode
    let bin_bytes: Vec<u8> = bincode::serialize(payload).expect("bincode ser");
    let bser = stats(&measure_iters(|| {
        let _ = bincode::serialize(payload).unwrap();
    }));
    let bde = stats(&measure_iters(|| {
        let _: T = bincode::deserialize(&bin_bytes).unwrap();
    }));
    // postcard
    let pc_bytes: Vec<u8> = postcard::to_allocvec(payload).expect("postcard ser");
    let _ = measure_iters(|| {
        let _ = postcard::to_allocvec(payload).unwrap();
    });
    let _ = measure_iters(|| {
        let _: T = postcard::from_bytes(&pc_bytes).unwrap();
    });

    let row = format!(
        "| {} | {} | {} | {} | {} | {} |",
        label,
        json_bytes.len(),
        fmt_pair(jser, jde),
        fmt_pair(bser, bde),
        pc_bytes.len(),
        bin_bytes.len()
    );
    (
        row,
        json_bytes.len(),
        (jser.0 * 1e3) as usize, // best json ser in µs, for the raw line
        bin_bytes.len(),
    )
}

fn main() {
    let mut out = String::new();
    let _ = writeln!(out, "E-006a re-run (2026-09-27) — native-side IPC serialization");
    let _ = writeln!(out, "timing: 3 warmup + 31 iters, min/median, ms; sizes: bytes");
    let _ = writeln!(out);
    let _ = writeln!(out, "| payload | json bytes | json ser/de (ms) | bincode ser/de (ms) | postcard bytes | bincode bytes |",
    );
    let _ = writeln!(out, "|---|---|---|---|---|---|");

    // 1. 1 000-command batch
    let mut rng = Rng::new(0xE006A_1);
    let batch = command_batch(1_000, &mut rng);
    let (row, jbytes, _, bbytes) = bench_payload("1 000-command batch", &batch);
    let _ = writeln!(out, "{}", row);
    let _ = writeln!(
        out,
        "  json/bincode size ratio: {:.2}x",
        jbytes as f64 / bbytes as f64
    );

    // 2. 20 000-clip snapshot
    let mut rng = Rng::new(0xE006A_2);
    let snaps = clip_snapshot(20_000, &mut rng);
    let (row2, jbytes2, _, bbytes2) = bench_payload("20 000-clip snapshot", &snaps);
    let _ = writeln!(out, "{}", row2);
    let _ = writeln!(
        out,
        "  json/bincode size ratio: {:.2}x",
        jbytes2 as f64 / bbytes2 as f64
    );

    // 3. 1 MiB RGBA byte payload
    let mut rng = Rng::new(0xE006A_3);
    let pixels: Vec<u8> = (0..(1024 * 1024)).map(|_| rng.below(256) as u8).collect();
    let (row3, jbytes3, jser_us, bbytes3) = bench_payload("1 MiB RGBA", &pixels);
    let _ = writeln!(out, "{}", row3);
    let blowup = jbytes3 as f64 / (1024.0 * 1024.0);
    let _ = writeln!(
        out,
        "  json blowup on byte payload: {:.2}x; json/bincode ratio: {:.2}x",
        blowup,
        jbytes3 as f64 / bbytes3 as f64
    );

    // Extrapolation (linear in bytes, sound for serde_json decimal-list encoding):
    // 1080p RGBA frame = 1920*1080*4 = 8 294 400 B = 7.9084 MiB; 60 fps one second = x60.
    let frame_mult = 8_294_400f64 / (1024.0 * 1024.0);
    let jser_stats = stats(&measure_iters(|| {
        let _ = serde_json::to_vec(&pixels).unwrap();
    }));
    let json_bytes_for_de: Vec<u8> = serde_json::to_vec(&pixels).unwrap();
    let jde_stats = stats(&measure_iters(|| {
        let _: Vec<u8> = serde_json::from_slice(&json_bytes_for_de).unwrap();
    }));
    let frame_ser_ms = jser_stats.0 * frame_mult;
    let frame_de_ms = jde_stats.0 * frame_mult;
    let _ = writeln!(out);
    let _ = writeln!(
        out,
        "extrapolation (linear in bytes from 1 MiB): 1080p RGBA frame ≈ {:.0} ms json ser (de {:.0} ms)",
        frame_ser_ms, frame_de_ms
    );
    let _ = writeln!(
        out,
        "  1080p60 one second ≈ {:.0} ms ser + {:.0} ms de (one core) — vs memcpy-scale on the binary path",
        frame_ser_ms * 60.0,
        frame_de_ms * 60.0
    );
    let _ = writeln!(
        out,
        "  (original-run cross-check: json ser 1 MiB was 10.67 ms; bincode path is memcpy-scale)"
    );
    let _ = writeln!(
        out,
        "  raw json ser 1 MiB this run: {:.3} ms",
        jser_us as f64 / 1e3
    );

    print!("{}", out);
}
