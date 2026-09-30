# MASTER ENGINE STATE — single authoritative work order

> RULE (takeover directive §6): this is the ONE current engineering truth.
> Other status documents are historical/evidence records and stay untouched.
> Update this file at the end of every major wave using the §40 report format.

WAVE: 21 COMPLETE (WAVE PLAN 0–21 COMPLETE) + REALWORLD VALIDATION WAVES COMPLETE (… / 9 GPU / 10–12 platform-leg evidence / 13 conformance / 14 headless batch / 15 AI-MCP / 16 scripting / 17 plugins / 18 security / 19 perf / 20 docs-release / 21 production audit — all landed; RLW-1/2/3/4/5/6 = real-world certification trail)
DATE: 2026-09-30
COMMIT: W3 = PR #5; W4 = PR #6; W5 = PR #7; W6 = PR #8; W7 = PR #9; W8 = PR #10; W9 = PR #11; W10–12 = PR #12; W13–14 = PR #13; W15 = PR #14; W16 = PR #15; RLW-1 = PR #16; W17 = PR #17; W18 = PR #18 (a8bc078); W19 = PR #19 (faec389); W20 = PR #20 (1d889a0); W21 = PR #21 (bee72c5, main CI 5/5 green, tagged v0.1.0 audited-state POINTER)

POST-W21 POSTURE (binding, per the W21 audit §3 decision 3): the wave plan
(0–21) is COMPLETE. The repository continues under the CERTIFICATION LOOP
only — any future engineering wave re-runs the real-media gate (RLW-7+) and
updates this file. The named residuals are the roadmap, not forgotten:
platform runtime legs (D7 PARTIAL), real-GPU (D8 EXPERIMENTAL), release
pipeline tooling (D10 PARTIAL), hostile REALWORLD/ corpus, RW-NOTE-1
clip_assets binding, log-flood CPU budget, CI bundled-FFmpeg SIGILL. Next
concrete engineering wave when one is commissioned: close RW-NOTE-1 or the
hostile-corpus leg (REALWORLD_VALIDATION §10) — both have committed reopen
conditions.

## WAVE 20 deltas (2026-09-30) — docs/release (documentation integrity, release hygiene, RLW-5)

1. **Documentation integrity — the ADR-023 file now exists**: the W19
   deltas and the ADR registry both referenced ADR-023, but the FILE was
   never committed (the W19 docs commit touched only the two state
   docs). `research/adr/ADR-023-export-decode-session-budget.md` now
   records the real decision from its committed evidence: the
   deterministic open-count instrument, the session-lifetime change,
   REALWORLD-BUG-4, the byte-identity pin, and the named residuals.
   No content was invented — every claim in the ADR cites the W19
   record.
2. **README refreshed to the honest posture**: the README was frozen at
   the takeover audit ("Waves 0–2 landed", 4 crates, ADR range 001..012).
   It now states waves 0–19, the real 13-crate layout with per-crate
   roles, the ADR range 001..023, the certification loop + security +
   performance posture, and re-qualifies STATUS.md as the AUDIT-DAY
   SNAPSHOT (kept untouched per the §6 rule — other status docs are
   historical records; the state doc is the only live truth). The
   libav adapter allowlist wording follows the CI-enforced closed set
   {ove-decode, ove-encode} (DECODER_SPEC §5 + ENCODER_SPEC §5, ADR-015).
3. **The §9 baseline generator now exists**: REALWORLD_VALIDATION §9
   step 2 referenced `baseline_facts.py`, whose own docstring referenced
   a `baseline_analysis.sh` that was never committed — the
   reproducibility recipe was not executable as written. The generator
   is now committed and was PROVEN this wave: it reproduced §3 exactly
   from scratch (2945 frames, 5,419,008 samples via full independent
   PCM decode — ffprobe has no `-count_samples`, so the sample count is
   byte-derived from a decode, which is more independent than metadata;
   48 keyframes from packet flags; decode CLEAN under `-xerror`).
4. **Release hygiene — 12 fully-merged stale branches deleted** (the
   wave-7/8/9/10-12/13-14/15/16/17/18/19 topic branches,
   realworld-validation, and fix/ci-bundled-ffmpeg-sigill — all verified
   `--merged main` before deletion). The repo is `main`-only. No tags,
   no CHANGELOG, no release machinery was invented: distribution
   strategy stays gated on the W21 production-readiness audit
   (anti-overbuild).
5. **RLW-5 certification (REALWORLD_VALIDATION §5)**: the source was
   re-provisioned byte-identically after the second environment wipe
   (sha256 gate `2d315daf…705f`) — via the RETAINED W19 farm artifact
   after a fresh farm run hit the expected datacenter bot-wall. The
   permanent proof re-ran commit-bound (`OVE_COMMIT=f2dd181`): 22.46 s,
   export sha256 `baf23d2a…` IDENTICAL, decoder opens = 1; W17 gate
   2.09 s and W18 gate 2.31 s re-passed with export sha256 `ff5e67f8…`
   IDENTICAL; independent verification 15/15 booleans true, 9/9 visual
   frames. Every RLW-1..4 PASS row stayed green.
6. **Honest scope**: W20 changed ZERO engine code (docs + one shell
   script); the workspace stays **189/189** GREEN, fmt/clippy GREEN.
   What W20 deliberately did NOT do: no STATUS.md rewrite (§6 rule),
   no tag/CHANGELOG/release pipeline (W21 decision), no CI SIGILL
   change (tracked), no clip_assets binding change (RW-NOTE-1 tracked
   until the multi-source/binding model lands).

## WAVE 21 deltas (2026-09-30) — production audit (PRODUCTION_READINESS_AUDIT, ADR-024, RLW-6)

1. **The production-readiness audit exists and is honest**:
   `research/audit/PRODUCTION_READINESS_AUDIT.md` classifies 12
   production dimensions using the charter's eight allowed labels, each
   citing committed evidence. The verdict stays in charter language: the
   headless core (D1/D2/D3/D4/D5/D11/D12) is TESTED end to end,
   real-media-certified six waves in a row; **the engine as a whole is
   NOT declared production-ready** — D7 PARTIAL (real-device platform
   legs), D8 EXPERIMENTAL (real GPU), D10 PARTIAL (release tooling),
   each with named reopen conditions. No label was upgraded by prose.
2. **ADR-024 — codec-artifact distribution decided** (the README
   decision W20 explicitly gated on this audit): the libav-free core
   ships MIT OR Apache-2.0 without qualification; libav-linked adapter
   builds are LGPL-2.1+-compliant SEPARATE artifacts (dynamic linkage,
   no GPL/nonfree flags — CI-audited by the confinement job +
   cargo-audit); GPL encoder packs stay out-of-tree (doc 35 §5); no
   vendored FFmpeg binaries in v1.
3. **Versioning rule recorded, machinery-free**: versions stay
   wave-indexed (`v0.<wave>` posture); crates stay `0.1.0` until a
   release pipeline exists — mass version-bumping without artifacts
   would be documentation-only progress. The audited state is tagged
   **v0.1.0** as an audited-state POINTER — not a completeness, quality,
   or production-readiness claim.
4. **RLW-6 certification (REALWORLD_VALIDATION §5)**: the source was
   re-provisioned byte-identically after the THIRD environment wipe
   (sha256 gate `2d315daf…705f`) — fresh ytagent-farm run 36767686335
   (DASH f136+f140, local stream-copy merge) after the new cobalt
   fallback path returned a RE-MUXED variant that failed the identity
   gate (`d7e3019a…` ≠ `2d315daf…`) — the gate did its job and the
   acquisition script was hardened accordingly (identity-gated tier
   escalation, `final_path`-based normalization). Independent baseline
   regenerated: 2945 frames / 48 keyframes / 5,419,008 samples / decode
   CLEAN — §3 reproduced exactly. The permanent proof re-ran
   commit-bound (`OVE_COMMIT=22a505a`, the audit tree): 22.29 s, export
   sha256 `baf23d2a…` IDENTICAL to the RLW-1..5 certified output,
   reopen re-export IDENTICAL, WAV 294,294 exact, decoder opens = 1
   (ADR-023). W17 gate 2.17 s and W18 gate 2.41 s re-passed with export
   sha256 `ff5e67f8…` IDENTICAL (W18 hostile 11k-proposal flood → typed
   abort, engine unchanged). Independent verification 15/15 booleans
   true, 9/9 visual frames, launch-site pixel identity (begin
   mean_abs_diff 0.0). Every RLW-1..5 PASS row stayed green.
