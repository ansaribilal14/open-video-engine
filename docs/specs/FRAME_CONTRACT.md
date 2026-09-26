# FRAME CONTRACT (v1, 2026-09-26)

> The normative definition of "a frame" inside OVE. Mandated by the directive: the frame
> abstraction must explicitly cover timestamps/duration/size/pixel format/color
> primaries/transfer/matrix/range/bit depth/memory location/ownership/lifetime/keyframe/
> backend identity — it may NOT be designed around CPU byte buffers only, nor around
> WebGPU textures only. Parent: ADR-004/006; consumer: ove-decode, ove-render, ove-encode.

## 1. FrameEnvelope (the only frame type that crosses crate boundaries)

```rust
pub struct FrameEnvelope {
    /// Identity & scheduling
    pub pts:      Rational,   // EXACT presentation timestamp, source-rate axis (ADR-007)
    pub duration: Rational,   // EXACT; VFR-safe (never inferred from a frame rate)
    pub timeline_pts: Option<Rational>, // set by remap when on the project axis
    pub stream_id: StreamId,  // stable asset+stream identity (content-hash based)
    pub kind:     FrameKind,  // Video | Audio { sample_rate, channels } | Metadata

    /// Geometry & format (video)
    pub width: u32, pub height: u32,            // coded + displayed pair handled by adapter
    pub pixel_format: PixelFormat,               // enum, not fourcc string
    pub bit_depth: BitDepth,                     // 8 | 10 | 12 | 16(F) …

    /// Color (all five tags REQUIRED for video — unknown is a value, not an absence)
    pub color: ColorTags {
        primaries: Primaries,      // e.g. Bt709, Bt601_525, Bt2020, DciP3
        transfer:  Transfer,       // e.g. Bt709, Srgb, Pq, Hlg
        matrix:    MatrixCoeffs,   // e.g. Bt709, Bt601, Bt2020Ncl
        range:     Range,          // Limited | Full
        chroma_loc: Option<ChromaLoc>,
    },

    /// Memory & lifecycle
    pub memory: FrameMemory,   // where the pixels/samples live (below)
    pub keyframe: bool,        // decode entry point flag (E-007/E-007b rely on it)
    pub backend: BackendId,    // which adapter produced it (FFmpegSw, MediaCodec, WebCodecs…)
    pub generation: u64,       // pool generation — stale-detection (see §5)
}
```

## 2. FrameMemory (the CPU/GPU duality, explicit)

```rust
pub enum FrameMemory {
    /// CPU bytes. Planar/interleaved layout described by pixel_format.
    /// Buffer is owned per §5. Strides explicit — no assumptions.
    Cpu(FrameBytes),
    /// GPU-resident, engine-neutral descriptor. The actual resource lives in the
    /// backend's world; core only routes it. Two-level GPU abstraction
    /// (ARCHITECTURE_AUDIT #4): the core never names wgpu/MediaCodec types.
    Gpu(GpuFrameRef {
        handle: GpuHandle,          // opaque, backend-interpreted
        upload_kind: GpuUploadKind, // ImportedExternalTexture | AHardwareBuffer |
                                    // MappedTexture | ZeroCopyPlatformSurface
        exportable: bool,           // can the encoder consume without readback?
    }),
    /// Undecoded/compressed passthrough (for StreamCopy routes).
    Packed(PackedSlice),            // demuxed packet, still muxable
}
```

Rules:
1. **Copy-path is the default v1**: every adapter can deliver `Cpu`; GPU paths are
   opt-in per backend capability (ADR-004 risk note, now normative).
2. `Gpu` frames declare `exportable`; if false, the export planner must readback or
   re-route (RENDER/ENCODER specs reference this field).
3. `Packed` exists so stream-copy is a first-class route, not a hack (ADR-005 amendment).

## 3. Timestamp rules (binding, from ADR-007)

1. pts/duration are exact rationals; adapters converting platform integers (FFmpeg
   pts*time_base, MediaCodec bufferInfo.presentationTimeUs µs, WebCodecs timestamp µs)
   must construct rationals exactly — integer→rational is exact; rational→integer uses
   floor_div semantics with the rate, never fp.
2. Audio: duration = samples/rate exactly; sample-count is authoritative for audio.
3. No frame may be scheduled by comparing fp seconds anywhere in the engine.

## 4. Color rules (binding)

1. All five tags travel with every video frame from decoder onward; "unknown" is an
   explicit enum value; ove-render never guesses silently (doc 22).
2. Conversion to the working color space happens once, in ove-render, driven by the
   project's working-space declaration — not scattered across adapters.
3. Golden frames record the color tags used, so hash failures are diagnosable.

## 5. Ownership & lifetime (the part history shows everyone gets wrong)

1. FrameEnvelope ownership = unique; moving is explicit; `Clone` is deep on `Cpu`,
   forbidden on `Gpu` (clone via backend `copy_frame` API instead — visible cost).
2. Pool semantics: CPU buffers come from a per-decoder pool keyed by (format, size,
   stride class); `generation` increments on pool recycle — a consumer holding a frame
   across recycle is a bug the pool can detect (debug asserts).
3. Frame lifetime ends at: consume-by-encoder, render-pass completion, or explicit drop —
   never tied to decoder session lifetime (decoders recycle pools independently).
4. Cancellation: a cancelled decode session invalidates un-delivered frames; delivered
   frames stay valid until their own drop (no use-after-cancel).

## 6. Backend identity & capability honesty

1. `BackendId` on every frame — golden tests can assert which backend produced what.
2. Adapters publish capabilities (see DECODER_SPEC §3); a frame must not violate them
   (e.g., a "SW" backend never emits `Gpu`).
3. Conformance tests compare *semantic* frame content across backends only when color
   tags match; otherwise the test asserts the tag difference explicitly (no silent
   normalization).

## 7. What this contract forbids (checklist for reviewers)

- No `Vec<u8>`-only frame params crossing crate boundaries ("what format? what color?").
- No wgpu/FFmpeg/MediaCodec types in any public signature outside their adapter crates.
- No fp anywhere in the scheduling path.
- No frame without all five color tags (unknown-as-value is fine, absence is not).
- No implicit ownership ( Rc/Arc frames on the hot path; sharing = explicit copy ).
