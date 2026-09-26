# 35_LICENSES — License audit (GATE-13)

> Status: PARTIAL (v0.1 — wave-3 depth pass 2026-09-24).

Scope: engine license choice, codec patent reality, per-dependency license verdicts
(GREEN = safe in permissive core; RED = excluded from core, adapter-only), GPL
contamination rules, font/LUT/CC-media handling. All license statements below were
verified against primary license files or registries this session unless marked
UNVERIFIED; fetch failures are logged at the end. This doc is evidence, not legal
advice.

## 1. Engine license choice: MIT vs Apache-2.0 vs dual

| Option | Grant | Patent clause | Fit for a codec-heavy engine |
|---|---|---|---|
| MIT | copyright only | none | simplest; no express patent grant — an encoder pipeline is exactly where patent claims bite |
| Apache-2.0 | copyright + trademark notices | express patent grant (§3) + termination on patent litigation (§3, retaliation) | aligns with media/AI infra norms |
| MIT OR Apache-2.0 (dual, SPDX `OR`) | user picks | retains Apache patent grant for Apache path | what wgpu ships [S-2d1][S-3f6] |

**Recommendation: `MIT OR Apache-2.0` (dual), matching wgpu/Tauri/Cutlass practice
[S-2d1][S-2e8][S-104]** — lowest adoption friction (MIT path) while keeping the
express patent grant + defensive termination (Apache path). Verified precedents:
wgpu `MIT OR Apache-2.0` (crates.io 30.0.1 [S-3f6]); OTIO Apache-2.0 [S-002/S-2b1];
Media3 Apache-2.0 [S-2e0]; OpenCV Apache-2.0 [S-3f5]; jni `MIT OR Apache-2.0`
[S-3f6]. Rationale: an engine whose OPTIONAL distributor may ship encoders should
grant and receive patent peace explicitly; MIT alone grants no patent license.

## 2. Codec patent reality (what DISTRIBUTING an encoder pipeline means)

- **H.264/AVC**: Via Licensing Alliance (formerly MPEG LA) operates the AVC/H.264
  Patent Portfolio License — verified live page: "provides access to essential
  patent rights for the AVC/H.264 (MPEG-4 Part 10) … reasonable royalties are
  apportioned throughout the AVC/H.264 value chain" (set-top boxes, players,
  subscription/VOD, broadcast…) [S-3f2]. OPEN-SOURCE CODE ≠ patent license: MIT/
  GPL code can be shipped royalty-free as source; the DISTRIBUTOR of an AVC
  encoder/decoder product may owe pool royalties. Engine policy therefore: prefer
  OS/hardware codecs at runtime, never build a stock software AVC encoder into
  core binaries we distribute; keep x264 out entirely (GPL, §5).
- **H.265/HEVC**: three pools (MPEG LA/Via LA HEVC, HEVC Advance/Access Advance,
  Velos). Verified: Via LA HEVC/VVC program page with per-unit royalty table
  [S-3f2]; Access Advance "HEVC Advance Licensing Program" live [S-3f3]. Multi-pool
  stacking is the classic reason HEVC distribution cost > AVC; treat HEVC as
  opt-in hardware-decode-only in v1.
- **AV1**: verified from the AOMedia Patent License 1.0 page: "royalty-free" grant;
  §1.3 "Defensive Termination" — "If any Licensee, its Affiliates, or its agents
  initiates patent litigation … asserting that any Implementation infringes
  Necessary Claims, any patent licenses granted under this License … are
  immediately terminated" [S-3f4]. AV1 encoders (rav1e — Apache-2.0 crates.io
  [S-2e7], SVT-AV1) are the safe open-source encode path for our stack.
- **VP9**: license situation (Google's royalty-free VP9 grant via webm/AOM VP9
  license) NOT verified this session → UNVERIFIED. Usage note unchanged: decode
  via platform/hw where possible.
- **Patent expiration (H.264 timelines)**: widely cited claim that most early AVC
  pool patents expire ~2023–2027; NOT verifiable from a primary source this
  session → UNVERIFIED. Verified counter-signal: Via LA still operates the AVC
  pool with current royalty schedules [S-3f2], so AVC must still be treated as
  licensed-not-free for distribution.
- **User-installed codecs escape hatch**: patents attach to making/selling/using
  the invention; a tool that uses a USER-PROVIDED FFmpeg/system codec stack
  (dlopen-style, not bundled) keeps the engine vendor out of the encoder-sale
  chain for formats we don't license — this is the standard LGPL-only-FFmpeg
  posture documented by the FFmpeg project (LGPL compliance checklist; no GPL/
  nonfree flags) [S-2c1][S-2c2]. UNVERIFIED as a legal shield; it is the
  established ecosystem practice, and FFmpeg itself documents the build-flag
  discipline.

## 3. Dependency audit table (GREEN = permissive core OK; RED = keep out of core)