5. **Acquisition recipe hardened by the loop itself (three findings,
   all gate-verified)**: the certification loop caught real robustness
   gaps in the reference-media recipe — (a) `acquire_source.sh` treated
   a byte-wrong tier-1 result as success (no identity gate inside the
   script; `video_*` glob missed ytagent's `final_path` naming) — fixed
   with `EXPECTED_SHA256`-gated tier escalation + `final_path`
   resolution; (b) normalization silently remuxed a video-only source
   from incomplete DASH parts — now a loud refusal; (c) UPSTREAM DEFECT:
   ytagent's `_download_artifact` extracts only the FIRST media file in
   the artifact zip and breaks (DASH f136+f140 came back video-only
   while the artifact zip itself is complete) — worked around in the
   acquire script by completing the fetch-back from the same run's
   artifact (no downloader logic moves into OVE). The full chain was
   re-proven end-to-end: tier-1 rejection → farm escalation →
   incomplete-part refusal → artifact completion → identity gate PASS.
   Also `docs/DEV_ENV.md` §3 was stale (libclang soname + clang resource
   path) — corrected against the actual 2026-09 trixie packages.
6. **Honest scope**: W21 changed ZERO engine code (audit + ADR + docs +
   one acquisition script hardening); the workspace stays **189/189**
   GREEN, fmt/clippy GREEN, main CI 5/5. What W21 deliberately did NOT
   do: no release pipeline/tooling (named leg, D10), no hostile-corpus
   acquisition beyond the primary (§10 planned), no STATUS.md rewrite
   (§6 rule), no CI SIGILL change (tracked), no clip_assets binding
   change (RW-NOTE-1 tracked).

## WAVE 19 deltas (2026-09-30) — performance (export decode-session budget, ADR-023)

1. **The hot spot was proven with a deterministic instrument, not wall-clock
   folklore**: the new `ove_engine::decoder_opens` counter exposed that
   `export_reencode` rebuilt the decode-source map (and its decoder
   sessions) for EVERY output frame — every exported frame re-OPENED the
   demuxer+decoder, re-seeked to the keyframe floor, and re-decoded the
   keyframe→target span for every placement. O(N·GOP) decodes where O(N)
   suffices. The fetch loop also converted EVERY intermediate decoded
   frame YUV→RGBA and kept only the last (dozens of wasted 1280×720
   conversions per fetch on real GOP structure).
2. **ADR-023 — the export decode-session budget**: ONE decoder open per
   source per export. `DecodeSource` now owns its data (cloned
   `SourceMedia`, owned dir path) so the source map carries no borrow of
   the engine; `export_reencode` builds the map ONCE (`build_decode_sources`)
   and renders through `render_frame_with` with sessions alive across the
   whole frame loop — the REALWORLD-BUG-3 sequential cursor-reuse
   discipline finally engages in the export path. `render_frame` keeps its
   one-shot signature (spot renders, MCP status: zero behavior change).
3. **REALWORLD-BUG-4 — a REAL latent defect EXPOSED by the fix** (the
   certification loop working as designed): the fetch loop DISCARDED the
   popped past-target frame, so with a surviving session the next
   sequential fetch (whose target is exactly that frame's pts under CFR)
   decoded past its floor and returned None (`SourceFrameMissing at
   1/24`). Invisible pre-W19 because per-frame session rebuilds re-seeked
   every fetch — the cursor-reuse discipline was dead code in exports.
   Fix: a ONE-FRAME `pending` pushback in `VideoSession` (bounded memory);
   a target that falls BETWEEN the last floor and the pending frame
   (VFR/multi-rate gap) falls back to the re-seek rebuild — the D-5 floor
   rule stays total. YUV→RGBA now converts ONCE, on the final floor frame.
4. **Determinism survives the optimization — the strongest pin**: the
   160-frame real-media export sha256 `baf23d2a…` is IDENTICAL between the
   pre-fix tree (worktree @ `adcafd9`) and the optimized tree, same source
   (sha256 gate `2d315daf…705f`), same machine. Every RLW-1/2/3 PASS row
   stayed green; W17 plugin gate re-ran green (2.08 s); W18 security gate
   re-ran green (2.33 s).
5. **Measured on the permanent proof (real NASA media, machine-relative)**:
   end-to-end proof 129.90 s → 22.26 s (**5.8×**); export leg ~65 s →
   10.9 s (~6×); decoder opens per 160-frame export **160 → 1**. The
   `realworld_record.json` `perf` section (additive) carries the per-leg
   wall-clock + open counts; the deterministic budget lives in
   `ove-engine/tests/export_session_budget.rs` (3 tests; the process-global
   counter is Mutex-serialized per measurement).
6. **Honest scope**: the two-track same-source shape still re-seeks per
   frame (backward jump between placements) — measured acceptable at
   10.9 s/export; per-placement session split is the named future leg.
   NOT done (future waves, software renderer stays the correctness
   reference): threading/pool wiring, GPU perf, blend-division tuning,
   allocator churn. Workspace **189/189** GREEN (+3 budget conformance);
   fmt GREEN; clippy GREEN (workspace, all targets).

## WAVE 18 deltas (2026-09-30) — security hardening (untrusted-input budgets, ADR-022)

1. **Threat posture made real, not theoretical**: every untrusted-input
   reader buffered unboundedly before this wave. The conformance probes
   PROVED the worst one — a 12-line Rhai string-doubling script was
   **SIGKILLed by the kernel OOM killer** against the pre-fix engine
   (Rhai defaults leave string/array/map sizes unbounded). The other
   surfaces (project files, plugin stdout, MCP stdio) had the same
   unbounded-read shape.
2. **ADR-022 — declared resource budgets at every client boundary**,
   enforced at READ time, failing TYPED: manifest 16 MiB + asset-path
   escape rejection (paths are data, not instructions); log line 8 MiB
   (take-window loader; quarantine records an oversized line by REASON,
   never by bytes); snapshot state 64 MiB before the wholesale read;
   plugin stdout line 1 MiB (read-time take-window + state-machine
   defense in depth) + 10 000-proposal receipt budget; MCP client line
   1 MiB → exactly one typed JSON-RPC −32000 error, tail discarded, the
   server keeps serving; Rhai string/array/map/call-depth/operations
   caps + 1 MiB `emit` output budget. All caps are named public
   constants (documented contract) with ≥1000× legitimate-scale headroom.
3. **Plugin launch law**: `PluginHost::spawn_with_args` — direct exec,
   arguments verbatim, NO shell anywhere in the host.
4. **Hostile-peer conformance instruments committed**: `ove-plugin-flood`
   (2 MiB line before any manifest / valid manifest + 11 000 default-DENY
   proposals with balanced reply draining — no host deadlock mid-attack);
   14 security tests (pure state machine + real wire + project files +
   scripts + MCP server) — the pre-fix tree failed ALL of them (including
   the OOM kill), the post-fix tree is 185/185 GREEN.
5. **W18 REAL-WORLD SECURITY GATE PASSED (certification loop,
   REALWORLD_VALIDATION §5 RLW-3 rows)**: hostile proposal-flood against
   a session holding the REAL NASA media → typed budget abort, state
   hash byte-unchanged, spot renders byte-identical → the legit plugin
   still applies its split through the SAME command bus → render
   invariance → exact 80-frame export → independent ffprobe (80 frames,
   24000/1001, 3.336667 s) → export sha256 `ff5e67f8…` IDENTICAL across
   two independent builds (determinism survives hardening) →
   deterministic record `realworld_record_security.json` (commit
   `85259f1a0f14`). The RLW-1 permanent proof AND the W17 plugin gate
   re-ran green on the hardened build (129.6 s / 35.2 s).
6. **SECURITY.md** (repo root): surface table, private disclosure path
   (GitHub security advisories), hardening posture, honest limits
   (decoder pixel-bomb + plugin log-flood CPU are NAMED residuals, not
   hidden).
7. Workspace **185/185** GREEN (14 new security conformance); fmt GREEN;
   clippy GREEN (workspace, all targets).

## WAVE 17 deltas (2026-09-30) — plugin tier (process plugins, ADR-021)

1. **ove-plugin (new workspace member)**: the directive's "typed plugin
   client at the same command-bus boundary" — 28_PLUGINS §7 Tier C lands
   first (Tier A/B stay future; the research marks WASM unverified at
   §11). Line-delimited JSON protocol v1 over the plugin process stdio:
   manifest (required first) → hello; host-pushed read-only context (state
   hash, tick axis, per-track clip windows, exact-rational strings);
   proposals; per-proposal `proposal_result` receipts; log relay; done.
   The ONLY write path is a proposal in the SAME typed grammar the other
   clients use, applied through the SAME engine surface — the plugin
   boundary adds zero new engine semantics (ADR-021).
2. **Boundary law enforced in code**: plugins never touch frames, the
   filesystem, the project uuid, or the clock (context = document state
   only, batch-`status` parity); ME-7 at the plugin edge (a JSON number in
   a rational field is rejected in-band, never coerced); default-DENY
   capabilities (host v1 knows only `propose.timeline`; unknown capability
   names abort the manifest); protocol violations abort typed (non-JSON,
   unknown type, manifest ordering/duplicates, version mismatch, EOF
   before done); payload problems reject in-band and the session continues
   (mirrors the MCP split).
3. **Receipts are deterministic evidence (28_PLUGINS §10)**: every proposal
   receipted (id, verb, applied/error, state hash after); report carries
   plugin name+version, NO timestamps — same project + same behavior →
   byte-identical receipts (pinned by test).
4. **Reference plugin `ove-plugin-stub`**: deterministic; derives one
   content-neutral structural proposal (split first clip of lowest track
   at half duration) from the context; doubles as the real-media gate
   instrument.
5. **W17 REAL-WORLD GATE PASSED (certification loop,
   REALWORLD_VALIDATION §5 W17 rows)**: permanent proof re-run on the
   SAME source (sha256 identity gate 2d315daf…705f) — every existing PASS
   row green; NEW gate `ove-plugin/tests/realworld_gate.rs` exercises the
   plugin tier on the real NASA media: manifest → context → split applied
   → receipt → state-hash advance → RENDER-INVARIANCE (spot renders
   byte-identical pre/post) → sample-exact audio across the split →
   export 80 frames with video/audio durations EXACT → independent
   ffprobe verification (80 frames, 24000/1001, clean -xerror decode,
   3.336667 s container) → visual inspection PASS (real launch-site
   footage, no corruption).
6. **Finding pinned**: undoing a plugin-applied Split restores the clip
   structure but never the pre-command hash — id-state is document state
   (ADR-016/E-012); redo identity is exact. (The RLW-1 undo identity held
   because SetKeyframes consumes no allocation.)
7. Workspace **171/171** GREEN (+6 plugin conformance); fmt GREEN; clippy
   GREEN (workspace, all targets).

## REALWORLD VALIDATION WAVE (RLW-1) deltas (2026-09-30) — includes THREE REAL engine bug fixes

1. **Permanent real-world reference media workflow** (directive mandate:
   every wave stays connected to real media): docs/REALWORLD_VALIDATION.md
   (protocol + provenance + evidence log + regression rule),
   scripts/realworld/{acquire_source.sh,baseline_facts.py,verify_output.py},
   engine/ove-engine/tests/realworld.rs (env-gated; CI stays corpus-only).
   Source: NASA "Artemis I Moon Mission: Launch to Splashdown Highlights"
   (public domain, 1280×720 H.264 CFR 24000/1001, AAC 44.1k stereo, sha256
   2d315daf…705f) acquired via ytagent 0.3.0 — direct chain blocked by the
   datacenter-IP bot-wall; the github_actions_farm tier worked (personal
   deployment of ytagent's farm workflow, WARP + android_vr + manual GVS PO
   token, farm run 36640479906). Media never enters git; provenance-gated.
2. **REALWORLD-BUG-1 (probe-vs-fetch color-tag lie)**: build_render_input
   declared RAW probe tags as the fetch contract while the boundary
   conversion stamps {src primaries/transfer, Bt709, Full} — any REAL source
   whose probe range equals the working-space range omitted the ColorConvert
   stamp pass and exec failed TagMismatch. The synthetic corpus (tag-Unknown)
   made every existing test pass vacuously. FIX: the plan declares
   boundary_rgba_stamp(video.color) — the tags fetch actually delivers.
3. **REALWORLD-BUG-2 (level crush on Full-range sources)**: YUV→RGBA
   hardwired LIMITED expansion regardless of declared range. FIX: range-aware
   integer conversion (Limited/Full exact; Unknown keeps the documented
   limited assumption). Pins: limited_black_maps_to_zero,
   full_range_luma_is_identity, stamp_matches_converted_envelope.
4. **REALWORLD-BUG-3 (D-5 floor unreachable on real NTSC media)**: fetch
   seeked EXACTLY at the mapped target; real NTSC targets fall BETWEEN frame
   pts and the adapter's Exact seek forward-drops frames ≤ target (D-4), so
   the floor frame was never delivered (SourceFrameMissing). FIX: fetch lands
   at the greatest keyframe ≤ target (the ADR-013 plan_seek discipline) and
   decodes forward; sequential same-GOP targets reuse the cursor; backward
   targets re-seek.
5. **Proof evidence (all verified)**: edit sequence (split/resize/keyframed
   overlay with exact sample-boundary cuts) → undo×4/redo×4 hash identity →
   spot renders deterministic → A/V export 160 frames + both durations exact
   → WAV 294,294 samples exact → reopen SAME state hash → re-export
   byte-identical (sha256 baf23d2a…, stable across runs) → independent
   ffprobe verification (160 frames, 24000/1001, clean -xerror decode,
   container 6.673333 s) → visual sanity PASS (40 %/53 % mid-fade blends and
   keyframed pan geometry verified by frame inspection). H.264 stream-copy
   export = typed rejection pinned (v1 mpeg4/aac surface, ADR-015).
   RW-NOTE-1 recorded: Split's right half has no clip_assets binding (inert
   under v1 single-source render; must close before render-by-binding).
6. Workspace **165/165** GREEN with the new tests; fmt GREEN; clippy GREEN
   (workspace, all targets).

## WAVE 15 deltas (2026-09-29) — includes a REAL engine bug fix

1. **ove-mcp (new workspace member)**: the AI-client leg (directive §1 —
   "AI agents are clients"). Minimal MCP 2024-11-05 stdio subset
   (initialize / tools/list / tools/call / notifications, JSON-RPC 2.0
   errors); stateless server (every tool carries `dir` — no hidden
   sessions); 12 tools mirroring the W14 batch grammar (create_project,
   add_track, import_media, add_clip, split, resize, move_clip,
   remove_clip, set_keyframes, undo, redo, get_status). Boundary
   discipline: rationals are "num/den" STRINGS — a JSON number in a
   rational field is a typed rejection (ME-7 AT THE AI EDGE, pinned by
   test); tool errors are isError content; protocol errors are -32601/
   -32700. Test drives the REAL stdio binary end-to-end (handshake,
   full edit cycle, undo→redo hash identity, float rejection, error
   codes, two-fresh-projects determinism).
2. **REAL ENGINE BUG found and fixed — cross-session redo was a second
   undo** (found BY the MCP boundary, exactly as the conformance plan
   predicts): the log fold's Undo branch pushed the INVERSE command onto
   the reconstructed redo stack; `redo()` applies what it pops, so a
   redo in a FRESH session re-applied the inverse (no-op-undo) instead
   of the original. In-session redo was always correct (runtime stack
   holds the true original) — which is why every prior undo/redo test
   passed while the CLI/MCP operator contract (one session per
   invocation) was broken. Fix: the fold pushes the deterministically
   recovered FORWARD command (`apply(inverse)` returns it). Regression
   pinned as P-10 (four-session undo→redo→undo hash-exact chain);
   workspace 158→160 GREEN.

## WAVE 10–12 deltas (2026-09-29, platform-leg evidence)

1. **§3 invariants executed across the WASM boundary** (CROSS_PLATFORM_
   CONFORMANCE_PLAN): new workspace member `ove-conformance` — ONE canonical
   scenario (batch/split/move/SetKeyframes/resize/remove + undo²→redo² +
   reopen-replay + exact rational spots), byte-stable KEY=VALUE output.
   native (L-0) vs wasm32-wasip1 under wasmtime v36.0.1 (L-3 core evidence):
   **state_hash identical (805d3e65…), REOPEN_HASH identical, keyframe eval
   identical, time spots identical** — CI job `platform-conformance` REQUIRES
   the empty diff (`scripts/ci/run_platform_conformance.sh`).
2. **REAL finding #1 — wasmtime does not honor POSIX O_APPEND on persistent
   handles**: every log write landed at offset 0 → the L-3 reopen leg failed
   with `LogCorruption (seq 11 at line 1)`; minimized repro (11 appends → 1
   line). FIX: LogWriter::open uses create+write-no-truncate + explicit
   seek-to-end (byte-identical on POSIX single-writer, correct on WASI,
   kill-9 flush-per-entry unchanged); P-suite 157/157 stays green.
3. **REAL finding #2 — blake3 C build blocks Android without the NDK**:
   target-scoped `blake3 = { features = ["pure"] }` for target_os = android
   (ove-media/ove-project/ove-render) — identical digests, no C toolchain;
   native keeps the default backend.
4. **L-2 compile evidence**: ove-time, ove-timeline, ove-media, ove-render,
   ove-project, ove-conformance all `cargo check` clean for
   aarch64-linux-android in CI (incl. the software renderer — Android-capable
   as designed). NOT claimed: DEVICE level (E-004c on-device JNI/MediaCodec/
   FGS drill — hardware-bound, per the plan's VALIDATION rule).
5. **NOT claimed**: CROSS_PLATFORM top-level status; L-1 REAL_GPU/transport;
   browser shells. Full honest status table:
   research/audit/W10-12_PLATFORM_LEGS_REPORT.md.

## WAVE 13–14 deltas (2026-09-29)

1. **W13 — conformance consolidation**: docs/CONFORMANCE_REPORT.md — the
   aggregated per-suite/per-leg status (L-0 reference suites, L-1 GPU
   parity, L-2 compile evidence, L-3 WASM core), declared divergences
   (plan §4: den bound, WASI log position, android blake3 backend), and
   the falsification clause (suites are the authority, not the report).
2. **W14 — headless batch mode** (ove-cli `batch <dir> <script>`): ONE
   engine session, MANY verbs (new/add-track/import/add-clip/split/
   resize/move/remove/undo/redo/status/export-copy/export-wav);
   `num/den` rationals only (floats rejected at the shell, ME-7 at the
   boundary); relative paths root at the PROJECT dir; fail-fast with a
   typed `ERR <lineno> <verb>` line; deterministic output contract —
   same script + same inputs → byte-identical stdout (state hashes
   included; the random-by-design project uuid excluded from batch
   status by a documented decision). Tests: ove-cli/tests/batch.rs
   (flow, byte-identical double-run, fail-fast abort). Workspace 158/158.

## Session verification record (2026-09-29, fresh continuation #5)

All W0–W8 claims re-verified from scratch before Wave 9 work:

- HEAD `160c898` on main; takeover baseline `33fde9d` IS an ancestor
  (38 commits since); tree clean; wave-8-keyframes fully merged.
- GitHub Actions check-runs on exact HEAD `160c898`: all 3 jobs SUCCESS
  (cargo-audit, fmt+clippy+tests libav-linked, libav-confinement).
- Local (rebuilt container: rustup 1.98.1, system FFmpeg 7.1.5 dev libs via
  DEV_ENV.md unprivileged recipe): 157/157 tests GREEN · fmt GREEN · clippy
  (workspace, all targets) GREEN. W8 claim confirmed exactly.
- Post-merge main (3be1a6c, PR #11): all 4 CI jobs SUCCESS — the new
  gpu-conformance job ran the parity suite for real on a runner (lavapipe).

## WAVE 9 deltas (2026-09-29, ADR-020)

1. **GPU reference executor exists and is BYTE-PARITY-Proven**
   (ove-render::gpu, feature `gpu`, wgpu 25 + pollster optional):
   `GpuRenderer::execute_frame` consumes the SAME RenderPlan and replicates
   exec.rs arithmetic exactly — Rgba8Uint textures (no float normalization),
   u32 blend pipeline (`d = 255·den`, `n_s = min(sa·a_num, d)`, `inv = d−n_s`,
   `out = (cs·n_s + cd·inv + d/2)/d`, alpha forced 255), integer translate
   with bounds clip, ColorConvert as tag stamp, coordinates from
   `@builtin(position)` (E-005 rule). Surface placement model: fetch surfaces
   are native-size (native bounds-check ≡ software's transparent-padded
   paste-at-origin); transform/blend targets are output-sized with content
   already placed — later passes sample directly (the first implementation
   double-counted the translation; the parity suite caught it at pixel 19).
2. **Typed bound + fallback**: u32 arithmetic bounds the blend to reduced
   den ≤ 65 000 (MAX_ALPHA_DEN; worst case 255·d + d/2 = 65 152.5·den < 2³²);
   exceeding it is `GpuError::UnsupportedAlphaDen` BEFORE any GPU work —
   software executes it instead (G-7 pins the fallback). `GpuRenderer::new()`
   → typed `NoAdapter` when feature-detect fails (verified in this container:
   no ICD → typed error; lavapipe ICD → adapter). GPU pass errors REUSE the
   software error set (G-6: identical TagMismatch from both executors).
3. **Parity suite G-1..G-8** (ove-render/tests/gpu_conformance.rs, tolerance
   0 vs software on every test): single layer; transforms incl. negative
   offsets + edge clipping; fractional alphas (1/3, 2/3, 1/2, 127/128,
   1/1000); three-layer stack with declared convert; retime frame selection
   (RG-4 via GPU); error parity; den-bound typed error + software fallback;
   span-level blake3 hash equality. **8/8 GREEN on Mesa lavapipe** (rootless,
   E-005 recipe) in this container.
4. **CI grows a dedicated `gpu-conformance` job** (ubuntu-latest + sudo
   apt mesa-vulkan-drivers): clippy (gpu feature) + the parity suite. The
   main pipeline is untouched (`--features ove-decode/bundled` never enables
   gpu) — zero risk to its cache/timing; the GPU leg gets its own signal.
5. **Perf honesty**: `examples/gpu_perf_probe.rs` stamps
   every line with the adapter class; committed record
   research/experiments/W9_gpu_perf_record.txt carries the llvmpipe numbers
   explicitly marked INVALID for any perf claim; real-hardware ratio gates
   (doc 45 T-7) stay a hardware-bound residual.
6. **Engine wiring deferred to platform waves** (recorded in ADR-020 §8):
   backend selection per platform rides waves 10–12 behind the same typed
   contract; wave 6's gate was parity + bench + fallback, all delivered.
7. Test IDs: gpu_conformance G-1..G-8 (8 tests; workspace count 157→165
   with the gpu feature enabled, 157 default unchanged). Files: gpu.rs
   (~700 l), gpu_conformance.rs (~530 l), 2 examples, ADR-020, CI job.

## Session verification record (2026-09-29, fresh continuation #4)

All W0–W7 claims re-verified from scratch before Wave 8 work:

- HEAD `b5f322f` on main; takeover baseline `33fde9d` IS an ancestor
  (35 commits since); tree clean; wave-7-audio fully merged (zero
  unmerged commits).
- GitHub Actions run 36496623143 on exact HEAD `b5f322f`: SUCCESS — W7
  post-merge main GREEN.
- Local (system FFmpeg 7.1.5 path, unprivileged recipe per DEV_ENV.md,
  rustc 1.98.1): 141/141 tests GREEN · fmt GREEN · clippy (workspace,
  all targets) GREEN · libav-confinement check PASS.

## WAVE 8 deltas (2026-09-29, ADR-019)

1. **Keyframes live on the Clip** (ove-timeline::property): ClipProperties
   {opacity, x, y} with LOCAL clip times, strictly-increasing keys,
   loud validation (typed PropertyError, never silently sorted). Move
   carries animation untouched; Resize strands keys inert (evaluation
   domain [0, dur) — growing back re-activates; duration-only inverse
   stays exact); **Split slices with the computed boundary value
   inserted on BOTH halves** — the split-preserve rule (S8, 300 seeded
   cases, exact evaluation continuity across the seam, Hold/Linear
   semantics survive the cut).
2. **Command::SetKeyframes** — wholesale replace of one property's keys;
   the exact inverse is the previous key list under the same command
   shape; undo/redo/replay need zero new machinery. Opacity values
   validated ∈ [0,1] at command time. Split's inverse batch embeds
   SetKeyframes restores (S9: hash-exact undo roundtrip).
3. **Exact evaluation**: per-key out-interp Linear (exact rational lerp,
   i128-checked) | Hold; boundary rules exact (on-key identity,
   hold-before/after). S7 pins evaluation against a naive oracle.
4. **Engine evaluation into the render path**: build_render_input(out, t)
   evaluates per frame; geometry via Rational::round_half_up (ove-time
   P13 — which PROVES the conversion is total over representable
   rationals); i32 overflow is a typed KeyframeValueOutOfRange, never
   saturation. W8 integration test: plan carries evaluated alpha/dx
   verbatim, undo/redo hash-exact, animated state survives reopen,
   renders byte-identical.
5. **Persistence**: StateMirror.properties (serde-default — old
   snapshots load unchanged, keys as exact pairs); log grammar +1
   payload (set_keyframes), embedded inverse needs no extra grammar.
   P-9: save/reopen hash equality through SetKeyframes AND a split
   inverse batch.
6. Test IDs: ove-time P13 (round_half_up exactness + totality proof);
   ove-timeline keyframes.rs (unit + S7 + S8 + S9, 11 tests);
   ove-project P-9; ove-engine w8_keyframes_exact_eval_and_stability.
   Workspace **157/157** (was 141).

## Session verification record (2026-09-28, independent takeover continuation)

Prior session claims were re-verified from scratch before any new work:

- HEAD `8b5aa81` on main; takeover baseline `33fde9d` IS an ancestor; tree clean.
- GitHub Actions: post-merge main run 36304812153 GREEN (and PR #1 runs
  36303605420 / 36304226310 GREEN) — W0.5 SIGILL fix confirmed externally.
- Local (system FFmpeg 7.1.5 path, unprivileged recipe per DEV_ENV.md):
  60/60 tests GREEN · fmt GREEN · clippy -D warnings GREEN (pre-W0.6 tree).
- Sources re-read: ove-time (297 l), ove-timeline (2303 l), ove-media (1332 l),
  ove-decode (145 l + conformance/probe suites). Spec set re-read: PROJECT_
  FORMAT / ENCODER / DECODER / FRAME_CONTRACT / MEDIA_ENGINE.

## Session verification record (2026-09-29, fresh continuation #2)

All W0.6/W1/W2 claims re-verified from scratch before Wave 3 work:

- HEAD `16d62a9` on main; baseline `33fde9d` IS an ancestor; tree clean.
- GitHub Actions run 36395951181 on exact HEAD `16d62a9`: SUCCESS (3 jobs:
  rust / audit / libav-confinement). All six recent runs SUCCESS.
- Local (system FFmpeg 7.1.5, unprivileged recipe per DEV_ENV.md, rustc
  1.98.1): 84/84 tests GREEN · fmt GREEN · clippy -D warnings GREEN.
- Source inventory re-counted: 4,223 l (ove-time 378, ove-timeline 1,611,
  ove-media 1,332, ove-decode 145+suites, ove-render 643).

## Session verification record (2026-09-29, fresh continuation #3)

All W0–W6 claims re-verified from scratch before Wave 7 work:

- HEAD `451e67e` on main; takeover baseline `33fde9d` IS an ancestor
  (30 commits since); tree clean; fix/ci-bundled-ffmpeg-sigill branch fully
  merged (zero unmerged commits).
- GitHub Actions run 36472738518 on exact HEAD `451e67e`: SUCCESS (push,
  main, all 3 jobs). Runs 24–31 all SUCCESS; last PR run (wave-6 branch)
  SUCCESS.
- Local (system FFmpeg 7.1.5 path, unprivileged recipe per DEV_ENV.md,
  rustc 1.98.1): 132/132 tests GREEN · fmt GREEN · clippy (workspace,
  all targets) GREEN · libav-confinement check PASS.
- W6 milestone test confirmed present and passing
  (ove-engine/tests/integration.rs::w6_vertical_slice_milestone).
- Bundled-feature path + cargo-audit verified via CI on the exact SHA
  (run 36472738518) rather than a local bundled rebuild; confidence 0.85
  for that split, 1.0 for everything above.
- Doc drift found & fixed in this commit: open-gaps list still claimed
  VERTICAL_SLICE_TRACE.md "not yet written" after W6 landed it (stale
  gap #6 removed).

## WAVE 7 deltas (2026-09-29, ADR-018)

1. **Audio decode leg** (ove-decode): audio streams decode through the
   same session machine — canonical planar-f32 ("fltp") surface at the
   SOURCE rate/layout (swresample format conversion ONLY; a resample is
   structurally impossible in the adapter), per-frame duration =
   nb_samples/rate exact, audio floor+trim seek policy (the straddling
   frame is delivered whole; the caller trims the exact sample in-point),
   audio frames unpooled with `reclaim()` typed-rejected. A-1..A-4 pin
   the corpus facts (priming trimmed, tail padded to the AAC grid,
   contiguity, exact in-points).
2. **AAC encoder leg** (ove-encode, native libavcodec aac): `AudioEncoder`
   trait + `FfmpegAacEncoder` — caller samples buffered to the 1024 grid,
   feed contiguity ENFORCED (typed mismatch on a wrong cursor),
   small-last-frame branch detected from `AV_CODEC_CAP_SMALL_LAST_FRAME`
   and reported, CBR-only (CRF typed `Unsupported`), libav identity
   recorded. `TrackSpec::initial_padding` added so the muxer records the
   priming delay and the container writes the trim edit list — the FILE
   duration stays sample-exact (verified against a CLI-produced reference:
   first packet pts −1024, elst 1024, duration 44100/44100).
3. **WAV/PCM out**: pure-Rust PCM16 writer with the pinned
   `round(clamp(x,−1,1)×32768)` conversion (no libav; deterministic
   bytes; mono/stereo v1).
4. **Engine audio** (ove-engine): `assemble_timeline_audio` — the FIRST
   track's clips assembled as sample-exact per-clip ranges (every cut on
   a sample boundary or typed `NonExactSampleCut`; speed ≠ 1 typed
   `AudioRetimeUnsupported`; span authority enforced — short sources are
   a typed error, never a silent pad). `export_reencode` pairs video
   re-encode with audio re-encode automatically (ENCODER_SPEC §3.3);
   `export_wav` delivers PCM out.
5. **Planner Deferred → executable with ZERO planner changes**: the W3
   pure logic already covered `reencode_available: true`. Test IDs:
   decode A-1..A-4; encode E-8/E-8b/E-9/E-10 (A/V mux with two streams,
   ffprobe + libav verified, drift ± 0 samples); engine
   w7_audio_assembly_and_av_export (assembly 192000 samples ±0, A/V
   export 4.000000 s both streams, WAV exact, document hash unchanged by
   export work). Workspace 141/141.

## WAVE 6 deltas (2026-09-29, PR #8)

1. **The directive W6 milestone is REAL and tested**
   (ove-engine/tests/integration.rs::w6_vertical_slice_milestone): real
   video ×2 (24 fps + 29.97 multi-rate) → probe → content-hash registry →
   timeline commands (explicit ids, per-clip asset bindings riding the
   log) → exact ADR-013 source mapping → decode → FrameEnvelope →
   RenderPlan → software render → encode/mux → valid MP4 → ffprobe + libav
   verified (120 frames, duration 5/1 exact) → save → kill → reopen →
   SAME state hash → re-export byte-identical (deterministic SW pipeline,
   file_sha256 equality).
2. **Per-clip asset bindings ride the log** (LogPayload::Insert asset
   field → record_entry_bindings on execute AND replay → StateMirror.
   clip_assets in the document hash). Multi-source render resolves each
   placement's source by binding; DecodeSource per source.
3. **D-5 floor rule implemented in the decode source**: fetch returns the
   greatest pts ≤ target (multi-rate sources floor honestly; exact hits
   are the CFR case).
4. **docs/VERTICAL_SLICE_TRACE.md written** — the three chains (import,
   command+replay, AI/client→command→render→export) at file→crate→function
   level, each hop exactness-annotated, each claim pinned by a named test,
   with the non-claims (audio/GPU/agent transport) named as such.

## WAVE 5 deltas (2026-09-29, PR #7)

1. **ove-engine created** (workspace member 8): the headless session wiring
   all five legs — probe (FfmpegProbe probe_full) → hash-addressed import
   with probe sidecars → ove-timeline commands (explicit ids, alloc before
   construct) → project persistence → decode (exact seeks, D-4/D-5) →
   render (ADR-013 mapping → compile → software exec) → export.
2. **ove-cli created** (workspace member 9): subcommands new/add-track/
   import/add-clip/split/undo/redo/status/export-copy driving the engine;
   smoke test drives the real binary end-to-end (CARGO_BIN_EXE).
3. **Integration suite 6 tests (workspace 131/131)**: save/reopen hash
   equality, kill-reopen-continue at multiple points, render determinism
   from real decode, re-encode export with exact frame count + duration
   (ffprobe + libav verified), keyframe-aligned copy export via the
   planner, undo/redo surviving reopen.
4. **Two real bugs found & fixed (both now mutation-pinned)**:
   a. **ove-media pool bug**: `into_frame_bytes()` cleared the buffer
      before moving it — EVERY decoded frame shipped an empty payload.
      Wave-2's decode suite passed vacuously (nothing compared pixel
      bytes); the engine's real renders exposed it. Fixed + payload-
      content gate added to the decode conformance suite (d13).
   b. **ContentHash identity confusion**: sidecar hydration re-parsed
      digests with from_bytes (which HASHES) — from_hex (round-trip)
      added and named so the two cannot be confused again.
   c. **Allocation state rides the log**: replay could not reproduce the
      id-alloc cursor (apply() never advances it) — every log entry now
      carries `nid` (cursor after execution), replay restores it via
      Timeline::set_next_id (monotonic, guarded). Project-layer tests had
      used literal ids and passed vacuously; the engine suite is the
      first mutation-strong caller.
5. **ADR-017** records: engine = integration layer (core stays libav-free),
   the boundary-conversion contract (YUV→RGBA once at decode, RGBA→YUV
   once at encode, integer/deterministic, matrix from tags), v1
   single-source render note (per-clip asset binding rides the log at W6).

## WAVE 4 deltas (2026-09-29, PR #6)

1. **ove-project created** (workspace member 7): PROJECT_FORMAT_SPEC v1 —
   manifest.json (identity + assets + track registry), commands.jsonl
   (append-only, write-through with flush-per-entry: the on-disk state
   after each execute IS the kill-9 state), snapshot/ (state-<seq>.json +
   meta.json authority, atomic temp+rename everywhere). Folds through
   ove-timeline commands with explicit ids (E-012 discipline carried:
   next_id + used_ids serialized as document state — new read-only
   accessors + from_parts on Timeline).
2. **Acceptance P-1..P-8 green (11 tests; workspace 123/123)** including:
   P-2a REAL subprocess kill-9 (re-exec child mode + abort, replay ==
   live hash), P-3 compaction equivalence + continuation, P-4 corruption
   drill (typed hard-stop error → explicit quarantine repair → last-good
   hash exact), P-5 float rejection + exact (num,den) round-trip, P-6
   hash-addressed assets (folder move survives; corrupt/missing typed,
   state intact), P-7 undo-marker replay (undo history rebuilt from the
   log per ADR-008), P-8 disposable dirs.
3. **ADR-016** records: authority order (log = record, snapshot meta =
   snapshot authority, manifest = reconciled hint), log grammar (embedded
   inverses on markers → self-contained suffixes; anchored seq contiguity;
   timestamps omitted in v1), track registry in the manifest (tracks are
   setup, not commands), canonical BLAKE3 state hash over the document
   mirror, the repair path, the kill-9/fsync honesty boundary.
4. **Design honesty note**: reopening restores undo history ONLY from the
   replayed suffix (ADR-008 forbids persisting the stack) — P-3/P-7 assert
   exactly this model; pre-snapshot undo steps are not undoable after a
   reopen, which is the recorded v1 semantics.

## WAVE 3 deltas (2026-09-29, PR #5)

1. **ove-encode created** (workspace member 6): ENCODER_SPEC §1 traits —
   `Encoder` (configure/feed/drain + the recorded `track_spec()` extension),
   `Muxer` (open/write/finalize), `EncodedPacket` with checkpoint fields;
   pure export planner (`planner.rs`): keyframe-aligned StreamCopy vs
   ReEncode vs Mixed segmentation, snaps always reported (receipt), audio
   grid alignment; FFmpeg adapter (`ffmpeg/`): `FfmpegSwEncoder` (native
   mpeg4, swscale RGB→YUV420P declared-conversion path), `FfmpegMuxer`
   (MP4, faststart + bitexact options, exact tick conversion against the
   EFFECTIVE post-header axis), `FfmpegCopySource` (packet-level demux,
   keyframe-floor landing, 0-start shift, byte-identical passthrough).
2. **Tests 28 new (workspace 112/112 green)**: planner P01–P10 (pure logic:
   snap table, RoundIn validity, ToEof, 23.976 exactness, audio grid,
   mixed coverage, shapes); conformance E-1..E-5 + E-7/E-7b/E-7c + H1–H5 —
   outputs verified through ffprobe CLI (when present), libav probe, and
   decoded-pixel equality; copy-route byte-identity to source GOP range
   (E-007 §3.4) pinned at both packet and pixel level; committed corpus
   (copy24/copyav/copyntsc + golden tables) via
   `scripts/corpus/gen_encode_corpus.py`; byte golden
   `encode_golden.json` keyed on the producing libav identity (7.1.5-0+
   deb13u1), same-run determinism asserted everywhere.
3. **ADR-015** records the v1 decisions: confinement restated as CLOSED
   adapter allowlist {ove-decode, ove-encode} (check script amended and
   green), mpeg4-first codec surface (OpenH264/SVT-AV1 typed Unsupported),
   snap semantics made normative, movenc timescale-widening (24→12288)
   handled by effective-axis conversion, 0-duration packets → None, mp4
   AVIO caller-ownership (the AVFMT_NOFILE inversion SEGFAULT class, found
   and pinned by tests), version-keyed byte goldens with explicit reported
   skips. ADR-005 promoted PROPOSED → ACCEPTED (v1 surface landed).
4. **Bugs found & fixed in-wave** (each now test-pinned): mixed-axis tick
   conversion after movenc widening; stale pict_type=I on the reused
   encoder frame forcing all-intra; codec 0-duration corrupting stts;
   non-refcounted packet aliasing; test fixture pts bug (24k s frames) —
   caught because e2b passed while e2/e3 failed.

## WAVE 2 deltas (2026-09-28)

1. **ove-render created** (workspace member 5): software reference renderer
   per RENDER_GRAPH_SPEC — plan.rs (RenderPlan/Pass/PassKind + canonical
   hash), compile.rs (PURE compiler: compile_frame/compile_span; layer order
   = track order; exact rationals; conservative culling), exec.rs (software
   executor: integer straight-alpha over opaque output, FrameSource D-5
   floor resolution, single color conversion).
2. **RG-1..RG-7 all green** (render_golden.rs): compile purity (runs +
   threads, 4 seeds × 3 tracks × 5 placements), golden frames (3 fixtures,
   blake3-committed, hand-verified pixel semantics asserted alongside),
   layer-order swap, retime 0.5×/2× exact frame prediction (through the
   ADR-013 seam), split continuity END-TO-END through the real Timeline
   (apply Split → walk → compile → render byte-equality across the seam),
   optimizer safety (cullable placements: no passes + byte-identical
   output), color-tag single conversion + TagMismatch honesty error.
3. **ADR-014** records the v1 decisions (opaque integer compositing,
   output-sized surfaces, declared-tag conversion, integer-translate
   geometry, FrameSource decoupling, per-frame compile shape) + reopen
   conditions. Key bug the goldens caught during development: opaque
   transform buffers → fixed to transparent-outside-region.
4. ENGINE_BUILD_PLAN waves 0–3 are now REAL crates; the directive's W2
   "software-first renderer" gate is satisfied; decode binding to the same
   FrameSource trait lands with the W6 vertical slice.

## WAVE 1 deltas (this session)

1. **Forensic finding**: 4 uncompiled files (clip.rs/command.rs/engine.rs/
   state.rs, 1,264 l) orphaned by the v0.7 reconciliation — lib.rs declared
   only avl/gap/oracle, so a "editor-grade state machine" LOOKED implemented
   while compiling zero times with zero running tests (the exact
   documentation≠implementation class the directive forbids). REMOVED with
   full provenance (recoverable from commit ee31fb9); ideas that survive live
   in the compiled lineage (explicit-id log discipline, state hash, exact
   inverses). Recorded in ADR-013.
2. **Seam implemented** (`ove-timeline::mapping`, ADR-013): ClipWindow +
   SourceClock (zero ove-media dependency — layering held); exact
   timeline↔source mapping (roundtrip identity), `frame_span` floor/ceil
   bounds, `validate_ingest` range gate, `plan_seek` keyframe-floor landing
   (D-5/E-007 encoded positively); typed SeamError for all caller-range bugs.
3. **ove-time additions**: `recip()` + `ceil_div_rate()` (exact; unit tests).
4. **Property suite S0–S6** (mapping_properties.rs): roundtrip exactness over
   retimed windows, clip-primitive agreement, ingest gate boundaries, snap
   keyframe membership + monotonicity (20k cases), frame-span exactness at
   23.976 boundaries, exact-plan target identity, clock boundary validation.
5. MEDIA_ENGINE_SPEC §3 annotated "Implemented (2026-09-28, ADR-013)".

## WAVE 0.6 deltas (2026-09-28, PR #2)

1. ove-time overflow policy DECIDED + pinned (ADR-007 amendment 2026-09-28):
   **overflow = contract violation = PANIC (fail-fast); never wraps, never
   saturates.** Rationale: saturation is silent approximation (fatal to E-002
   exactness); fixed-tick-axis discipline makes overflow a caller bug; p3b
   already asserted the panic. Doc drift (crate header said "saturates") fixed.
2. ove-time i64::MIN holes FIXED (real bugs found by the takeover re-audit):
   - `neg()`: `-i64::MIN` wrapped silently (release) — now checked_neg + panic
     with explicit contract message.
   - `gcd64/gcd128`: `.abs()` on i64::MIN/i128::MIN overflowed — gcd now
     computed on unsigned magnitudes.
   - `Rational::new(i64::MIN, n)` normalizes correctly end-to-end now.
3. Property suite extended P1–P10 → P1–P12: P11 extreme-safety (MIN/MAX
   construct/cmp/half/floor_div_rate/split/add/serde), P12 overflow-panics
   (neg/sub/add/mul/floor_div_rate) + must-NOT-panic exact-boundary guard.
4. Hash terminology single-policy reconciliation (audit gap #1b):
   PROJECT_FORMAT_SPEC §2 now carries a normative HASH POLICY (BLAKE3-256,
   64 lowercase hex, = ove-media ContentHash; sha256 confined to golden-
   artifact TEST tooling), §2 manifest example `sha256`→`content_hash`,
   §5 `assets/<sha256>/`→`assets/<content-hash>/`; ENCODER_SPEC E-5 +
   muxer finalize() annotated "artifact integrity, not identity".

## CI CONFIRMATION (2026-09-27, prior wave)

- PR #1 run 36303605420: ALL 3 JOBS GREEN (fresh portable build; wrapper
  marker present; guard scanned 1445 objects PASS; conformance 14/14).
- Post-merge main run 36304226310: ALL 3 JOBS GREEN — the original failure
  scenario (cache restored from another runner) now passes with portable
  artifacts; conformance 14/14; portability guard PASS.

## Current implementation inventory

| Crate | State | Evidence |
|---|---|---|
| ove-time | TESTED — 9 unit + 21 property (P1–P13 incl. round_half_up totality) | 157/157 workspace local (2026-09-29, W8) + CI |
| ove-timeline | TESTED — 10 properties (AVL primary, ADR-011) + 7 seam properties (S0–S6, ADR-013) + 11 keyframe suites (unit + S7/S8/S9, ADR-019) | CI + local; bench in ADR-011 |
| ove-media | TESTED — 13 unit (asset hash, FrameEnvelope, pool, probe types) | CI + local |
| ove-decode | TESTED — 14 conformance (D-1..D-12) + 6 probe + 4 audio (A-1..A-4); libav confined | CI + local, both system and bundled libav |
| ove-render | TESTED — RG-1..RG-7 (purity, goldens, layer order, retime, split continuity, optimizer safety, color) | CI + local; ADR-014 |
| ove-encode | TESTED — P01–P10 planner + E-1..E-5/E-7/H1–H5 video + E-8..E-10 audio (ffprobe/libav/pixel-verified, goldens committed) | 157/157 workspace local (2026-09-29); ADR-015/ADR-018 |
| ove-project | TESTED — P-1..P-9 acceptance (12 tests incl. subprocess kill-9, compaction equivalence, corruption drill, keyframe persistence) | 157/157 workspace local (2026-09-29); ADR-016/ADR-019 |
| ove-engine | TESTED — integration suite 7 + W7 audio vertical + W8 keyframe vertical (exact plan eval, hash/reopen stability) | 157/157 workspace local (2026-09-29); ADR-017/ADR-018/ADR-019 |
| ove-cli | TESTED — binary smoke (full flow via CARGO_BIN_EXE) | 157/157 workspace local (2026-09-29) |

## Current research / architecture status

- ADR-001/002/005/007/008/009/010/011/012/013/014/015/016/017/018/019 ACCEPTED
  (005 promoted 2026-09-29, v1 surface landed; 018 audio leg v1; 019 keyframes
  v1); ADR-003/004/006
  PROPOSED with named evidence legs. ADR-007: overflow policy; ADR-013: seam
  + dead-leg removal; ADR-014: software renderer v1.
- 45 research docs + 137-source ledger + gates (13 UNDERSTOOD / 1 PARTIAL env-bound).
- Architecture residuals (named, not hidden): E-005 real-GPU perf, E-004c device
  runtime, E-006b real-Tauri transport — all hardware/environment-bound.

## Current validation status

- 157/157 workspace tests GREEN locally (system FFmpeg 7.1.5 path, rustc
  1.98.1), fmt GREEN, clippy GREEN (workspace, all targets), after W8
  keyframes (2026-09-29). libav-confinement check PASS.
- Bundled path: exercised by CI on this wave's PR (bundled libav now also
  links swscale + swresample via the union feature set; portability guard
  scans the same object tree; E-5 byte gate self-skips with an explicit
  report on a different libav identity, structure gates always run).
- CI: PR run 36501739232 on `a42a975` SUCCESS (all 3 jobs, bundled path
  included); post-merge main run 36502138331 on `00a46b9` SUCCESS —
  WAVE 8 landed.

## Current CI state

- Workflow: fmt + clippy + tests (bundled) + portability guard + cargo-audit
  + libav-confinement. Cache prefix `v2-portable-ffmpeg` (poisoned caches
  unreachable).
- Latest confirmation: PR #18 checks 5/5 SUCCESS AND post-merge main run
  SUCCESS on HEAD `a8bc078` (2026-09-30). All recent runs SUCCESS.

## Current blockers

- None environmental.

## Current open gaps (top)

1. Audio named gaps (ADR-018): multi-track mixing/overlaps, audio
   retiming, multi-lane assembly, AAC byte goldens (E-5 keying), E-6
   audio checkpoint execution.
2. Keyframe named gaps (ADR-019): curves beyond Linear/Hold (bezier/ease),
   audio keyframing (volume automation) not wired into the assembly,
   animatable properties beyond the renderer's placement inputs, key-
   data duplication at every cut inside an animation (accepted v1 cost).
3. OpenH264/SVT-AV1 encoder legs; E-6 kill-resume execution at segment
   granularity (checkpoint fields exist).
4. GPU residuals (ADR-020): YUV GPU conversion + fractional scaling with
   declared tolerance + zero-copy frame import (hardware/driver legs);
   real-hardware perf baselines (llvmpipe record is INVALID-class);
   engine/platform backend selection rides waves 10–12.
5. Mutation-style "test the tests" not yet systematic.
6. Hardware-bound experiment residuals (E-004c/E-005/E-006b).
7. GitHub PAT used across chat sessions must be rotated by the owner
   (standing security rule, DEV_ENV.md) — outside engine scope, flagged.

## Current wave order (directive §37, unchanged)

0 audit ✓ → 0.5 SIGILL ✓ → 0.6 reconcile ✓ → 1 seam ✓ → 2 render ✓ →
3 encode+mux+export ✓ → 4 project ✓ → 5 engine+cli ✓ → 6 vertical slice ✓ →
7 audio ✓ → 8 keyframes ✓ → 9 GPU ✓ → 10–12 platform-leg evidence ✓ →
13 conformance ✓ → 14 headless ✓ → 15 AI/MCP ✓ → 16 scripting ✓ →
17 plugins ✓ → 18 security ✓ → 19 perf ✓ → 20 docs/release → 21 production audit.

## Resolved decisions (registry)

- ADR-023 (2026-09-30): export decode-session lifetime — the decode-source
  map is built ONCE per export (budget: ONE decoder open per source per
  export, pinned deterministically by the `decoder_opens` instrument +
  `export_session_budget.rs`, never wall-clock); sequential fetches reuse
  the decoder cursor (the REALWORLD-BUG-3 discipline now live in the
  export path); fetch keeps a ONE-FRAME pending pushback (bounded memory)
  serving the next sequential fetch; a target between the last floor and
  the pending frame (VFR/multi-rate gap) falls back to the re-seek rebuild
  (D-5 stays total); YUV→RGBA converts once on the final floor frame.
  Export bytes UNCHANGED — sha256 `baf23d2a…` identical pre/post on the
  real NASA source (determinism survives the optimization). Confidence
  0.9 (5.8× end-to-end / ~6× export leg on the reference machine; the
  two-track same-source shape still re-seeks per frame — measured
  acceptable; per-placement sessions are the named future leg). Reopen:
  a legit workload where the re-seek fallback dominates (the VFR corpus
  leg); per-placement session split; threading/pool decisions ride their
  own waves.
- ADR-022 (2026-09-30): security hardening v1 — declared untrusted-input
  resource budgets at every client boundary (project files, plugin
  stdio, MCP stdio, Rhai scripts), enforced at READ time via take-window
  bounded reads, failing TYPED with the budget named; project asset
  paths validated as data (relative, no traversal); plugin launch =
  direct exec, no shell; hostile-peer conformance instruments committed
  (ove-plugin-flood); caps are named public constants with ≥1000×
  honest-use headroom. Confidence 0.88 (the Rhai OOM-kill repro is
  converted to a typed error; W18 real-media gate + determinism
  re-export verified). Reopen: an untrusted path outside the five
  surfaces; a legit workload exceeding a cap; a Tier-B sandboxing
  requirement (session wall-clock, log-flood CPU); decoder pixel-bomb
  caps ride the difficult-corpus decode-hardening leg; Windows path
  semantics ride the first Windows platform leg.
- ADR-020 (2026-09-29): GPU reference executor v1 — ove-render::gpu behind
  feature `gpu` (wgpu 25), Rgba8Uint + u32 integer blend identical to
  software, `@builtin(position)` coordinates, native-size fetch surfaces /
  output-sized placed targets (no accumulated offset), typed den bound
  MAX_ALPHA_DEN = 65 000 with software fallback, typed NoAdapter
  feature-detect, error-set reuse, CI gpu-conformance job on lavapipe,
  perf probe with INVALID-class software-adapter stamping. Confidence 0.9
  (parity proven on software Vulkan; hardware legs are named residuals).
  Reopen: an adapter diverging from software bytes; den > 65 000 need
  (u64 emulation conversation); present-to-surface platform requirement.
- Cross-session undo/redo stack reconstruction (2026-09-29, wave 15): the
  log fold rebuilds undo/redo stacks; Undo marks push the recovered
  FORWARD command onto the redo stack (never the inverse). Confidence
  0.95 (P-10 four-session chain + MCP stdio test + full suite green).
  Reopen: a command family whose inverse does not return the exact
  forward (would break apply(inverse)→fwd recovery — none exists).
- WASI log-position policy (2026-09-29, waves 10–12): LogWriter appends via
  explicit seek-to-end, never the O_APPEND flag (wasmtime writes at offset
  0 for persistent append handles — verified, minimized repro). Confidence
  0.95 (native P-suite green + native≡WASI hash parity). Reopen: a runtime
  with different write-position semantics (the explicit-seek contract is
  the narrowest portable form for the single-writer model).
- Android blake3 backend policy (2026-09-29): target-scoped `pure` feature
  for target_os = android (identical digests; no NDK needed for the pure-
  Rust core). Confidence 0.9. Reopen: an Android build with the NDK that
  needs the C path for perf — features are additive, revert is one line.
- ADR-012 (2026-09-27): CI bundled-FFmpeg portability — wrapper + cache prefix
  + guard. Confidence 0.92. Reopen condition: ffmpeg-sys-next flag change (guard
  fails loudly) or CI SIGILL recurrence.
- ADR-018 (2026-09-29): audio leg v1 — canonical planar-f32 decoded surface
  (format conversion only, never a resample), audio floor+trim seek policy,
  sample-count authority with TrackSpec::initial_padding → container edit-list
  trim (reference-pipeline-verified), small-last-frame branch declared, WAV
  PCM16 out pure Rust, engine single-lane assembly with typed exactness
  errors. Confidence 0.88. Reopen: an encoder without SMALL_LAST_FRAME, a
  non-1/rate audio time base, or a mixing requirement (render-graph audio
  legs).
- ADR-007 amendment (2026-09-28): ove-time overflow policy = intentional panic
  (fail-fast); i64::MIN safe except negation (panics); checked variants deferred
  to first non-panicking consumer. Confidence 0.93. Reopen condition: an FFI
  consumer that cannot tolerate panics forces the checked-variant conversation.
- Hash policy (2026-09-28): BLAKE3-256 is the ONLY identity hash (assets +
  state); sha256 exists only in golden-artifact test tooling / render-record
  integrity fingerprints. Confidence 0.97 (code truth was already BLAKE3).
- ADR-013 (2026-09-28): timeline↔media seam = ove-timeline::mapping
  (ClipWindow/SourceClock, query-time mapping = Q-06 v1 resolution, typed
  seam errors, keyframe-floor seek planning) + dead state-machine leg removed
  (provenance ee31fb9). Confidence 0.88. Reopen: retime-verb semantics
  contradict S1; a backend cannot honor D-5 landing; ove-project needs
  persisted resolved-command journals.
- ADR-014 (2026-09-28): software reference renderer v1 — opaque integer
  straight-alpha compositing, output-sized surfaces, declared-tag single
  conversion, integer-translate geometry, FrameSource decoupling from
  ove-decode. Confidence 0.9. Reopen: intentional golden diff without
  byte-level note; scaling/affine need; stacked partial-coverage banding.
- ADR-015 (2026-09-29): encode leg v1 — adapter allowlist {ove-decode,
  ove-encode}; mpeg4-first codec surface; snap semantics normative
  (RoundIn start validity, RoundOut/ToEof end, Mixed inner core);
  effective-axis exact tick conversion; version-keyed byte goldens with
  explicit skips; refcounted packets; mp4 AVIO caller-ownership.
  Confidence 0.88. Reopen: a libav build whose movenc does NOT widen
  timescales still passes (conversion is effective-axis based); an
  encoder that needs B-frames forces the dts/ctts conversation; openh264
  licensing form (external lib vs bundled) at its adapter leg.
- ADR-016 (2026-09-29): project persistence v1 — authority order (log >
  snapshot meta > manifest hint), write-through kill-9 model, log grammar
  with embedded inverses + anchored contiguity, track registry in the
  manifest, BLAKE3 document hash, explicit repair path, id-state as
  document state (E-012). Confidence 0.9. Reopen: a second schema version
  (migration serializers + fixtures); a requirement for fsync-grade
  durability claims; track-structure commands at the W8 era.
- ADR-019 (2026-09-29): keyframe animation v1 — keys on the clip (local
  times; Move carries, Resize strands inert, Split slices with computed
  boundary keys both sides = exact split-preserve), per-key out-interp
  Linear/Hold with exact rational evaluation and loud validation,
  SetKeyframes wholesale-replace command whose inverse is the previous
  key list, animation as document state (hash + mirror + log grammar),
  engine-side evaluation with P13 round-half-up i32 conversion
  (proven total over representable rationals). Confidence 0.9. Reopen:
  curve families beyond Linear/Hold; animatable properties outside the
  placement inputs; audio keyframing; speed≠1 keyframe time mapping
  (rides the retime verb, ADR-013).
- ADR-021 (2026-09-30): plugin tier v1 — process plugins as typed
  command-bus clients (28_PLUGINS §7 Tier C; Tier A/B future). Protocol
  v1 line-JSON over stdio (manifest→hello / host-pushed read-only
  context / proposals / receipts / log / done); the ONLY write path is a
  proposal in the SAME typed grammar (no import_media — filesystem
  grant), applied through the SAME engine surface; default-DENY
  capabilities; ME-7 at the plugin edge; protocol violations abort
  typed, payload problems reject in-band; deterministic no-timestamp
  receipts. Confidence 0.9. Reopen: user-approval ladder step; Tier B
  WASM prototype; capability grants + permission broker;
  distribution/signing; protocol N-1 policy.

## Unresolved decisions (registry)

- NLE derived-verb surface (ripple/roll/slip/slide/insert/overwrite/extract/lift):
  record primitive-vs-derived table when timeline work resumes (directive §6.2) —
  not before.
- RW-NOTE-1 (TRACKED ARCHITECTURAL GAP, from RLW-1): Split's right half
  receives no `clip_assets` binding — `record_entry_bindings` does not
  consume the new clip id (5 of 6 clips bound). Inert under the v1
  single-source render mapping; MUST be closed before multi-source
  render-by-binding lands (until then it would bind the wrong source).
  Stays tracked until the multi-source/binding model is actually
  implemented and the realworld scenario asserts correct bindings.

## Explicit next action

WAVE 21 is COMPLETE (audit + ADR-024 + RLW-6 certification; this file and
REALWORLD_VALIDATION §5 record it). The wave plan 0–21 is COMPLETE — there
is no pre-committed W22. Per the POST-W21 POSTURE (top of this file), the
repository continues under the certification loop only; the named residual
legs are the roadmap. The next CONCRETE action, when engineering resumes:
close RW-NOTE-1 (clip_assets binding for Split's right half — committed
reopen condition, assertion already in the realworld scenario) or run the
hostile REALWORLD/ corpus leg (REALWORLD_VALIDATION §10) — each via its own
wave (RLW-7+) with the full certification gate, PR, and this file updated.
Standing gates for ANY future wave are unchanged: fmt/clippy/tests green,
real-media certification re-run byte-identical, independent verification,
wave + commit + hash recorded.
