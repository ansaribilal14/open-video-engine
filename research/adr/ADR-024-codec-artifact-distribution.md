# ADR-024: Codec-artifact distribution strategy v1 — LGPL-only adapters, libav-free core, no vendored binaries

- Status: ACCEPTED
- Date: 2026-09-30 (WAVE 21)
- Consumers: ove-decode, ove-encode (adapter allowlist), release engineering (D10),
  docs/research/35_LICENSES.md, SECURITY.md, CI (libav-confinement, cargo-audit)

## Context

The README has deferred the distribution decision to this audit since the
takeover: "Distribution strategy for codec-dependent artifacts remains gated
on the wave-21 production-readiness audit." The charter (dependency-license
conversation) requires, for every dependency: redistribution, static vs
dynamic linking, attribution, source obligations, commercial compatibility,
and patent terms. The license audit (docs/research/35_LICENSES.md) is
committed and source-verified; its load-bearing rows are:

- **FFmpeg (libavformat/libavcodec/libavutil/swscale)**: LGPL-2.1+ when built
  WITHOUT `--enable-gpl` / `--enable-nonfree` — GREEN **with flags**; RED if
  a GPL build is ever linked [S-2c1][S-2c2]. The documented ecosystem
  posture (LGPL compliance checklist, no GPL/nonfree build flags) is the
  established practice; it is recorded as UNVERIFIED as a legal shield.
- **GPL contamination**: linking GPL code into an LGPL build makes the whole
  build GPL; GPL capabilities must ship as SEPARATE, optional, out-of-tree
  artifacts (doc 35 §5) — e.g. an encoder pack, never in-tree.
- **Engine's own code**: dual-licensed MIT OR Apache-2.0; core crates are
  libav-free by construction (CI-enforced closed adapter allowlist
  {ove-decode, ove-encode} — DECODER_SPEC §5 + ENCODER_SPEC §5).

The engine TODAY builds two kinds of tree, and they must not be conflated:

1. The **libav-free core** (12 of 13 crates): time, timeline, media, render,
   project, engine session, CLI, script, MCP, plugin, conformance — no libav
   symbols, CI-checked.
2. The **libav-linked adapter surface** (ove-decode, ove-encode): LGPL-only
   build profile, CI-checked.

CI also builds a BUNDLED FFmpeg 7.1 for the conformance leg (DEV_ENV recipe,
ADR-012 portability wrapper). That bundled build is a BUILD-TIME conformance
instrument on CI runners — it is NOT a distribution artifact, and this ADR
keeps those concepts separate.

## Decision

**1. The libav-free core ships under MIT OR Apache-2.0 without
qualification.** A consumer that needs no codec adapters can take every
crate except ove-decode/ove-encode (or use the typed `Unsupported` decode
surface) under plain MIT OR Apache-2.0. The confinement gate is what makes
this statement auditable rather than aspirational.

**2. Codec-capable builds are LGPL-2.1+-compliant separate artifacts.**
Any distributable that includes ove-decode/ove-encode linkage:
- links libav* DYNAMICALLY (no static LGPL linkage in v1 — static relinking
  mechanics and object-file offers are the named future leg);
- is built ONLY from LGPL-only FFmpeg (no `--enable-gpl`, no
  `--enable-nonfree` — the build-flag discipline is checkable from
  `ffmpeg -version` / build config);
- carries the LGPL-2.1 license text, attribution for libav*, and a
  source-availability pointer (FFmpeg upstream + the exact build recipe in
  DEV_ENV.md) — satisfying the source-offer obligation for dynamic linking;
- does NOT relicense or weaken the engine's MIT OR Apache-2.0 code — the
  LGPL obligations attach to the libav-linked artifact, not to the engine's
  own sources.

**3. GPL codec capabilities never enter this repository's build.** An
encoder pack (x264 or other GPL components) would be a SEPARATE, optional,
out-of-tree artifact with its own distribution terms (doc 35 §5). The v1
encode surface stays native mpeg4 (LGPL, dependency-free); OpenH264/SVT-AV1
remain typed `Unsupported` until their own license legs are executed.

**4. No vendored FFmpeg binaries are distributed in v1.** The bundled-FFmpeg
path exists for CI conformance and the DEV_ENV build recipe (source-built,
portability-wrapped). Distributing prebuilt libav binaries would add
patent-flag and source-completeness obligations that no failing test has
forced yet (anti-overbuild) — the release pipeline leg (D10) will revisit
with evidence when real distribution demand exists.

**5. Patent posture is recorded, not claimed.** H.264/AVC decode in the
adapter is a libav capability; the license audit records the ecosystem
practice and the UNVERIFIED-legal-shield caveat. No patent-freedom claim is
made or implied anywhere in this repository's docs.

## Consequences

- The distribution story is now stated and enforceable: the CI confinement
  job keeps the adapter set closed, cargo-audit + deny.toml keep the
  dependency tree licensed, and the LGPL build-flag discipline is a recipe
  (DEV_ENV) rather than a hope.
- The gap between "buildable" and "distributable" is explicit: release
  tooling (artifact assembly, license-notice bundling, checksums,
  provenance) remains the named D10 leg; until it lands, releases are
  source-checkout + the documented reproducible build.
- Reopen conditions: a real distribution demand requiring static LGPL
  linkage or vendored binaries; a GPL-capability requirement (→ out-of-tree
  pack); a codec whose license leg fails the doc-35 verdict table (e.g.
  rubberband RED row) entering an adapter.

## Confidence

0.85. The decision follows the repository's own source-verified license
audit and the documented FFmpeg/GStreamer ecosystem practice; it invents no
tooling ahead of demand. What would reopen this ADR: the named release
pipeline landing (v1 mechanics then get concrete); a legal review
contradicting the UNVERIFIED-legal-shield caveat; a distribution
requirement for static linkage or vendored binaries.
