# OPEN GAPS — full contradiction + gap registry (2026-09-26)

> Mandate: search the whole repo for PARTIAL / PENDING / UNVERIFIED / UNKNOWN / PROPOSED /
> TODO / FIXME / BLOCKED / NOT STARTED / PLANNED and record every contradiction or gap
> WITHOUT hiding any. Nothing here is spin; each row names its resolution path.

## 1. Census (grep counts across docs/, research/, STATUS.md, README.md)

| Term | Hits | Interpretation |
|---|---|---|
| PARTIAL | 93 | mostly honest per-doc statuses and gate residuals — each mapped below or to an audit |
| UNVERIFIED | 300 | honesty markers (license cells, star drift, README-level claims) — not contradictions; tracked |
| PLANNED | 20 | futures (ADRs 012–036, experiments E-008..E-014, whitepaper) — legitimate |
| BLOCKED | 10 | hardware-bound residuals (E-004c/E-005/E-006b/E-008/E-011) + E-006 transport leg |
| TODO | 11 | all "v0.2 depth" markers in docs 08/09/10/11/12/15/18/19/20/31×2 |
| FIXME | 0 | — |
| UNKNOWN | 2 | STATUS vocabulary + directive vocabulary only |
| PENDING | 0 | — |
| NOT STARTED | 0 | — |

## 2. Contradictions (statement vs statement / statement vs repo)

| ID | Contradiction | Where | Resolution |
|---|---|---|---|
| X-1 | Prior session summary claimed `engine/ove-timeline` exists as a production crate with property tests/oracle/undo | repo: no such crate | repo wins (CURRENT_STATE_AUDIT A2). The claim was a session-summary artifact, never in committed files. Timeline crate = work item 1 |
| X-2 | Prior summary claimed CI/GitHub Actions exist | repo: no .github/ | repo wins; CI = work item 0 |
| X-3 | ADR-002 header "blocked on GATE-1..7 + E-001/E-003/E-004/E-006" vs GATE_STATUS "blocked only on E-005/E-004c/E-006b" | ADR-002 line 3 vs GATE_STATUS verdict | GATE_STATUS is current (post-v0.4); ADR-002 header is stale → refresh in work item 0 (D-6) |
| X-4 | Worklog (superseded session) says "uniffi 0.29" vs E-004b code says 0.32 vs C-004 says 0.32.2 | scripts/.../Cargo.toml | code wins: 0.32 family; worklog line belonged to the discarded duplicate session |
| X-5 | 41_EXPERIMENTS index claims E-006a "Code: E-006a_ipc_serialization.rs" | repo: file absent, dir empty, raw result missing | index row is wrong until re-run (D-1, work item 0) |
| X-6 | README describes research/ subdirs (github/ papers/ youtube/ transcripts/ conferences/ architecture/ benchmarks/ comparisons/ licenses/) | repo: all 9 empty | structure aspiration vs reality → populate via deep studies or fix README (D-2, work item 0: fix README pointer note) |
| X-7 | STATUS.md says "Phase 0 … 39 of 41 research docs" vs actual file count 42 (incl. 40/41/44 additions) | STATUS line 14 | minor bookkeeping drift in doc-count phrasing; refresh STATUS when updating statuses with this audit |
| X-8 | GATE-11 cites E-006a numbers as evidence; E-006a artifacts unrecoverable | GATE_STATUS row 11 vs EXPERIMENT_AUDIT | keep gate status only via remaining anchors (E-002c/E-006/E-007b); E-006a re-run restores the citation |
| X-9 | README rule 2 "No fake completion … Documentation is never substituted for engineering" vs directive progress being 90% documentation | repo-wide | not a contradiction yet — but becomes one the moment any doc claims implementation that isn't in engine/. Audit confirms no such claim exists in committed files today |
| X-10 | ADR-011 says "gap buffer primary" while worklog duplicate said "augmented AVL wins all edit verbs" | ADR-011 + E-002c/c2 records | both are true on different workloads (cursor-local vs random) — ADR-011's derived-index rule already encodes the reconciliation; E-012 is the referee |

## 3. Hard gaps (things the mission needs that do not exist anywhere)

| ID | Gap | Blocks | Work item (ROADMAP_RECONCILIATION) |
|---|---|---|---|
| G-1 | No timeline/commands/project code in Rust | everything above ove-time | 1 |
| G-2 | No decoder/encoder/frame/render/export code | vertical slice | 2–5 | RESOLVED-2026-09-27 (decoder leg): ove-media + ove-decode landed; DECODER_SPEC conformance D-1..D-12 green on committed corpus; FrameEnvelope per FRAME_CONTRACT; libav* confined to ove-decode (CI job). Encoder/render/project legs remain open (Waves 3–5) |
| G-3 | No CI | all regression guards | 0 |
| G-4 | No LICENSE files | public release | 0 |
| G-5 | No device/GPU/browser validation of any engine code | VALIDATION levels | 7, 10 |
| G-6 | Three mandated operation chains untraced | research depth mandate | 1–6 produce them in-repo |
| G-7 | Audio, keyframes, color pipelines | full engine | 8, 9 |
| G-8 | Whitepaper 44 (5-line stub) | final architecture closure | 11 (after gates) |
| G-9 | E-006a artifacts | evidence-chain integrity | 0 |
| G-10 | Testing-strategy doc (GATE-14 residual) | GATE-14 → COMPLETE | 1 (the property-suite port IS the beginning of it; doc follows) |

## 4. Deferred-by-design (recorded, not forgotten)

- Kerf/Velocut/OpenTake/Frontstage: nonexistent (R-11 CLOSED) — reopen only if principal supplies links.
- Browser HDR round-trip (Q-08), VFR map-vs-normalize (Q-06), pass-graph single-IR (Q-04), approval-tier table (Q-10), crate naming finalization (Q-12), WebKitGTK WebCodecs status (Q-13): open questions with named resolution paths — scheduled with their consuming work items.
- E-008 (audio latency matrix), E-010 (scrub burst), E-011 (upload drill), E-013 (real agent E2E): hardware/integration-gated, defined in 41_EXPERIMENTS.

## 5. Statement

Every contradiction above is either (a) already resolved in favour of the repository, or
(b) mapped to a numbered work item. **No gap in this file is being silently carried.**
This file must be updated (not rewritten) whenever a row resolves, keeping the ID.
