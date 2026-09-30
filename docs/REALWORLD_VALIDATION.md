# REALWORLD VALIDATION — the permanent real-world reference media workflow

> Directive rule (real-world proof task): EVERY WAVE MUST REMAIN CONNECTED TO
> REAL MEDIA. A wave is not complete if it silently breaks the real-video
> workflow. This file is the protocol, the provenance record, and the
> per-wave evidence log.

## 1. Mission

Answer one question with evidence, per wave:

> Can the actual OVE engine take an ordinary real-world video and process it
> with the capabilities that genuinely exist today?

This is not a demo. The reference media is ordinary real-world footage with
real cadence, real color metadata, real GOP structure, and real audio — the
exact conditions the committed synthetic corpus cannot reproduce.

## 2. Reference source — provenance record (WAVE-RLW-1, 2026-09-29)

| Field | Value |
|---|---|
| YouTube URL | https://www.youtube.com/watch?v=jrDv0OdMt5s |
| Title | NASA's Artemis I Moon Mission: Launch to Splashdown Highlights |
| Channel | NASA (official) |
| Publication | Nov 2022 (Artemis I launch day highlights) |
| License | Public domain — NASA media are works of the U.S. Government not protected by copyright (17 U.S.C. § 105; NASA Media Usage Guidelines). Download/testing/redistribution permitted. |
| Why this source | Real camera footage; launch + flight + splashdown scene changes; fine detail (tower structure, rocket skin, crowds); gradients (night sky, fireball); natural audio (comms + ambience); real consumer cadence (NTSC 24000/1001); 2 min 3 s — enough to exercise seeking without being enormous. |
| Selection rejections | Commercial/copyrighted footage rejected on license grounds; synthetic test assets rejected by task definition. |
| Acquisition tool | ytagent 0.3.0 (yt-dlp 2026.8.19) — the external acquisition mechanism; OVE grows no downloader |
| Acquisition path | Tier 1 (13-method chain) exhausted by datacenter-IP bot-blocks + 429 rate limits; Tier 2: ytagent `github_actions_farm` method against a personal deployment of ytagent's `yt-download-farm.yml` workflow (farm run 36640479906; WARP proxy + `android_vr` client + manual GVS PO token from the BGutil POT server). Artifact returned as unmerged DASH parts (136 = 720p avc1, 140 = AAC m4a; the runner image lacks ffmpeg). Normalized locally with `ffmpeg -c copy` (stream copy only, no re-encode). |
| Source identity | sha256 `2d315daf6130366d9a98c9f49716aa263036ba5fef8f241cefff727bc3b4705f`, 7,917,702 bytes |
| OVE asset identity | BLAKE3 `7f89f3726f09dbe6a79f660ebb86b59ff1aaa4a137d498289796390230cec0a1` (content-addressed import) |
| Media on git | NEVER. `*.mp4` is repo-ignored; the source is reproducible from this provenance record + the acquisition script. |

## 3. Independent baseline (ffprobe — OVE is never the authority)

| Property | Value |
|---|---|
| Container | mov/mp4, faststart |
| Video | H.264 Main, 1280×720, yuv420p |
| Cadence | CFR 24000/1001 (r = avg = 24000/1001), tbn 1/24000 |
| Frames | 2945 decoded |
| Keyframes | 48 (GOP ≈ 2.9–3.4 s), first at 0.0 |
| Color metadata | range=tv (LIMITED), bt709/bt709/bt709, chroma left (stream AND all 2945 frames) |
| Audio | AAC LC, 44100 Hz, stereo, 5,419,008 samples (122.88 s exactly) |
| Duration | 122.88 s container; 122.831 s video |
| Decode | clean end-to-end (`ffmpeg -xerror`) |

## 4. The permanent test

`engine/ove-engine/tests/realworld.rs` — one canonical deterministic scenario,
env-gated so CI stays corpus-only:

```
OVE_REALWORLD_SOURCE=<source.mp4> \
OVE_REALWORLD_EXPECT=<baseline_facts.json>   # from scripts/realworld/baseline_facts.py
OVE_REALWORLD_OUT=<artifact dir> \
cargo test -p ove-engine --release --test realworld
```

Scenario (exact parameters, tick axis 24000/1, output 1280×720 @ 24000/1001,
span 160 output frames = 160160 ticks = 6.673333 s):

