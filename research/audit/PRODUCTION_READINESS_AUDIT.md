# PRODUCTION READINESS AUDIT — WAVE 21 (2026-09-30)

> Charter rule (NO FAKE COMPLETION): the words "production ready",
> "world class", "best", "state of the art", and "complete" are forbidden
> as declarations unless evidence supports them. This audit therefore
> does NOT declare the engine production-ready. It classifies every
> production dimension using the charter's eight allowed labels, cites
> the committed evidence for each, and names the residuals that keep a
> dimension from the next label. Nothing here upgrades a classification
> without evidence committed to this repository.

Scope of the audited tree: `main` after WAVE 20 (PR #20, `1d889a0`).
Workspace 189/189 GREEN; fmt/clippy `-D warnings` clean; CI 5/5 (fmt+
clippy+tests incl. bundled-FFmpeg leg, cargo-audit, libav confinement,
platform conformance, GPU parity). Real-media certification trail
RLW-1..5 (docs/REALWORLD_VALIDATION.md); RLW-6 (this wave) certifies
the audit tree itself.

## 1. Dimension classification

| # | Dimension | Classification | Evidence | What keeps it from the next label |
|---|---|---|---|---|
| D1 | Decode → render → encode/mux core | **TESTED** (+ BENCHMARKED) | ove-decode D-1..D-12 conformance vs committed ffprobe tables (incl. 30000/1001 NTSC); ove-render golden frames + gpu_conformance 4096/4096; ove-encode E-1..E-5/E-7 gates; ove-engine integration.rs; the permanent real-media proof (RLW-1..5) | VFR/long-GOP hostile corpus leg (REALWORLD §10) not yet run; hardware codec legs absent by design (software renderer is the correctness reference) |
| D2 | Exact time + determinism | **TESTED** | ove-time 15/15 properties; replay determinism + verb-identity triples (E-012); state-hash identity save→kill→reopen→re-export byte-identical (realworld.rs, every RLW wave); export sha256 pinned identical ACROSS an optimization (RLW-4/5) | Nothing tracked — the discipline is regression-guarded in CI |
| D3 | Persistence & recovery | **TESTED** | ove-project acceptance.rs 13/13 (manifest + append-only log + snapshot compaction per PROJECT_FORMAT_SPEC) + security.rs 4/4 (read-time budgets, path validation); deterministic reopen in the real-media proof | Multi-machine recovery drill (E-011 upload drill) hardware/integration-gated |
| D4 | One command surface — human + AI | **TESTED** | E-009 36/36 (MCP vs direct equivalence); W15 MCP conformance; W16 Rhai scripting gate; W17 plugin tier gate (default-DENY capabilities, same-grammar proposals); every client re-gated on real media (RLW-2/3/5) | User-approval ladder step for plugin capabilities (ADR-021 reopen list); Tier-B WASM prototype absent by design |
| D5 | Security posture | **TESTED** | ADR-022: declared read-time budgets at all five untrusted surfaces, typed failures; hostile-flood conformance (pure + wire); OOM-bomb reproducibly converted SIGKILL→typed error; SECURITY.md; RLW-3 hostile-flood-on-real-media gate | Log-flood CPU budget (Tier-B sandboxing conversation); decoder pixel-bomb caps deferred to the hostile-media corpus leg (ADR-022 residuals) |
| D6 | Performance | **BENCHMARKED** (machine-relative) | ADR-023: deterministic decoder-open budget (1 open/source/export, pinned in export_session_budget.rs 3/3); real-media proof 129.9 s → 22.3 s (5.8×) with BYTE-IDENTICAL output; per-leg instruments in realworld_record.json | Wall-clock numbers are machine-relative (recorded as such); per-placement session split, threading/pool wiring, GPU perf = named future legs |
| D7 | Platform conformance | **PARTIAL** | L-0 host conformance green; wasm32-wasip1 run + aarch64-android check green IN CI (ove-conformance, CROSS_PLATFORM_CONFORMANCE_PLAN §3); Wave 10–12 platform-leg evidence recorded | Desktop (webkit2gtk E-006b) / mobile runtime (E-004c device JNI) / browser real-GPU legs are hardware-gated residuals — recorded in E-records, not silently dropped |
| D8 | GPU rendering | **EXPERIMENTAL** | ADR-020 render executor + wave-9 GPU parity job (lavapipe) green in CI; E-005 correctness leg 4096/4096 pixel-exact vs software reference | Real-GPU performance + zero-copy import unproven (no real GPU in any environment so far); GPU stays a backend, never the reference |
| D9 | Licensing & distribution | **TESTED** (audit) + ADR-024 | docs/research/35_LICENSES.md dependency table (source-verified verdicts); LGPL-only FFmpeg posture documented [S-2c1][S-2c2]; cargo-audit + deny.toml in CI; ADR-024 (this wave) fixes the v1 distribution decision | Distribution TOOLING (artifact assembly, license notice bundling, source-offer mechanics) does not exist yet — named leg, not a silent gap |
| D10 | Release engineering | **PARTIAL** (deliberate) | W20 documented the deliberate absence: no tags/CHANGELOG/pipeline was invented before the audit that scopes them (anti-overbuild). This audit (§3) blesses the v0.1.0 state pointer and the versioning rule | Release pipeline (artifact assembly + checksums + provenance) = the named future leg; until it exists, releases are source-checkout + documented DEV_ENV build |
| D11 | Real-media certification | **TESTED** | RLW-1..5 trail on ONE identity-gated NASA source (public domain, full provenance record): every wave re-ran the proof; 4 real defects found and fixed BY real media (REALWORLD-BUG-1..4); outputs independently verified each wave; RLW-6 re-certifies the audited tree | Harder REALWORLD/ corpus (portrait/VFR/long-GOP/audio-variant) PLANNED — the loop is in place, the corpus is not yet acquired |
| D12 | Documentation integrity | **TESTED** | MASTER_ENGINE_STATE single-truth rule enforced every wave; RLW-5 proved the §9 reproducibility chain executable (baseline regenerated from scratch, exact match); ADR-023 gap closed; README truthful as of W20 | STATUS.md remains a frozen audit-day snapshot BY RULE (§6) — new readers must be routed via README (done) |

## 2. The verdict, in charter language

- The **headless engine core** (D1, D2, D3, D4, D5, D11, D12) is
  **TESTED**, with the performance dimension **BENCHMARKED** and the
  determinism core regression-guarded in CI. This is the strongest,
  evidence-complete part of the repository, certified on real media
  six waves in a row.
- The dimensions that BLOCK a production claim are explicit and
  hardware/integration-gated: **D7 PARTIAL** (real-device platform
  legs), **D8 EXPERIMENTAL** (real-GPU), **D10 PARTIAL** (release
  tooling). Per the charter these labels stay until their evidence
  exists — the reopen conditions are named in the table and in the
  ADR/experiment records.
- Therefore the honest statement is: **the engine's documented v1 scope
  (headless, software-rendered, exact-time, real-media-certified core)
  is TESTED end to end; the engine as a whole is NOT classified
  production-ready**, because three production dimensions are PARTIAL/
  EXPERIMENTAL with named reopen conditions.

## 3. Decisions taken by this audit

1. **Distribution strategy for codec-dependent artifacts (ADR-024)**:
   the libav-free core ships under MIT OR Apache-2.0 without
   qualification; libav-linked adapter builds are LGPL-2.1+-compliant
   separate artifacts (dynamic linkage, no GPL/nonfree flags —
   CI-audited by the confinement job + cargo-audit); GPL encoder packs
   stay out-of-tree per doc 35 §5. Full reasoning in ADR-024.
2. **Versioning rule (recorded, machinery-free)**: versions are
   wave-indexed (`v0.<wave>` posture in docs since the takeover; crates
   stay `0.1.0` until a release pipeline exists — mass version-bumping
   without artifacts would be documentation-only progress). The state
   after this audit is tagged **v0.1.0** as an audited-state POINTER —
   it is not a completeness, quality, or production-readiness claim.
3. **Post-W21 posture**: the wave plan (…19 perf → 20 docs/release →
   21 production audit) is COMPLETE. The repository continues under the
   certification loop only: any future engineering wave re-runs the
   real-media gate and updates MASTER_ENGINE_STATE. The named residuals
   (platform runtime legs, real-GPU, release pipeline, hostile corpus,
   RW-NOTE-1 clip_assets binding, log-flood CPU budget) stay tracked
   with their reopen conditions — they are the roadmap, not forgotten.

## 4. Method note

Every classification above cites committed evidence (tests, conformance
suites, ADRs, certification records) — none is asserted from prose.
Where an environment lacked hardware (GPU, devices), the dimension was
downgraded honestly rather than argued around. This audit updates
research/audit/ in place (it is an audit record, not a status doc) and
does not touch STATUS.md (frozen per the §6 rule).
