# RESEARCH COMPLETENESS AUDIT (forensic, 2026-09-26)

> Question answered: is the research corpus deep enough to derive the media-engine
> architecture from, or is it README-level? Method: per-project depth grading against the
> three mandated operation chains; per-doc line-count + spot verification; honesty-marker
> census. Grade scale: DEEP (source-read, chain-traced, quoted files) / MEDIUM (clone or
> tree inspected, key files read, no full chain) / SHALLOW (README/metadata level) /
> NOT FOUND.

## 1. The 15 mandated projects — depth grades

| Project | Where studied | Depth | What is actually proven | What the directive requires that is MISSING |
|---|---|---|---|---|
| OpenCut (rewrite) | 02 §1, 39 R-101 | **MEDIUM-DEEP** | full tree read of the clone; workspace state (`crates/*` commented out), GPUI panels, web stack, roadmap | chain-trace of rewrite internals once crates land on main |
| OpenCut Classic | 02 §1 (lines 48–200), 39 R-102 | **DEEP** (best in corpus) | file-level: EditorCore 12 managers, CommandManager semantics incl. selection snapshots + reactors, action layer, keyframe 3-layer model, mediabunny export path, verbatim AGENTS.md | none structural; re-verify against a moving repo occasionally |
| Clypra | 02 §4, 39 R-105 | MEDIUM | stack verified (Tauri2+React19+wgpu24+FFmpeg hw-decode, WASM render crate, Capacitor); golden-harness existence noted | wgpu_compositor.rs + shaders + preview_golden.rs source-read; undo mechanism UNVERIFIED; AI scope uninspected |
| Cutlass | 02 §3, 39 R-104 | MEDIUM | 20-crate decomposition + ToolTier/dry-run/fuse pattern read from source comments | cutlass-commands/engine/storage full read (queued in 39 §4) |
| Kerf | 39 §3 | **NOT FOUND** (3 searches, honest record) | — | none (name unverifiable; R-11 CLOSED) |
| Velocut | 39 §3 | **NOT FOUND** (3 searches) | — | none |
| OpenReelio | 02 §5, 39 R-106 | MEDIUM | event-sourced undo + loopback MCP claim verified at README/lib.rs level | event-log schema read (39 §5 admits not read) |
| OpenTake | 39 §3 | **NOT FOUND** (2 searches) | — | none |
| Frontstage | 39 §3 | **NOT FOUND** (2 searches) | — | none |
| Katana | 02 §6, 39 R-107 | SHALLOW-MEDIUM | 5-file Tauri core, tree read; license type UNVERIFIED | export.rs/project.rs read |
| Kdenlive | 03 §4 (doc 03 total 237 lines for 4 NLEs) | SHALLOW-MEDIUM (survey) | architecture taxonomy, MLT relationship, undo lambda-pairs, dual-store format history | **source-level chain trace: USER EDIT→UI EVENT→COMMAND→STATE→RENDER→UNDO→SAVE** |
| Shotcut | 03 §5 | SHALLOW-MEDIUM (survey) | QUndoCommand-per-verb, MLT consumer | same chain trace |
| Olive | 03 §6 | SHALLOW-MEDIUM (survey) | node model, UndoStack/NodeUndo | same chain trace |
| Flowblade | 03 §7 | SHALLOW (survey) | MLT+Python positioning | same chain trace |
| LosslessCut | 02 §7 (secondary) | SHALLOW-MEDIUM | stream-copy fast-path role, Electron+FFmpeg | keyframe-cut planning logic read (queued 39 §4) |

**Net:** 1 DEEP (OpenCut classic), 4 MEDIUM-DEEP/MEDIUM, 4 SHALLOW-MEDIUM/SHALLOW,
4 NOT FOUND (honest). The directive's bar ("not README-level") is met for 2 of 15,
partially met for 6, and met-by-honest-absence for the 4 nonexistent names.

## 2. The three operation chains — NOT yet traced anywhere

