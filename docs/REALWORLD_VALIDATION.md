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

### Wave-RLW-7 (RW-NOTE-1 closure — Split clip_assets binding) results (2026-09-30)

Scope: identical scope-of-proof wording as above — a verified EDITED
SEGMENT (160 frames = 6.673333 s), never a full-source pass-through.
New capability under certification: the RW-NOTE-1 binding bookkeeping
closure (`record_entry_bindings` consumes Split; snapshot-mirror binding
restore at open). The engine's render path does not read `clip_assets`
(v1 single-source render), so the certified OUTPUT BYTES must not move —
and they did not: both pinned hashes re-verified IDENTICAL.

| Check | Result |
|---|---|
| source re-provision + sha256 identity gate `2d315daf…705f` | PASS — farm run 36787738825 (DASH f136+f140, local stream-copy merge); the sandbox lost the local copy and the acquire-script background run was reaped, so the script's documented farm steps were orchestrated manually (dispatch → poll → artifact fetch-back → normalize → gate); provenance chain identical to RLW-1..6 |
| independent baseline REGENERATED from scratch | PASS — `baseline_analysis.sh` + `baseline_facts.py` reproduced §3 exactly: 2945 frames, 5,419,008 samples (full independent PCM decode), 48 keyframes, decode CLEAN under `-xerror` |
| permanent proof re-run on the SAME source (commit-bound `OVE_COMMIT=c41f520`) | PASS — 22.26 s; export sha256 `baf23d2a…` IDENTICAL to the RLW-1..6 certified output; reopen re-export IDENTICAL; WAV 294,294 samples exact |
| ADR-023 session budget on real media | PASS — decoder opens per 160-frame export = 1 |
| W17 plugin gate re-run (existing certification) | PASS — 2.15 s, split applied through the command bus, receipt recorded, export sha256 `ff5e67f8…` IDENTICAL to the W18-certified value |
| W18 security gate re-run (existing certification) | PASS — 2.38 s; hostile 11k-proposal flood → typed budget abort, state hash unchanged by the attack, legit plugin still applied its edit, export sha256 `ff5e67f8…` IDENTICAL |
| RW-NOTE-1 binding assertion on real media | PASS — realworld scenario bindings 5/6 → **6/6** (clip2b carries the SAME asset binding as clip2; assertion updated in the test) |
| new binding conformance (unit level) | PASS — ove-project/tests/bindings.rs 6/6: split binds right half with no loss/reassignment; chain inheritance; undo/redo map stability (binding permanence); full replay parity (map + hash) incl. undo/redo markers; compaction parity (snapshot mirror restore + suffix append); split-only-history reopen |
| independent output verification | PASS — 15/15 boolean checks true, zero false: clean `-xerror` decode of all three gate outputs (160/80/80 frames @ 24000/1001), container 6.673333 s / 3.336667 s / 3.336667 s, WAV 294,294, launch-site pixel identity (begin mean_abs_diff 0.0) |
| visual sanity (representative frames) | PASS — begin (black fade-in, pixel-identical to source), split seam 2.70 s (left/right segments at the certified boundary), clip4 5.00 s (pad close-up, "929" burn timer), overlay mid-fade 1.50 s (keyframed composite exact); no corruption/frozen/black-error frames |

### Wave-RLW-8 (hostile/difficult real-media corpus, §10 executed) results (2026-10-01)

Scope: the §10 corpus leg — difficult-but-benign real media, NOT
intentionally weaponized media. Seven items across the four planned classes
(portrait/rotation, VFR, long-GOP, audio-geometry variants), one scenario
row each: import → probe vs INDEPENDENT ffprobe baseline → edits
(clip/split/resize) → undo/redo contract windows → spot renders +
determinism → exact-frame export → WAV → save/reopen (hash + 3/3 bindings)
→ byte-identical re-export. The PRIMARY NASA certification re-ran FIRST as
the regression control. The decoder pixel-bomb residual (ADR-022) is
deliberately NOT exercised and remains open (RLW-9 scope, user-separated).
New committed evidence tooling: `corpus_baseline.py` (ffprobe-authoritative
per-item baselines) and `corpus_verify.py` (independent output verification
+ visual frames); harness `ove-engine/tests/corpus_realworld.rs`
(env-gated, CI stays corpus-only).

