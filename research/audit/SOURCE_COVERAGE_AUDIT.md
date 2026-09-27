# SOURCE COVERAGE AUDIT (forensic, 2026-09-26)

> Question answered: is the source ledger complete, recoverable, and honestly graded?
> Method: counted and classified all rows; cross-checked ledger claims against cited
> evidence in ADRs/experiments; tested recoverability (URL/repo present, versioned where
> relevant).

## 1. Ledger inventory (verified by count)

| Ledger | Rows | ID space | Notes |
|---|---|---|---|
| SOURCE_LEDGER.md | **137** | S-001..S-137 (S-xxx + S-2a..2e/S-3d sub-families for detail studies) | quality order 1–11 respected; P/S column filled |
| LEDGER_W3a..W3f (wave 3) | 6 × 12 = **72** | W3- prefixed | audio/captions/CV/plugins/scripting/headless/storage/perf/security/UX/licenses/YouTube/transcripts waves |
| **Total** | **209** | — | STATUS.md's "137 sources" counts only the main ledger — figure is accurate but understates; keep both numbers visible |

## 2. Coverage by track (spot-checked against citations)

| Track | Coverage | Quality of key sources |
|---|---|---|
| Editors (new-gen) | OpenCut×2, Diffusion Studio, Cutlass, Clypra, OpenReelio, Katana, LosslessCut, Remotion, Kinocut — cloned or HTML-verified | QUALITY 1 (source) for all PRIMARY rows; honest SECONDARY/BONUS tiers |
| NLE architecture | Kdenlive/Shotcut/Olive/Flowblade/MLT/OTIO | ledgered QUALITY 1, but corpus depth is survey (RESEARCH_COMPLETENESS_AUDIT §1) — ledger is fine, depth is the gap, not the ledger |
| Media/codec | FFmpeg (incl. 9.0.2 + legal page), GStreamer 1.28.7, Media3 1.11.1, MediaCodec, WebCodecs/mediabunny | versions pinned where decisions cite them — good |
| GPU/web | wgpu/WebGPU/WebGL2, browser capability docs (BCD/caniuse) | E-001 support matrix gives first-party evidence |
| Papers | P-001..P-021 in doc 38 (separate ledger format) | 4 target works arXiv-verified; license cells partly UNVERIFIED |
| YouTube/transcripts | doc 36/37 design + W3e/f | transcripts not yet fetched (E-011-blocked) — honest |

## 3. Recoverability test (directive: "every important source must be recoverable")

- Every row carries URL/repo; most carry versions for decision-critical sources
  (FFmpeg 9.0.2, GStreamer 1.28.7, Media3 1.11.1, wgpu 29/24, Tauri 2.11, uniffi 0.32).
- **No archived snapshots** (e.g. Wayback links) for live-HTML-derived facts (star counts,
  BCD rows). Star counts self-declared as drifting snapshots — acceptable for v0.x, but
  decision-critical HTML facts should be snapshotted when cited in an ACCEPTED ADR.
- Kerf/Velocut/OpenTake/Frontstage: absence recorded with queries — the ledger treats
  non-existence as a finding (R-11 CLOSED). Correct handling.

## 4. Integrity checks

- Claim→source traceability: C-001..C-007 cite S-IDs that resolve to real rows
  (spot-checked C-001→S-2b1/S-2b3/S-2b6/S-2b8, C-003→S-4f*; all resolve).
- ADR→source traceability: all 11 ADRs cite doc numbers + experiment IDs; experiment
  records cite scripts + raw outputs — the chain holds everywhere except E-006a
  (CURRENT_STATE_AUDIT D-1).
- Fabrication scan: no row claims a "Kerf/Velocut" source; no invented stars beyond
  declared HTML-snapshot method; doc 39's NOT-FOUND list matches R-11.

## 5. Gaps + actions

1. 72 W3 rows are not in the main ledger numbering space — add a mapping note in
   SOURCE_LEDGER.md header (W3a..f are appendices) so global source counts are unambiguous.
2. Paper license cells UNVERIFIED (doc 38) — only blocks code/dataset reuse; batch-verify
   when any paper-derived code/dataset is actually considered (lazy verification is correct
   economics; keep the tracker).
3. No archived snapshot policy for decision-critical HTML facts — add one line to the
   ledger header and apply when ADR-002 is ACCEPTed (its evidence cites browser docs).
4. research/sources/ holds only ledgers; the empty `research/github/`, `papers/`,
   `youtube/`, `transcripts/`, `conferences/` dirs (D-2) should receive the deep-study
   outputs when they are produced — or be removed with a README pointer.

## 6. Verdict

The ledger system is **complete for its declared scope, recoverable, honestly graded, and
fabrication-free**. Coverage gaps are depth gaps (tracked in RESEARCH_COMPLETENESS_AUDIT),
not ledger gaps. The single integrity defect in the wider evidence chain remains E-006a.
