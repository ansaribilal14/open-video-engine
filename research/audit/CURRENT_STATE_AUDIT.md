# CURRENT STATE AUDIT (forensic, 2026-09-26)

> Auditor: principal (session 4). Method: every STATUS.md / worklog / prior-summary claim
> checked against actual files, git-tracked artifacts, and (where possible) re-executed code.
> Nothing below was accepted from documentation alone. Baseline commit: `999d8de`.
> Vocabulary (mandated): RESEARCH_STATUS · ARCHITECTURE_STATUS · IMPLEMENTATION_STATUS ·
> VALIDATION_STATUS — see VALIDATION_AUDIT.md for the full matrix.

## 1. Verdict in one paragraph

The repository contains a **real, internally consistent, honestly-labelled research
corpus** (42 numbered docs, 137+72 ledgered sources, 7 evidence-backed claims, 14 risks,
13 open questions, 11 experiment records) and **exactly one piece of engine code**:
`engine/ove-time` (399 lines, 15/15 tests, re-run live by this audit and PASSING).
Everything else that prior session summaries described as implemented — notably an
`ove-timeline` crate — **does not exist in the repository**. Three repo-integrity defects
were found (missing E-006a artifacts, nine empty research subdirectories, no CI at all).
STATUS.md itself is accurate on every line we could re-verify; the *drift* came from
session summaries, not from the repo.

## 2. Claim-by-claim verification table

Legend: VERIFIED = confirmed against code/artifacts/live run · DISPROVED = repository
contradicts the claim · PARTIAL = some legs true, named legs false/missing · UNRECOVERABLE
= artifact referenced but absent.

| # | CLAIM (source) | FILE | CODE | TEST | EXPERIMENT | EVIDENCE | STATUS (4-dim) | CONFIDENCE | REMAINING VALIDATION |
|---|---|---|---|---|---|---|---|---|---|
| A1 | "ove-time TESTED 15/15" (STATUS.md) | engine/ove-time/{src/lib.rs,tests/properties.rs} = 192+207 lines | exact Rational, i128 intermediates, overflow panics, fixed-tick-axis rule encoded as P3b | **re-run live: 4 unit + 11 property PASS** | E-002 + E-002c P3b | experiments/ove-time_property_suite.txt; ADR-007 ACCEPTED | RESEARCH COMPLETE · ARCH ACCEPTED (ADR-007) · IMPL EXPERIMENTAL · VALIDATION SOFTWARE | HIGH | none for software leg; FFI legs covered by E-004a/b |
| A2 | "engine/ove-timeline exists (timeline production crate)" (prior session summary only) | **no such directory** | none | none | none | `find` returns nothing; engine/ has only ove-time | IMPL NOT_STARTED | — (claim false) | the whole Phase-2 crate |
| A3 | "CI runs the tests" (prior summary: "GitHub Actions") | no `.github/` | none | — | — | `git ls-files` shows no yml/yaml | NOT_STARTED | — (claim false) | create workflow: cargo test --release + property seeds |
| A4 | E-006a native IPC serialization RUN | research/experiments/E-006a_ipc_serialization.md (tracked) | **code absent**: `scripts/experiments/E-006a_ipc_crate/` is an EMPTY untracked dir; referenced `E-006a_ipc_serialization.rs` nowhere | none recoverable | result file `experiments/E-006a_result.txt` **missing**; only the table embedded in the record survives | git ls-files: record only | RESEARCH PARTIAL · VALIDATION UNRECOVERABLE (single-run numbers, no code, no raw output) | LOW-MEDIUM | re-run and commit code + raw output (cheap: native-only Rust bench) |
| A5 | E-001 WebCodecs decode 48/48 real media | record + scripts (E-001_cdp_v2.cjs, E-001a_flag_sweep.cjs tracked) | scripts present | result JSONs present (E-001_result.json, E-001_packets.json) | RUN (2026-09-23) | GPU-import legs blocked by SwiftShader (honest) | RESEARCH COMPLETE leg · VALIDATION SOFTWARE-ONLY | HIGH (decode leg) | E-005 real-GPU rerun |
| A6 | E-002 fp-vs-rational 13/13 | record | E-002_time_representation.py + E-002b_timeline_bench.rs tracked | E-002_result.txt present | RUN | C-001 → HIGH | RESEARCH COMPLETE · VALIDATION SOFTWARE | HIGH | none (superseded in parts by ove-time suite) |
| A7 | E-002c/E-002c2 structure bench | records | E-002c_timeline_structure.rs + E-002c2_avl_random.rs tracked | E-002c_result.txt, E-002c2_result.txt present | RUN | gap-buffer + derived index; ADR-011 PROPOSED | RESEARCH COMPLETE · VALIDATION SOFTWARE | HIGH (perf micro); MEDIUM (integration behaviour — E-012 pending) | E-012 integration benchmark |
| A8 | E-003 command-log 9/9 | record | script tracked | result present | RUN | C-007 viable; ADR-008/009 basis | RESEARCH COMPLETE · VALIDATION SOFTWARE (Python model, not Rust) | MEDIUM-HIGH | Rust property suite in real crate |
| A9 | E-004a FFI 5/5 | record | rs+driver tracked | result present | RUN | C-004 evidence | RESEARCH COMPLETE · VALIDATION SOFTWARE | HIGH | device leg = E-004c |
| A10 | E-004b UniFFI 8/8 (Kotlin live roundtrip) | record | full crate tracked (Cargo.toml: **uniffi 0.32**, generated .kt, Kotlin test, bindgen) | E-004b_result.txt present | RUN | C-004 → ADR-001 ACCEPTED | RESEARCH COMPLETE · VALIDATION SOFTWARE (JVM, not device) | HIGH | E-004c on-device JNI |
| A11 | E-006 browser IPC leg | record | script tracked | E-006_result.{json,txt} present | RUN (browser leg) | JSON-vs-binary rule grounded | PARTIAL-RUN (honest) | HIGH | E-006b real-Tauri |
| A12 | E-007/E-007b smart-render | records | both scripts tracked | result txts present; **mp4 artifacts local-only** (*.mp4 gitignored) | RUN | keyframe-index mandatory; 6.8–11.9× copy | RESEARCH COMPLETE · VALIDATION SOFTWARE | HIGH | cross-encoder stitch leg (ADR-005 risk) |
| A13 | E-009 shared command surface 36/36 | record | script tracked | result present | RUN | Q-09 ANSWERED; ADR-010 ACCEPTED | RESEARCH COMPLETE · VALIDATION SOFTWARE | HIGH | E-013 real Claude Code E2E |
| A14 | "137 sources ledgered" (STATUS.md) | SOURCE_LEDGER.md = 137 S-rows; +LEDGER_W3a..f = 72 W3-rows | — | — | — | counted by script | RESEARCH COMPLETE (ledgering) | HIGH | source-coverage gaps → SOURCE_COVERAGE_AUDIT.md |
| A15 | "7 claims / 12→14 risks / 13 questions" | CLAIM_EVIDENCE_LEDGER.md (C-001..C-007), RISK_REGISTER.md (R-01..R-14), OPEN_QUESTIONS.md (Q-01..Q-13) | — | — | — | counted | COMPLETE | HIGH | — |
| A16 | "42 research docs, wave-3 13 docs" | docs/research/ 01..44 (no 42/43 files; risks/questions live under research/) | — | — | — | wc = 42 files, 8,135 lines | PARTIAL by design (depth) | HIGH | depth passes → RESEARCH_COMPLETENESS_AUDIT.md |
| A17 | GATE_STATUS "10/14 UNDERSTOOD, 4 PARTIAL" | research/gates/GATE_STATUS.md | — | — | — | read in full; residuals named (E-005/E-004c/E-006b/testing doc) | ACCURATE | HIGH | hardware legs |
| A18 | ADR statuses | 11 ADR files | — | — | — | 001/007/010 ACCEPTED; 002–006/008/009/011 PROPOSED | ACCURATE (ADR-002 header text is STALE vs GATE_STATUS — flagged) | HIGH | — |
| A19 | "no fake completion" (README rule) | STATUS.md vocabulary respected; empty dirs labelled; 44 whitepaper is a 5-line PLANNED stub | engine/ has exactly 1 crate and says so | — | — | no IMPLEMENTED/BENCHMARKED label found on anything not evidenced | **HOLDS** | HIGH | keep under audit |
| A20 | Prior-summary claim "UniFFI 0.29" | — | E-004b Cargo.toml says **0.32** | — | — | worklog-vs-code mismatch resolved in favour of code | — (summary artifact only) | — | — |

