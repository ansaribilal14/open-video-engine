# 22 — COLOR SCIENCE FUNDAMENTALS FOR THE ENGINE

> Status: PARTIAL (v0.1).

Owner: agent 3-c. Ledger IDs: S-2c2, S-2c7, S-2c9.
Scope: YUV/RGB, matrices, subsampling, range, transfer functions, primaries,
HDR10, tone mapping, LUTs, OpenColorIO, and where conversion must live in an
editing pipeline. Minimum set for engine v1 is at the end (normative for v1).

Primary sources: ITU-R BT.601/709/2020/2100 (verified URLs, S-2c9),
FFmpeg source (color-related fields and new AV_FRAME_DATA_RAW_COLOR_PARAMS,
S-2c2), mpv manual (S-2c7).

---

## 1. YUV vs RGB

- Displays are RGB; video compression is **YCbCr** (often written YUV):
  luma (Y) + two chroma difference signals. Rationale: human vision is more
  sensitive to luma detail than chroma detail, so chroma can be subsampled,
  and luma/chroma separation maps better onto compression transforms.
- Conversions are lossy in practice once subsampled; every YUV<->RGB round
  trip risks: rounding error, range mismatch, wrong matrix. An engine should
  minimize round trips, not merely get them right.

## 2. YCbCr matrices (URLs: https://www.itu.int/rec/R-REC-BT.601 ,
https://www.itu.int/rec/R-REC-BT.709 , https://www.itu.int/rec/R-REC-BT.2020)

- The matrix defines luma coefficients (how R,G,B combine into Y) and chroma
  scaling. Three tags matter:
  - **BT.601** (SD; coefficients ~0.299/0.587/0.114)
  - **BT.709** (HD; ~0.2126/0.7152/0.0722)
  - **BT.2020** (UHD; same luma coefficients as 709 but different primaries;
    constant-luminance ICtCp exists but is rare)
- Decoding YCbCr with the wrong matrix shifts colors/hues (red/green skew).
  Containers signal the matrix (MP4 colr/uncC boxes; MKV colour elements) but
  streams are frequently untagged; decoders then guess by resolution
  (SD->601, HD->709). The guess is often right and occasionally wrong —
  engines must surface the assumption, not bake it in silently.
- FFmpeg exposes these as AVFrame->colorspace (color matrix), color_primaries,
  color_trc, color_range; 2026-06 API (doc/APIchanges n9.0.2) added
  AV_FRAME_DATA_RAW_COLOR_PARAMS frame side data — evidence that signal color
  parameters are being pushed down into frames (HDR/CI paths).

## 3. Chroma subsampling (4:2:0 / 4:2:2 / 4:4:4)

- 4:4:4 = no subsampling; 4:2:2 = half horizontal chroma (broadcast, ProRes);
  4:2:0 = half horizontal AND vertical (H.264/HEVC/VP9/AV1 consumer, ~all web
  video; half resolution chroma in both dimensions).
- **Chroma siting** (where the chroma samples sit relative to luma: MPEG-style
  co-sited vs JPEG/centered vs top-left) changes the correct resampling
  filter; ignoring siting causes soft/offset color edges. FFmpeg exposes
  chroma_location; it is almost always missing from user pipelines.
- Editing implications: compositing/text rendering should happen with chroma
  at full resolution (4:4:4 or RGB working space), then subsample on export;
  rendering text into 4:2:0 directly produces fringing.

## 4. Full vs limited range (URL: https://ffmpeg.org/ffmpeg-codecs.html and
color_range fields)

- **Limited (TV/mpeg) range**: luma 16..235 (8-bit), 64..940 (10-bit); **full
  (PC/jpeg) range**: 0..255/0..1023. Chroma centered at 128/512.
- Mis-tagging produces washed-out (limited treated as full) or crushed
  (full treated as limited) images — the most common color bug in user
  pipelines and players.
- Range is a per-stream tag (AVFrame->color_range / AVPacket side data) that
  must survive every conversion; swscale and hw decoders must be told the
  range explicitly or output defaults will lie.

## 5. Transfer functions (URL: https://www.itu.int/rec/R-REC-BT.2100)

- **Gamma-style** (SDR): camera OETF + display EOTF conventions; BT.1886 for
  displays, sRGB EOTF for computer graphics; 709 OETF in cameras. The exact
  math differs (power ~2.4 with offsets vs pure 2.2), which matters for
  gradients.
- **PQ (SMPTE ST 2084, in BT.2100)**: absolute luminance coding up to
  10,000 nits, 10/12-bit; used by HDR10/HDR10+/Dolby Vision base layer.
- **HLG (ARIB STD-B67, in BT.2100)**: relative, display-invariant-ish;
  backwards-friendly with SDR displays; used by broadcast.
- Mixing tags (decoding PQ as gamma) yields obviously wrong contrast; the
  engine must treat transfer function as metadata that drives renderer
  behavior, never as a silent label.

