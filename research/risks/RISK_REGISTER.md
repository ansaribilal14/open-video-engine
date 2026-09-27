# RISK REGISTER (v0.1)

Risk = ID · description · category · likelihood(I)×impact(I) · mitigation · status.

| ID | RISK | CAT | L×I | MITIGATION | STATUS |
|---|---|---|---|---|---|
| R-01 | Adapter-trait leakage: MediaCodec EOS/reuse + hw-frame import semantics leak platform types into core trait design | arch | H×H | E-004 experiment first; traits modeled after documented failure modes (docs 04,16); core never includes platform headers | OPEN |
| R-02 | Browser capability holes: Firefox-Android (no WebCodecs/WebGPU), Safari audio <26, WebGL2 fallback quality | plat-web | H×M | Capability probing + explicit degraded modes (doc 18); WebGL2 pass-graph subset maintained | OPEN |
| R-03 | iOS memory jetsam (~2048MB tab kill) kills long edits in browser | plat-web | M×H | Snapshot project (C-007), sub-GB frame pools, proxy-first on mobile web (doc 11) | OPEN |
| R-04 | FFmpeg API churn (yearly majors; avcodec 62) breaks bindings | media | H×M | Pin + isolate libav* behind thin ABI boundary; CI against LTS+current (doc 05) | OPEN |
| R-05 | GPL/nonfree contamination in distributed builds | legal | M×H | Default LGPL-only build profile; GPL is optional variant; license audit gate GATE-13 (doc 05) | OPEN |
| R-06 | Command-log replay non-determinism (project format) | arch | M×H | E-003 benchmark + property tests; fallback snapshot-only design kept alive (doc 19) | OPEN |
| R-07 | Android export killed by FGS 6h budget / process death | plat-android | M×H | Checkpointed resumable export design from day one (doc 14) | OPEN |
| R-08 | Tauri IPC JSON bottleneck for frame traffic | plat-desktop | M×M | Hard rule: frames via channels/custom-protocol only; E-006 benchmark (doc 17) | OPEN |
| R-09 | AI agent destructive actions / prompt injection via media metadata | security | M×H | Deny-by-default tiers, transactions+undo, receipts, human approval gates (docs 26,27) | OPEN |
| R-10 | Scope explosion (23 phases, 36 ADRs, 25 deliverables) exceeds any single-session capacity | program | H×M | Phase 0 strict gating; vertical slice discipline (directive); STATUS.md honest tracking | MITIGATED (ongoing) |
| R-11 | Named prior-art repos (Kerf, Velocut, OpenTake, Frontstage) don't exist → directive assumptions partly unverifiable | research | Confirmed×Low | Recorded NOT FOUND with queries (doc 39); proceed on verified corpus | CLOSED (verified absent) |
| R-12 | Single-maintainer sustainability across Rust+Kotlin+TS surfaces | program | M×M | Codegen from one core (UniFFI/WASM), minimal shell code, plugin/script surface for community | OPEN |
| R-13 | Hostile-media parser exploit reaching the core process; proxy-relink cache inconsistency (wrong media after relink) | security, media | M×H | Parse untrusted media only in sidecar/worker/wasm boundary (doc 33 §2, FFmpeg 513-CVE history); relink by content hash never path-only (doc 32; generalizes E-007) | OPEN |
| R-14 | Performance regressions across the platform matrix silently eroding editor-grade budgets | perf | M×M | Ratio-based perf-CI gates on committed baselines (E-002c 140ns/rational-add, E-006 IPC, E-007b copy speed); tracing + Chrome Trace Event Format (doc 32 §6) | OPEN |