## 3. Defects found (repo-integrity, not honesty)

| ID | DEFECT | IMPACT | REQUIRED FIX |
|---|---|---|---|
| D-1 | E-006a: implementation code + raw result unrecoverable; index row claims "Code: E-006a_ipc_serialization.rs" | directive's "every important source must be recoverable" violated for one experiment | re-run bench, commit code + raw output (half-day) |
| D-2 | 9 research subdirectories empty (github/ papers/ youtube/ transcripts/ conferences/ architecture/ benchmarks/ comparisons/ licenses/) while README describes them | discoverability + directive structure drift | either populate (deep-study outputs land there) or amend README to point at the numbered docs |
| D-3 | No CI of any kind | "property tests in CI" (ADR-009 risk) currently unenforceable; regression guard is manual | add minimal GitHub Actions: cargo test --release on ove-time; fmt/clippy; md link check |
| D-4 | 50 files with mode churn 644→755 uncommitted (no content diff) | noisy status; blocks clean audits | normalize mode bits in one commit |
| D-5 | E-007b golden .mp4s local-only | golden outputs not reproducible from fresh clone by third parties | commit tiny generated clips (or hashes+regen script) |
| D-6 | ADR-002 header lists stale blockers | minor doc inconsistency vs GATE_STATUS | refresh header text |

## 4. What is actually proven vs believed vs missing

- **PROVEN (software-validated):** exact-time representation (C-001 HIGH + ove-time 15/15),
  command-log/snapshot determinism (E-003, Python-model), shared human+AI command surface
  (E-009), UniFFI Rust→Kotlin binding viability (E-004b), browser WebCodecs decode leg
  (E-001), smart-render stream-copy economics + keyframe-index requirement (E-007/E-007b),
  timeline primary-structure micro-benchmarks (E-002c/c2).
- **BELIEVED (research-grounded, not yet code):** layered-hybrid architecture C (ADR-002
  PROPOSED), Decoder/Encoder trait shapes (ADR-004/005), project-format layout (ADR-008),
  gap-buffer primary structure at integration scale (ADR-011), all crate boundaries.
- **MISSING entirely:** everything from decode to export as Rust code; frame contract;
  render graph; any GPU-validated leg; any device/browser-validated engine leg; audio;
  CI; the ove-timeline crate prior summaries spoke about.

## 5. Consequence for the next engineering step

The audit supports the mandated move: stop widening research, start the **Media
Foundation vertical slice** with `ove-media`/`ove-decode` first (decoder-first per
directive), carrying ove-time's rules forward. Gate remains: no new high-level features
until MEDIA+TIMELINE+PROJECT+RENDER+EXPORT cooperate end-to-end.