## 6. Primaries / gamuts: Rec.601 vs Rec.709 vs Rec.2020 vs Display P3
(URL: https://www.itu.int/rec/R-REC-BT.2020 ; Display P3:
https://www.color.org/chromacity.phtml)

- Primaries define the RGB color triangle. 709 covers HD; 2020 is much wider
  (UHD/HDR); Display P3 (Apple ecosystem) is between (wider than 709, smaller
  than 2020); DCI-P3 uses a different white point than Display P3.
- Wide-gamut content (P3/2020) shown through a 709-only path desaturates;
  the renderer must know the target display gamut to convert correctly.
- sRGB is the default UI/document space on the web; a video engine's preview
  must be explicit about preview space (usually sRGB-composed UI, video
  converted to the display's actual space).

## 7. HDR10 basics (URL: https://www.itu.int/rec/R-REC-BT.2100 ;
https://professionalsupport.dolby.com for DV)

- HDR10 = PQ transfer + 2020/P3 primaries + **SMPTE ST 2086 static metadata**
  (mastering display primaries/white point/min/max luminance) + **MaxCLL /
  MaxFALL** (content light level), 10-bit typical.
- Dynamic metadata variants: HDR10+ (ST 2094-40) and Dolby Vision (RPU side
  data; needs profile handling — note FFmpeg 9.0 added
  AV_STREAM_GROUP_PARAMS_DOLBY_VISION in 2026-05, verified doc/APIchanges,
  evidence that DV grouping is becoming first-class in engines).
- Player/renderer contract: ST 2086/CLL tell the tone mapper what the
  mastering range was; ignoring them produces washed or clipped HDR.

## 8. Tone mapping concept (verified vocabulary: mpv options.rst, S-2c7)

- Tone mapping maps a higher dynamic range (PQ) to a lower one (SDR display):
  choose target, preserve highlights/saturation, avoid hard clipping.
  Algorithms: clip, reinhard, hable, mobius, **BT.2390** (EOTF-aware spline,
  the professional default), plus inverse tone mapping (SDR->HDR) and black
  point compensation (verified in mpv docs).
- It belongs at the *output* stage (display or export), parameterized by the
  target's capability; per-effect tone mapping destroys grading headroom.

## 9. LUTs (look-up tables)

- 1D LUT: per-channel curve (exposure/contrast/levels). 3D LUT: color
  transformation sampled on a lattice (typically 17^3..65^3), interpolated
  (tetrahedral/trilinear); .cube is the de-facto interchange format
  (Adobe/IRIDAS origin; format description widely documented — cite exact
  spec in v0.2, UNVERIFIED in this pass).
- LUTs are defined **relative to a color space pair** ("from log to 709");
  applying one in the wrong space is a silent correctness bug. Engine must
  store (input space, output space, LUT) triples.

## 10. OpenColorIO's role (URL: https://opencolorio.org)

- OCIO provides a color-management config: named color spaces, displays,
  looks, transforms; used across VFX/animation (and ACES workflows). OCIO v2
  adds GPU rendering of transforms (suitable for a wgpu-based renderer).
- For engine v1: optional dependency, but adopt the *concept* — one central
  color config describing working space and output transforms, instead of
  scattered per-filter conversions.

## 11. Where conversion should live in an editing pipeline

Recommended dataflow (normative for design, not yet an ADR):

```
decode (tagged YCbCr, native range/depth)
  -> [single ingest normalization] -> working space
working space: full-range, 4:4:4 or RGB, >= 12-bit int / fp16
  -> effects/compositing/text/grading (no color surprises)
  -> [single output transform] per target:
       preview: display space + tone map if HDR
       export: encode space (e.g. 709 limited 420 8-bit, or PQ 2020)
```

Rules:
1. Convert **once per boundary**, never per filter.
2. Every decoded stream gets an explicit resolved tag set (matrix, primaries,
   transfer, range, chroma location); missing tags are defaulted by explicit,
   logged policy (SD->601, HD->709, HDR streams->PQ/2020), never silently.
3. Effects see working space only; they never re-implement color conversion.
4. Output transforms are parameterized by target; tone mapping happens here.
5. Keep tags intact through trim/scale/rotate; scale filters must carry
   color metadata forward (FFmpeg scale filter propagates but swscale
   conversions can drop tags if the app does not re-apply them).

## 12. Engine v1 minimum (recommendation)

1. Parse and propagate the four tags (matrix, primaries, transfer, range) plus
   chroma location for every clip; verify with ffprobe on ingest.
2. Never mix 601/709 silently: when converting, do it explicitly with the
   correct matrix and record it in the project (UI-visible "assumed BT.601"
   state is acceptable; silent conversion is not).
3. Full/limited range correctness end-to-end (the #1 practical bug).
4. 4:2:0 -> 4:4:4 upsample once at ingest; render text/graphics in working
   space; subsample on export.
5. Working space: full range, 4:4:4, >= 10-bit (16-bit int or fp16 preferred).
6. Tone mapping only at output stages; BT.2390-style default for HDR->SDR.
7. Container-level tag round trip on export (write colr/uncC or MKV colour
   elements; do not emit untagged exports).
8. HDR10 support = read ST 2086/CLL and pass to renderer; do not attempt full
   DV/HDR10+ authoring in v1.

## 13. Sources

- ITU-R BT.601/709/2020/2100 recommendation pages (verified HTTP 200, S-2c9)
- FFmpeg doc/APIchanges n9.0.2: AV_FRAME_DATA_RAW_COLOR_PARAMS (2026-06-10),
  AV_STREAM_GROUP_PARAMS_DOLBY_VISION (2026-05-31) (S-2c2)
- mpv DOCS/man/options.rst: tone-mapping, target-colorspace-hint, black point
  compensation (S-2c7)
- https://opencolorio.org (S-2c9)
- Display P3 white/primaries: https://www.color.org/chromacity.phtml (S-2c9)

## 14. Depth tracking (v0.1 -> v0.2)

- Fetch actual BT.709/BT.2020 matrix coefficient tables and BT.2100 PQ/HLG
  equations into this doc (currently referenced, not reproduced).
- Extract .cube format spec and decide LUT engine representation.
- Prototype tag-propagation test: ffprobe a 601-tagged SD clip, confirm no
  silent 709 conversion through our pipeline.
