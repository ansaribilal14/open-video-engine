# ADR-023: Export decode-session budget — one decoder open per source per export

- Status: ACCEPTED
- Date: 2026-09-30 (WAVE 19)
- Consumers: ove-engine (export/fetch/render paths), ove-decode (session state machine)

## Context

The W18 certification re-run left the engine green but slow in a way nobody
had ever measured: `export_reencode` rebuilt the decode-source map — and
therefore every decoder session — for EVERY output frame. On the permanent
real-media proof (NASA Artemis I highlights, 122.88 s, CFR 24000/1001,
160-frame edited export) the new deterministic instrument
(`ove_engine::decoder_opens`) measured **160 decoder opens for one
160-frame export** — one full `avformat_open_input` + keyframe re-seek +
GOP re-decode per placed frame. O(N·GOP) decodes where O(N) suffices.
The fetch loop compounded it: every intermediate decoded frame between the
keyframe floor and the target was converted YUV→RGBA and then discarded,
keeping only the last.

The defect had been invisible because every prior correctness gate asserts
output identity, not resource shape — and determinism masked the waste
(the wrong-but-slow path produced byte-identical output). Wall-clock
folklore ("exports feel slow") is not evidence; the open counter is.

This is a performance-decision ADR about the EXPORT path only. The
software renderer stays the correctness reference (ADR-015); GPU and
threading remain future waves. The anti-overbuild rule bounds the fix:
no frame cache, no prefetch machinery, no speculative pooling — only the
session lifetime change the instrument justifies.

## Decision

**1. ONE decoder open per source per export.** `export_reencode` builds
the decode-source map ONCE (`build_decode_sources`) before the frame
loop and renders through `render_frame_with(&sources, &output, t)` with
the decoder sessions alive for the whole export. `render_frame` keeps
its one-shot signature — spot renders and MCP status frames are
unchanged in behavior and cost.

**2. `DecodeSource` owns its data.** The map previously borrowed the
engine, which is what forced the per-frame rebuild. `DecodeSource` now
owns a cloned `SourceMedia` and an owned asset-directory path, so the
source map carries no borrow of the engine and can outlive a single
call frame.

**3. The fetch cursor is reused across placements.** The
REALWORLD-BUG-3 sequential cursor-reuse discipline finally engages in
the export path (it was dead code pre-W19: every fetch re-seeked via
the per-frame session rebuild).

**4. A one-frame `pending` pushback in `VideoSession` (REALWORLD-BUG-4).**
Fixing the session lifetime EXPOSED a real latent defect: the fetch loop
discarded the popped past-target frame, so the next sequential fetch —
whose target equals that frame's pts under CFR — decoded past its floor
and returned `None` (`SourceFrameMissing at 1/24`). The fix: a bounded
ONE-frame pushback in `VideoSession`. A target that falls BETWEEN the
last floor and the pending frame (VFR / multi-rate gap) falls back to
the re-seek rebuild — the D-5 floor rule stays total. Memory stays
bounded by construction (one FrameEnvelope, not a cache).

**5. Convert once, on the floor frame.** YUV→RGBA conversion happens a
single time per fetch, on the final floor frame, not on every
intermediate decoded frame.

**6. The budget is a committed conformance test, not a benchmark
screenshot.** `ove-engine/tests/export_session_budget.rs` (3 tests,
Mutex-serialized — the open counter is process-global) pins the
open-count contract: an N-frame export opens the decoder ONCE per
source, and the count is part of the regression surface forever.

## Consequences

- Measured on the permanent proof, same source (identity gate
  `2d315daf…705f`), same machine, BEFORE = worktree @ `adcafd9`:
  end-to-end proof 129.90 s → 22.26 s (**5.8×**); export leg ~65 s →
  10.9 s (~6×); decoder opens per 160-frame export **160 → 1**.
- **Determinism survives the optimization — the strongest pin**: the
  160-frame export sha256 `baf23d2a…` is IDENTICAL between the pre-fix
  and post-fix trees. Every RLW-1/2/3 PASS row stayed green; the W17
  plugin gate re-ran green (2.08 s); the W18 security gate re-ran green
  (2.33 s).
- Workspace 189/189 GREEN (186 pre-existing + 3 budget conformance);
  fmt/clippy GREEN.
- The record is additive and deterministic: `realworld_record.json`
  carries a `perf` section (per-leg wall-clock + open counts), so future
  waves inherit a baseline instead of re-deriving one.

## Named residuals (not done in v1, deliberate)

- **Two-track same-source shape still re-seeks per frame** (backward
  jump between placements with one session per source). Measured
  acceptable at 10.9 s/export; a per-placement session split is the
  named future leg.
- **No threading/pool wiring, no GPU perf, no blend-division tuning, no
  allocator-churn work** — all belong to the performance waves that
  follow the correctness-reference discipline (software renderer stays
  the reference until a wave is justified by a failing budget).
- **The open counter is process-global** (Mutex-serialized in tests).
  Per-session counters would be finer-grained but are machinery no
  failing test has asked for yet.

## Confidence

0.9. The decision is backed by a deterministic instrument (open counts),
a before/after wall-clock measurement on real media, and a byte-identity
pin across the change. What would reopen this ADR: a workload where the
one-session-per-source budget regresses correctness (VFR shapes the
re-seek fallback cannot cover); an export shape needing more than one
session per source; a budget test that becomes flaky under CI parallelism
(then the counter moves per-session as a precondition of any further
performance work).
