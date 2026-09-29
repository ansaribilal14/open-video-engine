# MASTER ENGINE STATE — single authoritative work order

> RULE (takeover directive §6): this is the ONE current engineering truth.
> Other status documents are historical/evidence records and stay untouched.
> Update this file at the end of every major wave using the §40 report format.

WAVE: 9 COMPLETE (0 / 0.5 / 0.6 / 1 seam / 2 render / 3 encode / 4 project / 5 engine+cli / 6 vertical slice / 7 audio / 8 keyframes / 9 GPU — all landed)
DATE: 2026-09-29
COMMIT: W3 = PR #5 (d0f1ce0); W4 = PR #6 (37d1e59a); W5 = PR #7 (ae7c1d9); W6 = PR #8; W7 = PR #9 (b5f322f); W8 = PR #10 (00a46b9); W9 = PR #11

## Session verification record (2026-09-29, fresh continuation #5)

All W0–W8 claims re-verified from scratch before Wave 9 work:

- HEAD `160c898` on main; takeover baseline `33fde9d` IS an ancestor
  (38 commits since); tree clean; wave-8-keyframes fully merged.
- GitHub Actions check-runs on exact HEAD `160c898`: all 3 jobs SUCCESS
  (cargo-audit, fmt+clippy+tests libav-linked, libav-confinement).
- Local (rebuilt container: rustup 1.98.1, system FFmpeg 7.1.5 dev libs via
  DEV_ENV.md unprivileged recipe): 157/157 tests GREEN · fmt GREEN · clippy
  (workspace, all targets) GREEN. W8 claim confirmed exactly.