| Mandated chain | Coverage today | Gap |
|---|---|---|
| IMPORT→PROBE→ASSET→TIMELINE→PLAYBACK→COMPOSITING→EXPORT | fragments: OpenCut classic render/export (02), E-007 smart-render semantics, doc 04/05 pipeline rules | no single end-to-end trace in any external codebase or in our own future code |
| USER EDIT→UI EVENT→COMMAND→STATE→RENDER→UNDO→SAVE | best fragments: OpenCut classic commands/actions (deep), E-003/E-009 models | external chain-trace absent (Kdenlive/Shotcut); own implementation absent |
| AI→TOOL→COMMAND→TIMELINE→PREVIEW→VALIDATE→APPLY→EXPORT | strongest on paper: E-009 (own), Cutlass ToolTier, kinocut receipts, Diffusion Studio tool catalog | Diffusion Studio `dapi` internals queued but unread; own integration pending |

**Consequence:** chains must be traced during the vertical slice — first in our own code
(by construction), with one external reference trace (Cutlass or OpenCut-classic) as the
cross-check. Recording this as planned work, not as completed research.

## 3. Academic papers (doc 38) — audit against "what requirement does it create"

21 papers ledgered, 4 target works (P-001..P-004) independently arXiv-verified,
verification column filled per row. Relevance-to-engine is stated for every row — the
directive's question is answered at index level. Weaknesses:

1. No paper was *method-level* audited (no "what would we steal / what breaks at 60fps"
   notes). For P-001 (EditDuet) and P-003 (Timeline Assembler) — the two papers that
   shape our command surface — a method-level read is cheap and should happen before
   ADR-027/028.
2. License cells UNVERIFIED for P-005, P-007, P-008, P-009, P-011..P-020 (doc 38 admits).
   Only blocks *reusing code/datasets*, not design learning — acceptable, track it.
3. Papers create these engine requirements (extracted, to feed the specs):
   - P-001/P-003/P-014/P-015 → the command API must accept *batch/plan-shaped* intents,
     not only single verbs (E-009 batches already probe this).
   - P-006/P-007/P-008/P-010/P-011/P-013 → analysis services must be **advisory commands**
     (propose→human/machine approve→apply), never auto-applied (fits ADR-010 tiers).
   - P-002 → project context/narrative memory is a *project-format* concern (asset
     registry + annotations), not an engine-core concern — keep out of core.
   - P-004 → generative effects arrive as frame-source adapters; frame contract must not
     assume only decoder-produced frames.
   - P-017/P-018 → MCP adapter threat model: tool descriptions are an attack surface;
     receipts + approval tiers are mandatory (feeds SECURITY_AUDIT).

## 4. YouTube research (doc 36) — audit

196 lines; Data API v3 pipeline designed (quota math, 308/Range resume → E-011 defined);
transcript pipeline in doc 37 (153 lines). Gaps: no long-form technical talk transcripts
have actually been fetched yet (transcripts/ dir empty); quota-gated work is correctly
defined as E-011-blocked. Verdict: pipeline designed, evidence not yet collected — honest
but incomplete; lower priority than code until the vertical slice exists.

## 5. Corpus-wide depth metrics

- 42 numbered docs, 8,135 lines total; median ~200 lines/doc — survey-grade with
  verified-source anchors where it matters (02, 05, 11, 14, 16, 17, 19, 20).
- UNVERIFIED honesty markers: 300 mentions across 30+ files — a *feature* (no fabrication),
  not a bug; the largest clusters are license cells (papers) and star counts (drift).
- TODO census: 11 TODOs, all "v0.2 depth" markers (19, 08, 20, 11, 31×2, 09, 15, 10, 18,
  12) — i.e., self-declared depth gaps, consistent with §1 grades above.
- Wave-3 docs (21/23/24/28–37): 1 pass each, designed-to-experiment, no primary-source
  deep reads beyond spec/vendor docs — sufficient for architecture framing, insufficient
  for implementation-detail decisions (acceptable: those decisions need our own code +
  benches anyway).

## 6. Verdict + required actions

The corpus is **deep enough to support ADR-002 ACCEPT-provisional and the vertical-slice
crate plan**, and honest about every gap. It is NOT deep enough to claim "we understand
the 15 named projects' internals". Required actions (in priority order):

1. Trace the 3 chains through **our own** vertical slice as it is built (strongest form:
   the trace becomes the engine's own documentation).
2. One external chain cross-check: Cutlass `cutlass-commands` + storage (closest to our
   crate plan) — half-day.
3. Method-level read of P-001 + P-003 before ADR-027/028 work (half-day).
4. Close the 11 self-declared doc TODOs opportunistically, driven by slice needs only —
   no research-for-research's-sake.