| Check | Result |
|---|---|
| source re-provision + sha256 identity gate `2d315daf…705f` | PASS — farm run 36833816289 (DASH f136+f140, local stream-copy merge); tier-1 correctly REJECTED the known re-mux variant `d7e3019a…` and escalated (hardened acquisition re-proven) |
| independent baseline REGENERATED from scratch | PASS — §3 reproduced exactly: 2945 frames, 5,419,008 samples, 48 keyframes, decode CLEAN under `-xerror` |
| permanent proof re-run on the SAME source (commit-bound `OVE_COMMIT=1e01804`) | PASS — 22.23 s; export sha256 `baf23d2a…` IDENTICAL to the RLW-1..7 certified output (FIFTH full environment rebuild — rustc 1.98.1, user-prefix libav 7.1.5 dev — and the bytes did not move); reopen re-export IDENTICAL; WAV 294,294 exact; decoder opens = 1 |
| W17 plugin gate re-run | PASS — 2.06 s, export sha256 `ff5e67f8…` IDENTICAL |
| W18 security gate re-run | PASS — 2.37 s, export sha256 `ff5e67f8…` IDENTICAL |
| C1 portrait_true (NASA AVATAR vertical, 406×720 true-portrait, PD) | PASS — probe==ffprobe (dims/KF/CFR), edits+renders at 406×720 exact, export 96 frames @ 30/1 clean, WAV 153,600 exact, reopen 3/3 bindings + byte-identical re-export |
| C2 rotation_metadata (primary + Display Matrix rotation=90, derived) | PASS (with finding RLW-8-F5) — decodes/edits/exports correctly in STORAGE orientation (1280×720, pixels verified against the pad footage); the display matrix is neither applied nor exposed (v1 probe schema has no rotation field — U-3 CONFIRMED as a capability limit, typed surface, visual proof captured) |
| C3 vfr_constructed (3 real-footage segments 15/30/10 fps concat; silent) | TYPED-FAIL (finding RLW-8-F3) — probe detects genuine VFR (D-12 ✓, 3 nominal deltas vs ffprobe), silent-media WAV = typed `NoAudioStream` ✓; renders/exports at source targets where frames VERIFIABLY exist (ffprobe pts 1.000000 s / 2.600000 s) fail typed `SourceFrameMissing` — fetch-cursor defect on B-frame VFR media, reproducible, no corruption/crash |
| C4 long_gop (primary re-encode x264 −g 600 −sc_threshold 0 −bf 3; 5 KF, max GOP 25.025 s) | PASS — mid-GOP trims/renders decode forward through 25 s GOPs, export 80 frames exact, WAV 147,147 exact, reopen identical; decoder opens 3 (keyframe-relative re-seek, see RLW-8-F6) |
| C5 audio_48k (primary video copy + AAC 48 kHz stereo) | PASS — WAV 160,160 exact; A/V out AAC 48 kHz stereo verified |
| C6 audio_mono (48 kHz mono) | PASS — mono geometry preserved end-to-end (probe 1 ch → A/V out 1 ch → WAV 1 ch, 160,160 exact) |
| C7 audio_51 (48 kHz 5.1, 6 ch) | PASS (with typed capability limit RLW-8-F4) — 6-ch geometry survives probe/edits/AAC re-encode (independent ffprobe: 6 ch 48 kHz out); WAV export typed-rejects `InvalidConfig("WAV v1 supports 1..=2 channels, got 6")` — honest v1 surface, no corruption |
| probe VFR report on B-frame CFR media | FALSE-POSITIVE (finding RLW-8-F1) — VfrReport is computed from PACKET-order pts; B-frame reordering makes any B-frame CFR file report `is_vfr=true` with reordering-shaped deltas (`1001/6000`, `−1001/12000`, …) vs ffprobe frame-order uniform `1001/24000`; informational surface (scheduling never consumes it), render/export bytes unaffected |
| independent output verification (corpus_verify.py) | PASS — 6 A/V exports: decode CLEAN under `-xerror`, frame counts EXACT (96/80/80/80/80/80), cadence + dims exact, audio geometry preserved (2/2/2/1/6 ch); vfr_constructed: 2/2 source-frame evidence booleans |
| visual sanity (36 output frames + 2 VFR source frames) | PASS — true-portrait title card at 406×720, pad footage in storage orientation (rotation finding proof), mid-GOP launch footage clean, no corruption/frozen/black frames |
| workspace / fmt / clippy | PASS — 200/200 (+5 corpus scenarios), fmt GREEN, clippy `-D warnings` GREEN |
| harness-found engine behavior (positive) | a first harness draft passed AXIS ticks as seconds (clip dur 76,797 s); the engine returned a TYPED `Internal("audio source exhausted: …")` instead of garbage or a crash — typed-contract behavior on absurd input, recorded (RLW-8-F2, harness-side, fixed in-harness) |

