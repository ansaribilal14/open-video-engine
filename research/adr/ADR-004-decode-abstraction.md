# ADR-004: Decode abstraction

- **Status**: PROPOSED
- **Date**: 2026-09-24 · **Confidence**: MEDIUM-HIGH

## CONTEXT
The timeline/compositor core must consume decoded frames without knowing which decoder
(hardware/software, per-platform) produced them; playback scrubbing, trimming, and the
E-007b segment-copy renderer all need seeking semantics with exact time (ADR-007).

## OPTIONS
A. Call FFmpeg directly from core · B. **Rust `Decoder` trait** + per-platform impls with
capability probing and SW fallback · C. GStreamer appsink as the only decode path

## EVIDENCE
- doc 04 rules: keyframe-index + accurate seek with codec flush; VFR-safe PTS-driven;
  hw decode needs per-stream capability probing + SW fallback (verified FFmpeg hwcontext,
  MediaCodec surface model) — [04, 16, S-2c0, S-2e5]
- E-002/E-002c: timestamps must flow as exact rationals (i64 ticks + rate), never fp
  seconds — fp drift measured; tick-axis overflow guard in ove-time suite (P3b)
- E-001: browser leg decodes via WebCodecs VideoDecoder (48/48 real H.264) — the trait
  must abstract async frame delivery (WebCodecs is callback-based; FFmpeg is pull)
- doc 22: color tags (matrix/primaries/transfer/range) must propagate with frames —
  trait frame type carries them from day one

| Criterion | A direct FFmpeg | B trait + impls | C appsink only |
|---|---|---|---|
| Performance | best-case C speed | vtable cost negligible vs decode | good |
| Portability | desktop-only realistically | 4 legs | desktop + some mobile |
| Complexity | low initially, high later | trait design upfront | GStreamer learning curve |
| Security | core exposed to CVEs | impl isolation (sidecar parse possible) | moderate |
| License | LGPL entangles core | per-impl licensing | LGPL |
| Maintenance | API churn hits core | churn hits impls only | framework churn |

## DECISION (provisional)
B — `trait Decoder` (open → probe capabilities → seek(pts_exact) → frames with
(pts, data, color-tags, keyframe flag) → close); async-friendly; implementations:
FFmpeg-SW, FFmpeg-hw, MediaCodec, WebCodecs; frame data crosses via zero-copy where the
platform allows (E-001 importExternalTexture / AHardwareBuffer), copy otherwise.

## REJECTED ALTERNATIVES
A (couples core to one LGPL C library + churn); C (mobile + browser impossible).

## CONFIDENCE & RISK
MEDIUM-HIGH. Risks: async/pull impedance mismatch (mitigate: internal frame queue);
zero-copy heterogeneity (V1 ships copy path everywhere, zero-copy as adapter opt-in);
seek accuracy differences per backend (verify with cross-backend seek conformance tests).
