# E-006 — IPC transport costs for the desktop shell (browser leg)

> Status: PARTIAL-RUN (browser-leg complete and decision-grade; real-Tauri leg BLOCKED).

- **QUESTION**: Tauri v2 offers three IPC transports — `invoke()` (JSON), `Channel<T>`
  (streamed serialization), and custom-protocol responses (raw bytes). Doc 17
  established the rule "no frame payloads via invoke JSON" from experience; this
  experiment quantifies where the JSON tax actually starts for engine traffic.
- **HYPOTHESIS**: JSON invoke is fine for small commands but crosses the editor frame
  budget somewhere between 100 KB and 1 MB payloads; a binary path stays ~2 orders of
  magnitude cheaper.
- **IMPLEMENTATION**: `scripts/experiments/E-006_ipc_browser_leg.cjs` — headless
  Chromium (chrome-headless-shell 1243 / Chrome 154-class webview engine, same family
  as E-001) measuring webview-side costs: JSON stringify+parse roundtrip,
  structuredClone, TextEncoder(JSON) bytes, JSON parse-only, typed-array copy, and
  MessageChannel postMessage round-trip (task hop included). Payload tiers model
  engine traffic: S≈128 B (single command), M≈4.7 KB (command batch), L≈161 KB
  (snapshot delta), XL≈1.4 MB (asset/thumbnail manifest).
- **HARDWARE**: container CPU (2 cores). µs/op, warmup + 50–2000 iters.
- **RESULT** (`experiments/E-006_result.txt`, raw JSON `E-006_result.json`):

  | payload | JSON RT | structuredClone | binary copy | MsgChannel RT |
  |---|---|---|---|---|
  | 128 B    | 1.0 µs | 7.9 µs | **0.05 µs** | 15.8 µs |
  | 4.7 KB   | 23.6 µs | 67.0 µs | **0.05 µs** | 27.2 µs |
  | 161 KB   | 752 µs | 2074 µs | **3.5 µs** | 720 µs |
  | 1.37 MB  | 6.95 ms | 18.6 ms | **78 µs** | 7.24 ms |

  - Frame-budget reading (60 fps = 16.7 ms; scrub-burst target ~1 ms): JSON commands
    are FREE at command scale (1–24 µs); a 1.4 MB JSON payload burns **42% of a frame**
    while the binary path burns **0.5%** (~89× gap at XL, ~214× at L).
  - structuredClone is SLOWER than JSON for object-heavy payloads (2.6× at XL) —
    "clone instead of serialize" is not a free win for deep object trees.
  - MessageChannel adds a ~15 µs fixed task-hop floor; overhead is payload-dominated
    beyond ~256 KB.
- **LIMITATIONS**: measures webview-side costs only; real Tauri adds native-side JSON
  handling and webkit2gtk task IPC (not runnable in this container: no webkit2gtk/
  display) — numbers are lower bounds and RELATIVE comparators, absolute Tauri
  overhead unmeasured (E-006b, blocked). MessageChannel approximates but is not the
  Tauri channel implementation. The native-side serialization leg has since been
  measured in **E-006a** (serde_json/bincode/postcard): both edges pay ~5–27 ms/MB
  for JSON — the rule holds independently at each end of the transport.
- **DECISION**: Doc-17's rule is now numerically grounded: **invoke()-style JSON for
  commands ≤ ~10 KB; binary custom-protocol/typed-array path for anything ≥ ~100 KB
  (snapshots, thumbnails, manifest, any frame data)**; never structuredClone deep
  object trees as a "cheaper" alternative. The Channel/binary split is a hard
  architecture constraint for the desktop shell (feeds ADR-002 platform adapter and
  doc 18 web architecture).
