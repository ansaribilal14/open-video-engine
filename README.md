# Open Video Engine (OVE)

A research-driven, open-source **universal video engine**: media pipeline + timeline +
compositor + project model + render engine + AI/agent API + plugin system + headless
automation — designed so that editors (mobile / browser / desktop), AI agents, scripts,
and servers are all **clients of the same engine**.

> **Mission directive**: [`docs/MISSION_DIRECTIVE.md`](docs/MISSION_DIRECTIVE.md)
>
> Operating principle: `LEARN → VERIFY → COMPARE → EXPERIMENT → BENCHMARK → ARCHITECT →
> PROTOTYPE → IMPLEMENT → TEST → BENCHMARK → AUDIT → FIX → REPEAT`

## Repository layout

```
docs/
  MISSION_DIRECTIVE.md        # The governing mission directive (verbatim)
  MASTER_ENGINE_STATE.md      # The single current work order (update every wave)
  REALWORLD_VALIDATION.md     # Real-world certification trail (RLW-1..N, binding per wave)
  SECURITY.md                 # Security posture: untrusted-input budgets, reporting
  CONFORMANCE_REPORT.md       # Cross-platform conformance records (L-0/L-2/L-3)
  DEV_ENV.md                  # Canonical unprivileged build recipe
  specs/                      # Normative contracts (FRAME_CONTRACT, DECODER_SPEC, ...)
  research/                   # Numbered research documents (01..45, per directive)
research/
  adr/                        # Architecture decision records (001..024)
  audit/                      # Forensic audits incl. FRESH_TAKEOVER_AUDIT.md
  sources/                    # SOURCE_LEDGER.md — every source, recoverable
  claims/                     # CLAIM_EVIDENCE_LEDGER.md — beliefs tracked to evidence
  experiments/ gates/ risks/ unresolved/ synthesis/      # evidence + decision tracking
engine/                       # The engine workspace (Rust, 13 crates):
  ove-time                    # exact rational time (ADR-007)
  ove-timeline                # edit verbs with exact inverses, undo/redo, AVL (E-012)
  ove-media                   # asset identity (content hash), FrameEnvelope, frame pool
  ove-decode                  # decoder abstraction + FFmpeg-SW adapter — one of the TWO
                              #   crates allowed to link libav* (with ove-encode; CI-enforced
                              #   closed allowlist, DECODER_SPEC §5 / ENCODER_SPEC §5, ADR-015)
  ove-render                  # deterministic software reference renderer (GPU = backend only)
  ove-encode                  # encoder + muxer + export routes (stream copy / re-encode);
                              #   the second (and last) libav-linked adapter crate (ADR-015)
  ove-engine                  # headless session: import → commands → persistence → export
  ove-project                 # manifest + append-only command log + snapshots (ADR-008)
  ove-cli                     # human CLI client of the engine session
  ove-mcp                     # MCP stdio JSON-RPC server client (AI agents)
  ove-script                  # Rhai scripting client (same command semantics)
  ove-plugin                  # process-tier plugin client (capability default-DENY, ADR-021)
  ove-conformance             # cross-platform conformance runner (host / wasm / android)
scripts/                      # corpus generation, CI guards, real-world acquisition
STATUS.md                     # Honest subsystem classification — AUDIT-DAY SNAPSHOT
                              #   (historical; see "Current phase" below)
```

## Rules this repository enforces on itself

1. **No architecture without evidence.** Major claims live in the claim/evidence ledger
   with confidence levels, counter-evidence, and (where feasible) experiments.
2. **No fake completion.** Every subsystem is classified honestly in `STATUS.md`.
   Documentation is never substituted for engineering.
3. **No architectural fashion.** Subsystems justify themselves; smallest sufficient
   architecture wins.
4. **Engine first.** Applications, AI, scripting, MCP, and plugins are clients.
5. **Non-destructive by construction.** Edits are references + ranges + transforms +
   effects + commands; source media is never modified.
6. **Human and AI share one command API.** No secret alternate editing path for agents.

