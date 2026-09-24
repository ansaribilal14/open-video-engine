# W3f REPORT — wave-3 research agent (docs 34_UX / 35_LICENSES / 36_YOUTUBE)

Date: 2026-09-24 · Agent: W3-f (research + docs only; no engine code touched)

## Docs written

| Doc | Lines | Status header |
|---|---|---|
| docs/research/34_UX.md | 179 | PARTIAL (v0.1 — wave-3 depth pass 2026-09-24) |
| docs/research/35_LICENSES.md | 210 | PARTIAL (v0.1 — wave-3 depth pass 2026-09-24) |
| docs/research/36_YOUTUBE.md | 196 | PARTIAL (v0.1 — wave-3 depth pass 2026-09-24) |

Ledger: research/sources/LEDGER_W3f.md — NEW rows S-3f0..S-3f9 in the canonical
14-column pipe format (SOURCE_LEDGER.md untouched). No other repo files modified.

## Key findings (per doc)

**34_UX** — interaction grammar derived from already-surveyed editors + new primary
verification: Kdenlive 26.08 manual confirms J/K/L with speed-cycling, tool keys
(S selection / X razor / M spacer), snap-point navigation keys, and Shift+Return
preview render [S-3f0]. Core contract: drag = command draft (validate → preview →
commit → inverse-command undo), grounded in Kdenlive request*Action (nothing-
modified-on-failure), Cutlass dry-run default, Clypra draft store, ADR-010 batching.
UI reactivity = subscriber of the command log: two rehydration paths (event apply vs
snapshot+suffix replay) are both proven hash-equal by E-003 P3/P1b. Hit-testing and
snapping must sit on the ADR-011 derived index. A/V sync: EBU R37 verified (5/15 ms
per stage; ≤40/≤60 ms end-to-end) [S-3f1]; the assignment's "~±22 ms" figure was NOT
found in a fetchable primary source → recorded UNVERIFIED, engine budgets keyed to
EBU numbers instead. v1 exclusions stated: no custom UI framework; platform kits only.

**35_LICENSES (GATE-13)** — verdict: engine = `MIT OR Apache-2.0` (dual, wgpu/
Tauri-style): keeps the express Apache-2.0 patent grant + defensive termination for a
codec-heavy engine while retaining MIT adoption friction near zero. Codec reality:
AVC = Via LA pool, royalties "apportioned throughout the value chain" (verified);
HEVC = multi-pool (Via LA HEVC/VVC verified incl. rate table; Access Advance live);
AV1 = royalty-free with §1.3 Defensive Termination verified verbatim → AV1 is the
safe open-source encode path; AVC/HEVC = platform/hw decode only in distributed
core. H.264 patent-expiration dates: UNVERIFIED (pool still operating). RED/GREEN:
all audited deps GREEN except rubberband (GPL-2.0-or-later + commercial option,
App-Store clause verified verbatim) and x264 (GPL) — both core-excluded; LGPL-2.1+
dynamic-style + MPL-2.0 file-level (verified §1.7/§3.3 Larger Work semantics)
allowed in core. Correction recorded: uniffi-rs LICENSE = MPL-2.0 (ledger S-2e7
tagged MIT+Apache-2.0 — fixed in my doc; S-2e7 row left untouched per rules).
Policy rules block (7 rules) written for ADR consumption, incl. LGPL-only FFmpeg
build flags, code-license vs weights-license bookkeeping (pyannote: code MIT,
weights MIT but gated), OFL font embedding (verified clause), LUT/CC handling.