- Post-merge main (3be1a6c, PR #11): all 4 CI jobs SUCCESS — the new
  gpu-conformance job ran the parity suite for real on a runner (lavapipe).

## WAVE 9 deltas (2026-09-29, ADR-020)

1. **GPU reference executor exists and is BYTE-PARITY-Proven**
   (ove-render::gpu, feature `gpu`, wgpu 25 + pollster optional):
   `GpuRenderer::execute_frame` consumes the SAME RenderPlan and replicates
   exec.rs arithmetic exactly — Rgba8Uint textures (no float normalization),
   u32 blend pipeline (`d = 255·den`, `n_s = min(sa·a_num, d)`, `inv = d−n_s`,
   `out = (cs·n_s + cd·inv + d/2)/d`, alpha forced 255), integer translate
   with bounds clip, ColorConvert as tag stamp, coordinates from
   `@builtin(position)` (E-005 rule). Surface placement model: fetch surfaces
   are native-size (native bounds-check ≡ software's transparent-padded
   paste-at-origin); transform/blend targets are output-sized with content
   already placed — later passes sample directly (the first implementation
   double-counted the translation; the parity suite caught it at pixel 19).
2. **Typed bound + fallback**: u32 arithmetic bounds the blend to reduced
   den ≤ 65 000 (MAX_ALPHA_DEN; worst case 255·d + d/2 = 65 152.5·den < 2³²);
   exceeding it is `GpuError::UnsupportedAlphaDen` BEFORE any GPU work —
   software executes it instead (G-7 pins the fallback). `GpuRenderer::new()`
   → typed `NoAdapter` when feature-detect fails (verified in this container:
   no ICD → typed error; lavapipe ICD → adapter). GPU pass errors REUSE the
   software error set (G-6: identical TagMismatch from both executors).
3. **Parity suite G-1..G-8** (ove-render/tests/gpu_conformance.rs, tolerance
   0 vs software on every test): single layer; transforms incl. negative
   offsets + edge clipping; fractional alphas (1/3, 2/3, 1/2, 127/128,
   1/1000); three-layer stack with declared convert; retime frame selection
   (RG-4 via GPU); error parity; den-bound typed error + software fallback;
   span-level blake3 hash equality. **8/8 GREEN on Mesa lavapipe** (rootless,
   E-005 recipe) in this container.
4. **CI grows a dedicated `gpu-conformance` job** (ubuntu-latest + sudo
   apt mesa-vulkan-drivers): clippy (gpu feature) + the parity suite. The
   main pipeline is untouched (`--features ove-decode/bundled` never enables
   gpu) — zero risk to its cache/timing; the GPU leg gets its own signal.
5. **Perf honesty**: `examples/gpu_perf_probe.rs` stamps
   every line with the adapter class; committed record
   research/experiments/W9_gpu_perf_record.txt carries the llvmpipe numbers
   explicitly marked INVALID for any perf claim; real-hardware ratio gates
   (doc 45 T-7) stay a hardware-bound residual.
6. **Engine wiring deferred to platform waves** (recorded in ADR-020 §8):
   backend selection per platform rides waves 10–12 behind the same typed
   contract; wave 6's gate was parity + bench + fallback, all delivered.
7. Test IDs: gpu_conformance G-1..G-8 (8 tests; workspace count 157→165
   with the gpu feature enabled, 157 default unchanged). Files: gpu.rs
   (~700 l), gpu_conformance.rs (~530 l), 2 examples, ADR-020, CI job.

## Session verification record (2026-09-29, fresh continuation #4)

All W0–W7 claims re-verified from scratch before Wave 8 work:

- HEAD `b5f322f` on main; takeover baseline `33fde9d` IS an ancestor
  (35 commits since); tree clean; wave-7-audio fully merged (zero
  unmerged commits).
- GitHub Actions run 36496623143 on exact HEAD `b5f322f`: SUCCESS — W7
  post-merge main GREEN.
- Local (system FFmpeg 7.1.5 path, unprivileged recipe per DEV_ENV.md,
  rustc 1.98.1): 141/141 tests GREEN · fmt GREEN · clippy (workspace,
  all targets) GREEN · libav-confinement check PASS.

## WAVE 8 deltas (2026-09-29, ADR-019)

1. **Keyframes live on the Clip** (ove-timeline::property): ClipProperties
   {opacity, x, y} with LOCAL clip times, strictly-increasing keys,
   loud validation (typed PropertyError, never silently sorted). Move
   carries animation untouched; Resize strands keys inert (evaluation
   domain [0, dur) — growing back re-activates; duration-only inverse
   stays exact); **Split slices with the computed boundary value
   inserted on BOTH halves** — the split-preserve rule (S8, 300 seeded
   cases, exact evaluation continuity across the seam, Hold/Linear
   semantics survive the cut).
2. **Command::SetKeyframes** — wholesale replace of one property's keys;
   the exact inverse is the previous key list under the same command
   shape; undo/redo/replay need zero new machinery. Opacity values
   validated ∈ [0,1] at command time. Split's inverse batch embeds
   SetKeyframes restores (S9: hash-exact undo roundtrip).
3. **Exact evaluation**: per-key out-interp Linear (exact rational lerp,
   i128-checked) | Hold; boundary rules exact (on-key identity,
   hold-before/after). S7 pins evaluation against a naive oracle.
4. **Engine evaluation into the render path**: build_render_input(out, t)
   evaluates per frame; geometry via Rational::round_half_up (ove-time
   P13 — which PROVES the conversion is total over representable
   rationals); i32 overflow is a typed KeyframeValueOutOfRange, never
   saturation. W8 integration test: plan carries evaluated alpha/dx
   verbatim, undo/redo hash-exact, animated state survives reopen,
   renders byte-identical.
5. **Persistence**: StateMirror.properties (serde-default — old
   snapshots load unchanged, keys as exact pairs); log grammar +1
   payload (set_keyframes), embedded inverse needs no extra grammar.
   P-9: save/reopen hash equality through SetKeyframes AND a split
   inverse batch.
6. Test IDs: ove-time P13 (round_half_up exactness + totality proof);
   ove-timeline keyframes.rs (unit + S7 + S8 + S9, 11 tests);
   ove-project P-9; ove-engine w8_keyframes_exact_eval_and_stability.
   Workspace **157/157** (was 141).

## Session verification record (2026-09-28, independent takeover continuation)

Prior session claims were re-verified from scratch before any new work:

- HEAD `8b5aa81` on main; takeover baseline `33fde9d` IS an ancestor; tree clean.
- GitHub Actions: post-merge main run 36304812153 GREEN (and PR #1 runs
  36303605420 / 36304226310 GREEN) — W0.5 SIGILL fix confirmed externally.
- Local (system FFmpeg 7.1.5 path, unprivileged recipe per DEV_ENV.md):
  60/60 tests GREEN · fmt GREEN · clippy -D warnings GREEN (pre-W0.6 tree).
- Sources re-read: ove-time (297 l), ove-timeline (2303 l), ove-media (1332 l),
  ove-decode (145 l + conformance/probe suites). Spec set re-read: PROJECT_
  FORMAT / ENCODER / DECODER / FRAME_CONTRACT / MEDIA_ENGINE.

## Session verification record (2026-09-29, fresh continuation #2)

All W0.6/W1/W2 claims re-verified from scratch before Wave 3 work:

- HEAD `16d62a9` on main; baseline `33fde9d` IS an ancestor; tree clean.
- GitHub Actions run 36395951181 on exact HEAD `16d62a9`: SUCCESS (3 jobs:
  rust / audit / libav-confinement). All six recent runs SUCCESS.
- Local (system FFmpeg 7.1.5, unprivileged recipe per DEV_ENV.md, rustc
  1.98.1): 84/84 tests GREEN · fmt GREEN · clippy -D warnings GREEN.
- Source inventory re-counted: 4,223 l (ove-time 378, ove-timeline 1,611,
  ove-media 1,332, ove-decode 145+suites, ove-render 643).

## Session verification record (2026-09-29, fresh continuation #3)

All W0–W6 claims re-verified from scratch before Wave 7 work:

- HEAD `451e67e` on main; takeover baseline `33fde9d` IS an ancestor
  (30 commits since); tree clean; fix/ci-bundled-ffmpeg-sigill branch fully
  merged (zero unmerged commits).
- GitHub Actions run 36472738518 on exact HEAD `451e67e`: SUCCESS (push,
  main, all 3 jobs). Runs 24–31 all SUCCESS; last PR run (wave-6 branch)
  SUCCESS.
- Local (system FFmpeg 7.1.5 path, unprivileged recipe per DEV_ENV.md,
  rustc 1.98.1): 132/132 tests GREEN · fmt GREEN · clippy (workspace,
  all targets) GREEN · libav-confinement check PASS.
- W6 milestone test confirmed present and passing
  (ove-engine/tests/integration.rs::w6_vertical_slice_milestone).
- Bundled-feature path + cargo-audit verified via CI on the exact SHA
  (run 36472738518) rather than a local bundled rebuild; confidence 0.85
  for that split, 1.0 for everything above.
- Doc drift found & fixed in this commit: open-gaps list still claimed
  VERTICAL_SLICE_TRACE.md "not yet written" after W6 landed it (stale
  gap #6 removed).

## WAVE 7 deltas (2026-09-29, ADR-018)

1. **Audio decode leg** (ove-decode): audio streams decode through the
   same session machine — canonical planar-f32 ("fltp") surface at the
   SOURCE rate/layout (swresample format conversion ONLY; a resample is
   structurally impossible in the adapter), per-frame duration =
   nb_samples/rate exact, audio floor+trim seek policy (the straddling
   frame is delivered whole; the caller trims the exact sample in-point),
   audio frames unpooled with `reclaim()` typed-rejected. A-1..A-4 pin
   the corpus facts (priming trimmed, tail padded to the AAC grid,
   contiguity, exact in-points).
2. **AAC encoder leg** (ove-encode, native libavcodec aac): `AudioEncoder`
   trait + `FfmpegAacEncoder` — caller samples buffered to the 1024 grid,
   feed contiguity ENFORCED (typed mismatch on a wrong cursor),
   small-last-frame branch detected from `AV_CODEC_CAP_SMALL_LAST_FRAME`
   and reported, CBR-only (CRF typed `Unsupported`), libav identity
   recorded. `TrackSpec::initial_padding` added so the muxer records the
   priming delay and the container writes the trim edit list — the FILE
   duration stays sample-exact (verified against a CLI-produced reference:
   first packet pts −1024, elst 1024, duration 44100/44100).
3. **WAV/PCM out**: pure-Rust PCM16 writer with the pinned
   `round(clamp(x,−1,1)×32768)` conversion (no libav; deterministic
   bytes; mono/stereo v1).
4. **Engine audio** (ove-engine): `assemble_timeline_audio` — the FIRST
   track's clips assembled as sample-exact per-clip ranges (every cut on
   a sample boundary or typed `NonExactSampleCut`; speed ≠ 1 typed
   `AudioRetimeUnsupported`; span authority enforced — short sources are
   a typed error, never a silent pad). `export_reencode` pairs video
   re-encode with audio re-encode automatically (ENCODER_SPEC §3.3);
   `export_wav` delivers PCM out.
5. **Planner Deferred → executable with ZERO planner changes**: the W3
   pure logic already covered `reencode_available: true`. Test IDs:
   decode A-1..A-4; encode E-8/E-8b/E-9/E-10 (A/V mux with two streams,
   ffprobe + libav verified, drift ± 0 samples); engine
   w7_audio_assembly_and_av_export (assembly 192000 samples ±0, A/V
   export 4.000000 s both streams, WAV exact, document hash unchanged by
   export work). Workspace 141/141.

## WAVE 6 deltas (2026-09-29, PR #8)

1. **The directive W6 milestone is REAL and tested**
   (ove-engine/tests/integration.rs::w6_vertical_slice_milestone): real
   video ×2 (24 fps + 29.97 multi-rate) → probe → content-hash registry →
   timeline commands (explicit ids, per-clip asset bindings riding the
   log) → exact ADR-013 source mapping → decode → FrameEnvelope →
   RenderPlan → software render → encode/mux → valid MP4 → ffprobe + libav
   verified (120 frames, duration 5/1 exact) → save → kill → reopen →
   SAME state hash → re-export byte-identical (deterministic SW pipeline,
   file_sha256 equality).
2. **Per-clip asset bindings ride the log** (LogPayload::Insert asset
   field → record_entry_bindings on execute AND replay → StateMirror.
   clip_assets in the document hash). Multi-source render resolves each
   placement's source by binding; DecodeSource per source.
3. **D-5 floor rule implemented in the decode source**: fetch returns the
   greatest pts ≤ target (multi-rate sources floor honestly; exact hits
   are the CFR case).
4. **docs/VERTICAL_SLICE_TRACE.md written** — the three chains (import,
   command+replay, AI/client→command→render→export) at file→crate→function
   level, each hop exactness-annotated, each claim pinned by a named test,
   with the non-claims (audio/GPU/agent transport) named as such.

## WAVE 5 deltas (2026-09-29, PR #7)

1. **ove-engine created** (workspace member 8): the headless session wiring
   all five legs — probe (FfmpegProbe probe_full) → hash-addressed import
   with probe sidecars → ove-timeline commands (explicit ids, alloc before
   construct) → project persistence → decode (exact seeks, D-4/D-5) →
   render (ADR-013 mapping → compile → software exec) → export.
2. **ove-cli created** (workspace member 9): subcommands new/add-track/
   import/add-clip/split/undo/redo/status/export-copy driving the engine;
   smoke test drives the real binary end-to-end (CARGO_BIN_EXE).
3. **Integration suite 6 tests (workspace 131/131)**: save/reopen hash
   equality, kill-reopen-continue at multiple points, render determinism
   from real decode, re-encode export with exact frame count + duration
   (ffprobe + libav verified), keyframe-aligned copy export via the
   planner, undo/redo surviving reopen.
4. **Two real bugs found & fixed (both now mutation-pinned)**:
   a. **ove-media pool bug**: `into_frame_bytes()` cleared the buffer
      before moving it — EVERY decoded frame shipped an empty payload.
      Wave-2's decode suite passed vacuously (nothing compared pixel
      bytes); the engine's real renders exposed it. Fixed + payload-
      content gate added to the decode conformance suite (d13).
   b. **ContentHash identity confusion**: sidecar hydration re-parsed
      digests with from_bytes (which HASHES) — from_hex (round-trip)
      added and named so the two cannot be confused again.
   c. **Allocation state rides the log**: replay could not reproduce the
      id-alloc cursor (apply() never advances it) — every log entry now
      carries `nid` (cursor after execution), replay restores it via
      Timeline::set_next_id (monotonic, guarded). Project-layer tests had
      used literal ids and passed vacuously; the engine suite is the
      first mutation-strong caller.
