# VALIDATION AUDIT (forensic, 2026-09-26)

> Mandate: introduce explicit four-dimension status vocabulary and classify every
> milestone; forbid calling software-experiment results production-validated. Method:
> full matrix over all milestones; each VALIDATION level is the strongest evidence that
> actually exists in-repo.

## 1. Vocabulary (binding)

- **RESEARCH_STATUS**: COMPLETE (all directive questions for the slice answered with
  evidence) / PARTIAL (named residuals) / GAP (unstudied).
- **ARCHITECTURE_STATUS**: PROPOSED / ACCEPTED (evidence + falsification conditions
  recorded) / REJECTED.
- **IMPLEMENTATION_STATUS**: NOT_STARTED / EXPERIMENTAL (exists, not consumed by engine,
  no CI) / IMPLEMENTED (consumed + integration-tested + CI) / PRODUCTION (device/browser
  users + perf budgets held).
- **VALIDATION_STATUS**: NONE / SIMULATED (model, e.g. Python) / SOFTWARE (real code,
  container CPU / software GPU) / DEVICE (real hardware) / REAL_GPU (vendor GPUs via API)
  / CROSS_PLATFORM (matrix holds).

## 2. Master matrix

| Milestone | RESEARCH | ARCHITECTURE | IMPLEMENTATION | VALIDATION | Strongest evidence | Next validation step |
|---|---|---|---|---|---|---|
| Exact rational time | COMPLETE | ACCEPTED (ADR-007) | EXPERIMENTAL | **SOFTWARE** | ove-time 15/15 live re-run; E-002 13/13 | consumed by ove-timeline; FFI re-verify per release |
| fp-forbidden rule | COMPLETE | ACCEPTED (ADR-007) | ENCODED | SOFTWARE | P3b guard; E-002 boundary census | lint-level enforcement in new crates |
| Timeline edit verbs + inverses | COMPLETE (model) | PROPOSED (ADR-006/009/011) | NOT_STARTED | SIMULATED (E-003/E-009 Python) | hash-exact replay/undo | ove-timeline property suite |
| Gap buffer + derived index | COMPLETE (micro) | PROPOSED (ADR-011) | NOT_STARTED | SOFTWARE (bench only) | E-002c/c2 4-structure × 4 workloads | E-012 integration bench |
| Command bus (human+AI one API) | COMPLETE | ACCEPTED (ADR-010) | NOT_STARTED | SIMULATED (E-009) | 36/36 MCP-stdio equivalence | real MCP server in ove-engine; E-013 |
| Project format log+snapshot | COMPLETE | PROPOSED (ADR-008) | NOT_STARTED | SIMULATED (E-003) | 9/9 | save/kill/reopen/replay/hash suite in ove-project |
| FFI boundary pattern | COMPLETE | ACCEPTED (ADR-001) | EXPERIMENTAL (scratch crate) | SOFTWARE (JVM) | E-004a 5/5; E-004b 8/8 live Kotlin | E-004c device |
| WebCodecs decode leg | COMPLETE (leg) | PROPOSED (ADR-003 leg) | NOT_STARTED (engine) | SOFTWARE (headless Chrome) | E-001 48/48 real H.264 | engine adapter + browser E2E |
| WebCodecs→WebGPU import | PARTIAL (Q-02 open) | PROPOSED | NOT_STARTED | **NONE** (SwiftShader-blocked) | support matrix only | E-005 real GPU |
| wgpu compositor | PARTIAL (design) | PROPOSED | NOT_STARTED | NONE | doc 08/09 design | E-005 promotion: real compositor crate, golden frames |
| Decoder abstraction | COMPLETE (design) | PROPOSED (ADR-004) | NOT_STARTED | NONE | — | ove-decode conformance suite |
| Encoder/mux/stream-copy | COMPLETE (design) | PROPOSED (ADR-005) | NOT_STARTED | SOFTWARE (CLI-level E-007b) | 6.8–11.9× copy at 1080p | in-engine encoder + ffprobe-verified outputs |
| Render graph | PARTIAL (design) | PROPOSED | NOT_STARTED | NONE | — | minimal compiler + golden frames |
| Smart-render semantics | COMPLETE | PROPOSED (ADR-005) | NOT_STARTED | SOFTWARE | E-007 72/72; E-007b timing | segment-copy route in ove-encode |
| Android device leg | PARTIAL | PROPOSED | NOT_STARTED | NONE (device) | UniFFI JVM only | E-004c JNI; MediaCodec adapter |
| Tauri transport | PARTIAL | PROPOSED | NOT_STARTED | NONE (native leg serialized-only) | E-006a numbers (artifacts lost), E-006 browser | E-006b real-Tauri |
| Audio path | PARTIAL (doc 21) | PROPOSED | NOT_STARTED | NONE | — | minimal decode→mix→sync in slice |
| Keyframes | PARTIAL (doc 20/OpenCut model) | PROPOSED | NOT_STARTED | NONE | — | property-tested evaluator in ove-timeline |
| Testing strategy | PARTIAL (residual named by GATE-14) | — | — | — | 15/15 + property habits | testing-strategy doc + CI |
| Security model | COMPLETE (survey) | PROPOSED (doc 33 rules) | NOT_STARTED | NONE | 513-CVE sidecar rule | cargo-audit/deny in CI |
| Licensing | COMPLETE | PROPOSED verdict (MIT OR Apache-2.0) | NOT_STARTED (LICENSE file absent!) | — | doc 35 RED/GREEN | add LICENSE files; license CI check |
| CI | GAP | — | NOT_STARTED | NONE | none | **create** (D-3) |

## 3. Inflation check (mandated anti-false-compliance)

- No milestone above carries DEVICE/REAL_GPU/CROSS_PLATFORM labels — none exists yet.
  GATE_STATUS's "UNDERSTOOD" labels are *research* claims, not validation claims, and are
  correctly evidence-linked; the audit endorses 10/14 with the 4 PARTIALs exactly where
  GATE_STATUS puts them.
- Two label risks caught: (a) E-006a numbers are cited in GATE-11 evidence but their
  artifacts are unrecoverable (D-1) — GATE-11 keeps its UNDERSTOOD status via E-002c/E-007b
  anchors alone, and E-006a must be re-run to remain citable; (b) "OVE-time TESTED" in
  STATUS.md means SOFTWARE-validated only — correct, but keep it from being quoted as
  "production-validated" anywhere.

## 4. What would move the top items one level

| To reach → | Cheapest honest step |
|---|---|
| ove-time EXPERIMENTAL→IMPLEMENTED | consume in ove-timeline + CI |
| timeline SIMULATED→SOFTWARE | port E-003/E-009 invariants to ove-timeline property tests |
| decoder NONE→SOFTWARE | ove-decode + FFmpeg adapter + conformance suite on container media |
| encoder SOFTWARE(engine)→SOFTWARE(verified) | MP4 output + ffprobe hash + golden outputs committed |
| browser NONE→SOFTWARE(engine) | WASM timeline + WebCodecs adapter behind feature detection |
| Android NONE→DEVICE | E-004c on-device JNI harness |
| GPU NONE→REAL_GPU | E-005 on vendor GPU runner + golden frames |
| CROSS_PLATFORM | conformance suite green across desktop+browser+Android matrix (last, not first) |
