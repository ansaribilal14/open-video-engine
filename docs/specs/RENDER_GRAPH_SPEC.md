# RENDER GRAPH SPEC — deterministic compile + golden correctness (v1, 2026-09-26)

> Parent: ADR-002/006 + doc 08 pass-graph design. Directive: minimal prototype now; E-005
> upgraded into a real compositor crate later with pixel-exact tests preserved and
> correctness separated from performance. No node-editor UI, no dynamic recompilation.

## 1. The two objects

```rust
/// Pure function of (project state, render span) — determinism contract RG-1.
pub fn compile(project: &ProjectState, span: RenderSpan) -> RenderPlan;

pub struct RenderPlan {
    pub passes: Vec<Pass>,           // ordered; the ONLY execution authority
    pub resources: ResourceTable,    // frames/surfaces referenced by passes
    pub output: OutputSpec,          // size, pixel format, working color space
}

pub struct Pass {
    pub kind: PassKind,              // Decode | Transform | Blend | ColorConvert | Encode
    pub inputs: Vec<ResourceRef>,
    pub outputs: Vec<ResourceRef>,
    pub params: PassParams,          // exact rationals for any time; floats allowed ONLY
                                     // for geometry coefficients (documented, no time use)
}
```

## 2. Compile rules (the determinism spine)

1. **Pure function**: identical project state (hash) → identical plan (canonical
   serialization hash). Property test with random timelines (fuzz).
2. **Layer order = track order** (bottom-up), tie-broken by explicit ids — no layout
   algorithms, no hidden z-order.
3. Every parameter that depends on time is an exact rational evaluated at the frame's
   timeline pts (keyframes in wave 7 follow the same rule).
4. Plan is backend-agnostic; backends choose implementation per pass kind — they may not
   reorder, skip, or merge passes without producing a byte-identical output (golden
   frames arbitrate).
5. Compaction/culling (off-screen, 0-alpha) is allowed only when provably
   output-identical; property-tested against the unoptimized plan.

## 3. Execution model (v1 software path)

- CPU RGBA working buffers from a frame pool (FRAME_CONTRACT §5 pool rules).
- Per output frame: for pass in plan.passes → execute → produce output frame.
- Decode-on-demand orchestration: Decode passes pull from ove-decode sessions; the plan
  carries seek hints (previous frame pts, direction) so scrubbing is seek-friendly.
- Single-threaded v1 for determinism; frame-level parallelism later (frames are
  independent — plan-pure property makes this safe).

## 4. Correctness suite (committed, CI-runnable)

| ID | Test |
|---|---|
| RG-1 | Compile purity: N random projects → canonical plan hash equality across runs/threads |
| RG-2 | Golden frames: committed PNG/RGBA hashes for a fixture set (1-layer, 2-layer overlap, opacity, transform, mixed rates 24+30, VFR source, audio-affecting none) |
| RG-3 | Layer order: swapping track order changes output exactly as specified (not identity) |
| RG-4 | Retime: 0.5×/2× retime renders the *exact* source frames rational-arithmetic predicts |
| RG-5 | Split-continuity: split at t renders identically to unsplit across the seam (P7 invariant end-to-end) |
| RG-6 | Optimizer safety: compacted plan output == unoptimized output byte-identical |
| RG-7 | Color-tag propagation: fixture with Bt601 source into Bt709 workspace converts once (tag the conversion; assert no double conversion) |

## 5. wgpu upgrade path (wave 6 — E-005 promotion, correctness ≠ performance)

- Same RenderPlan; GPU backend implements pass kinds; **golden frames RG-2 must pass
  through the GPU path with pixel-exact (or declared-tolerance) parity** — the directive's
  "pixel-exact tests preserved" rule.
- Known hazard carried from E-001: importExternalTexture is single-use-per-pass →
  multi-layer compositing may require a copy path; the plan's ResourceTable supports
  explicit materialization passes so the choice is a plan decision, not a backend whim
  (Q-02 resolution instrument).
- Perf bench separate: committed timings, ratio-gated (R-14) — never merged into the
  correctness suite.

## 6. Explicit non-goals v1

Node UI/graph editing · dynamic recompile mid-frame · cross-frame temporal effects
(motion blur) · GPU feature detection beyond backend presence · shaders beyond
transform/blend/color-convert.