5. **ADR-017** records: engine = integration layer (core stays libav-free),
   the boundary-conversion contract (YUV→RGBA once at decode, RGBA→YUV
   once at encode, integer/deterministic, matrix from tags), v1
   single-source render note (per-clip asset binding rides the log at W6).

## WAVE 4 deltas (2026-09-29, PR #6)

1. **ove-project created** (workspace member 7): PROJECT_FORMAT_SPEC v1 —
   manifest.json (identity + assets + track registry), commands.jsonl
   (append-only, write-through with flush-per-entry: the on-disk state
   after each execute IS the kill-9 state), snapshot/ (state-<seq>.json +
   meta.json authority, atomic temp+rename everywhere). Folds through
   ove-timeline commands with explicit ids (E-012 discipline carried:
   next_id + used_ids serialized as document state — new read-only
   accessors + from_parts on Timeline).
2. **Acceptance P-1..P-8 green (11 tests; workspace 123/123)** including:
   P-2a REAL subprocess kill-9 (re-exec child mode + abort, replay ==
   live hash), P-3 compaction equivalence + continuation, P-4 corruption
   drill (typed hard-stop error → explicit quarantine repair → last-good
   hash exact), P-5 float rejection + exact (num,den) round-trip, P-6
   hash-addressed assets (folder move survives; corrupt/missing typed,
   state intact), P-7 undo-marker replay (undo history rebuilt from the
   log per ADR-008), P-8 disposable dirs.
