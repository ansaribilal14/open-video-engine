# E-006a — IPC serialization costs, native side (serde_json vs binary)

> Status: RUN (2026-09-23); **RE-RUN with committed artifacts 2026-09-26** (audit D-1:
> the original code and raw output were lost; the re-run restores the evidence chain,
> improves the method to median-of-30, and FIXES the original's extrapolation basis —
> see §RESULT-2). Companion to E-006 (webview leg): E-006 measured the
> webview side and named native-side JSON handling as unmeasured — this record fills
> exactly that hole. Real-Tauri round-trip remains E-006b (blocked).

- **QUESTION**: what does the NATIVE half of the IPC path cost — serde_json (what
  Tauri `invoke` does on the Rust side) vs binary formats (bincode/postcard) that
  channel/raw-payload paths resemble — for engine traffic shapes?
- **HYPOTHESIS**: JSON is acceptable for command-scale (KB) and full-state-sync scale
  (MB, ≤1 Hz), and disqualified for frame-scale pixel payloads at 60 fps.
- **IMPLEMENTATION**: crate `scripts/experiments/E-006a_ipc_crate/` (committed
  2026-09-26; the original loose .rs path cited by the index was the artifact-chain
  defect). serde 1 / serde_json 1 / bincode 1 / postcard 1 (alloc feature — 1.1
  default change). Three payloads from the engine's own data model:
  1. 1 000-command batch (ADR-010 verbs: enum + i64 args + short string),
  2. 20 000-clip metadata snapshot (E-002c scale),
  3. 1 MiB RGBA byte payload, with linear extrapolation to 1080p / 1080p60-second.
- **HARDWARE**: container CPU (2 cores); re-run method = median of 30 iterations
  (original was single run), release build. Relative orders are the finding.
- **RESULT-1** (original 2026-09-23 run, output lost — superseded by RESULT-2):

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
- **RESULT-2** (re-run 2026-09-26, `experiments/E-006a_result.txt`, artifacts committed):

  | payload | json bytes | json ser/de ms | bin bytes | bin ser/de ms | post bytes |
  |---|---|---|---|---|---|
  | 1 000-command batch | 74 021 | 0.145 / 0.146 | 39 386 | 0.021 / 0.017 | 13 847 |
  | 20 000-clip snapshot | 2 195 717 | 4.136 / 5.144 | 1 517 938 | 0.886 / 0.930 | 580 363 |
  | 1 MiB RGBA | 4 194 305 | 5.592 / 10.481 | 1 048 584 | 2.356 / 2.351 | 1 048 579 |

  - JSON byte-payload blowup **4.00×**; extrapolated on payload bytes (consistent
    basis): 1080p RGBA JSON ≈ **44 ms ser + 83 ms de per frame** → 1080p60 ≈ 2.65
    CPU-s/s + 4.97 CPU-s/s — catastrophically disqualified. Binary ≈ 18.6 ms/frame,
    and the correct native path is NO serialization (shared buffer/handle).
  - **Honest correction of RESULT-1**: the original "1080p60 ≈ 640 ms ser + 1 005 ms
    de" line divided by ENCODED JSON bytes (already 3.57× inflated), understating by
    the same factor, and was internally inconsistent with its own 84 ms/frame figure.
    RESULT-2 uses one basis (payload bytes) throughout. The DECISION is unchanged —
    in fact strengthened.
- **LIMITATIONS**: serialization layer only — transport/kernel hops add cost, never
  subtract, so the frame-payload verdict is conservative. Extrapolation assumes
  linearity in payload bytes (sound for serde_json byte-array escaping). Container
  CPU, absolute values not portable; relative orders are the finding.
  Tauri-version-specific channel internals unmeasured (E-006b).
- **DECISION**: adopts the E-006 rule on the native side with numbers: commands and
  state snapshots MAY use JSON invoke (0.15 ms/1 000 commands; 4.1 ms/20k-clip full
  state at 1 Hz is affordable); frame/pixel payloads MUST use the binary path
  (channel / custom protocol / shared buffer) — JSON is never allowed on a per-frame
  route from either side. Feeds ADR-002 (layered hybrid) and the desktop-shell design.
  Evidence chain CLOSED 2026-09-26 (code + raw output in git; index row corrected).
