# ADR-014: Software reference renderer (ove-render v1)

- **Status**: ACCEPTED (2026-09-28) — acceptance condition met: RG-1..RG-7
  suite green in-repo (engine/ove-render/tests/render_golden.rs), including
  committed golden-frame hashes.
- **Date**: 2026-09-28 · **Confidence**: 0.9 (contract is RENDER_GRAPH_SPEC;
  the v1 subset boundaries below are the only new decisions)

## CONTEXT

WAVE 2 (directive §37) = ove-render, software-first (directive §11: "software
renderer is the correctness reference; GPU is only a backend"). RENDER_GRAPH_SPEC
fixes the contract (pure compile, layer order = track order, exact rationals
for time, culling only when provably identical, golden frames). The v1 subset
needed explicit decisions for: compositing algebra, color handling without a
decoder yet, and geometry scope.

## DECISION

1. **Compositing algebra**: straight-alpha "over" onto an OPAQUE output
   (black base). Integer-only: `N_s = src_alpha × alpha_num`,
   `D = 255 × alpha_den`, `out = (c_s·N_s + c_d·(D−N_s) + D/2) / D`
   (round-half-up, u64 intermediates) — byte-identical on every platform.
   The output FrameEnvelope is RGBA8, alpha 255, working-space tags.
   Rationale: avoids premultiply round-trip error at 8-bit, keeps the final
   video frame opaque (export convention), and is trivially verifiable by
   hand (fixture C: 50% blue over red → (128, 0, 128), pinned).
2. **Surface discipline**: every post-fetch surface is OUTPUT-SIZED.
   SourceFetch normalizes the fetched frame into a transparent output-sized
   surface (paste at origin, clipped); Transform composes within output
   bounds and leaves TRANSPARENT (alpha 0) outside the translated region —
   the following blend-over leaves the destination unchanged there. (The
   first implementation had opaque transform buffers; the golden asserts
   caught it — exactly what RG-2 exists for.)
3. **Color (RG-7)**: Placement carries the source's DECLARED tags (probe
   data, known at ingest — not per-frame). Compile emits exactly one
   ColorConvert pass when declared ≠ working space, zero otherwise; the
   executor FAILS LOUDLY (TagMismatch) when a fetched frame's tags differ
   from the plan's assumption with no convert pass — no silent
   normalization. v1 conversion is a tag stamp for RGB data; pixel-level
   matrix work arrives with YUV decode (W6) inside the same pass.
4. **Geometry**: Transform = integer translation in v1 (floats appear
   nowhere, tighter than the spec's "floats only for geometry"); affine
   fixed-point lands with real decode sources.
5. **Sources**: the `FrameSource` trait (fetch: greatest pts ≤ target, the
   D-5 floor rule) decouples the renderer from ove-decode. Tests use
   deterministic synthetic sources (solid, frame-counting); the W6 vertical
   slice binds Decode passes to real sessions behind the same trait.
6. **Compile shape**: `compile_span(input, span) -> Vec<RenderPlan>` with one
   plan per output frame (per-frame `frame_index` + exact `src_pts`); purity
   is pinned per-frame AND across the span (RG-1, incl. cross-thread).
7. Audio tracks contribute no video passes in v1 (RENDER_GRAPH_SPEC RG-2
   "audio-affecting none" is satisfied structurally; sample-accurate audio
   is wave 7).

## ALTERNATIVES CONSIDERED

- Premultiplied compositing: more correct for repeated compositing with
  partial coverage, but adds an unpremultiply rounding stage that made the
  single-layer identity inexact at 8-bit; revisit if a use case stacks >2
  partial-coverage layers with measured drift (golden diff would show it).
- Compile-time source frame resolution (plan names concrete source frames):
  rejected — compile must stay a pure function of (input, span); source
  frame grids belong to the source (clock), resolved at exec via D-5 floor.
- Taking `&Timeline` directly in compile: rejected — the engine layer (W5)
  builds RenderInput from the timeline walk + probe records; render stays
  independent of track container internals (RG-5 exercises the real
  Timeline → walk → placement path to prove the integration).

## REOPEN CONDITIONS

- A golden frame changes without a documented, intentional reason → revert.
- Real decode sources need scaling/affine → extend Transform with
  fixed-point coefficients; RG-2 goldens re-baselined with byte-level notes.
- Stacked partial-coverage compositing shows visible banding → revisit
  premultiplied algebra (decision 1).
