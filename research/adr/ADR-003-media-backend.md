# ADR-003: Media backend (decode/encode/filter libraries)

- **Status**: PROPOSED (per-platform adapters; final selection gated on integration benches)
- **Date**: 2026-09-24 · **Confidence**: MEDIUM-HIGH

## CONTEXT
The engine must demux/decode/encode on four legs (desktop, Android, browser, headless)
under one Rust core. No single library serves all legs under acceptable license and
hardware-acceleration constraints.

## OPTIONS
A. FFmpeg (libav*) everywhere · B. GStreamer pipeline everywhere · C. Platform-native
per leg (Media3/MediaCodec, AVFoundation, WebCodecs) · D. **Layered adapters**: browser =
WebCodecs + Mediabunny mux/demux; Android = MediaCodec (+ Media3 Transformer as export
backend); desktop/headless = FFmpeg (LGPL build); audio decode = symphonia where sufficient.

## EVIDENCE
- FFmpeg 9.0.2 verified; LGPL-only builds possible (legal.html) — [04, 05, S-2c0..S-2c9]
- GStreamer 1.28.7 LGPL-2.1 core+plugins; mobile footprint + negotiation complexity — [06]
- WebCodecs video baseline on all desktop engines; Mediabunny closes mux/demux — [11, E-001]
- Media3 1.11.1 Transformer = HW export primitive, NOT a timeline engine; CompositionPlayer
  preview; @UnstableApi — [15, S-2e0..S-2e9]
- E-007b: ffmpeg keyframe-aligned stream-copy 6.8–11.9× faster than re-encode at 1080p →
  segment-copy renderer needs library-level (not CLI) access to stream copy — [E-007b]
- E-001: real H.264 decode 48/48 frames in Chrome 154 (WebCodecs leg proven)

| Criterion | A FFmpeg everywhere | B GStreamer | C per-OS native | D layered adapters |
|---|---|---|---|---|
| Performance | excellent, hw via hwcontext | good | best per-OS | best per-OS + shared core |
| Portability | 4 legs | desktop-strong, mobile weak | per-leg code | per-leg adapters, core portable |
| Complexity | C FFI + API churn | high (pipes/caps) | high (N stacks) | trait boundary isolates churn |
| Security | CVE surface (doc 33: sidecar parsing) | medium | platform-managed | untrusted-parse isolation per leg |
| License | LGPL-2.1+ build discipline required | LGPL-2.1+ | Apache/proprietary mix | per-adapter license isolation |
| Maintenance | 1 library + bindings | 1 framework | N stacks | traits + thin adapters |

## DECISION (provisional)
D — layered media adapters behind a Rust `MediaBackend` trait; FFmpeg (LGPL build) for
desktop/headless, WebCodecs+Mediabunny for browser, MediaCodec for Android with Media3
Transformer as optional export backend. Final ACCEPT gated on decode/encode integration
bench (successor of E-007b at adapter level) and doc-35 license audit sign-off.

## REJECTED ALTERNATIVES
B (mobile weakness + complexity); C alone (N stacks, no shared guarantees); A alone
(browser leg impossible — WebCodecs IS the browser media stack).

## CONFIDENCE & RISK
MEDIUM-HIGH. Risks: FFmpeg API churn at majors (05 §pitfalls); WebKitGTK codec fragility
(doc 17); Media3 @UnstableApi churn; Android FGS 6h budget forces checkpointed exports
(resume design required regardless of backend).