3. **ADR-016** records: authority order (log = record, snapshot meta =
   snapshot authority, manifest = reconciled hint), log grammar (embedded
   inverses on markers → self-contained suffixes; anchored seq contiguity;
   timestamps omitted in v1), track registry in the manifest (tracks are
   setup, not commands), canonical BLAKE3 state hash over the document
   mirror, the repair path, the kill-9/fsync honesty boundary.
4. **Design honesty note**: reopening restores undo history ONLY from the
   replayed suffix (ADR-008 forbids persisting the stack) — P-3/P-7 assert
   exactly this model; pre-snapshot undo steps are not undoable after a
   reopen, which is the recorded v1 semantics.

## WAVE 3 deltas (2026-09-29, PR #5)

1. **ove-encode created** (workspace member 6): ENCODER_SPEC §1 traits —
   `Encoder` (configure/feed/drain + the recorded `track_spec()` extension),
   `Muxer` (open/write/finalize), `EncodedPacket` with checkpoint fields;
   pure export planner (`planner.rs`): keyframe-aligned StreamCopy vs
   ReEncode vs Mixed segmentation, snaps always reported (receipt), audio
   grid alignment; FFmpeg adapter (`ffmpeg/`): `FfmpegSwEncoder` (native
   mpeg4, swscale RGB→YUV420P declared-conversion path), `FfmpegMuxer`
   (MP4, faststart + bitexact options, exact tick conversion against the
   EFFECTIVE post-header axis), `FfmpegCopySource` (packet-level demux,
   keyframe-floor landing, 0-start shift, byte-identical passthrough).