**RLW-8 findings registry** (full detail in the wave report):

| ID | Class | Type | Disposition |
|---|---|---|---|
| RLW-8-F1 | correctness (probe surface) | typed, deterministic | NEW DEFECT — VfrReport from packet-order pts false-positives on B-frame CFR media; fix = frame-order analysis (follow-up wave; NOT fixed in RLW-8) |
| RLW-8-F2 | harness bug (not engine) | — | units bug in the first harness draft; engine behaved typed-correct; fixed in-harness |
| RLW-8-F3 | correctness (render fetch) | typed, deterministic | NEW DEFECT — `SourceFrameMissing` at verifiably-existing frames on B-frame VFR media; VFR class currently render-blocked; fix = reorder-aware floor logic (follow-up wave; NOT fixed in RLW-8) |
| RLW-8-F4 | capability limit | typed | WAV v1 caps at 2 ch (A/V route carries 6 ch correctly) — recorded, not a defect |
| RLW-8-F5 | capability limit (U-3 confirmed) | typed | display-matrix rotation neither applied nor exposed; pixels correct; multi-source/render waves own the semantic decision |
| RLW-8-F6 | observation | — | decoder opens 1–3 across corpus cuts: keyframe-relative D-5 floor re-seeks (same media: t0=30 → 1 open, t0=10 → 3); ADR-023 pin holds on the primary scenario; per-open-reason instrumentation suggested for a future wave |

## 6. Real defects found by real media (the point of this workflow)

