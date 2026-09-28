# ADR-015: Encode leg v1 (ove-encode — encoder, muxer, stream-copy route)

- **Status**: ACCEPTED (2026-09-29) — acceptance condition met: ENCODER_SPEC
  §4 gates E-1..E-5 (+ E-7 planner/executor gates) green in-repo
  (engine/ove-encode/tests/{planner,conformance}.rs, 28 tests), outputs
  ffprobe- and libav-verified, golden bytes committed.
- **Date**: 2026-09-29 · **Confidence**: 0.88 (confinement amendment and
  golden keying are the durable decisions; codec surface is intentionally
  small and grows without re-opening this ADR)

## CONTEXT

WAVE 3 (ENGINE_BUILD_PLAN W4) = ove-encode: `Encoder` trait
(configure/feed/drain) + `Muxer` + the StreamCopy export route chosen by a
pure export planner (ENCODER_SPEC §1/§3, ADR-005 promoted). Constraints:
libav confined to adapter crates; LGPL-only usage (x264 excluded, ADR-005);
exact rational timestamps end-to-end (E-002); outputs must be inspectable by
ffprobe; deterministic software path for goldens.

## DECISION

1. **Crate home + confinement amendment**: the FFmpeg encode/mux/copy
   adapter lives in `ove-encode` as a SECOND adapter crate. The confinement
   invariant is restated as a CLOSED ALLOWLIST {ove-decode, ove-encode} of
   crates permitted to link libav\* (DECODER_SPEC §5 + ENCODER_SPEC §5;
   `scripts/ci/check_libav_confinement.sh` enforces the allowlist via cargo
   metadata + source scan). Core crates stay libav-free; adding an adapter
   crate to the allowlist requires an ADR. Rationale: decode and encode are
   symmetric adapter concerns; stuffing the encoder into ove-decode would
   corrupt crate semantics to satisfy an over-narrow reading of the rule.
2. **v1 codec surface**: `mpeg4` (MPEG-4 Part 2, libavcodec-native) as the
   first ReEncode codec — LGPL, dependency-free in BOTH the system and the
   bundled FFmpeg builds, deterministic given pinned libav. Native `aac` is
   exercised via stream-copy passthrough only. `OpenH264`/`SvtAv1` are
   declared targets that report TYPED `Unsupported` (capability honesty;
   ENCODER_SPEC §2 named them; their adapter legs land later).
3. **Trait extension**: `Encoder::track_spec()` added to the ENCODER_SPEC §1
   trait — MP4 muxing is header-before-packets, so the muxer needs the
   codec parameters + extradata up front. (Spec annotation records it.)
4. **Snap semantics (made normative by tests)**:
   - `KeyframeAlignedOnly`: START must be a keyframe — exact if the request
     lands on one, floor (RoundOut) otherwise, RoundIn to the first
     keyframe when the request starts before it (a copy must OPEN on a
     keyframe); END rounds out to the ceil keyframe, or to EOF when past
     the last keyframe. The snapped span IS the exported span — every snap
     is in the receipt (ENCODER_SPEC §3.1 "never silent").
   - `AllowReencodeBoundaries` (Mixed): exact requested span =
     re-encode slivers around the copyable INNER core
     [kf_ceil(start), kf_floor(end)). E-7-style boundary exactness tests
     pin the segmentation.
5. **Audio grid rule**: audio stream-copy only when both boundaries are
   sample-exact on the codec packet grid (t × sample_rate divisible by
   frame_samples); otherwise the route is ReEncode — declared `Deferred`
   until the AAC encoder leg lands (W7). Misalignment is never silently
   cut (E-007c pins the receipt).
6. **Exactness mechanics**: encoder time base = 1/frame_rate; container
   track timescale = minimal exact axis (video: num/gcd(num,den); audio:
   sample rate). tick conversion is exact in BOTH directions — rational→
   ticks demands divisibility (typed `NonExactTimestamp`), ticks→rational
   is always exact. movenc widens video timescales (observed 24 → 12288),
   so the muxer converts against the EFFECTIVE post-header axis and derives
   the frame duration from it (axis × den/num). Codec-reported 0-duration
   packets are reported as `None`, never as a fake 0 (a 0-delta stts table
   corrupts container timing — found and fixed in this wave).
7. **Golden policy (E-5)**: byte goldens are keyed on the producing libav
   identity (`av_version_info` + per-lib versions, committed in
   `encode_golden.json`); the byte gate applies when the running libav
   matches the pinned identity, and the test ALWAYS asserts same-run
   determinism plus the structure gates (E-1..E-4). Version mismatch is
   REPORTED on stderr — an explicit, recorded skip, never silent.
8. **AVIO ownership**: mp4 does NOT set AVFMT_NOFILE — the caller must
   provide the AVIOContext. Getting this backwards leaves `pb` NULL and
   libav 7.1.5 SEGFAULTS in `avformat_init_output` (verified empirically
   2026-09-29; the fix and the crash class are pinned by the passing
   suite). Packets are handed to the muxer refcounted (`av_new_packet`),
   never aliased into caller memory.

## NON-GOALS (deferred, named)

- AAC seam re-encode (W7), OpenH264/SVT-AV1 legs, checkpoint/kill-resume
  execution (E-6; the `EncodedPacket` checkpoint fields exist from day one),
  project-axis export planning (planner works per source segment until W6
  composes the vertical slice), VFR encode (v1 retimes to CFR upstream via
  the ADR-013 seam).

## REJECTED ALTERNATIVES

- Encoding inside ove-decode (semantically wrong crate; would make decode
  tests pay for encode deps).
- FFmpeg CLI sidecar for export (process boundary; kills determinism and
  the in-process checkpoint story, ADR-005 risk).
- Non-refcounted packet aliasing into caller buffers (libavqueue may
  outlive the call frame — use-after-free class).

## CONFIDENCE & RISK

0.88. Risks: movenc timescale-widening varies by libav build (handled by
converting against the effective axis — tests pin 24→12288 behavior);
encoder byte identity across libav patch releases (handled by version-keyed
goldens; decode-side determinism premise covers the copy route); bundled
CI build now also links swscale (union feature set — portability guard
scans the same object tree).