2. **Tests 28 new (workspace 112/112 green)**: planner P01–P10 (pure logic:
   snap table, RoundIn validity, ToEof, 23.976 exactness, audio grid,
   mixed coverage, shapes); conformance E-1..E-5 + E-7/E-7b/E-7c + H1–H5 —
   outputs verified through ffprobe CLI (when present), libav probe, and
   decoded-pixel equality; copy-route byte-identity to source GOP range
   (E-007 §3.4) pinned at both packet and pixel level; committed corpus
   (copy24/copyav/copyntsc + golden tables) via
   `scripts/corpus/gen_encode_corpus.py`; byte golden
   `encode_golden.json` keyed on the producing libav identity (7.1.5-0+
   deb13u1), same-run determinism asserted everywhere.
3. **ADR-015** records the v1 decisions: confinement restated as CLOSED
   adapter allowlist {ove-decode, ove-encode} (check script amended and
   green), mpeg4-first codec surface (OpenH264/SVT-AV1 typed Unsupported),
   snap semantics made normative, movenc timescale-widening (24→12288)
   handled by effective-axis conversion, 0-duration packets → None, mp4
   AVIO caller-ownership (the AVFMT_NOFILE inversion SEGFAULT class, found
   and pinned by tests), version-keyed byte goldens with explicit reported
   skips. ADR-005 promoted PROPOSED → ACCEPTED (v1 surface landed).
4. **Bugs found & fixed in-wave** (each now test-pinned): mixed-axis tick
   conversion after movenc widening; stale pict_type=I on the reused
   encoder frame forcing all-intra; codec 0-duration corrupting stts;
   non-refcounted packet aliasing; test fixture pts bug (24k s frames) —
   caught because e2b passed while e2/e3 failed.

## WAVE 2 deltas (2026-09-28)

1. **ove-render created** (workspace member 5): software reference renderer
   per RENDER_GRAPH_SPEC — plan.rs (RenderPlan/Pass/PassKind + canonical
   hash), compile.rs (PURE compiler: compile_frame/compile_span; layer order
   = track order; exact rationals; conservative culling), exec.rs (software
   executor: integer straight-alpha over opaque output, FrameSource D-5
   floor resolution, single color conversion).
2. **RG-1..RG-7 all green** (render_golden.rs): compile purity (runs +
   threads, 4 seeds × 3 tracks × 5 placements), golden frames (3 fixtures,
   blake3-committed, hand-verified pixel semantics asserted alongside),
   layer-order swap, retime 0.5×/2× exact frame prediction (through the
   ADR-013 seam), split continuity END-TO-END through the real Timeline
   (apply Split → walk → compile → render byte-equality across the seam),
   optimizer safety (cullable placements: no passes + byte-identical
   output), color-tag single conversion + TagMismatch honesty error.
3. **ADR-014** records the v1 decisions (opaque integer compositing,
   output-sized surfaces, declared-tag conversion, integer-translate
   geometry, FrameSource decoupling, per-frame compile shape) + reopen
   conditions. Key bug the goldens caught during development: opaque
   transform buffers → fixed to transparent-outside-region.
4. ENGINE_BUILD_PLAN waves 0–3 are now REAL crates; the directive's W2
   "software-first renderer" gate is satisfied; decode binding to the same
   FrameSource trait lands with the W6 vertical slice.

## WAVE 1 deltas (this session)

1. **Forensic finding**: 4 uncompiled files (clip.rs/command.rs/engine.rs/
   state.rs, 1,264 l) orphaned by the v0.7 reconciliation — lib.rs declared
   only avl/gap/oracle, so a "editor-grade state machine" LOOKED implemented
   while compiling zero times with zero running tests (the exact
   documentation≠implementation class the directive forbids). REMOVED with
   full provenance (recoverable from commit ee31fb9); ideas that survive live
   in the compiled lineage (explicit-id log discipline, state hash, exact
   inverses). Recorded in ADR-013.