| Dependency | License (verified source) | Verify evidence | Verdict |
|---|---|---|---|
| FFmpeg (libs) | LGPL-2.1+ (no --enable-gpl/--enable-nonfree) | legal.html + n9.0.2 tree [S-2c1][S-2c2] | GREEN with flags; RED if GPL build |
| GStreamer core + -bad + -ugly | LGPL-2.1 (COPYING files) | [S-2c5] | GREEN; exclude GPL-linked plugins (x264enc) |
| MLT | LGPL-2.1-or-later (header) | [S-2b3] | GREEN (reference only) |
| symphonia | MPL-2.0 | crates.io 0.6.1 [S-3f6] | GREEN (§4) |
| mediabunny | MPL-2.0 | LICENSE fetched [S-3f5] | GREEN (§4) |
| libass | ISC | COPYING fetched [S-3f5] | GREEN |
| harfbuzz | "Old MIT" (COPYING wording) | COPYING fetched [S-3f5] | GREEN |
| rustybuzz | MIT (README §License) | README fetched [S-3f5] | GREEN |
| rubberband | GPL-2.0-or-later + commercial option | README+COPYING fetched [S-3f5] | RED for core (§5) |
| soundtouch | LGPL-2.1 (COPYING.TXT) | codeberg fetch [S-3f5] | GREEN |
| rnnoise | BSD-3 (Mozilla/Valin/Xiph) | COPYING fetched [S-3f5] | GREEN |
| whisper.cpp | MIT | LICENSE fetched [S-3f5] | GREEN |
| pyannote.audio (code) | MIT (CNRS) | LICENSE fetched [S-3f5] | GREEN |
| pyannote MODEL weights | MIT (HF tag) but gated="auto" (acceptance flow) | HF API [S-3f5] | GREEN w/ access terms; code≠weights distinction noted |
| PySceneDetect | BSD-3-Clause | LICENSE fetched [S-3f5] | GREEN |
| TransNetV2 | MIT | verified by 3-f [25] | GREEN |
| OpenCV | Apache-2.0 | LICENSE fetched [S-3f5] | GREEN |
| ONNX Runtime | MIT | LICENSE fetched [S-3f5] | GREEN |
| wgpu | MIT OR Apache-2.0 | crates.io 30.0.1 [S-3f6] | GREEN |
| uniffi | **MPL-2.0** (LICENSE = MPL text) | LICENSE fetched [S-3f5] | GREEN (§4) — corrects S-2e7's "MIT+Apache-2.0" tag |
| jni | MIT OR Apache-2.0 | crates.io 0.22.4 [S-3f6] | GREEN |
| cpal | Apache-2.0 (crates.io 0.18.2) | [S-3f6] | GREEN |
| blake3 | CC0-1.0 OR Apache-2.0 OR Apache-2.0 WITH LLVM-exception | crates.io 1.8.7 [S-3f6] | GREEN |
| tokio | MIT | LICENSE fetched [S-3f5] | GREEN |
| rusqlite | MIT | LICENSE fetched [S-3f5] | GREEN |
| opendal | Apache-2.0 | LICENSE fetched [S-3f5] | GREEN |
| LosslessCut | GPL-2.0 | [S-108] | RED (reference only) |
| x264/libx264 | GPL-2.0+ | via FFmpeg legal [S-2c1] | RED |

Model-weights caveat (repeated for any ML dep): CODE license (repo) and WEIGHTS
license (HF model card) are separate grants; record both per asset, plus any
gating (pyannote/segmentation-3.0: license:mit, gated:auto [S-3f5]).

## 4. MPL-2.0 in an MIT/Apache project — verified semantics

MPL-2.0 is FILE-level copyleft, compatible with permissive Larger Works. From the
license text itself (mediabunny LICENSE fetched):
- §1.7: `"Larger Work" means a work that combines Covered Software with other
  material, in a separate file or files, that is not Covered Software.`
- §3.3: `You may create and distribute a Larger Work under terms of Your choice,
  provided that You also comply with the requirements of this License for the
  Covered Software.`
Practical rules: (1) MPL files (vendored or modified) stay MPL and their source
must be available; (2) our files remain MIT/Apache; (3) no obligation to open the
engine. Same verdict for symphonia + uniffi (both MPL-2.0, [S-3f5][S-3f6]).
Note: ledger row S-2e7 tagged uniffi "MIT+Apache-2.0" — corrected here against the
actual uniffi-rs LICENSE file (MPL-2.0); S-2e7 author informed via this doc.

## 5. GPL contamination rules

- **C/C++ world (FFmpeg/GStreamer plugins)**: linking GPL code into an LGPL build
  makes the whole build GPL — FFmpeg's legal page states the --enable-gpl/
  --enable-nonfree consequences and the no-GPL-libs-in-LGPL-build rule [S-2c1];
  GStreamer mirrors this per-plugin (x264enc + libx264 ⇒ GPL artifact) [S-2c5].
  Rule: core = LGPL-only builds; GPL capabilities ship as SEPARATE, optional,
  out-of-tree artifacts (own download/repo) — e.g. a "GPL encoder pack" adapter.