1. create project + 2 tracks; import real source (probe vs INDEPENDENT
   baseline: geometry, cadence, audio rate/channels, keyframe index);
2. track 1 (bottom layer = v1 audio lane, contiguous, every cut sample-exact
   at 44100): clip1 [0,2.0) src 0–2 · clip2 [2,3.5) src 8–9.5 · SPLIT at
   2.75 · RESIZE right half to 1.0 s · clip4 [3.75, 6.6733) src 20–22.9067;
3. track 2 (overlay): filler [0,1) src 40–41 (opacity Hold 0) + overlay
   [1,3.5) src 30–32.5 with SetKeyframes opacity (0→1→1→0, Linear) +
   x pan (−640→640 over 1.9 s) + y drift (30→90);
4. exact source-mapping spot checks (ADR-013 windows);
5. undo ×4 → pre-keyframe hash · redo ×4 → final hash (hash identity);
6. spot renders at 0 / 2.5 / 5.0 s (RGBA, geometry, non-black) +
   determinism (same t → identical BLAKE3);
7. `export_reencode` 160 frames → MP4 (mpeg4 CRF6 bitexact + AAC 128k CBR):
   frame count, video duration, audio duration all EXACT;
8. `export_wav` → 294,294 s16 samples (== span × 44100, exact);
9. `export_copy` → TYPED rejection on H.264 (v1 mpeg4/aac copy surface,
   ADR-015 — honest unsupported, pinned);
10. save → drop → reopen: SAME state hash, bindings alive, keyframed overlay
    re-render byte-identical;
11. re-export after reopen → file sha256 == first export (deterministic
    software pipeline);
12. machine-readable record → `$OVE_REALWORLD_OUT/realworld_record.json`.

## 5. Wave-RLW-1 results (2026-09-29, commit at HEAD c4b1d48 + this wave)

> **Scope of proof — binding wording for every claim derived from this
> workflow.** The certified artifact is a deterministic **edited segment**:
> 160 output frames = 6.673333 s, derived from the real source through real
> engine edits (split, resize, keyframed overlay, sample-exact audio cuts)
> and independently verified. The correct claim is: *"the engine processed a
> real source and produced a verified edited segment."* It is **NOT**
> *"the engine successfully processed the entire original video end-to-end"*
> — the full 122.88 s pass-through is not claimed anywhere, was not
> exercised, and is not a certified capability. Full-source traversal may
> become its own scenario row in a later wave; until that row exists and
> passes, this scope statement is the exact meaning of every PASS row below.

| Check | Result |
|---|---|
| import/probe vs independent baseline | PASS (geometry/cadence/audio/keyframes exact) |
| timeline edits (split/resize/move-free, keyframes) | PASS |
| undo/redo hash identity | PASS (4× round trip) |
| spot renders + determinism | PASS |
| re-encode export (A/V) | PASS — 160 frames, 6.673333 s both durations exact at the engine layer |
| WAV export | PASS — 294,294 samples exactly |
| persistence / reopen | PASS — same state hash, keyframes alive, byte-identical re-render |
| deterministic re-export | PASS — file sha256 identical (`baf23d2a…`) |
| independent output decode | PASS — ffprobe: 160 frames, 24000/1001, container 6.673333 s, clean `-xerror` decode |
| A/V duration relationship | PASS — video 6.673333 s; AAC stream 6.672993 s (Δ 15 samples ≈ 0.3 ms, one-granule edit-list accounting; WAV carries the exact 294,294) |
| visual sanity (9 output frames + mid-fade pair) | PASS — composites, fades (40 %/53 % blends verified visually), pan geometry lands on the computed pixel, no corruption/frozen/black frames |
| stream-copy export of H.264 | NOT YET IMPLEMENTED — typed rejection pinned (v1 = mpeg4/aac, ADR-015) |
| GPU parity on this media | N/A this wave (engine wiring is a platform-wave item; parity suite is corpus-level) |

### Wave-RLW-2 (W17 plugin tier) results (2026-09-30)

Scope: identical scope-of-proof wording as above — a verified EDITED
SEGMENT (here 80 frames = 3.336667 s), never a full-source pass-through.