**36_YOUTUBE** — resumable upload protocol verified end-to-end (initiate → session
URI → PUT/Content-Range → 308 + Range resume) = native checkpoint mapping for our
render→upload job (ties E-007b segment pipeline + doc 30 headless). Quota model
CHANGED vs the assignment's assumption: current official text = separate buckets
(100 videos.insert calls/day + 10,000 units/day for everything else); the same page
still carries the legacy "1600 points per videos.insert". Both models recorded
verbatim; the confirmed legacy math 10000/1600 ≈ 6 uploads/day is kept as the
conservative default (design for 6, confirm bucket model per project — transition
UNVERIFIED). captions.insert = 400 units, `sync` parameter DEPRECATED (auto-sync is
server-side now) — send our own exact-tick-derived SRT. `status.containsSynthetic
Media` exists (AI-disclosure hook). ToS: compliance-audit quota-extension path and
the download-prohibition clause verified; inauthentic-content policy rename
(2025-07-15) + reused-content definitions verified from YouTube Help; Shorts ≤3 min
+ vertical uploads verified (precise API aspect gate UNVERIFIED). youtubeuploader
(Apache-2.0) and yt-dlp (Unlicense) verified as reference/external tools, not deps.

## License verdicts (one-glance)

- Engine: **MIT OR Apache-2.0** (dual).
- Core deps: LGPL-2.1+ (FFmpeg w/o gpl/nonfree, GStreamer, soundtouch), MPL-2.0
  (mediabunny, symphonia, uniffi), MIT/Apache/BSD/ISC/CC0 (all Rust crates audited,
  libass, harfbuzz/rustybuzz, rnnoise, whisper.cpp, ONNX, OpenCV, PySceneDetect,
  TransNetV2, pyannote code).
- RED (core-excluded): rubberband (GPL-2+ w/ commercial escape), x264/any GPL crate
  or GPL-flagged FFmpeg build, LosslessCut (GPL-2.0, reference only).
- RED/GREEN summary: 29 GREEN rows, 3 RED rows, 0 unknown after verification.

## NOT-FOUND / fetch failures (honest log)

1. gnu.org GPL-FAQ (3 attempts, connection timeouts) → FSF linking language cited
   only as paraphrase, marked UNVERIFIED-fetch in doc 35.
2. shotcut.org/how-tos/keyboard-shortcuts/ → 404 template; Shotcut repo shortcut
   doc probes 404 → Shotcut J/K/L claim left UNVERIFIED (Kdenlive carries the
   verified grammar).
3. Netflix partner-help A/V-sync page (554-byte JS shell) → "±22 ms" figure
   UNVERIFIED (doc 34 + ledger S-3f1 limitations).
4. developers.google.com client-rendered pages: per-method quota table (thumbnails
   .set / playlistItems.insert costs), OAuth scope reference page, quota-and-
   compliance-audits page → recorded UNVERIFIED; core costs verified elsewhere.
5. AOM license URL chain redirects (aomedia.org/license/patent → /patent-license/)
   resolved; no data loss.
6. Via LA older URLs (vialicensing.com/licensing/avc-primer etc.) 404 → replaced by
   live via-la.com program pages (S-3f2).

## Source IDs used

S-3f0 (Kdenlive shortcuts) · S-3f1 (EBU R37) · S-3f2 (Via LA AVC+HEVC) · S-3f3
(Access Advance) · S-3f4 (AOM Patent License 1.0) · S-3f5 (LICENSE/COPYING bundle +
HF model card + OFL text) · S-3f6 (crates.io registry data) · S-3f7 (YouTube API
docs bundle) · S-3f8 (YouTube Help bundle) · S-3f9 (youtubeuploader + yt-dlp).
Reused prior rows by citation: S-001..S-017, S-101..S-110, S-2b0..S-2b9, S-2c0..S-2c9,
S-2d0..S-2d9, S-2e0..S-2e9, S-4f0..S-4f9, docs 02/03/05/20/23/25/26/30, ADR-007/010/011,
E-003, E-007/E-007b.

## Next actions (handoffs)

1. Owner: pick engine license (doc 35 §1 recommendation) → ADR-012.
2. Confirm per-project quota model on the owner's Google Cloud project (§2 doc 36);
   start compliance audit early if >6 uploads/day needed.
3. E-009 (proposed): private-video upload drill — resume-after-kill + captions.insert
   burn measurement.
4. E-008 (proposed): scrub-burst + drag→commit timing harness vs 1 ms budget.
5. v0.2 depth: GPL-FAQ via mirror; thumbnails.set/playlistItems unit costs via
   headless render of JS pages; HEVC/VVC royalty appendix.
