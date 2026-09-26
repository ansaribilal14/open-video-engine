# ENCODER SPEC — encoder + muxer + stream-copy route (v1, 2026-09-26)

> Parent: ADR-005 as amended (ARCHITECTURE_AUDIT #2): muxing is separated from encoding;
> stream-copy is an export *route*, not an "encoder mode". Directive: exported MP4 must be
> inspectable with ffprobe; golden outputs committed.

## 1. Types

```rust
pub trait Encoder: Send {
    fn configure(cfg: EncoderConfig) -> Result<Self, EncodeError>;  // caps-checked
    fn feed(&mut self, frame: FrameEnvelope) -> Result<(), EncodeError>;
    fn drain(&mut self) -> Result<Vec<EncodedPacket>, EncodeError>; // flush + EOS
    fn capabilities(&self) -> &EncoderCaps;
}

pub trait Muxer: Send {
    fn open(sink: OutputSink, container: Container, tracks: Vec<TrackSpec>) -> Result<Self, MuxError>;
    fn write(&mut self, pkt: EncodedPacket) -> Result<(), MuxError>;  // any track, any order muxer accepts
    fn finalize(self) -> Result<OutputInfo, MuxError>;                // duration, hashes, tags
}

/// The export planner chooses per segment:
pub enum ExportRoute {
    ReEncode { encoder: EncoderImpl, profile: VideoProfile },
    StreamCopy { /* demux-remux; keyframe-index-driven */ },
    Mixed(Vec<(TimelineRange, ExportRoute)>), // smart-render segmentation
}
```

## 2. EncoderConfig essentials (v1)

- codec: H264(software openh264 or FFmpeg-lgpl encoder) | AV1(SVT-AV1) — x264 excluded
  from distributed core (GPL; ADR-005). Hardware (MediaCodec/WebCodecs/NVENC) = adapter
  concern, capability-gated, later wave.
- rate control: CBR/VCR/CQ + maxrate/bufsize; GOP: keyint/minkeyint; profile/level.
- timestamps in rationals → muxer converts exactly (FRAME_CONTRACT §3); audio sample
  counts authoritative (samples/rate exact duration).
- Checkpointing hooks: `EncodedPacket` carries {stream_id, pts, duration, keyframe,
  byte_range_hint} — the unit of segment-sequential render for Android FGS resume
  (R-07) and headless retries (doc 30). Checkpoint file = manifest of finalized segments.

## 3. StreamCopy route rules (the E-007/E-007b lessons, made normative)

1. Cut boundaries MUST resolve to keyframes unless the profile explicitly allows
   re-encode at boundaries; snapping is reported, never silent (receipt lists snaps).
2. Copy route consumes `FrameMemory::Packed` packets; no decode is invoked; color/pts
   tags pass through untouched.
3. Cross-boundary A/V sync: audio may not align to video keyframes — the planner must
   either cut audio exactly and re-encode the audio segment (AAC) or use mixed routes
   (video copy + audio re-encode). v1 default: video copy + audio re-encode at segment
   seams (E-007b showed video-only copy is duration-exact; audio seam strategy is the
   recorded open risk — ADR-005's cross-encoder stitch note).
4. Golden content-correctness: a copied segment's bytes == demux of that GOP range
   (E-007's 72/72 pattern as a permanent test).

## 4. Output verification (the "ffprobe must be able to inspect it" gate)

| ID | Test |
|---|---|
| E-1 | ffprobe -show_streams/-show_frames parses output; nb_frames/frame count matches plan exactly |
| E-2 | Duration exactness: container duration == project render span ± 0 frames (video), ± 0 samples (audio) |
| E-3 | Timestamp monotonicity + start offsets per track (edit lists / negative-pts policy documented) |
| E-4 | Codec parameters round-trip: profile/level/colour tags as configured |
| E-5 | Golden hashes: committed tiny outputs (8s/24fps + 29.97 + VFR + audio) with sha256 + ffprobe JSON dumps |
| E-6 | Kill-resume: kill during export → checkpoint file → resume → final output == no-kill output (hash) — **v1 at segment granularity** |
| E-7 | Mixed route: smart-render segmentation on a 3-clip timeline produces exact expected segment boundaries (E-007b 6.8–11.9× economics as perf smoke, separate from correctness) |

## 5. Muxer rules

- MP4 (v1): faststart option, per-track timescales chosen to keep rationals exact
  (video: rate num/den; audio: sample rate) — no fp anywhere; edit-list policy explicit.
- `finalize()` reports {duration, track tables, sha256} — these feed E-2/E-5 and the
  project-format render records.
- WebM/mkv + browser muxers (mediabunny) are adapter-stage, not v1 core.

## 6. Capability honesty

- EncoderCaps: {codecs, profiles, rate_controls, hw: Option<HwInfo>, memory_inputs
  (Cpu required; Gpu exportable per FRAME_CONTRACT §2), threaded, deterministic: bool}.
- Deterministic flag: SVT-AV1/FFmpeg SW = deterministic given pinned versions (record
  versions in output metadata); hw encoders = NOT deterministic → golden-hash tests run
  on SW only; hw tests assert structure (E-1..E-4) but not bytes.