| ID | Defect | Fix | Pin |
|---|---|---|---|
| REALWORLD-BUG-1 | Render plans declared the RAW probe color tags as the fetch contract, but the boundary conversion stamps fetched frames {src primaries/transfer, Bt709, Full}. Whenever probe range == working-space range (ANY real file with real LIMITED metadata), the plan omitted the ColorConvert stamp pass and exec failed `TagMismatch`. The synthetic corpus probes tag-Unknown — every existing test took the convert branch vacuously. | `build_render_input` now declares `boundary_rgba_stamp(video.color)` — the tags fetch actually delivers (single source of truth shared with the conversion). | `boundary_stamp_tests::stamp_matches_converted_envelope` |
| REALWORLD-BUG-2 | YUV→RGBA conversion hardwired LIMITED-range expansion even for declared FULL-range sources (level crush on real consumer media that declares Full). | Range-aware integer expansion (Limited / Full exact; Unknown keeps the documented limited assumption). | `limited_black_maps_to_zero`, `full_range_luma_is_identity` |
| REALWORLD-BUG-3 | `DecodeSource::fetch` seeked EXACTLY at the mapped target; on real NTSC media the target falls BETWEEN frame pts and the adapter's Exact seek forward-drops frames ≤ target (D-4) — the D-5 floor frame was never delivered (`SourceFrameMissing`). Corpus targets were always exact frame pts. | Fetch now lands at the greatest keyframe ≤ target (the ADR-013 `plan_seek` discipline) and decodes forward keeping the last frame ≤ target; sequential same-GOP targets reuse the decoder position; targets at/before the last floor re-seek. | the realworld test itself (renders the NTSC source end-to-end) |
| REALWORLD-BUG-4 | `fetch` DISCARDED the popped past-target frame. Latent while decode sessions died every frame (per-frame rebuilds re-seeked everything — the BUG-3 cursor reuse was dead code in exports); the W19 export session lifetime (ADR-023) made the next sequential fetch — whose target IS that frame's pts under CFR — decode past its floor and return None (`SourceFrameMissing at 1/24`). Found by the W19 failing budget test the same day the lifetime fix landed. | One-frame `pending` pushback in `VideoSession` (bounded memory): the past-target frame is cached and served as the next fetch's floor candidate; a target between the last floor and the pending frame (VFR/multi-rate gap) re-seeks and rebuilds — the D-5 floor rule stays total. YUV→RGBA converts once on the final floor frame. | `export_session_budget.rs` (the failing test that exposed it) + the permanent proof re-export sha256 identity |

RW-NOTE-1 (CLOSED by RLW-7, 2026-09-30 — was: recorded, unfixed by design
this wave): `record_entry_bindings` now consumes Split — the right half
carries the SAME asset binding as the split clip (execute AND replay; the
snapshot mirror carries the map across compaction). The realworld scenario
asserts 6/6 bound clips. Closed WITHOUT touching the v1 single-source
render semantics (the render path never read the binding map); the
certified export hashes stayed byte-identical (`baf23d2a…`, `ff5e67f8…`).
The gap remains a live correctness REQUIREMENT for the future
multi-source render-by-binding wave: any binding-model change must keep
the RLW-7 replay/compaction parity pins green.

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

## 10. Harder real-media corpus (EXECUTED by RLW-8, 2026-10-01 — primary stays primary)

The NASA clip is the PRIMARY permanent reference; it is never replaced or
demoted. RLW-8 executed this corpus as a hardening/evidence wave: every
item passed acquisition → identity gate → independent baseline → OVE
processing → independent verification → visual inspection → recorded
result (§5 RLW-8 rows + the wave report). Results were recorded as found —
two new typed defects (RLW-8-F1/F3) and three capability limits were
classified rather than normalized.

