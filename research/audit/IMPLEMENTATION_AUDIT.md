# IMPLEMENTATION AUDIT (forensic, 2026-09-26)

> Question answered: what engine code actually exists, what state is it in, and what does
> its test surface actually prove? Method: every git-tracked source file under engine/ and
> scripts/ inspected; test suite re-executed; line counts and API surfaces read.

## 1. Complete implementation inventory

| Component | Path | Lines | State (4-dim) | Proof |
|---|---|---|---|---|
| ove-time crate | engine/ove-time (4 files tracked) | 399 (192 lib + 207 tests) | IMPL EXPERIMENTAL · VALIDATION SOFTWARE | **live re-run this audit: 15/15 PASS** (cargo 1.98.1, --release) |
| E-004b UniFFI experiment crate | scripts/experiments/E-004b_uniffi/ (7 files) | ~1,400 gen. Kotlin + Rust lib + Kotlin test | EXPERIMENT (not engine code) | E-004b_result.txt 8/8; NOT re-run this audit (needs JVM/kotlinc — not installed in container) |
| Experiment scripts (E-001..E-009) | scripts/experiments/ (21 files) | — | EXPERIMENT | raw outputs in experiments/; all tracked EXCEPT E-006a (see D-1) |
| **Total engine implementation** | — | **399 lines** | — | — |

**That is the entire engine.** No timeline crate, no command bus crate, no media crate,
no decoder, no renderer, no project system, no CLI. `engine/crates/ove-time/` contains
only an orphaned build cache (untracked target/ artifacts, zero source).

## 2. ove-time — code-level review (the one real artifact)

**What it is:** exact rational time (`i64 num / i64 den`, normalized, den>0), i128
intermediates, saturate-via-panic overflow policy, fp forbidden on the authoritative path
(`from_f64_seconds_quantized` quarantined as display/legacy-only).

**Correct properties actually encoded (verified by re-run):**
- P1 associativity, P2 normalization invariants, P3 order-invariant sums on fixed tick
  axis (200 cases × 50–550 clips × 3 orders), P3b **free-rational accumulator overflow as
  a regression guard** (asserts the i64 lcm-chain blowup DOES happen — a discovered
  failure mode turned into a permanent test), P4 boundary floor exactness across 5 rates
  × 5000 frames × both sides, P5 23.976-alias no-drift over 172,262 frames, P6 add/sub
  roundtrip, P7 split invariant (both parities), P8 total order vs independent i128 eval,
  P9 decimal-alias drift existence, P10 large-magnitude exactness (20k cases).

**Honest limitations found by this audit (not bugs — declared scope):**
1. `half()` uses arithmetic shift (floor) — fine for the split invariant, but semantics
   for negative durations are untested; the timeline crate must test split at negative
   positions when they exist.
2. Overflow policy = panic, not typed error. Correct for ove-time (documented), but the
   FFI layer (E-004b pattern) must continue translating panics to typed errors — this is
   E-004a's proven pattern and must be preserved in every future crate's FFI surface.
3. No `div()`, no `rescale()` (E-004b tested rescale via the FFI crate's own logic) — the
   timeline crate will need exact division/rescale; those must land with the same
   property-test discipline.
4. No serialization — deliberate (project format is ove-project's job), but the (num,den)
   schema rule from E-003 must be mirrored exactly.

**Verdict:** ove-time is small, honest, and does exactly what ADR-007 requires. Its
discipline (property tests as acceptance evidence, honest test-of-own-test fixes recorded)
is the standard every new crate must meet. IMPL EXPERIMENTAL is the right label — it
becomes IMPLEMENTED only when consumed by a real timeline crate with integration tests.

## 3. Scripts/experiments as implementation evidence

| Experiment | Re-runnable from repo? | Notes |
|---|---|---|
| E-001 (.cjs CDP harnesses) | YES (needs Chrome; GPU legs SwiftShader-blocked as documented) | packets + result JSONs committed |
| E-002 (py) | YES | result committed |
| E-002b/c/c2 (rs) | YES | results committed; E-002c2 = addendum under random positions |
| E-003 (py) | YES | 9/9 |
| E-004a (rs+py driver) | YES | 5/5 |
| E-004b (crate) | YES (rustup + JDK/kotlinc needed) | 8/8 |
| E-006 (.cjs) | YES (browser leg) | result json+txt committed |
| **E-006a** | **NO — code absent** | **D-1: must re-run + commit** |
| E-007/E-007b (py) | YES (ffmpeg present in container) | mp4s local-only (D-5) |
| E-009 (py) | YES | 36/36; debug session script also committed (good hygiene) |

## 4. What "EXPERIMENTAL vs IMPLEMENTED" must mean here (anti-inflation rules)

- A crate is EXPERIMENTAL until: consumed by another crate in-tree, integration-tested,
  and covered by CI. ove-time is EXPERIMENTAL today (nothing consumes it yet — E-004b
  consumed a *copy* of its API in a scratch crate).
- Nothing may be labelled IMPLEMENTED with only Python-model evidence (E-003/E-009 are
  Python models of the design — their evidence is *architectural*, not implementation).
- Per-platform claims stay VALIDATION-level below SOFTWARE until a device/browser runs
  engine code (none has).

## 5. Build-system / tooling audit

- No workspace Cargo.toml (single crate, fine for now; create `engine/Cargo.toml`
  workspace when the second crate lands — avoid premature workspace).
- No CI (D-3). No rustfmt/clippy config (add with CI). No deny/audit config (SECURITY_AUDIT).
- Toolchain churn: env resets wiped rustup twice (sessions 3 and 4) — CI would also make
  the repo independent of container state (recorded as D-3 impact).

## 6. Implementation backlog implied by this audit (feeding ENGINE_BUILD_PLAN)

1. ove-timeline (commands + inverses + gap buffer + derived index + property suite incl.
   E-003/E-009 invariants ported to Rust).
2. ove-project (manifest + log + snapshot + replay determinism tests).
3. ove-media + ove-decode (decoder-first: probe→asset→decoder→FrameEnvelope; FFmpeg SW
   adapter; conformance suite).
4. ove-render minimal pass compiler (software raster, golden frames).
5. ove-encode (FFmpeg-based encode + stream-copy route; MP4 out; ffprobe-verified).
6. CI harness (cargo test workspace-wide + fmt/clippy + link check).