2. **Seam implemented** (`ove-timeline::mapping`, ADR-013): ClipWindow +
   SourceClock (zero ove-media dependency — layering held); exact
   timeline↔source mapping (roundtrip identity), `frame_span` floor/ceil
   bounds, `validate_ingest` range gate, `plan_seek` keyframe-floor landing
   (D-5/E-007 encoded positively); typed SeamError for all caller-range bugs.
3. **ove-time additions**: `recip()` + `ceil_div_rate()` (exact; unit tests).
4. **Property suite S0–S6** (mapping_properties.rs): roundtrip exactness over
   retimed windows, clip-primitive agreement, ingest gate boundaries, snap
   keyframe membership + monotonicity (20k cases), frame-span exactness at
   23.976 boundaries, exact-plan target identity, clock boundary validation.
5. MEDIA_ENGINE_SPEC §3 annotated "Implemented (2026-09-28, ADR-013)".

## WAVE 0.6 deltas (2026-09-28, PR #2)

1. ove-time overflow policy DECIDED + pinned (ADR-007 amendment 2026-09-28):
   **overflow = contract violation = PANIC (fail-fast); never wraps, never
   saturates.** Rationale: saturation is silent approximation (fatal to E-002
   exactness); fixed-tick-axis discipline makes overflow a caller bug; p3b
   already asserted the panic. Doc drift (crate header said "saturates") fixed.
2. ove-time i64::MIN holes FIXED (real bugs found by the takeover re-audit):
   - `neg()`: `-i64::MIN` wrapped silently (release) — now checked_neg + panic
     with explicit contract message.
   - `gcd64/gcd128`: `.abs()` on i64::MIN/i128::MIN overflowed — gcd now
     computed on unsigned magnitudes.
   - `Rational::new(i64::MIN, n)` normalizes correctly end-to-end now.
3. Property suite extended P1–P10 → P1–P12: P11 extreme-safety (MIN/MAX
   construct/cmp/half/floor_div_rate/split/add/serde), P12 overflow-panics
   (neg/sub/add/mul/floor_div_rate) + must-NOT-panic exact-boundary guard.
4. Hash terminology single-policy reconciliation (audit gap #1b):
   PROJECT_FORMAT_SPEC §2 now carries a normative HASH POLICY (BLAKE3-256,
   64 lowercase hex, = ove-media ContentHash; sha256 confined to golden-
   artifact TEST tooling), §2 manifest example `sha256`→`content_hash`,
   §5 `assets/<sha256>/`→`assets/<content-hash>/`; ENCODER_SPEC E-5 +
   muxer finalize() annotated "artifact integrity, not identity".

## CI CONFIRMATION (2026-09-27, prior wave)

- PR #1 run 36303605420: ALL 3 JOBS GREEN (fresh portable build; wrapper
  marker present; guard scanned 1445 objects PASS; conformance 14/14).
- Post-merge main run 36304226310: ALL 3 JOBS GREEN — the original failure
  scenario (cache restored from another runner) now passes with portable
  artifacts; conformance 14/14; portability guard PASS.

## Current implementation inventory

| Crate | State | Evidence |
|---|---|---|
| ove-time | TESTED — 9 unit + 21 property (P1–P13 incl. round_half_up totality) | 157/157 workspace local (2026-09-29, W8) + CI |
| ove-timeline | TESTED — 10 properties (AVL primary, ADR-011) + 7 seam properties (S0–S6, ADR-013) + 11 keyframe suites (unit + S7/S8/S9, ADR-019) | CI + local; bench in ADR-011 |
| ove-media | TESTED — 13 unit (asset hash, FrameEnvelope, pool, probe types) | CI + local |
| ove-decode | TESTED — 14 conformance (D-1..D-12) + 6 probe + 4 audio (A-1..A-4); libav confined | CI + local, both system and bundled libav |
| ove-render | TESTED — RG-1..RG-7 (purity, goldens, layer order, retime, split continuity, optimizer safety, color) | CI + local; ADR-014 |
| ove-encode | TESTED — P01–P10 planner + E-1..E-5/E-7/H1–H5 video + E-8..E-10 audio (ffprobe/libav/pixel-verified, goldens committed) | 157/157 workspace local (2026-09-29); ADR-015/ADR-018 |
| ove-project | TESTED — P-1..P-9 acceptance (12 tests incl. subprocess kill-9, compaction equivalence, corruption drill, keyframe persistence) | 157/157 workspace local (2026-09-29); ADR-016/ADR-019 |
| ove-engine | TESTED — integration suite 7 + W7 audio vertical + W8 keyframe vertical (exact plan eval, hash/reopen stability) | 157/157 workspace local (2026-09-29); ADR-017/ADR-018/ADR-019 |
| ove-cli | TESTED — binary smoke (full flow via CARGO_BIN_EXE) | 157/157 workspace local (2026-09-29) |

