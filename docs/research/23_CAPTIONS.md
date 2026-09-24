# 23_CAPTIONS — Caption/subtitle architecture (Track J)

> Status: PARTIAL (v0.1 — wave-3 depth pass 2026-09-24).

## 0. Method

All primary sources fetched 2026-09-24, unauthenticated (no GITHUB_TOKEN): W3C spec pages,
raw GitHub (LICENSE/README), crates.io + PyPI + npm registry APIs with custom User-Agent,
Wikipedia, one vendor page. New rows S-3b0..S-3b3 in `research/sources/LEDGER_W3b.md`.
Existing rows reused: S-005/S-2c2 (FFmpeg), S-2d0 (browser-compat-data), S-2e0 (Media3),
S-4f9 (media-derived text = untrusted data). Scope overlap: transcription models live in
37_TRANSCRIPTS.md; AI-agent governance in 26/27; this doc owns formats, rendering, timing,
and the caption-authoring loop.

## 1. Format matrix (primary-source verified)

| Format | Precision | Styling/positioning | Reference impl | License | VERIFIED |
|---|---|---|---|---|---|
| SubRip (SRT) | ms, comma decimal [S-3b3] | plain text; de-facto tags only | CCExtractor (extract) [S-3b3] | GPL-2.0 (tool, not format) | format: community-de-facto; no normative spec exists |
| WebVTT | ms, dot decimal [S-3b0] | cue settings (line/position/size/align/vertical/region) + `::cue` CSS [S-3b0] | browsers (TextTrack) [S-2d0, S-3b2] | spec, royalty-free | yes |
| ASS/SSA | **centiseconds** [S-3b3 analysis] | karaoke `\k`, typesetting `\pos \move \t \fad \clip`, fonts, colors | libass 0.17.5 [S-3b2] | ISC [S-3b2] | yes |
| TTML/IMSC | arbitrary clock values; IMSC constrains | full XML styling; profiles for interchange | IMSC 1.3 REC 2026-05-21 [S-3b1] | W3C Document license | spec status yes; impls UNVERIFIED |
| SCC/CEA-608/708 | frame timecode (29.97 etc.) [S-3b1] | 608: 2-channel char grid; 708: windowed | CCExtractor (decode) [S-3b3] | standards sold by CTA; GPL-2.0 tool | standard identity yes; bit-level details UNVERIFIED |

## 2. SRT: the exactness analysis (task brief correction)

Wikipedia (quality 10, only community source): timecode is `hours:minutes:seconds,milliseconds`,
"two zero-padded digits" for H/M/S and "three zero-padded digits" for the fraction, comma as
decimal separator — i.e. **SRT is millisecond precision: rational (n,1000), NOT centiseconds**
[S-3b3]. The centisecond trap belongs to **ASS/SSA** (`H:MM:SS.cc`, 2 digits → (n,100)) [S-3b3
analysis]. Both are exact rationals under ADR-007; the ingest conversion is
`ss + mm*60 + hh*3600 + frac/den` in integer arithmetic. No fp seconds anywhere in the model
(E-003 P5 proved fp payloads break replay determinism).

Dialect risk (no normative SRT spec): overlapping cues tolerated by players, HTML-ish tags
(`<i>`, `<font>`) in payloads, UTF-8/BOM variance, frame-based variants from misexports.
Engine stance: strict parser for the documented core grammar + explicit error list; never
silently "fix" timing. Rust ecosystem exists (crates.io `subtitle`, `srt_subtitles_parser`,
`libbitsub` … found via registry search; per-crate quality UNVERIFIED) [S-3b3].

## 3. WebVTT (browser + streaming sidecar)

- Status: latest published snapshot is a **Candidate Recommendation Draft, 2026-05-20**
  (TR/2026/CRD-webvtt1-20260520/); prior CR 2019-04-04. **Not yet a W3C Recommendation** [S-3b0].
- Timestamp grammar (verbatim-verified): optional hours (required if non-zero, 2+ digits),
  `MM:SS`, U+002E FULL STOP, "three ASCII digits, representing the thousandths of a second"
  → again (n,1000), hours-optional is a parse gotcha [S-3b0].
- Cue text blocks separated by `-->` lines; payload must not contain `-->` [S-3b0].
- Styling: `::cue` / `::cue-region` pseudo-elements (120/38 occurrences in spec) + cue
  settings `vertical`, `line`, `position`, `size`, `align`, `region` [S-3b0].
