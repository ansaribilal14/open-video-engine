# OPEN QUESTIONS (v0.1)

Unresolved engineering problems. Each maps to an experiment (41_EXPERIMENTS) or a depth
research pass. Nothing here may be quietly assumed.

## Architecture

| ID | QUESTION | RESOLUTION PATH |
|---|---|---|
| Q-01 | Can a decode-adapter trait express MediaCodec surface reuse/EOS + FFmpeg hw-frame import + WebCodecs callback model without leaking platform types? | E-004 + trait spike against 3 documented APIs (docs 04,11,16) |
| Q-02 | Is `importExternalTexture` sufficient for multi-layer compositing (single-use constraint) or is a copy path mandatory for layers >1? | E-001 micro-benchmark (doc 11) |
| Q-03 | Command-log replay determinism across platform float-free paths | E-003 + property tests; defines C-007 final |
| Q-04 | Pass-graph compiler: one IR for wgpu/WebGL2/Android GL ES or per-backend passes? | design spike after E-001/E-004 |
| Q-05 | Timeline primitive set: are move/resize/splice/extract/retime/split sufficient for ripple+roll+slip+slide as derived ops? | formal model + property tests (doc 20) |

## Media

| ID | QUESTION | RESOLUTION PATH |
|---|---|---|
| Q-06 | VFR handling: per-clip PTS map in project format vs normalize-at-import? | depth pass + bench (doc 04) |
| Q-07 | Smart-render boundary GOP re-encode: quality/size trade-offs per codec | E-007 (remux path experiment) |
| Q-08 | Browser HDR: can WebCodecs/WebGPU round-trip PQ/HLG on Safari 26/Chrome? | depth pass (docs 10,11,22) |

## AI / Agents

| ID | QUESTION | RESOLUTION PATH |
|---|---|---|
| Q-09 | Do AI agent edits and UI edits share one transaction log without schema divergence? | **ANSWERED 2026-09-24 — YES**: E-009 36/36 PASS (10 command surfaces wrapped in MCP 2026-07-28 stdio; UI/wire invocations hash-identical per step; owner-agnostic replay; cross-ownership undo; atomic batches; float payloads rejected at both edges). ADR-010 ACCEPTED. Residual: real Claude Code end-to-end run at integration |
| Q-10 | Approval UX: which command tiers are auto-allowed vs human-gated? | doc 26 tier table → validate with ADR-027 |

## Program

| ID | QUESTION | RESOLUTION PATH |
|---|---|---|
| Q-11 | License of OVE itself (MIT/Apache-2.0 vs GPL concerns from codecs) | GATE-13 licensing audit (doc 35) |
| Q-12 | Repository naming: engine crate vs app workspace boundaries in monorepo | after ADR-002 acceptance |
| Q-13 | WebKitGTK WebCodecs support status (affects Tauri Linux preview) | depth pass (doc 17) |