| Check | Result |
|---|---|
| permanent proof re-run on the SAME source (sha256 gate `2d315daf…705f`) | PASS — every Wave-RLW-1 row above stayed green |
| plugin tier on real media (ADR-021): manifest → context → proposal applied | PASS — stub split proposal applied through the SAME command bus; receipt `realworld_plugin_receipt.json` |
| state-hash advance + receipt determinism | PASS (receipts byte-identical across runs) |
| render-invariance of the structural split | PASS — spot renders byte-identical pre/post at t=0.25 s and t=2 s |
| sample-exact audio across the plugin edit | PASS — audio duration == 147,147/44100 exactly |
| export of the plugin-edited project | PASS — 80 frames, video duration 80080/24000 exact |
| independent output decode | PASS — ffprobe: 80 frames, 24000/1001, container 3.336667 s, clean `-xerror` decode |
| visual sanity (plugin export frames at the invariance points) | PASS — real launch-site footage, exact geometry, no corruption |
| finding (pinned, not a defect) | undo of a plugin-applied Split restores structure but not the pre-command hash — id-state is document state (ADR-016/E-012); redo identity exact |

### Wave-RLW-3 (W18 security hardening) results (2026-09-30)

Scope: identical scope-of-proof wording as above — a verified EDITED
SEGMENT (80 frames = 3.336667 s), never a full-source pass-through. New
capability under certification: the ADR-022 untrusted-input resource
budgets, exercised ON the real reference media
(`ove-plugin/tests/realworld_security.rs`, env-gated; record
`realworld_record_security.json`, commit `85259f1a0f14`).

| Check | Result |
|---|---|
| permanent proof re-run on the SAME source (sha256 gate `2d315daf…705f`) | PASS — every Wave-RLW-1/RLW-2 row above stayed green on the hardened build (RLW-1 full proof 129.6 s run incl. save/kill/reopen/deterministic re-export) |
| W17 plugin gate re-run (existing certification) | PASS — plugin tier on real media unchanged (35.2 s run) |
| hostile plugin proposal-flood (11k default-DENY proposals) against the real-media session | PASS — typed budget abort (`proposal budget exhausted: more than 10000…`), state hash byte-unchanged, spot renders byte-identical |
| legit plugin still applies its edit after the hostile abort | PASS — split applied through the SAME command bus, receipt, hash advance |
| render invariance (structural split on real media) | PASS — t=0.25 s / t=2 s byte-identical pre/post |
| export of the post-attack post-edit project | PASS — 80 frames, video duration 80080/24000 exact; export sha256 `ff5e67f8…` IDENTICAL across two independent builds (determinism survives the hardening) |
| independent output decode | PASS — ffprobe: 80 frames, 24000/1001, container 3.336667 s |
| deterministic record (wave + commit + hash) | PASS — `realworld_record_security.json` with hostile-flood evidence (abort, unchanged-hash), legit-plugin receipt, export sha256 |

### Wave-RLW-4 (W19 performance) results (2026-09-30)

Scope: identical scope-of-proof wording as above — a verified EDITED
SEGMENT (160 frames = 6.673333 s), never a full-source pass-through. New
capability under certification: the ADR-023 export decode-session budget
(ONE decoder open per source per export), exercised ON the real reference
media. The source was RE-ACQUIRED this wave (the work environment was
wiped; media never lives in git) — the sha256 identity gate proves it is
byte-identical to the RLW-1 reference.

| Check | Result |
|---|---|
| source re-acquisition + sha256 identity gate `2d315daf…705f` | PASS — byte-identical to the §2 provenance record (ytagent farm run 36693710457, DASH parts, local stream-copy merge) |
| permanent proof re-run on the SAME source | PASS — every Wave-RLW-1/2/3 row stayed green on the optimized build (22.3 s run, was 129.9 s pre-fix) |
| W17 plugin gate re-run (existing certification) | PASS — 2.08 s |
| W18 security gate re-run (existing certification) | PASS — 2.33 s |
| ADR-023 session budget on real media | PASS — decoder opens per 160-frame export = 1 (was 160 pre-fix; measured by the `decoder_opens` instrument, pre-fix tree = worktree @ `adcafd9`) |
| export determinism ACROSS the optimization | PASS — export sha256 `baf23d2a…` IDENTICAL pre-fix vs post-fix (same source, same machine) — byte-identical output at 5.8× the speed |
| performance evidence (machine-relative, instrumented) | PASS — end-to-end proof 129.90 s → 22.26 s (5.8×); export leg ~65 s → 10.9 s (~6×); `realworld_record.json` `perf` section carries per-leg wall-clock + open counts |
| independent output decode | PASS — ffprobe: 160 frames, 24000/1001, container 6.673333 s, clean `-xerror` decode, WAV 294,294 samples |
| visual sanity (launch + overlay composite + clip4 close-up) | PASS — real launch-site footage, keyframed pan/fade geometry exact, no corruption |

