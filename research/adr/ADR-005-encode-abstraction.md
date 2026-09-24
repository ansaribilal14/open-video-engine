# ADR-005: Encode abstraction

- **Status**: PROPOSED
- **Date**: 2026-09-24 · **Confidence**: MEDIUM

## CONTEXT
Export must map one render graph onto per-platform encoders under patent/license reality
(doc 35) and the Android 6h FGS resume budget; smart-render (E-007b) needs stream-copy
as a first-class "encoder" mode, not an afterthought.

## OPTIONS
A. FFmpeg libx264/x265 everywhere · B. Platform hw encoders only (MediaCodec/WebCodecs/
NVENC) · C. **Encoder trait**: hw per-platform + AV1 (SVT-AV1/rav1e) + AVC software
fallback + stream-copy mode · D. AV1-only pipeline

## EVIDENCE
- doc 35: AV1 royalty-free with §1.3 defensive termination (verified verbatim); AVC/HEVC
  = pool royalties for DISTRIBUTORS; platform hw encode of AVC/HEVC = device-licensed →
  engine ships software fallback, hw is an adapter concern
- SVT-AV1 verified thread scaling (≈16 cores @1080p presets 4–6) + scene-chunk split —
  [30, S-3d*] → CPU-only headless deterministic fallback is viable
- E-007b: 1080p keyframe-aligned copy 67–69ms vs 458–815ms re-encode → smart-render
  segment-copy is 6.8–11.9×; requires mux-level copy in the encode path
- doc 16: MediaCodec input-surface encode = zero-copy GPU path (createInputSurface)
- E-001: WebCodecs VideoEncoder available on the same baseline as decoder (browser leg)
- Android 6h/24h FGS budget → checkpoint/resume REQUIRED: encoder trait must support
  segment-sequential render with done-file checkpoints (doc 30)

| Criterion | A FFmpeg SW | B hw only | C trait hybrid | D AV1-only |
|---|---|---|---|---|
| Performance | slow CPU, deterministic | fastest | fastest per leg | slow on old hw |
| Portability | 4 legs | no SW fallback | 4 legs | weak old-device story |
| Complexity | low | device quirk matrix | trait + impls | codec availability |
| Security | in-process CVE surface | platform-managed | impl isolation | in-process |
| License | x264 GPL! (x264 = GPL-2.0+; LGPL build impossible) → openh264/AVC patents | clean | per-impl | cleanest |
| Maintenance | medium | high (per device) | medium | low |

## DECISION (provisional)
C — `trait Encoder` (configure → feed frames → drain → mux; plus `StreamCopy` mode).
Implementations: WebCodecs VideoEncoder (browser), MediaCodec (Android), FFmpeg-embedded
or command-sidecar (desktop/headless), SVT-AV1 software AV1 as the royalty-clean
universal fallback. x264 excluded from distributed core (GPL); hardware AVC/HEVC used
where the platform provides it. Muxing via Mediabunny (browser) / mp4-box writers.

## REJECTED ALTERNATIVES
A (x264 GPL + CPU-only speed); B (no SW fallback = broken old devices + headless);
D (device support too thin in v1).

## CONFIDENCE & RISK
MEDIUM. Risks: SVT-AV1 speed on low-end hardware; WebCodecs encoder gaps (Safari audio
26+); segment-stitch A/V sync at copy boundaries (E-007b showed duration-exact cuts but
only on same-GOP structure — verify across encoders); patent landscape drift (doc 35
tracked annually).
