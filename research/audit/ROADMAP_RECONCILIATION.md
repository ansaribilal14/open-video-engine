# ROADMAP RECONCILIATION (2026-09-26)

> Reconciles: (a) the original 22-phase directive roadmap, (b) the new audit→vertical-slice
> directive, (c) the actual repo state after this audit. Produces one authoritative order
> of work. Nothing existing is discarded; anything not verified is treated as claim.

## 1. Reconciliation principle

The new directive supersedes phase numbering where they conflict: the next unit of work
is the **Media Foundation vertical slice** (no UI), with quality gate
MEDIA+TIMELINE+PROJECT+RENDER+EXPORT cooperating before any new high-level feature.
Research continues only as gap-filling driven by slice needs (RESEARCH_COMPLETENESS_AUDIT
§6 list) — no research-for-research's-sake.

## 2. Single authoritative work order

| # | Work item | Consumes (evidence) | Produces | Gate to pass |
|---|---|---|---|---|
| 0 | Repo integrity repairs: re-run E-006a (+commit code/raw), mode-churn normalize commit, LICENSE files, ADR-002 header refresh, README dir-structure fix, CI workflow v1 | this audit's D-1..D-6 | clean baseline | CI green on ove-time 15/15 |
| 1 | ove-timeline crate: clip model, gap buffer + derived index (ADR-011), verbs + inverses (ADR-006/009), command state machine, **property suite porting E-003/E-009 invariants** | E-002c/c2, E-003, E-009 | first IMPLEMENTED crate (consumes ove-time) | property tests incl. apply→inverse→apply identity; batch atomicity; replay determinism in Rust |
| 2 | ove-media + ove-decode: probe (container/streams/pix_fmt/color tags/keyframe index) → asset model → Decoder trait → **FFmpeg-software adapter** → FrameEnvelope per FRAME_CONTRACT | ADR-004, doc 04/22, E-007 keyframe finding | decoder-first media leg | DECODER_SPEC conformance suite (PTS exactness, seek, flush, cancel, error model, caps) |
| 3 | ove-render minimal: deterministic pass compile from timeline state → software raster path → golden-frame tests | doc 08, ARCHITECTURE_AUDIT #8 | correctness-first compositor | golden frames hash-stable; graph compile pure function of project state |
| 4 | ove-encode + mux + stream-copy route → MP4 via FFmpeg; **ffprobe-verified outputs + golden hashes committed** | ADR-005, E-007b | export leg | duration-exact outputs; keyframe-aligned copy path; re-encode path |
| 5 | ove-project: manifest + commands.jsonl + snapshots + asset registry (content hash) → save/kill/reopen/replay/hash-equality suite | ADR-008, E-003 | durable projects | kill-at-random-point recovery = replay hash equal |
| 6 | Integration vertical slice: ove-cli/headless — 1 video + 1 audio, multiple clips, trim/split/move/undo/redo/save/reopen → decode → preview frames → render → export MP4 (ffprobe-inspectable) | all above | the directive's quality gate | MEDIA+TIMELINE+PROJECT+RENDER+EXPORT cooperating end-to-end |
| 7 | wgpu compositor upgrade (E-005 promotion): same golden frames rendered via GPU, correctness suite == software, perf bench separate | E-001 harness, doc 09 | REAL_GPU validation begins | golden parity; perf bench committed |
| 8 | Audio minimal path + A/V sync (sample-clock exactness per doc 21) | doc 21 | audio leg | sample-exact sync test |
| 9 | Keyframe architecture: property→keyframes→interpolation→easing→evaluation over exact time | doc 20, OpenCut model | keyframes | property tests (monotonic keys, exact boundaries) |
| 10 | Platform legs (parallel where hardware allows): E-004c Android JNI device run; E-006b Tauri transport; browser WASM+WebCodecs feature-gated adapter | E-004a/b, E-006, doc 11/14/17 | DEVICE/REAL_GPU/CROSS_PLATFORM evidence | conformance matrix per CROSS_PLATFORM_CONFORMANCE_PLAN |
| 11 | ADR-002/003/004/005/006/008/009/011 ACCEPT decisions once their named gates pass; whitepaper 44 produced | gate results | architecture formally closed | GATE_STATUS update |

Phases 4–22 of the original directive remain the long-term roadmap; nothing in them is
cancelled — they are simply sequenced behind the slice and gated by it (DO-NOT-OVERBUILD:
no UI, no plugins, no marketplaces, no cloud collab until the gate).

## 3. What each audit requires the roadmap to carry forward

| Audit | Carry-forward |
|---|---|
| CURRENT_STATE | defect repairs = work item 0 (blocking, cheap) |
| RESEARCH_COMPLETENESS | 3 chains traced through our own slice docs; Cutlass cross-check + P-001/P-003 reads when ADR-027/028 work starts |
| ARCHITECTURE | frame contract + mux/encoder split + capability-gated web = slice design inputs (specs deliver them) |
| IMPLEMENTATION | crate plan + anti-inflation labels; workspace created at second crate |
| VALIDATION | every work item names its validation step (column above) |
| EXPERIMENT | E-006a re-run in item 0; E-005 → suite+bench split in item 7 |
| SOURCE/LICENSE/SECURITY/DEPENDENCY | ledger header note; LICENSE files; CI audit/deny; libav* isolation |
| OPEN_GAPS | every OPEN_GAPS row maps to a work item or is explicitly deferred |

## 4. Non-goals for the next N waves (binding)

UI of any kind · plugin system · scripting surface beyond command API · MCP server in
the engine binary (E-009 server remains the seed) · cloud/marketplace/AI-generation
features · multi-editor concurrency (ADR-008 explicitly out-of-scope v1) · OTIO/MLT
interchange adapters (storage format first, interchange later).

## 5. Success criteria for the slice (from the directive, verbatim-derived)

- A project survives save / kill / reopen / replay with hash-equal state.
- An exported MP4 is duration-exact and inspectable by ffprobe with correct timestamps.
- Decoder conformance suite passes on real media (PTS, seek, flush, cancel, error model).
- Golden frames prove deterministic render; the same frames pass through wgpu later.
- No new subsystem starts until MEDIA+TIMELINE+PROJECT+RENDER+EXPORT cooperate.