### Wave-RLW-5 (W20 docs/release) results (2026-09-30)

Scope: identical scope-of-proof wording as above — a verified EDITED
SEGMENT (160 frames = 6.673333 s), never a full-source pass-through.
W20 is a documentation/release-hygiene wave: NO engine code changed
(gate tree `f2dd181` = `5e9d03b` + docs only), so this leg certifies
that the shipped RELEASE DOCS describe a machine whose real-media
certification still passes byte-identically. The source was re-provisioned
after the second environment wipe — via the W19 farm ARTIFACT (same
provenance chain), after a fresh acquisition attempt hit the expected
datacenter bot-wall.

| Check | Result |
|---|---|
| source re-provision + sha256 identity gate `2d315daf…705f` | PASS — byte-identical via the retained ytagent-farm artifact (run 36693710457, DASH f136+f140, local stream-copy merge); fresh farm run 36698485885 failed the bot-wall ("Sign in to confirm you're not a bot", all clients) — artifact reuse is the provenance-preserving path |
| independent baseline REGENERATED from scratch | PASS — the newly committed `scripts/realworld/baseline_analysis.sh` reproduced §3 exactly from ffprobe/ffmpeg: 2945 frames, 5,419,008 samples (full independent PCM decode), 48 keyframes, decode CLEAN |
| permanent proof re-run on the SAME source (commit-bound `OVE_COMMIT=f2dd181`) | PASS — 22.46 s; export sha256 `baf23d2a…` IDENTICAL to the RLW-1/2/3/4 certified output |
| ADR-023 session budget on real media | PASS — decoder opens per 160-frame export = 1 |
| W17 plugin gate re-run (existing certification) | PASS — 2.09 s, export sha256 `ff5e67f8…` IDENTICAL to the W18-certified value |
| W18 security gate re-run (existing certification) | PASS — 2.31 s |
| independent output verification | PASS — 15/15 boolean checks true, zero false: ffprobe 160 frames @ 24000/1001, WAV 6.673333 s, 9/9 visual frame extractions, launch-site pixel identity (begin mean_abs_diff 0.0) |
| release hygiene | 12 fully-merged stale branches deleted (wave-7/8/9/10-12/13-14/15/16/17/18/19, realworld-validation, fix/ci-bundled-ffmpeg-sigill); repo is `main`-only |
| documentation integrity | ADR-023 file committed (was referenced by the state doc, never written); README refreshed to the honest waves-0–19 / 13-crate posture; the §9 baseline generator now exists and is executable |

### Wave-RLW-6 (W21 production audit) results (2026-09-30)

Scope: identical scope-of-proof wording as above — a verified EDITED
SEGMENT (160 frames = 6.673333 s), never a full-source pass-through.
W21 is the production-audit wave: NO engine code changed (gate tree
`22a505a` = the audit commit; the docs commit lands after the gate).
This leg certifies that the AUDITED state — the tree the production
classifications and ADR-024 cite — still carries the full real-media
certification. The source was re-provisioned after the THIRD environment
wipe; the identity gate caught a new acquisition pitfall on the way.