## Current research / architecture status

- ADR-001/002/005/007/008/009/010/011/012/013/014/015/016/017/018/019 ACCEPTED
  (005 promoted 2026-09-29, v1 surface landed; 018 audio leg v1; 019 keyframes
  v1); ADR-003/004/006
  PROPOSED with named evidence legs. ADR-007: overflow policy; ADR-013: seam
  + dead-leg removal; ADR-014: software renderer v1.
- 45 research docs + 137-source ledger + gates (13 UNDERSTOOD / 1 PARTIAL env-bound).
- Architecture residuals (named, not hidden): E-005 real-GPU perf, E-004c device
  runtime, E-006b real-Tauri transport — all hardware/environment-bound.

## Current validation status

- 157/157 workspace tests GREEN locally (system FFmpeg 7.1.5 path, rustc
  1.98.1), fmt GREEN, clippy GREEN (workspace, all targets), after W8
  keyframes (2026-09-29). libav-confinement check PASS.
- Bundled path: exercised by CI on this wave's PR (bundled libav now also
  links swscale + swresample via the union feature set; portability guard
  scans the same object tree; E-5 byte gate self-skips with an explicit
  report on a different libav identity, structure gates always run).
- CI: PR run 36501739232 on `a42a975` SUCCESS (all 3 jobs, bundled path
  included); post-merge main run 36502138331 on `00a46b9` SUCCESS —
  WAVE 8 landed.

## Current CI state

- Workflow: fmt + clippy + tests (bundled) + portability guard + cargo-audit
  + libav-confinement. Cache prefix `v2-portable-ffmpeg` (poisoned caches
  unreachable).
- Latest confirmation: post-merge run 36502138331 on HEAD `00a46b9`
  SUCCESS (2026-09-29). All recent runs SUCCESS.

## Current blockers

- None environmental.

## Current open gaps (top)

1. Audio named gaps (ADR-018): multi-track mixing/overlaps, audio
   retiming, multi-lane assembly, AAC byte goldens (E-5 keying), E-6
   audio checkpoint execution.
2. Keyframe named gaps (ADR-019): curves beyond Linear/Hold (bezier/ease),
   audio keyframing (volume automation) not wired into the assembly,
   animatable properties beyond the renderer's placement inputs, key-
   data duplication at every cut inside an animation (accepted v1 cost).
3. OpenH264/SVT-AV1 encoder legs; E-6 kill-resume execution at segment
   granularity (checkpoint fields exist).
4. GPU residuals (ADR-020): YUV GPU conversion + fractional scaling with
   declared tolerance + zero-copy frame import (hardware/driver legs);
   real-hardware perf baselines (llvmpipe record is INVALID-class);
   engine/platform backend selection rides waves 10–12.
5. Mutation-style "test the tests" not yet systematic.
6. Hardware-bound experiment residuals (E-004c/E-005/E-006b).
7. GitHub PAT used across chat sessions must be rotated by the owner
   (standing security rule, DEV_ENV.md) — outside engine scope, flagged.

## Current wave order (directive §37, unchanged)

0 audit ✓ → 0.5 SIGILL ✓ → 0.6 reconcile ✓ → 1 seam ✓ → 2 render ✓ →
3 encode+mux+export ✓ → 4 project ✓ → 5 engine+cli ✓ → 6 vertical slice ✓ →
7 audio ✓ → 8 keyframes ✓ → 9 GPU ✓ → 10–12 platforms → 13 conformance →
14 headless → 15 AI/MCP → 16 scripting → 17 plugins → 18 security →
19 perf → 20 docs/release → 21 production audit.

## Resolved decisions (registry)

- ADR-020 (2026-09-29): GPU reference executor v1 — ove-render::gpu behind
  feature `gpu` (wgpu 25), Rgba8Uint + u32 integer blend identical to
  software, `@builtin(position)` coordinates, native-size fetch surfaces /
  output-sized placed targets (no accumulated offset), typed den bound
  MAX_ALPHA_DEN = 65 000 with software fallback, typed NoAdapter
  feature-detect, error-set reuse, CI gpu-conformance job on lavapipe,
  perf probe with INVALID-class software-adapter stamping. Confidence 0.9
  (parity proven on software Vulkan; hardware legs are named residuals).
  Reopen: an adapter diverging from software bytes; den > 65 000 need
  (u64 emulation conversation); present-to-surface platform requirement.
