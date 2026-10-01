# Changelog

All notable states of the Open Video Engine, newest first.

> **What the tags are (read this first).** Per the recorded versioning rule
> (`research/audit/PRODUCTION_READINESS_AUDIT.md` §3.2), the repository's tags
> are **audited-state POINTERS**, not artifact releases: they mark a tree whose
> claims are evidenced in-repo (tests, conformance suites, certification
> records). Workspace crates stay `0.1.0` until a release pipeline exists —
> mass version-bumping without artifacts would be documentation-only progress.
> The intended consumption model is source checkout + the documented build
> (`docs/DEV_ENV.md`). Codec-capable artifact distribution is specified by
> ADR-024 (codec-artifact distribution) and is a named future leg, not a
> silent gap.

## v0.2.0 — 2026-10-02 — completion pointer (`4173211`)

Engineering merged via PR #24 (`2f2e0ab`); certification re-run on the merge
commit byte-identical; CI 5/5 on `2f2e0ab` and `4173211`.

- **RLW-8-F1 FIXED — presentation-order VFR analysis** (`eebfcac`): packet-order
  pts are sorted into presentation order before the VFR delta pass; B-frame CFR
  media no longer false-positives `is_vfr=true`. The corpus harness now pins
  `probe verdict == ffprobe verdict` on ALL corpus items in BOTH directions;
  3 CI-runnable unit pins added.
- **RLW-8-F3 FIXED — zero-duration frames delivered** (`eebfcac`): container-
  declared zero durations are no longer typed `Corrupt("frame without
  duration")`; they are delivered as exact `0/1` (never inferred from a rate;
  negative stays typed corrupt). Committed CI fixture `vfr_zerodur.mp4` (8.7 KB,
  23 zero-duration packets) pins the contract. The hostile-corpus VFR class is
  upgraded to FULL certification: 96/96 export frames, reopen re-export
  byte-identical (`a5ed13ab…`), visual proof on real launch footage.
- **RLW-9 — decoder input budgets** (`eebfcac` + `c961c14`; ADR-024 "decoder
  input budgets"): declared caps `DECODE_MAX_DIM = 16384` and
  `DECODE_MAX_PIXELS = 2^25`; typed `BeyondDeclaredLimits` on `DecodeError` +
  `ProbeError`; enforced at the probe/import boundary AND decoder open — before
  any geometry-scaled allocation (the ADR-022 "decoder pixel-bomb" residual is
  CLOSED). Committed hostile-header conformance: runtime-patched v210/MOV
  fixtures reject 16400×64 and 4000×9000 typed at BOTH boundaries; unpatched
  negative control opens clean.
- **Certification invariants held**: primary export `baf23d2a…` and W17/W18
  artifact `ff5e67f8…` byte-identical through all of the above; hostile corpus
  score 7/7 pass-or-recorded-limits (portrait/rotation, VFR, long-GOP, audio
  variants), zero untyped deaths.

## v0.1.0 — 2026-09-30 — audited-state pointer (`bee72c5`)

Wave plan W0–W21 complete (PRs #5–#21); production-readiness audit D1–D12
(`research/audit/PRODUCTION_READINESS_AUDIT.md`).

- **Core**: exact rational time (i64, ADR-007); command-driven timeline with
  exact inverse verbs, undo/redo, deterministic replay (E-012/ADR-011);
  content-addressed assets (blake3); deterministic software reference renderer
  with GPU parity backend (ADR-020, tolerance-0 lavapipe conformance in CI);
  encode/mux export routes with ffprobe-gated output (ADR-015).
- **Surfaces (one command grammar, four clients)**: human CLI (ove-cli), AI
  agents (ove-mcp, MCP stdio JSON-RPC; E-009 36/36 equivalence), scripting
  (ove-script, Rhai), process-tier plugins (ove-plugin, capability
  default-DENY, ADR-021) — every client re-gated on real media.
- **Persistence**: manifest + append-only command log + snapshot compaction
  (ADR-008, PROJECT_FORMAT_SPEC P-1..P-8); save/kill/reopen/re-export proven
  byte-identical on real media.
- **Security posture (ADR-022)**: declared read-time budgets and typed failures
  at five untrusted-input surfaces (project files, plugin stdout, scripts, MCP
  lines, media); hostile 11k-proposal flood → typed budget abort with state
  hash unchanged; OOM-bomb → typed error.
- **Performance (ADR-023)**: deterministic one-open-per-source export budget,
  conformance-pinned; real-media proof export 129.90 s → 22.26 s (~5.8×) with
  BYTE-IDENTICAL output; numbers recorded machine-relative.
- **Real-world certification RLW-1..6**: NASA public-domain reference source
  (122.88 s, sha256 `2d315daf…705f`, full provenance record); real defects
  found and fixed BY real media (REALWORLD-BUG-1..4), each fixed with
  byte-identical certified output; independent verification (ffprobe + hashes)
  every wave.
- **Platform evidence**: wasm32-wasip1 native-identical scenario + aarch64
  Android compile check in CI (desktop/mobile runtime and real-GPU legs are
  hardware-gated residuals, recorded — not silently dropped).
- **Licensing (ADR-024 "codec-artifact distribution")**: libav-free core under
  MIT OR Apache-2.0; libav linkage confined to {ove-decode, ove-encode}
  (CI-enforced closed allowlist); LGPL-only FFmpeg configuration; GPL codec
  packs out-of-tree; no vendored FFmpeg binaries.

## Unreleased — final productization (docs/release surfacing only)

- Truth-in-documentation pass on this README-era state (current phase,
  libav allowlist comment, ADR range).
- This changelog added; GitHub Release objects created for `v0.1.0` / `v0.2.0`
  from these notes (release surfacing, not new engineering).
- ADR registry addendum clarifying the ADR-024 numbering collision
  (codec-artifact distribution, 2026-09-30 vs decoder input budgets, 2026-10-02).
- No engine, decoder, renderer, or security-behavior changes. No certification
  baseline changes. Workspace gates re-run; the real-media gate re-executed at
  the resulting HEAD (evidence recorded in `docs/MASTER_ENGINE_STATE.md`).