| Check | Result |
|---|---|
| source re-provision + sha256 identity gate `2d315daf…705f` | PASS — but only after the gate REJECTED a non-identical variant: ytagent's new `cobalt_community` method (Tier-1 success, `d7e3019a…`, 122.92 s re-mux) failed the sha256 check; the certified bytes came from a fresh ytagent-farm run (DASH f136+f140, local stream-copy merge) — the same provenance chain as RLW-1..5 |
| acquisition recipe hardened from THREE findings | PASS — (a) `acquire_source.sh` now resolves the tier-1 file via `ytagent_result.json` `final_path` (the old `video_*` glob missed ytagent's naming) and gates every tier result against `EXPECTED_SHA256` (defaults to the §2 identity), escalating tiers on mismatch instead of accepting a byte-wrong "success"; (b) normalization REFUSES incomplete DASH parts (video without audio) with a loud typed-style error instead of silently remuxing a video-only source; (c) UPSTREAM DEFECT found and worked around: ytagent's `_download_artifact` extracts only the FIRST media file in the artifact zip and breaks — DASH artifacts (f136.mp4 + f140.m4a) come back video-only even though the artifact itself is complete (verified by zip listing). `acquire_source.sh` now completes the fetch-back by extracting ALL media parts from the same run's artifact (the YouTube download itself stays in the farm/ytagent mechanism — OVE grows no downloader). Full chain re-proven end-to-end: tier-1 rejection → farm escalation → incomplete-part refusal → artifact completion → identity gate PASS |
| independent baseline REGENERATED from scratch | PASS — `baseline_analysis.sh` + `baseline_facts.py` reproduced §3 exactly: 2945 frames, 5,419,008 samples (full independent PCM decode), 48 keyframes, decode CLEAN under `-xerror` |
| permanent proof re-run on the SAME source (commit-bound `OVE_COMMIT=22a505a`) | PASS — 22.29 s; export sha256 `baf23d2a…` IDENTICAL to the RLW-1/2/3/4/5 certified output; reopen re-export IDENTICAL; WAV 294,294 samples exact |
| ADR-023 session budget on real media | PASS — decoder opens per 160-frame export = 1 (and 1 for the reopen re-export) |
| W17 plugin gate re-run (existing certification) | PASS — 2.17 s, split applied through the command bus, receipt recorded, export sha256 `ff5e67f8…` IDENTICAL to the W18-certified value |
| W18 security gate re-run (existing certification) | PASS — 2.41 s; hostile 11k-proposal flood → typed budget abort (`proposal budget exhausted: more than 10000…`), state hash unchanged by the attack, legit plugin still applied its edit, export sha256 `ff5e67f8…` IDENTICAL |
| independent output verification | PASS — 15/15 boolean checks true, zero false: ffprobe 160 frames @ 24000/1001, container 6.673333 s, clean `-xerror` decode, AAC 6.672993 s, WAV 294,294, launch-site pixel identity (begin mean_abs_diff 0.0), seam/clip4/end frames consistent with the certified composites |
| visual sanity (9 frames at 0/0.5/1/2/2.7/3/4/5/6.6 s) | PASS — real Artemis I launch footage: night-crowd establishing shot ("NOVEMBER 16, 2022" title), keyframed overlay composite, pad close-up with ignition steam ("929" burn timer); exact geometry, no corruption/frozen/black frames |
| audited-tree integrity | PASS — the audit's evidence tree is the gate tree: workspace 189/189 GREEN, fmt/clippy clean, zero engine diffs between `22a505a` and the W20 gate tree (`f2dd181` + README/audit/ADR docs) |

## 6. Real defects found by real media (the point of this workflow)

| ID | Defect | Fix | Pin |
|---|---|---|---|
| REALWORLD-BUG-1 | Render plans declared the RAW probe color tags as the fetch contract, but the boundary conversion stamps fetched frames {src primaries/transfer, Bt709, Full}. Whenever probe range == working-space range (ANY real file with real LIMITED metadata), the plan omitted the ColorConvert stamp pass and exec failed `TagMismatch`. The synthetic corpus probes tag-Unknown — every existing test took the convert branch vacuously. | `build_render_input` now declares `boundary_rgba_stamp(video.color)` — the tags fetch actually delivers (single source of truth shared with the conversion). | `boundary_stamp_tests::stamp_matches_converted_envelope` |
| REALWORLD-BUG-2 | YUV→RGBA conversion hardwired LIMITED-range expansion even for declared FULL-range sources (level crush on real consumer media that declares Full). | Range-aware integer expansion (Limited / Full exact; Unknown keeps the documented limited assumption). | `limited_black_maps_to_zero`, `full_range_luma_is_identity` |
| REALWORLD-BUG-3 | `DecodeSource::fetch` seeked EXACTLY at the mapped target; on real NTSC media the target falls BETWEEN frame pts and the adapter's Exact seek forward-drops frames ≤ target (D-4) — the D-5 floor frame was never delivered (`SourceFrameMissing`). Corpus targets were always exact frame pts. | Fetch now lands at the greatest keyframe ≤ target (the ADR-013 `plan_seek` discipline) and decodes forward keeping the last frame ≤ target; sequential same-GOP targets reuse the decoder position; targets at/before the last floor re-seek. | the realworld test itself (renders the NTSC source end-to-end) |
| REALWORLD-BUG-4 | `fetch` DISCARDED the popped past-target frame. Latent while decode sessions died every frame (per-frame rebuilds re-seeked everything — the BUG-3 cursor reuse was dead code in exports); the W19 export session lifetime (ADR-023) made the next sequential fetch — whose target IS that frame's pts under CFR — decode past its floor and return None (`SourceFrameMissing at 1/24`). Found by the W19 failing budget test the same day the lifetime fix landed. | One-frame `pending` pushback in `VideoSession` (bounded memory): the past-target frame is cached and served as the next fetch's floor candidate; a target between the last floor and the pending frame (VFR/multi-rate gap) re-seeks and rebuilds — the D-5 floor rule stays total. YUV→RGBA converts once on the final floor frame. | `export_session_budget.rs` (the failing test that exposed it) + the permanent proof re-export sha256 identity |

RW-NOTE-1 (recorded, unfixed by design this wave): `record_entry_bindings`
does not consume Split's right half — `clip_assets` has no entry for the new
clip id (5 of 6 clips). Inert under the v1 single-source render; MUST be
closed before multi-source render-by-binding lands (it would bind the wrong
source). Assertion in the realworld test documents the current count.

## 7. Capability matrix (as exercised by THIS wave)

| Capability | Status |
|---|---|
| Asset ingestion + BLAKE3 content identity | IMPLEMENTED, TESTED, EXERCISED |
| Media probing (streams/keyframes/VFR/color tags) | IMPLEMENTED, TESTED, EXERCISED |
| Timeline insertion / split / resize / remove surface | IMPLEMENTED, TESTED, EXERCISED (explicit ids) |
| Exact rational timing + NTSC cadence | IMPLEMENTED, TESTED, EXERCISED |
| Source time mapping (ADR-013) | IMPLEMENTED, TESTED, EXERCISED |
| Keyframe animation (opacity/x/y, Linear/Hold) | IMPLEMENTED, TESTED, EXERCISED |
| Undo/redo with hash identity | IMPLEMENTED, TESTED, EXERCISED |
| Frame decoding → FrameEnvelope → boundary conversion | IMPLEMENTED, TESTED, EXERCISED (after REALWORLD-BUG-1/3 fixes) |
| RenderPlan compile + software render | IMPLEMENTED, TESTED, EXERCISED |
| A/V re-encode export (mpeg4 + AAC, exact durations) | IMPLEMENTED, TESTED, EXERCISED |
| WAV/PCM export (sample-exact) | IMPLEMENTED, TESTED, EXERCISED |
| Project persistence, kill-safe logs, reopen replay | IMPLEMENTED, TESTED, EXERCISED |
| Deterministic re-export | IMPLEMENTED, TESTED, EXERCISED |
| Plugin tier (process plugins proposing commands, ADR-021) | IMPLEMENTED, TESTED, EXERCISED (real media, wave-RLW-2) |
| CLI / batch / MCP / Rhai clients | IMPLEMENTED, TESTED (corpus-level) — not exercised by this test yet |
| Stream-copy export of H.264 sources | NOT YET IMPLEMENTED — typed rejection (pinned) |
| Multi-track audio mixing, audio retime | NOT YET IMPLEMENTED (ADR-018 gaps) |
| GPU rendering wired into the engine | NOT YET IMPLEMENTED at engine level (parity suite exists) |
| Scaling/affine transforms | NOT YET IMPLEMENTED (integer translate only) |
| H.264/AV1 encoding out | NOT YET IMPLEMENTED (v1 mpeg4-first, ADR-015) |

## 8. Regression rule and the certification loop (binding per wave)

For every future major wave: re-run this test against the SAME source
(re-acquire via `scripts/realworld/acquire_source.sh` if the local copy is
gone — the sha256 in §2 is the identity gate), add the newly implemented
capabilities to the scenario, extend §5 with a new wave-RLW row, and keep the
previous records (the machine-readable `realworld_record.json` per run is the
evidence trail). A wave that regresses any PASS row above is not complete
unless the change is an intentional, documented contract change.

The certification loop is the definition of wave progress — every wave
passes through it before merge:

```
    Real reference video
            │
            ▼
    Current OVE capabilities
            │
            ▼
    Technical verification   (independent tools; OVE is never the authority)
            │
            ▼
    Visual proof             (frame inspection at representative points)
            │
            ▼
    Record wave + commit + hash   (realworld_record.json evidence trail)
            │
            ▼
         NEXT WAVE  ──▶ repeat
```

Required wave-completion statement (no weaker paraphrase counts):

> "Wn implemented. Existing real-media certification still passes. The new
> capability was exercised on real media. Output independently verified."
> — with wave id, merge commit, engine state hash, and the
> `realworld_record.json` path.

A wave that reports only "implemented, N tests pass" is NOT complete under
this protocol. The rhythm is: build wave → real-video gate → fix genuine
regressions → verify → merge → next wave. The gate exists to catch real
defects early (RLW-1 caught three) — not to replace engine building; if the
gate passes, move on.

## 9. Reproducibility

```bash
# 1. acquire (never commits media; identity-gated)
scripts/realworld/acquire_source.sh "https://www.youtube.com/watch?v=jrDv0OdMt5s" /tmp/rlw/source
sha256sum -c   # must equal 2d315daf…705f (§2)

# 2. independent baseline facts (ffprobe authority)
scripts/realworld/baseline_facts.py <baseline_dir> baseline_facts.json

# 3. run the permanent proof
OVE_REALWORLD_SOURCE=/tmp/rlw/source/source.mp4 \
OVE_REALWORLD_EXPECT=baseline_facts.json \
OVE_REALWORLD_OUT=/tmp/rlw/proof \
cargo test -p ove-engine --release --test realworld

# 4. independent output verification + visual frames
scripts/realworld/verify_output.py /tmp/rlw/proof <source.mp4>

# 5. W17 plugin-tier gate (certification loop leg)
OVE_REALWORLD_SOURCE=/tmp/rlw/source/source.mp4 \
OVE_REALWORLD_EXPECT=baseline_facts.json \
OVE_REALWORLD_OUT=/tmp/rlw/proof \
cargo test -p ove-plugin --release --test realworld_gate

# 6. W18 security gate — hostile flood + legit edit on the real media
OVE_REALWORLD_SOURCE=/tmp/rlw/source/source.mp4 \
OVE_REALWORLD_EXPECT=baseline_facts.json \
OVE_REALWORLD_OUT=/tmp/rlw/proof \
cargo test -p ove-plugin --release --test realworld_security
```

Environment notes (2026-09-29): datacenter IPs are hard-blocked by YouTube
(429 + LOGIN_REQUIRED); the ytagent farm tier (WARP + android_vr + manual GVS
PO token) is the observed-working acquisition path. GitHub's 2026 runner
images lack ffmpeg — the farm returns unmerged DASH parts; normalization is a
local stream copy. The YouTube-side JSON license metadata could not be
fetched from this environment; the public-domain status rests on NASA's
published media guidelines (recorded above).

## 10. Harder real-media corpus (PLANNED — primary stays primary)

The NASA clip is the PRIMARY permanent reference; it is never replaced or
demoted. The next robustness step (its own later wave, NOT a substitution)
is a small corpus of deliberately difficult real media:

```
REALWORLD/
├── primary_nasa.mp4      # the permanent reference (§2) — proves ordinary operation
├── portrait_phone.mp4    # rotation / display-matrix metadata stress        (planned)
├── vfr_phone.mp4         # variable-frame-rate phone capture                (planned)
├── long_gop.mp4          # long-GOP source: seek + forward-decode stress    (planned)
└── audio_variant.mp4     # different audio geometry (48 kHz mono / 5.1)     (planned)
```

The primary proves ordinary real-world operation; the harder corpus proves
robustness. Rules for every corpus addition — identical discipline to §2–§3:
license-clean public-domain/CC source · full provenance record · independent
ffprobe baseline · sha256 identity gate · never committed to git · each new
file gets its own scenario rows and per-run evidence records. No addition may
weaken or replace the primary NASA scenario.
