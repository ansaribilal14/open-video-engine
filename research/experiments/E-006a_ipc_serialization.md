# E-006a — IPC serialization costs, native side (serde_json vs binary)

> Status: RUN (2026-09-23). Companion to E-006 (webview leg): E-006 measured the
> webview side and named native-side JSON handling as unmeasured — this record fills
> exactly that hole. Real-Tauri round-trip remains E-006b (blocked).

- **QUESTION**: what does the NATIVE half of the IPC path cost — serde_json (what
  Tauri `invoke` does on the Rust side) vs binary formats (bincode/postcard) that
  channel/raw-payload paths resemble — for engine traffic shapes?
- **HYPOTHESIS**: JSON is acceptable for command-scale (KB) and full-state-sync scale
  (MB, ≤1 Hz), and disqualified for frame-scale pixel payloads at 60 fps.
- **IMPLEMENTATION**: `scripts/experiments/E-006a_ipc_serialization.rs` (crate
  `E-006a_ipc_crate/`, rustc via cargo, `--release`). serde 1 / serde_json 1 /
  bincode 1 / postcard 1. Three payloads from the engine's own data model:
  1. 1 000-command batch (ADR-010 verbs: enum + i64 args + short string),
  2. 20 000-clip metadata snapshot (E-002c scale),
  3. 1 MiB RGBA byte payload, with linear extrapolation to 1080p / 1080p60-second.
- **HARDWARE**: container CPU (2 cores), single run, relative orders are the finding.
- **RESULT** (`experiments/E-006a_result.txt`):

  | payload | json bytes | json ser/de | bincode ser/de | postcard bytes |
  |---|---|---|---|---|
  | 1 000-command batch | 91 850 B | 0.33 / 0.29 ms | 0.034 / 0.047 ms | 22 314 B |
  | 20 000-clip snapshot | 2 497 329 B | 5.38 / 5.76 ms | 1.02 / 2.19 ms | — |
  | 1 MiB RGBA | 3 743 329 B | 10.67 / 16.76 ms | 2.39 / 2.31 ms | ≈1.0 MiB |

  - JSON blowup on byte payloads: **3.57×** (byte arrays become decimal lists).
  - Extrapolated (linear in bytes): **1080p RGBA frame ≈ 84 ms JSON serialize**;
    **1080p60 one second ≈ 640 ms ser + 1 005 ms de** on one core — vs ~memcpy
    (µs–low ms) on the binary path.
  - Cross-check vs E-006 (webview side): webview JSON 1.4 MB round-trip = 6.95 ms,
    native serde_json 1 MiB ser+de ≈ 27 ms — both edges pay ~5–27 ms/MB; the
    "JSON is fine for commands, banned for frames" rule holds at BOTH ends of the
    transport, independently measured.
- **LIMITATIONS**: serialization layer only — transport/kernel hops add cost, never
  subtract, so the frame-payload verdict is conservative. Extrapolation assumes
  linearity (sound for serde_json byte-array escaping). Tauri-version-specific
  channel internals unmeasured (E-006b).
- **DECISION**: adopts the E-006 rule on the native side with numbers: commands and
  state snapshots MAY use JSON invoke (0.33 ms/1 000 commands; 5.4 ms/20k-clip full
  state at 1 Hz is affordable); frame/pixel payloads MUST use the binary path
  (channel / custom protocol / shared buffer) — JSON is never allowed on a per-frame
  route from either side. Feeds ADR-002 (layered hybrid) and the desktop-shell design.
