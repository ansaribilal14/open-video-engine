# EXPERIMENT AUDIT (forensic, 2026-09-26)

> Question answered: are the experiments real, reproducible, honestly limited, and
> decision-relevant? Method: per-experiment check of record ↔ script ↔ raw output ↔
> decision links; reproducibility assessment from repo state; limitations honesty review.

## 1. Experiment-by-experiment verdicts

| ID | Question (record) | Script in git | Raw output in git | Record honest about limits? | Fed which decision? | Verdict |
|---|---|---|---|---|---|---|
| E-001 | WebCodecs→WebGPU browser pipeline | YES (v2 + flag sweep + final) | YES (result.json, packets.json) | YES — SwiftShader GPU-import block documented, harness left for rerun | C-005 decode leg; GATE-6 | VALID (decode leg); GPU legs OPEN |
| E-002 | fp vs rational time | YES (py + bench rs) | YES | YES — includes 3 self-caught bad test constructions (3×(1/3) trap → 0.1×10; sample-boundary census 178) | C-001 HIGH; ADR-007 | VALID |
| E-002c | timeline structure (4 candidates × 4 workloads) | YES | YES | YES — E-002b's 45.6ms flagged as front-cluster-optimistic; two-phase rekey bug found in own bench | ADR-011 PROPOSED | VALID (micro); integration unproven (E-012 gate recorded) |
| E-002c2 | AVL under random positions (addendum) | YES | YES | YES — independently confirms rekey-collision rule | ADR-011 refinement | VALID (addendum) |
| E-003 | command-log replay determinism | YES | YES | YES — float divergence 300/300 reported as finding, not hidden | C-007; ADR-008/009/010 basis | VALID (SIMULATED — Python model; Rust port required) |
| E-004a | FFI boundary pattern | YES (rs + py driver) | YES | YES | C-004; ADR-001 | VALID |
| E-004b | UniFFI Kotlin codegen + live roundtrip | YES (full crate + generated kt + Kotlin test) | YES | YES — records the pub-export trap (non-pub exports silently emit no metadata) | C-004; ADR-001 ACCEPT | VALID (JVM level; device leg open) |
| E-006 | IPC transport browser leg | YES | YES (json+txt) | YES — transport leg BLOCKED, honest | R-08 rule; GATE-7 partial | VALID (browser leg) |
| **E-006a** | native-side serialization | **NO — code lost** | **NO — result txt missing** | Record table survives; narrative honest | R-08/E-006a rule (JSON banned for frames both edges) | **INCOMPLETE — re-run required (D-1)** |
| E-007 | smart-render semantics | YES | YES | YES — timing leg self-invalidated (micro-scale) | keyframe-index mandatory | VALID (semantics) |
| E-007b | smart-render timing @1080p | YES | YES (txt; mp4s local-only D-5) | YES — uniform veryfast preset declared; sandbox quirk documented | ADR-003/005 stream-copy route | VALID |
| E-009 | shared command surface | YES (+debug session) | YES | YES — residual real-agent run named (E-013) | Q-09; ADR-010 ACCEPT | VALID (SIMULATED surface; production MCP server pending) |
| ove-time suite | permanent property tests | engine/ove-time | committed output txt | YES — P3b is a discovered-failure-mode guard | ADR-007 ACCEPT | VALID (live re-run PASS this audit) |

## 2. Cross-cutting findings

1. **Reproducibility:** 12/13 re-runnable from repo state; E-006a is the only hole.
   Hardware variance is declared in every record ("container CPU, 2 cores, single run")
   — relative-order conclusions are drawn, absolute numbers not oversold. Good practice.
2. **Self-catch rate is unusually high and healthy:** E-002 (3 bad constructions), E-002c
   (bench bug: silent BTreeMap rekey overwrite; negative-cast panic), E-002c2 (independent
   reimplementation), E-007 (self-invalidated timing leg). This is the evidence-driven
   culture working; keep it.
3. **Decision linkage:** every experiment feeds at least one ADR/claim/gate; no
   orphan experiments. Conversely, ADR-002 ACCEPT is blocked on exactly 3 experiments
   (E-005/E-004c/E-006b) + testing doc — matching GATE_STATUS.
4. **Simulation-level honesty:** E-003/E-009 are Python *models* of the command/log
   design. They validate the design's determinism properties, not Rust code. Both records
   say so; VALIDATION_AUDIT carries them as SIMULATED. Their invariants must be ported
   into the Rust crates as property tests (ENGINE_BUILD_PLAN W1/W2).
5. **ID registry discipline:** 41_EXPERIMENTS.md maintains a collision-free ID registry
   (run/defined/proposed, next free ID tracked). Wave-3 agent proposals (E-010..E-014)
   mapped without collision. Good.

## 3. Required actions

1. **Re-run E-006a and commit code + raw output** (D-1) — cheapest honest repair; do it
   inside the first slice wave (it is a 30-line serde bench at heart).
2. Commit tiny golden mp4s or a regeneration script for E-007b (D-5) when ove-encode
   lands (the slice will regenerate better ones anyway — defer until then).
3. When the wgpu compositor crate exists, E-005 must be upgraded from "experiment" to
   "permanent correctness suite + separate perf bench" (directive: separate correctness
   from performance) — record that intention in the E-005 stub now (done via RENDER_GRAPH_SPEC).
4. CI (D-3) must run: ove-time suite + future property suites on every push — experiments
   stay manual, invariants become CI.

## 4. Verdict

The experiment program is **real, decision-linked, and honest**, with one broken
evidence chain (E-006a) and the expected hardware-bound residuals. No fabricated
results found; every "PASS" checked traces to a committed script + output.
