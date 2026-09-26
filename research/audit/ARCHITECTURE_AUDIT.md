# ARCHITECTURE AUDIT — reconciliation + falsification attack (2026-09-26)

> Mandate: do NOT assume the current architecture (C: shared Rust core + platform media
> adapters + shared timeline/command API/render graph + platform GPU/media execution) is
> final. Attack it. Where it survives, record why; where it breaks, redesign before the
> vertical slice freezes it.

## 1. What the architecture actually is today (evidence state)

- ADR-002 PROPOSED: layered hybrid (option C of 40_ARCHITECTURE_COMPARISON). Provisional,
  blockers = GATE-4/5/7 residuals (E-005 real GPU, E-004c device, E-006b real Tauri).
- Accepted layers: ADR-001 (Rust core + Kotlin/TS shells), ADR-007 (exact rationals,
  fixed tick axis for aggregates), ADR-010 (one command API for human+AI).
- Proposed layers: ADR-003 (media backend adapters), ADR-004 (Decoder trait),
  ADR-005 (Encoder trait + StreamCopy), ADR-006 (hybrid timeline representation),
  ADR-008 (manifest + command log), ADR-009 (inverse commands), ADR-011 (gap buffer +
  derived index).
- **Nothing above the ove-time crate exists in code.** The architecture is therefore a
  hypothesis with unusually good evidence *at the edges* (time, commands, IPC, codegen)
  and zero code in the middle (decode→frame→timeline→render→encode).

## 2. Attack record (10 mandated abstractions)

| # | Target | Strongest attack | Does it survive? | Resolution carried into the slice |
|---|---|---|---|---|
| 1 | **Decoder trait** (ADR-004) | (a) async callback (WebCodecs) vs pull (FFmpeg) vs stagefright-buffer (MediaCodec) models may not unify behind one trait without allocation churn; (b) seek/flush semantics differ per backend (keyframe-snap vs exact) — one trait invites lying; (c) zero-copy paths (importExternalTexture, AHardwareBuffer) are type-incompatible — a trait returning "frames" must either erase to a common type (copy) or expose backend-specific payloads (leak) | **Survives with amendments** | trait = state machine with explicit session: open(config)→probe caps→seek(Exact|Snap)→deliver(FrameEnvelope)→flush→close; FrameEnvelope carries backend id + memory location (see FRAME_CONTRACT); v0 ships copy-path everywhere (already ADR-004 risk note); seek-accuracy differences become *declared capabilities*, not hidden behaviour; conformance suite (DECODER_SPEC) is the anti-lying device |
| 2 | **Encoder trait** (ADR-005) | stream-copy "mode" inside an Encoder trait is a category error — remux is not encode; forcing it into one trait distorts both; cross-encoder stitch A/V sync unproven (recorded risk) | **Survives split** | split muxing from encoding: `Muxer` + `Encoder` + `StreamCopyPath` as a *route* chosen by the export planner (RENDER_GRAPH_SPEC); trait keeps configure/feed/drain only |
| 3 | **Frame abstraction** | today undefined. Attack: a frame is NOT bytes — it is (memory location, pixel format, color metadata, timestamps, ownership). A CPU-bytes-only frame type would later force a breaking rewrite; a GPU-texture-only type breaks software legs | **Resolved by new spec** | FRAME_CONTRACT.md: FrameEnvelope with (Pts, Dur, WxH, PixelFormat, ColorXform{primaries,transfer,matrix,range}, BitDepth, MemoryLoc CPU/GPU, Ownership rules, KeyframeFlag, BackendId). This is the single most consequential new decision of this audit |
| 4 | **GPU resource abstraction** | wgpu is not on Android mainline GPU surface path (SurfaceTexture/AHardwareBuffer), and browser importExternalTexture is single-use-per-pass — a naive "GPUFrame = wgpu Texture" abstraction breaks 2 of 4 legs | **Survives as two-level** | engine core speaks *import/copy intents*; per-leg GPU exec adapters own real resources; wgpu is the desktop/headless renderer, not the universal GPU type (matches C-002 MEDIUM) |
| 5 | **Browser boundary** | WASM core + WebCodecs + WebGPU + OPFS + Workers assumed; attack: Firefox-Android no WebCodecs, Safari audio<26, iOS jetsam ~2GB (R-02/R-03) — "universal browser support" claim would be false | **Survives with capability-gate rule** | runtime feature detection is MANDATORY and user-visible (degraded modes, never silent); cross-platform matrix in CROSS_PLATFORM_CONFORMANCE_PLAN |
| 6 | **Android boundary** | UniFFI proven for domain data (E-004b) but MediaCodec Surface→GPU preview + export on-device unproven; FGS 6h budget forces checkpointed export design | **Survives; proof scheduled** | E-004c + MediaCodec adapter are slice-adjacent work; checkpoint/resume enters ENCODER_SPEC from day one |
| 7 | **Tauri boundary** | E-006 browser leg + E-006a (numbers, though artifacts lost) prove JSON is fine for commands, catastrophic for frames; transport leg (webkit2gtk) never measured | **Survives; frames-never-JSON rule stands on both edges** | E-006b remains the only missing transport number; not slice-blocking |
| 8 | **Render graph** | attack: a "graph" may be over-engineering for v1 slice (1 video + 1 audio); under-engineering risk for compositing later | **Survives as compile-to-passes** | RENDER_GRAPH_SPEC: minimal deterministic compile (timeline state → ordered pass list) with golden-frame tests; no node-editor UI, no dynamic recompile in v1 |
| 9 | **Project format** | event-sourced log replay determinism across engine versions is the hard invariant (C-007 counter-evidence: no NLE persists its log); schema drift could brick projects | **Survives with versioned-snapshot rule** | PROJECT_FORMAT_SPEC: manifest(schema_version, tick axis, asset registry by content hash) + commands.jsonl + snapshot compaction; save/kill/reopen/replay/hash-equality test is the acceptance gate |
| 10 | **Plugin boundary** | 3-tier proposal (doc 28) is unvalidated; attack: plugins before a working engine = scope explosion (R-10) | **Deferred honestly** | no plugin surface in the slice; ADR-025 stays PLANNED; command API + scripting = first "plugin-ish" surface (already ACCEPTED ADR-010) |