- **rubberband**: verified precisely — README: "distributed under the GNU General
  Public License … either version 2 … or (at your option) any later version … If
  you wish to distribute code using Rubber Band Library under terms other than
  those of the GNU General Public License, you must obtain a commercial licence
  from us … you may not legally distribute through any Apple App Store unless you
  have a commercial licence." [S-3f5] ⇒ RED in core; options: (a) buy the
  Breakfast Quay commercial license for the app distribution, (b) adapter behind
  a time-stretch trait with an LGPL default (soundtouch) [S-3f5].
- **Rust crates = linking? (gray area, positions stated honestly):**
  - Conservative position (treat like C linking): a statically-linked Rust crate
    is part of the combined program; GPL crate ⇒ GPL binary. This mirrors the FSF
    FAQ's long-standing static/dynamic linking position. **GNU GPL-FAQ page could
    NOT be fetched this session (gnu.org unreachable from sandbox) — the FAQ
    wording is UNVERIFIED-fetch; position paraphrased from common knowledge.**
  - Liberal position: crate-API composition is aggregation; GPL applies only to
    the crate's own code. No authoritative resolution exists; projects diverge.
  - Adopted policy (conservative, matches ecosystem norm of `license = GPL` crates
    being excluded from permissive products): NO GPL crates in engine core; GPL
    only in optional adapters distributed separately. Our audited crate set
    contains zero GPL (§3).
- **AGPL**: none in the audited set; policy: not in core (server-side copyleft
  interacts badly with an installable engine; revisit only if a hosted service
  appears).

## 6. Fonts, LUTs, CC media

- **Fonts (captions/titles)**: Noto + Google Fonts ship under SIL OFL 1.1 —
  verified from the official OFL text: fonts "can be bundled, embedded,
  redistributed and/or sold with any software", provided each copy carries the
  copyright notice + license; derivatives must not use Reserved Font Names; the
  fonts/derivatives cannot be relicensed under other terms [S-3f5]. Engine rules:
  bundle OFL fonts with license files; renaming derivatives may violate RFN; keep
  per-font LICENSE directory. Web shells can also rely on platform fonts.
- **LUTs (.cube)**: no universal license; vendor LUTs (camera makers) are typically
  copyrighted-with-permissions, freeware LUTs have bespoke terms. Policy: ship NO
  third-party LUTs in core; ship a CC0 example set we author; treat imported LUTs
  as user assets with user-supplied license metadata. Specific vendor LUT terms:
  UNVERIFIED this session.
- **CC-licensed media**: attribution propagation is a data-model requirement, not
  just docs: asset records must carry `license`, `attribution`, `source_url`, and
  derivative flags; exports must preserve attribution (CC BY requires attribution;
  CC BY-SA applies to adaptations; CC0 = no conditions). Verified-by-purpose here
  (CC license suite texts not re-fetched this session — UNVERIFIED-fetch for
  verbatim clauses). Asset-manifest precedent: Diffusion Studio's hashed assets.yml
  [S-103].

## 7. Policy rules (normative for ADRs)

1. **Core** = `MIT OR Apache-2.0`, depends only on: MIT/Apache/BSD/ISC/CC0/Unlicense
   + MPL-2.0 (file-level, no source obligations for our files) + LGPL-2.1+
   libraries used dynamic-style (never vendored/modified) in LGPL-only builds.
2. **RED licenses** (GPL any version, AGPL, nonfree, unknown-source weights):
   excluded from core; allowed only as separate optional adapters with their own
   distribution channel and license notice.
3. **FFmpeg discipline**: LGPL-only configure set (--disable-gpl --disable-nonfree
   posture); hw encoders/encoders via platform APIs; no x264.
4. **AV1 = the royalty-free encode path; AVC/HEVC = platform/hw decode, never a
   stock software encoder in distributed core binaries.**
5. **Every third-party asset/weight/model entry records code-license AND
   content/weights-license + gating requirements.**
6. **License metadata travels with projects**: assets carry license/attribution;
   export paths preserve them (§6).

## VERIFIED / UNVERIFIED summary

VERIFIED this session: all §3 license rows via raw LICENSE/COPYING files or
crates.io/HF registry data; AOM §1.3 Defensive Termination verbatim [S-3f4];
Via LA AVC + HEVC program pages [S-3f2]; Access Advance page live [S-3f3];
rubberband commercial/App-Store clause verbatim [S-3f5]; OFL embedding clause
verbatim [S-3f5]; MPL §1.7/§3.3 verbatim [S-3f5].

UNVERIFIED / fetch failures (logged): GNU GPL-FAQ verbatim text (gnu.org
unreachable — 3 attempts, timeouts); VP9 patent-license terms; H.264 pool patent
expiration dates; per-crate historical license changes (cpal: crates.io shows
Apache-2.0 at 0.18.2; older dual MIT claims unexamined); vendor LUT terms; CC
suite verbatim texts (paraphrased, standard suite).

## Depth remaining (v0.2+)

- Fetch gpl-faq.en.html from a mirror; pin exact FSF linking language.
- Pull Velos + via-la HEVC royalty tables into an appendix; confirm SISVEL/AVC
  patent-list renewal cadence.
- SPDX manifest generation + `cargo deny` policy experiment (E-00x proposal).
- Legal review of MPL crate modification policy (do we ever patch MPL crates?).
