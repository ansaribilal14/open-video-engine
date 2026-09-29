# CONFORMANCE REPORT — consolidated status (waves 0–14)

> Living report: where every suite stands, on every leg, with recorded subsets.
> Sources of truth are the suites themselves (CI-enforced); this document only
> aggregates and records what is NOT CI-visible. Companion:
> research/audit/W10-12_PLATFORM_LEGS_REPORT.md (per-leg evidence detail).

## L-0 headless/CLI — the conformance REFERENCE leg

| Suite | Scope | CI enforcement | Status |
|---|---|---|---|
| ME-1..ME-8 (engine) | command semantics, undo/redo, replay, hash | rust job (`cargo test --workspace`) | GREEN (W5–W8 suites) |
| D-1..D-12 (decoder) | probe/seek/decode/error model, conformance | rust job | GREEN |
| A-1..A-4 (audio decode) | planar-f32 surface, floor+trim, contiguity | rust job | GREEN (W7) |
| RG-1..RG-7 (render) | plan purity, goldens, retime, culling | rust job | GREEN |
| E-1..E-10 (encode/mux) | byte goldens, ffprobe-verified A/V mux | rust job | GREEN (W3/W7) |
| P-1..P-9 (project) | save/reopen/kill-replay hash equality, float rejection | rust job | GREEN |
| S7–S9 (keyframes) | evaluation oracle, split-preserve, undo hash-exact | rust job | GREEN (W8) |
| libav confinement | zero libav* outside ove-decode | dedicated job | GREEN |
| RustSec audit | lockfile scan | dedicated job | GREEN |

Workspace count at this report's date: 158/158 default-feature tests GREEN.

## L-1 desktop — GPU backend (wave 9, PR #11)

| Item | Evidence | Status |
|---|---|---|
| GPU↔software byte parity (G-1..G-8) | ove-render gpu_conformance, tolerance 0, Mesa lavapipe | GREEN (dedicated CI job) |
| Typed feature-detect + fallback | NoAdapter typed error verified; UnsupportedAlphaDen → software (G-7) | GREEN |
| REAL_GPU perf baselines | llvmpipe record committed, explicitly INVALID-class (E-005 rule) | hardware-bound residual |
| Tauri transport bench (E-006b) | not runnable in this container | pending (deps-bound) |

## L-2 android — compile-level evidence (waves 10–12, PR #12)

| Item | Evidence | Status |
|---|---|---|
| Pure-Rust core compiles for aarch64-linux-android | ove-time, ove-timeline, ove-media, ove-render, ove-project, ove-conformance `cargo check` GREEN (CI job platform-conformance) | COMPILE-LEVEL |
| blake3 NDK independence | target-scoped `pure` feature (identical digests) | fixed (W10–12 finding #2) |
| DEVICE level: E-004c JNI, MediaCodec D-suite subset, FGS drill | requires device + NDK | NOT claimed |

## L-3 browser — WASM core evidence (waves 10–12, PR #12)

| Item | Evidence | Status |
|---|---|---|
| §3 invariant table native≡WASI | ove-conformance platform_core: state_hash, reopen hash, keyframe eval, exact rational spots — byte-identical under wasmtime 36.0.1; CI REQUIRES the empty diff | GREEN (core level) |
| Project persistence portability | LogWriter explicit seek-to-end (wasmtime O_APPEND gap fixed, W10–12 finding #1) | fixed |
| WASM core build | wasm32-wasip1 build GREEN | GREEN |
| WebCodecs decode leg (E-001) | historical 48/48 software-GPU run (2026-09-24 record) | recorded subset |
| Browser shells (Chromium/FF/Safari matrix, OPFS, importExternalTexture) | requires browsers | NOT claimed |

## Declared divergences (plan §4 — none silent)

- GPU backend: reduced-denominator bound 65 000 → typed error → software
  fallback (by construction; G-7 pins it).
- WASI: project persistence uses explicit-seek appends (POSIX-equivalent for
  the single writer; the O_APPEND flag itself is not portable — W10–12).
- Android: blake3 pure backend (identical digests, different implementation).

## What would falsify this report

- Any CI job turning red (suites are the authority, not this file).
- A platform target producing a different platform_core output (the CI diff).
- An adapter silently degrading instead of declaring a capability limit
  (D-9) — found by the D-suite error model or the platform diff.
