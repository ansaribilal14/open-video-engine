# DECODER SPEC — decoder-first contract + conformance suite (v1, 2026-09-26)

> Directive: decoder is the first thing to implement. This spec fixes the trait, the
> semantics, and the conformance suite that decides whether an implementation is real.
> Parent: ADR-004 (as amended by ARCHITECTURE_AUDIT #1); frame type: FRAME_CONTRACT.

## 1. Trait (session state machine — not a bag of methods)

```rust
pub trait Decoder: Send {
    /// open + probe capabilities; may reject unsupported configs with typed error
    fn open(asset: &AssetRef, stream: StreamId, cfg: DecodeConfig) -> Result<Self, DecodeError>;

    /// what this instance can actually do (declared honesty — see §3)
    fn capabilities(&self) -> &DecoderCaps;

    /// seek: Exact decodes forward from the ≤ pts keyframe and drops until exact;
    /// Snap lands on the keyframe (copy-route path). Returns the landed pts.
    fn seek(&mut self, pts: Rational, mode: SeekMode) -> Result<Rational, DecodeError>;

    /// deliver the next frame at/after the internal cursor (pull model; adapters
    /// with callback APIs adapt via internal queue — ADR-004 risk mitigation)
    fn next(&mut self) -> Result<Option<FrameEnvelope>, DecodeError>; // None = EOS

    /// reset internal state; next() continues from the last seek/position
    fn flush(&mut self);

    /// cooperative cancel: stop at the next safe point; session unusable after
    fn cancel(&mut self);
}
```

- Pull model is the trait's shape; callback-based backends (WebCodecs) buffer internally
  and surface via `next()` — the impedance mismatch is contained inside adapters.
- All times are Rational; adapter-internal platform integers convert exactly (FRAME_CONTRACT §3).

## 2. Mandatory semantics (each is a conformance test)

| ID | Semantic | Test sketch (committed tiny media) |
|---|---|---|
| D-1 | PTS exactness: delivered pts == container pts exactly (rational compare) | decode all frames of an 8s/24fps + a 29.97 clip; compare against ffprobe -show_frames (precomputed table committed) |
| D-2 | Duration exactness: per-frame duration sums == stream duration | VFR sample included; sum must equal exactly |
| D-3 | Keyframe metadata: `keyframe` flag matches the container/index truth 100% | compare against ffprobe key frames + skip-frame decode census |
| D-4 | Seek Exact: seek(t) then next() == sequential decode to t (frame-identical bytes) | for N timestamps incl. exact frame boundaries and keyframe+1 |
| D-5 | Seek Snap: seek(t, Snap) lands on ≤ t keyframe; returned pts is that keyframe's | E-007's 1.5s→0.0s trap encoded as a positive test |
| D-6 | Flush: seek→flush→next == seek→next (no stale state) | after mid-GOP seeks |
| D-7 | Cancel: cancel mid-decode returns; no further frames; resources freed (pool debug asserts) | race-harness with seeds |
| D-8 | Error model: truncated/corrupt/tiny inputs → typed Corrupt/Unreadable, never panic, never silent skip | fuzz-lite corpus committed |
| D-9 | SW fallback: if hw requested but unavailable → typed Unsupported, caller chooses SW; never a hidden downgrade | capability table check |
| D-10 | Color tags: all five tags present on every video frame; unknown-as-value allowed | FRAME_CONTRACT §4 rule enforced by type |
| D-11 | Memory honesty: SW adapter emits only Cpu/Packed; caps declare it | §3 rule |
| D-12 | VFR safety: pts sequence need not be uniform; nothing may assume CFR | VFR sample (screen recording) committed |

## 3. Capability report (declared, not discovered by crashing)

```rust
pub struct DecoderCaps {
    pub hw: Option<HwInfo>,            // None for SW builds
    pub seek_modes: Vec<SeekMode>,     // Exact always; Snap if keyframe index available
    pub memory_outputs: Vec<MemoryKind>, // Cpu always for SW; Gpu variants per backend
    pub pixel_formats: Vec<PixelFormat>,
    pub max_bit_depth: BitDepth,
    pub threaded: bool,                // internal threading (FFmpeg thread_count)
}
```

## 4. FFmpeg-SW adapter notes (v1 reference implementation)

- Linkage: libavformat/libavcodec/libavutil/swscale, LGPL-only build profile (doc 35);
  **no other crate may link libav*** (CI link-check — SECURITY/DEPENDENCY audit rules).
- Packet reading vs frame decode split: the adapter keeps demux+decode in one session for
  v1; ove-media owns the static probe/keyframe index (built once at import, cached by
  content hash — the asset is the cache key, per ove-project §5).
- Threaded decode allowed (thread_count caps documented); determinism note: decode output
  must be bit-identical regardless of thread count for the conformance corpus (D-1
  compares exact pts; bit-exactness asserted on the golden set).
- Scaling/pixel-format conversion is OUT of the decoder (swscale used by render pipeline
  separately when needed) — decode delivers native format (D-10 honesty).

## 5. What "decoder done" means

1. All conformance tests D-1..D-12 green on the committed corpus (CI-runnable, software).
2. No libav* symbols outside ove-decode.
3. Golden decode tables committed (pts lists + hashes) so regressions are diffs, not vibes.
4. FRAME_CONTRACT §7 checklist passes review on the adapter code.