- ADR-012 (2026-09-27): CI bundled-FFmpeg portability — wrapper + cache prefix
  + guard. Confidence 0.92. Reopen condition: ffmpeg-sys-next flag change (guard
  fails loudly) or CI SIGILL recurrence.
- ADR-018 (2026-09-29): audio leg v1 — canonical planar-f32 decoded surface
  (format conversion only, never a resample), audio floor+trim seek policy,
  sample-count authority with TrackSpec::initial_padding → container edit-list
  trim (reference-pipeline-verified), small-last-frame branch declared, WAV
  PCM16 out pure Rust, engine single-lane assembly with typed exactness
  errors. Confidence 0.88. Reopen: an encoder without SMALL_LAST_FRAME, a
  non-1/rate audio time base, or a mixing requirement (render-graph audio
  legs).
- ADR-007 amendment (2026-09-28): ove-time overflow policy = intentional panic
  (fail-fast); i64::MIN safe except negation (panics); checked variants deferred
  to first non-panicking consumer. Confidence 0.93. Reopen condition: an FFI
  consumer that cannot tolerate panics forces the checked-variant conversation.
- Hash policy (2026-09-28): BLAKE3-256 is the ONLY identity hash (assets +
  state); sha256 exists only in golden-artifact test tooling / render-record
  integrity fingerprints. Confidence 0.97 (code truth was already BLAKE3).
- ADR-013 (2026-09-28): timeline↔media seam = ove-timeline::mapping
  (ClipWindow/SourceClock, query-time mapping = Q-06 v1 resolution, typed
  seam errors, keyframe-floor seek planning) + dead state-machine leg removed
  (provenance ee31fb9). Confidence 0.88. Reopen: retime-verb semantics
  contradict S1; a backend cannot honor D-5 landing; ove-project needs
  persisted resolved-command journals.
- ADR-014 (2026-09-28): software reference renderer v1 — opaque integer
  straight-alpha compositing, output-sized surfaces, declared-tag single
  conversion, integer-translate geometry, FrameSource decoupling from
  ove-decode. Confidence 0.9. Reopen: intentional golden diff without
  byte-level note; scaling/affine need; stacked partial-coverage banding.
- ADR-015 (2026-09-29): encode leg v1 — adapter allowlist {ove-decode,
  ove-encode}; mpeg4-first codec surface; snap semantics normative
  (RoundIn start validity, RoundOut/ToEof end, Mixed inner core);
  effective-axis exact tick conversion; version-keyed byte goldens with
  explicit skips; refcounted packets; mp4 AVIO caller-ownership.
  Confidence 0.88. Reopen: a libav build whose movenc does NOT widen
  timescales still passes (conversion is effective-axis based); an
  encoder that needs B-frames forces the dts/ctts conversation; openh264
  licensing form (external lib vs bundled) at its adapter leg.
- ADR-016 (2026-09-29): project persistence v1 — authority order (log >
  snapshot meta > manifest hint), write-through kill-9 model, log grammar
  with embedded inverses + anchored contiguity, track registry in the
  manifest, BLAKE3 document hash, explicit repair path, id-state as
  document state (E-012). Confidence 0.9. Reopen: a second schema version
  (migration serializers + fixtures); a requirement for fsync-grade
  durability claims; track-structure commands at the W8 era.
- ADR-019 (2026-09-29): keyframe animation v1 — keys on the clip (local
  times; Move carries, Resize strands inert, Split slices with computed
  boundary keys both sides = exact split-preserve), per-key out-interp
  Linear/Hold with exact rational evaluation and loud validation,
  SetKeyframes wholesale-replace command whose inverse is the previous
  key list, animation as document state (hash + mirror + log grammar),
  engine-side evaluation with P13 round-half-up i32 conversion
  (proven total over representable rationals). Confidence 0.9. Reopen:
  curve families beyond Linear/Hold; animatable properties outside the
  placement inputs; audio keyframing; speed≠1 keyframe time mapping
  (rides the retime verb, ADR-013).

## Unresolved decisions (registry)

- NLE derived-verb surface (ripple/roll/slip/slide/insert/overwrite/extract/lift):
  record primitive-vs-derived table when timeline work resumes (directive §6.2) —
  not before.

## Explicit next action

WAVE 10–12 — platform legs (BUILD_PLAN wave 8, evidence collection, parallel
where hardware allows): browser WASM core (timeline+project) + WebCodecs
adapter behind mandatory runtime feature detection; desktop transport leg;
Android/MediaCodec per E-004c spike design. Each leg lands behind runtime
feature detection with typed fallbacks, then WAVE 13 cross-platform
conformance, 14 headless/batch, 15 AI/MCP, 16 scripting, 17 plugins,
18 security, 19 perf, 20 docs/release, 21 production audit. Land via PR
with fmt/clippy/tests green and this file updated.
