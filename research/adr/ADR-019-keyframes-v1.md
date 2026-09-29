# ADR-019: Keyframe animation v1 (property → keys → interpolation → exact evaluation)

- **Status**: ACCEPTED (2026-09-29) — acceptance condition met: ENGINE_
  BUILD_PLAN wave-7b keyframe gate green (S7 oracle-equivalence, S8 split-
  preserve, S9 command-exactness suites; P-9 persistence acceptance; W8
  engine integration with exact plan-level values; workspace 157/157).
- **Date**: 2026-09-29 · **Confidence**: 0.9 (the exact-time model, split
  slicing, and round-half-up conversion are durable; the fixed 3-property
  v1 set and Linear/Hold-only interpolation are named, short-lived
  limitations with recorded reopen conditions)

## CONTEXT

ENGINE_BUILD_PLAN wave 7 names the second leg: "Keyframes:
property→keyframes→interpolation (linear/hold first) → evaluation over
exact time; property tests (monotonic keys, boundary exactness, split-
preserve)." The renderer v1 (ADR-014) shipped constant per-placement
`alpha` and `offset` with the explicit note that keyframes land by making
"any time-dependent value an exact rational at the frame's pts". The
asset binding (W6) was deliberately kept OUT of ove-timeline because it
is media identity, not time; keyframes are the opposite case — they are
exact-time document state that structural verbs must TRANSFORM (Split
must slice them), so they live where the verbs live. Constraints
unchanged: exact rationals (ADR-007), command-driven state with exact
inverses (ADR-009), replay/hash determinism (E-003/E-012), libav
confinement (untouched — zero libav surface in this wave).

## DECISION

1. **Keyframes live on the Clip** (`ove_timeline::Clip.properties:
   ClipProperties`), with LOCAL clip times (0 = clip start). Consequences,
   each pinned by test: Move carries the animation untouched (local times
   are placement-independent); Remove/Insert round-trips ride the clip
   value; **Resize leaves keys untouched** — keys beyond the duration are
   inert data (the evaluation domain is [0, dur); growing the clip back
   re-activates them, which is exactly why Resize's duration-only inverse
   stays exact; S9 pins this); **Split slices** (see 4).
2. **Property set v1 = the software renderer's animatable inputs exactly**:
   `opacity` (blend alpha, values validated ∈ [0, 1] at command time —
   the renderer's blend domain, never a render-time surprise), `x`, `y`
   (integer translation, unbounded in the document). Fixed struct fields
   in declaration order — no maps, so hashing/serialization order is
   structural. New animatable properties (scale, rotation, volume) extend
   this struct; they are NOT v1.
3. **Interpolation = per-key out-interp, Linear | Hold**. Linear is the
   exact rational lerp v(t) = v0 + (v1−v0)·(t−t0)/(t1−t0) — every step
   checked i128 arithmetic (ADR-007 fail-fast; the strict-monotonic
   invariant makes the reciprocal division-by-zero-free). Evaluation is
   exact at every boundary: t ≤ first key → first value; t ≥ last key →
   last value; t exactly on a key → that key's value; empty track → None
   (caller falls back to the static value). S7 pins evaluation against a
   naive oracle over 300 seeded layouts.
4. **Split slices the animation with the computed boundary value inserted
   on BOTH halves** — the split-preserve rule. Left gets keys < at plus a
   key at `at` (value = evaluate(at); inert for the half-open display
   domain but LOAD-BEARING: it terminates the interpolation of the segment
   before it). Right gets keys ≥ at shifted by −at, plus a key at local 0
   with the cut segment's out-interp when no key existed exactly at `at`
   (a key at `at` shifts to 0 carrying its own interp — Hold/Linear
   semantics survive the cut; S8 pins interp preservation separately).
   The property is STRONG: for every key layout and split point,
   evaluating left on [0, at) and right (shifted back) on [at, dur)
   reproduces the original evaluation EXACTLY (S8, 300 seeded cases at
   half-tick probe resolution). The Split inverse batch extends with
   SetKeyframes restores per animated property — undo/redo stay hash-
   exact (S9).
5. **The command is a wholesale replace**: `Command::SetKeyframes {
   track, id, property, keys }` — validated loudly (negative time, non-
   strictly-increasing → typed errors; opacity out of [0,1] → typed
   error; the engine never sorts or dedupes caller data silently). The
   exact inverse is the PREVIOUS key list under the same command shape
   (set is its own inverse family) — undo, redo, log replay, and
   compaction need zero new machinery (P-9 pins save/reopen hash equality
   through SetKeyframes AND through a split inverse batch embedding
   SetKeyframes restores). Empty keys = clear animation (static value
   takes over).
6. **Animation is document state**: clip state hash mixes the three
   tracks (tagged per property, empty tracks mix a zero count);
   StateMirror serializes keys as exact {num, den} pairs with a
   serde-default `properties` field (v0.1/v1 pre-animation snapshots
   load unchanged); the log grammar gains one payload (`set_keyframes`)
   whose embedded inverse needs no extra grammar. Existing hash VALUES
   change (new fields mixed in) — nothing pins absolute hashes; every
   hash claim in the suites is relative (equality between states), which
   the 157/157 run confirms.
7. **The engine evaluates; the renderer consumes** (the compile rule
   already written into ove-render at W2): `build_render_input(output, t)`
   evaluates each placement's tracks at LOCAL time (t − start; speed 1
   in v1) and bakes exact rationals into `Placement.alpha`; geometry is
   converted to the renderer's i32 domain by **round-half-up**
   (`Rational::round_half_up`, ove-time P13 — the exact floor(x + 1/2),
   matching the u16 round-half-up blending convention). P13 also PROVES
   the conversion is total over representable rationals (den ≥ 2 bounds
   |value| ≤ MAX/2; den = 1 is an exact integer), so the fail-fast panic
   arm is unreachable by construction; an i32-range overflow at the
   engine boundary is a typed `KeyframeValueOutOfRange` error, never a
   saturation. Render determinism with animation is pinned (W8: two
   renders byte-identical; plan carries the evaluated alpha/dx verbatim).

## CONSEQUENCES

- The directive's W6 vertical-slice chain now supports per-frame animation
  end-to-end: commands → replay → decode → RenderPlan → software render →
  encode, with the animation exact at every hop.
- Replay/snapshot/compaction/kill-9 semantics hold unchanged (the new
  command flows the existing machinery; P-9 + S9 prove it).
- **Named gaps (not hidden)**: no bezier/step-ease curves (Linear/Hold
  only); no per-property static base value distinct from the unanimated
  default (the static value IS the fallback); audio keyframing (volume
  automation) not wired into the audio assembly; speed is not animatable
  (retime itself is still a named seam gap, ADR-013); the boundary-key
  insertion doubles key data at every cut inside an animation (accepted:
  deterministic, exact, and the alternative — seam-time evaluation —
  would push render-time cost into every frame forever).
- **Reopen conditions**: a curve family beyond Linear/Hold (adds a per-key
  enum variant + oracle cases); an animatable property outside the
  renderer's placement inputs (extends ClipProperties + the engine eval);
  speed≠1 keyframe time mapping (rides the retime verb decision, ADR-013
  S1); a UI that needs sub-tick key granularity (times are rationals — no
  engine change, only caller discipline).
