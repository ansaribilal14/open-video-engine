# DEPENDENCY AUDIT (forensic, 2026-09-26)

> Question answered: what does the repo actually depend on today, what will the slice
> need, and are the candidate dependencies license/security/version-safe? Method:
> cargo tree / Cargo.toml inspection of all tracked Rust packages; doc 35 RED/GREEN
> cross-check; supply-chain rules from doc 33.

## 1. Current dependency surface (small by design)

| Package | Where | Version pinned | License (declared) | Audit note |
|---|---|---|---|---|
| ove-time | engine/ove-time | **zero runtime deps** (Cargo.lock: self only) | n/a | matches "no deps until needed" discipline; property tests use no crates (seeded xorshift RNG hand-rolled — deliberate) |
| E-004b_uniffi | scripts/experiments/ | uniffi 0.32 (+cli), serde untouched | MPL-2.0 (uniffi) | experiment-scope only; codegen output committed for reference |
| doc-35 dependency audit | research | — | — | 32-row RED/GREEN table exists (verified by GATE-13 evidence row); audited, not re-derived here |

Toolchain (not repo deps): rustup 1.98.1 stable (re-installed twice after env resets — CI
will fix reproducibility), Node/Chrome used by E-001/E-006 harnesses, ffmpeg CLI used by
E-007/E-007b (container presence verified by result files; version not pinned in repo —
pin when ove-encode lands).

## 2. Candidate dependencies for the vertical slice (from ADRs 003–008; to verify at integration)

| Need | Candidate | License | RED/GREEN (doc 35) | Risk |
|---|---|---|---|---|
| Demux/decode/encode (desktop/headless) | FFmpeg libav* via bindings, **LGPL-only build** | LGPL-2.1+ | GREEN with build discipline | API churn (R-04); x264 GPL exclusion already decided (ADR-005) |
| Software AVC fallback | openh264 (Cisco) | BSD-2 + patent grant | GREEN | encoder quality/params; binary redistribution terms to re-verify |
| Software AV1 | SVT-AV1 / rav1e | BSD / MIT | GREEN (AV1 §1.3 defensive termination verified verbatim) | speed on low-end |
| Audio decode | symphonia | MPL-2.0 | GREEN | coverage vs FFmpeg — adapter decides |
| Browser mux/demux | mediabunny | MIT | GREEN | version churn (v1.29 observed in OpenCut) |
| Android encode | MediaCodec (platform) | platform | GREEN | device quirk matrix (R-01 family) |
| Undo/state props | proptest or hand-rolled seeded loops | MIT/Apache | GREEN | hand-rolled keeps zero-dep discipline; decide per crate |
| CLI args | clap | MIT/Apache | GREEN | only for ove-cli |
| **Rejected for core** | x264 (GPL), GStreamer (mobile footprint decision), ffmpeg CLI as engine runtime path (sidecar-parse rule from doc 33 applies to untrusted media) | — | RED/conditional | recorded in ADRs |

## 3. Supply-chain rules that bind the slice (from doc 33, verified in corpus)

1. Untrusted media parsing isolated (sidecar/worker/wasm) — decoder adapters must be the
   only libav* linkage surface; core stays clean.
2. `cargo audit` + `cargo deny` in CI from the first workspace commit (D-3 CI work).
3. Pin + lockfile discipline: every crate commits Cargo.lock (ove-time already does).
4. No proc-macro-heavy stacks in core without review (compile-time + supply chain).
5. License files must ship in-repo (see LICENSE_AUDIT — currently absent, flagged there).

## 4. Verdict

Current dependency hygiene is **exemplary for a research repo** (zero-dep core crate,
experiment-scoped deps, lockfiles committed). All slice candidates are license-vetted by
doc 35 with two open verifications (openh264 redistribution terms; mediabunny version
pin). The dependency plan supports the architecture without RED-flagged contamination.