## 3. Falsification conditions that would flip architecture C (kept live)

1. Decoder/Encoder trait leakage during MediaCodec/WebCodecs adapter implementation
   (R-01) → if platform types can't stay behind adapters, fall back toward B (per-leg
   media engines + shared spec).
2. E-005 real-GPU: if importExternalTexture + multi-layer compositing forces a copy per
   layer at 1080p60 on real GPUs, browser GPU plan changes (Q-02 resolution).
3. E-006b: if Tauri channel transport cannot carry frames near budget, desktop shell
   design changes (frames stay native-side).
4. Project replay determinism across engine versions (test: replay old-log on new engine
   = hash-equal) — if unsolvable, snapshot-only format becomes primary (fallback kept
   alive by C-007 counter-evidence).

## 4. Crate-boundary derivation (from evidence, not aesthetics)

Mandate: "don't create crates because the names look good." Derived rules:

- A crate exists only if it has (a) a distinct acceptance-testable contract, (b) a
  dependency direction that never cycles, (c) a reason to evolve independently.
- From the attack table: time (exists), timeline model + commands + project fold (one
  crate — they share one state machine), media probe/decode (one), encode/mux (one),
  compositor/render (one), plus a thin `ove-engine` façade + `ove-cli` binary.

**Accepted crate plan for the slice** (dependency-acyclic):

```
ove-time (exists) ← ove-timeline ← ove-project ← ove-engine ← ove-cli
                      ↑               ↑
   ove-media (probe/assets) ← ove-decode   ove-render (graph+compositor)
                                  ↓
                             ove-encode (encoder+muxer+copy path)
```

- ove-timeline: gap-buffer + derived index + command verbs + inverses (ADR-006/009/011).
- ove-project: manifest + log + snapshot + asset registry (ADR-008) — folds commands.
- ove-media: MEDIA FILE→PROBE→ASSET (metadata, keyframe index, color tags) — no decoding.
- ove-decode: Decoder trait + FFmpeg-software adapter first (DECODER_SPEC).
- ove-encode: Encoder/Muxer/StreamCopy (ENCODER_SPEC).
- ove-render: pass-graph compile + software raster first; wgpu compositor upgrade (E-005
  promotion) second — correctness and performance separated per directive.
- Rejected for now: ove-cache, ove-color, ove-audio, ove-analysis as separate crates —
  they are *modules* inside the above until size forces separation (anti-overbuild).

## 5. Verdict

Architecture C **survives the attack with three amendments**: (1) explicit frame contract
carrying color/memory/ownership from day one, (2) muxer/encoder separation with export
routing, (3) capability-gated web story. ADR-002 remains PROPOSED until the three
hardware-bound experiments land — but the slice may proceed under C without accepting it,
because the slice itself is the next falsification instrument.
