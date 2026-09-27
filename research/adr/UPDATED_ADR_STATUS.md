# UPDATED ADR STATUS (post-forensic-audit, 2026-09-26)

> Delta document over research/adr/*. Statuses re-verified against files + tests by the
> audit. New ADRs proposed by the audit are listed at the bottom (created as spec docs;
> they become ADRs when the vertical slice proves their contracts).

## Current registry (verified)

| ADR | Subject | Status | Audit verification | Falsification / revisit conditions |
|---|---|---|---|---|
| 001 | Core language: Rust + Kotlin/TS shells | **ACCEPTED** 2026-09-24 | stands (E-004a/b chains verified; uniffi 0.32 confirmed in code) | E-004c on-device failure; UniFFI panic-semantics gap |
| 002 | Core architecture: layered hybrid (C) | PROPOSED | stands; **header blocker text stale** → refresh (D-6); attack record in ARCHITECTURE_AUDIT §2 — C survives with 3 amendments | R-01 trait leakage; E-005; E-006b; replay-determinism failure |
| 003 | Media backend: layered adapters | PROPOSED | stands; slice = FFmpeg-SW adapter first | integration bench + license sign-off |
| 004 | Decode abstraction | PROPOSED | stands + **amended**: session state machine, declared seek capabilities, FrameEnvelope (FRAME_CONTRACT) | conformance suite design fails on 2+ backends |
| 005 | Encode abstraction | PROPOSED | stands + **amended**: muxer/encoder split; stream-copy = export *route* not encoder mode (ENCODER_SPEC) | cross-encoder stitch sync fails |
| 006 | Timeline representation | PROPOSED | stands (hybrid clip-lists + compiled graph) | keyframe model Phase-2 spike contradicts |
| 007 | Time representation | **ACCEPTED** 2026-09-24 | stands, **strengthened**: 15/15 live re-run by audit | — (regression-guarded) |
| 008 | Project format | PROPOSED | stands; PROJECT_FORMAT_SPEC fixes the acceptance test | replay determinism across versions fails → snapshot-only fallback |
| 009 | Undo/redo | PROPOSED | stands (inverse commands + markers) | per-verb property failure in Rust port |
| 010 | Command system | **ACCEPTED** 2026-09-24 | stands (E-009 36/36 chain verified) | verb-set growth breakage at integration (tracked) |
| 011 | Timeline structure: gap buffer + derived index | PROPOSED | stands; audit X-10 reconciles AVL-vs-gap (different workloads) | E-012 integration bench |

## New contracts created by the audit (spec stage — ADR promotion criteria attached)

| Spec doc | Would-be ADR | Promotion condition (when it becomes ACCEPTED) |
|---|---|---|
| docs/specs/FRAME_CONTRACT.md | ADR-012 (frame abstraction) | ove-decode + ove-render both consume FrameEnvelope; conformance suite passes on FFmpeg-SW adapter |
| docs/specs/DECODER_SPEC.md | ADR-004's operational body | decoder conformance suite green on real media |
| docs/specs/ENCODER_SPEC.md | ADR-005's operational body | ffprobe-verified golden MP4 outputs committed |
| docs/specs/RENDER_GRAPH_SPEC.md | ADR-013 (render graph) | golden frames deterministic across runs; wgpu parity later |
| docs/specs/PROJECT_FORMAT_SPEC.md | ADR-008's operational body | save/kill/reopen/replay/hash suite green |
| docs/specs/MEDIA_ENGINE_SPEC.md | ADR-002's slice-scoped body | vertical slice gate passes |
| docs/CROSS_PLATFORM_CONFORMANCE_PLAN.md | ADR-014 (conformance) | matrix first fully green on desktop leg |

## Registry rules going forward

1. No ADR flips status on documentation alone — the promotion conditions above are tests.
2. ADR-002 header refresh (work item 0) must not change status, only the stale blocker list.
3. The four nonexistent projects named in the mission (Kerf/Velocut/OpenTake/Frontstage)
   generate no ADRs (R-11 CLOSED).
4. ADR numbering 012+ is reserved as above; ADR-025 (plugins) stays PLANNED and must not
   be written before the slice gate (anti-overbuild).