- Spec states the "SRT file format was used as the basis for the WebVTT text track file
  format" [S-3b0].
- Browser-native rendering: BCD `api.TextTrack`: Chrome 23, Firefox 31, Safari 6, Edge 12,
  Safari iOS 7 [S-2d0, S-3b2]. This is the "bundled renderer" fact: every modern browser
  already ships a WebVTT renderer; a web adapter should feed `<track>` for *playback* and
  reserve our compositor for *burn-in/export* and for styling beyond CSS-cue limits.

## 4. ASS/SSA + libass

- libass: "portable subtitle renderer for the ASS/SSA ... mostly compatible with VSFilter";
  latest release **0.17.5 (2026-06-24)**; **ISC license verified from COPYING** ("Copyright
  (C) 2006-2016 libass contributors") [S-3b2]. ISC is permissive → usable in the Rust core
  via FFI or vendored without copyleft obligations.
- Dependency burden (FFI + ports): freetype/fontconfig/fribidi/harfbuzz are typical libass
  deps; their individual licenses NOT re-verified this pass (UNVERIFIED — see §10) [S-3b2].
- What ASS buys: karaoke (\k/\kf/\ko word timing), 2D typesetting (\pos, \move, \t animated
  transforms, \clip masks), per-style fonts/outlines/shadows. What it costs: a full C renderer
  + font stack, and frame-rate context for frame-based tags (UNVERIFIED which tags libass
  quantizes to which fps) [S-3b2].
- Frame-rate locking: ASS timestamps are centiseconds; cue boundaries map exactly to (n,100)
  rationals; any frame-snapped effect must be computed against the project rate from
  ADR-007, not against fp seconds.

## 5. TTML/IMSC and SCC/CEA-608/708 (broadcast/streaming leg)

- TTML2: W3C Recommendation, edition at /TR/ttml2/ = TR/2018/REC-ttml2-20181108/ [S-3b1].
- IMSC: **IMSC Text Profile 1.3, W3C Recommendation 2026-05-21** (TR/2026/
  REC-ttml-imsc1.3-20260521/); IMSC 1.2 REC 2020-08-04 also verified [S-3b1]. IMSC =
  constrained TTML profile for interchange (streaming/broadcast); adoption-by-streamers
  claims UNVERIFIED this pass.
- CTA-708 (formerly EIA-708/CEA-708) = closed-captioning standard for ATSC DTV in US/Canada,
  developed by the EIA consumer-electronics sector that became the Consumer Technology
  Association; replaced EIA-608 (analog NTSC). CEA-608 = analog TV closed captioning standard
  [S-3b1 — Wikipedia, quality 10]. Standards documents are sold by CTA (not freely fetched);
  bitstream details UNVERIFIED.
- Practical read path: CCExtractor (GPL-2.0, GitHub repo label + LICENSE.txt verified)
  extracts closed captions from TV/DVD recordings and converts CC→subtitles [S-3b3]. GPL
  tool → keep as an *external* pre-pass binary or behind a process boundary; never link
  into the engine core. FFmpeg demuxers already expose 608/708 as subtitle streams
  (S-005/S-2c2; per-format specifics UNVERIFIED). Media3 ships CEA-608/708 parsing
  (S-2e0; file-level verification of the text package UNVERIFIED).
- v1 stance: ingest/read + burn-in of 608/708 as converted cue data; no SCC *authoring*.

## 6. Render path decision space

Three viable paths, not mutually exclusive:

A. **Burn-in via compositor pass.** Text → shaped glyphs → raster → alpha texture →
   compositor quad pass (docs 08-09 pass-graph). Deterministic if the shaping stack is
   pinned; required for export with styling; costs: font stack + shaping + raster in core.
B. **Sidecar text track in players.** Web adapter: `<track>`/TextTrack (browser renders,
   zero code, CSS-limited styling) [S-2d0, S-3b2]. Container export: mux WebVTT/mov_text
   (mkv/mp4; FFmpeg subtitle codecs, S-005 — per-muxer support matrix UNVERIFIED). Native
   players render their own bundled renderers.
C. **Both, from one cue model.** Same cues feed preview burn-in, sidecar mux, and player
   text track. Recommended v0.2 shape: cues are engine data; renderers are adapters.
   Burn-in styling must be *opt-in identical* across platforms → pin one shaping engine
   (rustybuzz, §7) and one raster path; accept browser `<track>` variance as display-only.

## 7. Text styling pipeline (fonts, shaping, bidi, CJK, emoji)

- Shaping: **rustybuzz 0.20.1 — "A complete harfbuzz shaping algorithm port to Rust",
  MIT verified (Copyright HarfBuzz developers; (c) 2020 Yevhenii Reizner)** [S-3b2].
  Pure-Rust shaping avoids the C harfbuzz FFI; HarfBuzz itself is the C reference
  (its exact license string UNVERIFIED this pass). Choice for ove: rustybuzz first,
  harfbuzz C fallback only if feature gaps bite.
- Bidi/RTL: Unicode UAX #9 required for Arabic/Hebrew line+run ordering; candidate crate
  `unicode-bidi` (version/license UNVERIFIED this pass); fribidi is the C reference.
- CJK wrapping: line-break rules (UAX #14); no verified Rust crate chosen yet (UNVERIFIED).
- Emoji: color-emoji fallback fonts and font-selection plumbing; depth item (UNVERIFIED).
- Font assets: engine needs font enumeration + fallback chains + per-style font binding;
  browser adapter may reuse CSS font stack for `<track>` path only.

## 8. Caption-authoring AI loop (transcript → cues → edits)

1. Input: transcript with word-level timestamps (37_TRANSCRIPTS.md: whisper.cpp /
   faster-whisper `word_timestamps=True` [S-3b9], WhisperX wav2vec2-aligned word times
   [S-3b9]).
2. Segmentation service: split into cues by line-length/reading-speed norms per platform —
   norms are community/industry convention, not specs; concrete numbers UNVERIFIED (depth
   item). WhisperX v3 itself segments "per-sentence ... for better subtitling" [S-3b9].
3. Timing adjust: snap cue boundaries to word intervals; never to fp seconds; karaoke
   word-highlight = per-word spans (ASS `\k` on export).
4. YouTube auto-captions: programmatically retrievable — `youtube-transcript-api` 1.2.4,
   MIT, "get the transcripts/subtitles for a given YouTube video ... also works for
   automatically generated subtitles" [S-3b3]. Compliance/ToS caveats out of scope here
   (doc 36).
5. Edit verbs map 1:1 onto §9 commands (move/resize/edit/split), so the AI loop and the
   human caption editor share the same surface (ADR-010).

## 9. ove caption model proposal (E-003/ADR-007 conformant)

```
CaptionTrack { id, cues: ordered by start }
Cue { id, start: Rational(project axis), end: Rational,
      payload: [Span],            // styled text spans
      word_times: Option<[SpanTime]>, // per-word rationals
      layout: Option<LayoutHint>, // WebVTT settings / ASS margins, per-source
      origin: SourceRef }         // transcript id + segment (37)
```

Commands (explicit ids, exact rationals, inverses; schema v0 rules from E-003):
`caption.import{format, src_hash} -> [add_cue...]` (analysis batch, user-approvable),
`caption.add_cue{id,start,end,payload}`, `caption.move_cue{id,start}`,
`caption.resize_cue{id,end}`, `caption.split_cue{id,at,right_id}`,
`caption.edit_text{id,spans}`, `caption.set_style{track_id,style}`,
`caption.export{format}` (render/serialize side, non-mutating).
Storage: cues live in the project snapshot; the command log stays the only mutation path.

## 10. UNVERIFIED items

- libass transitive dep licenses (freetype/fontconfig/fribidi/harfbuzz exact strings).
- HarfBuzz C library license string; unicode-bidi / UAX-14 crate versions+licenses.
- IMSC adoption claims (Netflix/broadcast usage); TTML2 later editions beyond 2018-11-08.
- CEA-608/708 bitstream details; per-muxer subtitle codec support matrix (mp4/mkv/WebM).
- Media3 CEA parser file-level verification; Whisperer token-level timestamps in
  whisper.cpp (`--dtw`) — deferred to 37_TRANSCRIPTS.md depth list.
- Caption length/reading-speed norms per platform (docs 34 UX).
- Browser `<track>` styling limits per engine (CSS cue properties support matrix).

## 11. Depth remaining (v0.2)

- libass integration spike: FFI or vendored ISC; measure burn-in path cost.
- WebVTT cue-settings ↔ ove LayoutHint round-trip spec; ASS tag subset we honor.
- Parser property tests: SRT/VTT/ASS round-trip vs rational time (extends E-002 suite).
- Karaoke rendering path decision (spans vs ASS export); emoji/CJK fallback matrix.
- SCC/608 read path experiment with CCExtractor as external pre-pass.