```
REALWORLD/  (media NEVER in git; identities below are the gates)
├── primary_nasa.mp4      # the permanent reference (§2) — certified RLW-1..8
├── portrait_true.mp4     # true-portrait storage: NASA "Artemis II Science -
│                         #   AVATAR Vertical Video" (~medium rendition), public
│                         #   domain (images-assets.nasa.gov), h264 406x720 CFR
│                         #   30/1, AAC 48k stereo, 2273 f, 75.77 s
│                         #   sha256 0f2c250f76c3746ab1df8a4b9837415c471c7a0eec
│                         #   7706758bda42fc5d4620e1
├── rotation_metadata.mp4 # DERIVED (documented transform): primary + Display
│                         #   Matrix rotation=90 via
│                         #   `ffmpeg -display_rotation 90 -i primary -c copy`
│                         #   (the phone-portrait pattern; pixels untouched)
│                         #   sha256 06a07df4197638b0855ff4ce6c31448a42d0b66c1a
│                         #   dc07770e47303d3c10d2b7
├── vfr_constructed.mp4   # DERIVED: three real-footage segments (src 0-4 @15fps,
│                         #   8-12 @30fps, 20-24 @10fps) concat-filtered and
│                         #   x264-encoded — genuinely uneven frame timing
│                         #   (avg 220000000/11933333 ≠ r 30/1, 5 nominal
│                         #   delta clusters), silent (no audio stream)
│                         #   sha256 a1c6549a22ba8f1dabfcadbeba32089a973aa5dcf41
│                         #   ae3d1007c8b9968e315c0
│                         #   NOTE: the planned GENUINE phone-capture VFR item
│                         #   (Commons "InFocus M330 screencast Mobizen",
│                         #   Apache-2.0) was BLOCKED: upload.wikimedia.org
│                         #   returns 429 to this datacenter IP (recorded
│                         #   attempt evidence); the constructed item derives
│                         #   from the certified PD primary and is labeled as
│                         #   derived, not phone-captured
├── long_gop.mp4          # DERIVED: primary re-encode
│                         #   `x264 -preset veryfast -crf 20 -g 600
│                         #   -sc_threshold 0 -bf 3 -b-pyramid normal`
│                         #   + AAC audio — 5 keyframes, max GOP 25.025 s
│                         #   sha256 b9cd7602591d1d5f31bbc50d8e1ab0ad14268cc6e
│                         #   a74f74bbdcfddf5c23d3788
├── audio_48k.mp4         # DERIVED: primary video copy + `-c:a aac -ar 48000`
│                         #   sha256 4720f0d565f13d7cbebc05f65badc7a5e1a44c5196
│                         #   01fe870072ff1130d54954
├── audio_mono.mp4        # DERIVED: primary video copy + `-c:a aac -ar 48000
│                         #   -ac 1`   sha256 9e75a0022dee552f64dea4d2799227c16
│                         #   c834db1a675465b4efe4b52338bd972
└── audio_51.mp4          # DERIVED: primary video copy + `-c:a aac -ar 48000
                          #   -ac 6 -channel_layout 5.1`
                          #   sha256 b9c22f065fc196771d4e2a0d9f461d6a25929c6c7f
                          #   cbf71e91cb08f369c69723
```

Re-run recipe (identical discipline to §9):

```bash
# per-item independent baselines (ffprobe authority)
scripts/realworld/corpus_baseline.py <item.mp4> <item.baseline.json>

# corpus scenarios (env-gated; CI stays corpus-only)
OVE_COMMIT=<gate-commit> OVE_CORPUS_OUT=<proof-dir> \
OVE_CORPUS_PORTRAIT=<portrait_true.mp4> \
OVE_CORPUS_PORTRAIT_BASELINE=<portrait_true.baseline.json> \
OVE_CORPUS_ROTATION=<rotation_metadata.mp4> \
OVE_CORPUS_ROTATION_BASELINE=<rotation_metadata.baseline.json> \
OVE_CORPUS_VFR=<vfr_constructed.mp4> \
OVE_CORPUS_VFR_BASELINE=<vfr_constructed.baseline.json> \
OVE_CORPUS_LONGGOP=<long_gop.mp4> \
OVE_CORPUS_LONGGOP_BASELINE=<long_gop.baseline.json> \
OVE_CORPUS_AUDIO_DIR=<dir with audio_48k/mono/51 .mp4+baseline.json> \
cargo test -p ove-engine --release --test corpus_realworld

# independent output verification + visual frames
scripts/realworld/corpus_verify.py <proof-dir> <frames-dir>
```

Rules that governed RLW-8 (unchanged for future corpus legs): license-clean
sources with full provenance · independent ffprobe baselines · sha256
identity gates · never committed to git · per-item scenario rows and
evidence records · defects are CLASSIFIED, never normalized · no addition
may weaken or replace the primary NASA scenario · weaponized media
(pixel-bomb) stays OUT of this corpus (ADR-022 residual — RLW-9 scope).