## Current phase

**Waves 0–21 landed, tagged `v0.1.0` (audited-state pointer); the real-world
certification trail RLW-1..9 has since completed and the state is tagged `v0.2.0`
(completion pointer): decode → render → encode/mux → persistence → engine → CLI →
audio → keyframes → GPU parity → platform evidence → conformance → headless batch →
AI/MCP → scripting → plugins → security hardening → export performance → production
audit → hostile/difficult-media corpus (7/7 pass or recorded capability limits) →
corpus-defect fixes (VFR class fully certified) → decoder input budgets.**
Workspace tests green; fmt/clippy clean at `-D warnings`; CI 5/5 on the tagged commits.

- **Real-world certification loop (binding per wave)**: the engine is gated on REAL
  media — a NASA public-domain source (see `docs/REALWORLD_VALIDATION.md` for the
  full provenance record). Every wave re-runs the proof on the SAME source: existing
  PASS rows stay green, the new capability is exercised on real media, and the output
  is verified independently (ffprobe + deterministic hashes). The engine has
  **processed a real source and produced a verified edited segment**, and the trail
  now extends to a hostile/difficult-media corpus (portrait/rotation, VFR, long-GOP,
  audio variants): **7/7 items pass or carry only recorded capability limits, with
  zero untyped deaths** — real defects found by real media are recorded in the
  validation doc (REALWORLD-BUG-1..4, RLW-8-F1..F6, two typed defects since FIXED).
- **Security posture**: every untrusted-input boundary (project files, plugin stdout,
  scripts, MCP lines, media) carries a declared resource budget that fails typed —
  see `SECURITY.md` and ADR-022.
- **Performance**: the export path holds a decoder-session budget (one open per
  source per export, ADR-023) pinned by a deterministic open-count conformance test;
  the real-media proof export is byte-identical pre/post optimization
  (129.90 s → 22.26 s, ~5.8×, machine-relative).
- **Decoder input budgets (RLW-9, ADR-024)**: hostile declared geometry is rejected
  typed (`BeyondDeclaredLimits`) at the probe/import boundary AND decoder open —
  `DECODE_MAX_DIM = 16384`, `DECODE_MAX_PIXELS = 2^25` — before any
  geometry-scaled allocation (see `SECURITY.md`).
- **Release surface**: tags `v0.1.0` / `v0.2.0` are audited-state pointers, not
  artifact releases; see `CHANGELOG.md` and the versioning note in
  `research/audit/PRODUCTION_READINESS_AUDIT.md` §3.2.

**Current work order**: `docs/MASTER_ENGINE_STATE.md` (single source of truth —
`STATUS.md` is the audit-day snapshot and is kept as a historical record; per-wave
truth lives only in the state doc). The planned wave series (… 19 perf →
20 docs/release → 21 production audit) is COMPLETE; the repository continues
under the real-media certification loop, with the named residuals tracked in
`research/audit/PRODUCTION_READINESS_AUDIT.md`.
Detailed per-subsystem classification as of the takeover audit: `STATUS.md`.

## License

The engine's own code is dual-licensed `MIT OR Apache-2.0` (LICENSE-MIT,
LICENSE-APACHE, per-crate `license` fields). Dependency license discipline is
audited in `docs/research/35_LICENSES.md`; libav* linkage is confined to the
adapter crates `engine/ove-decode` and `engine/ove-encode` (CI-enforced closed
allowlist, DECODER_SPEC §5 + ENCODER_SPEC §5) and configured LGPL-only
(DECODER_SPEC §4). Distribution of codec-dependent artifacts follows
ADR-024: the libav-free core ships under MIT OR Apache-2.0 without
qualification; codec-capable builds are LGPL-2.1+-compliant separate
artifacts (dynamic linkage, no GPL/nonfree build flags, license notices +
source pointer); GPL codec capabilities ship only as out-of-tree packs and
never enter this repository's build; no vendored FFmpeg binaries are
distributed in v1.
