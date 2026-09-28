//! ove-encode — encoder + muxer + export-route abstraction (first leg).
//!
//! Design authority: ADR-005 (promoted by ADR-015), ENCODER_SPEC v1
//! (docs/specs/ENCODER_SPEC.md), FRAME_CONTRACT §3 (exact timestamps),
//! PROJECT_FORMAT_SPEC §2 (hash policy).
//!
//! Layout:
//!   * `lib.rs` (this file) — backend-neutral types: `Encoder`/`Muxer`
//!     traits, configs, caps, `EncodedPacket`, `TrackSpec`, errors, and
//!     the re-exported pure `planner` (export routes).
//!   * `planner.rs` — PURE export-route logic: keyframe-aligned stream-copy
//!     vs re-encode vs mixed segmentation (ENCODER_SPEC §3 made executable).
//!     No libav, no I/O — property-testable in isolation.
//!   * `ffmpeg` module — the FFmpeg software encoder (native mpeg4, v1),
//!     the MP4 muxer, and the packet-level stream-copy source.
//!     **This is one of the ONLY two crates in the workspace permitted to
//!     link libav\*** (DECODER_SPEC §5 + ENCODER_SPEC §5, amended by
//!     ADR-015; CI enforces via dependency + source scan of the closed
//!     adapter allowlist {ove-decode, ove-encode}).
//!
//! Honesty rules this crate enforces (never negotiable):
//!   * Every timestamp is an exact `ove_time::Rational`; conversion to any
//!     container/encoder tick axis MUST divide exactly or it is a typed
//!     error (`NonExactTimestamp`) — never silent rounding (E-002).
//!   * Unsupported codecs/configs are TYPED (`Unsupported`), never hidden
//!     downgrades (capability honesty, ENCODER_SPEC §6).
//!   * Snap decisions made by the planner are REPORTED in receipts, never
//!     silent (ENCODER_SPEC §3.1).

pub mod ffmpeg;
pub mod planner;

use ove_media::{ColorTags, PixelFormat, StreamId};
use ove_time::Rational;

// ---------------------------------------------------------------------------
// Errors (typed; D-8 class: failures are values, never panics/silent skips)
// ---------------------------------------------------------------------------

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum EncodeError {
    /// The requested codec/config is not supported by this backend.
    /// Carry the NAMED reason (review reject if empty).
    Unsupported(String),
    /// A timestamp did not land exactly on the encoder tick axis.
    NonExactTimestamp {
        pts: Rational,
        time_base: Rational,
    },
    /// Configuration rejected before any work started (odd dims for 4:2:0,
    /// zero GOP, non-positive rate, ...).
    InvalidConfig(String),
    /// Fed frame does not match the configured geometry/format.
    FrameMismatch(String),
    Io(String),
    /// Backend-internal failures that map to no cleaner variant. MUST carry
    /// context; empty Internal strings are a review reject.
    Internal(String),
}

impl std::fmt::Display for EncodeError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            EncodeError::Unsupported(d) => write!(f, "unsupported: {d}"),
            EncodeError::NonExactTimestamp { pts, time_base } => {
                write!(f, "pts {pts} not exactly representable at tb {time_base}")
            }
            EncodeError::InvalidConfig(d) => write!(f, "invalid config: {d}"),
            EncodeError::FrameMismatch(d) => write!(f, "frame mismatch: {d}"),
            EncodeError::Io(d) => write!(f, "io: {d}"),
            EncodeError::Internal(d) => write!(f, "internal: {d}"),
        }
    }
}

impl std::error::Error for EncodeError {}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum MuxError {
    /// TrackSpec rejected (unknown codec for this backend, zero timescale...).
    InvalidTrackSpec(String),
    /// A packet timestamp did not land exactly on the track timescale.
    NonExactTimestamp {
        pts: Rational,
        timescale: i64,
    },
    /// Packet references an unknown track / wrong ordering contract.
    UnknownTrack(StreamId),
    Io(String),
    /// Backend-internal failures; MUST carry context.
    Internal(String),
}

impl std::fmt::Display for MuxError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            MuxError::InvalidTrackSpec(d) => write!(f, "invalid track spec: {d}"),
            MuxError::NonExactTimestamp { pts, timescale } => {
                write!(
                    f,
                    "pts {pts} not exactly representable at timescale {timescale}"
                )
            }
            MuxError::UnknownTrack(s) => write!(f, "unknown track {s:?}"),
            MuxError::Io(d) => write!(f, "io: {d}"),
            MuxError::Internal(d) => write!(f, "internal: {d}"),
        }
    }
}

impl std::error::Error for MuxError {}

// ---------------------------------------------------------------------------
// Codecs & configs (v1: backend-neutral names, adapter maps to libav ids)
// ---------------------------------------------------------------------------

/// Video codecs the engine names. v1 shipped adapter support: `Mpeg4`
/// (libavcodec-native, LGPL, dependency-free, deterministic). `OpenH264` and
/// `SvtAv1` are DECLARED targets (ADR-005) that the v1 software adapter
/// reports as `Unsupported` until their adapter legs land (capability
/// honesty — a named gap, not a hidden failure).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum VideoCodec {
    Mpeg4,
    OpenH264,
    SvtAv1,
}

/// Audio codecs named by the engine. `Aac` (libavcodec-native) is used by
/// the W3 conformance suite only via stream-copy passthrough; the AAC
/// *encoder* leg lands with the audio wave (W7).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum AudioCodec {
    Aac,
}

/// Rate control (ENCODER_SPEC §2). v1: CBR-style bitrate and CRF-style
/// quantizer; the adapter maps honestly per codec (mpeg4: b / qscale).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RateControl {
    Cbr { bitrate: u32 },
    Crf { quality: u8 },
}

/// Video encoder profile. `frame_rate` is the exact CFR axis; frames whose
/// pts do not land on it are a typed error (E-002 exactness).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct VideoProfile {
    pub width: u32,
    pub height: u32,
    pub frame_rate: Rational,
    pub pixel_format: PixelFormat,
    pub color: ColorTags,
    /// GOP length in frames (keyint); 1 = all-intra.
    pub gop: u32,
    /// Bit-exact mode: strips encoder identity metadata (encoder settings
    /// strings, muxer ©too tags) so equal configs produce equal bytes.
    pub bitexact: bool,
}

impl VideoProfile {
    pub fn validate(&self) -> Result<(), EncodeError> {
        if self.width == 0 || self.height == 0 {
            return Err(EncodeError::InvalidConfig("zero dimension".into()));
        }
        if self.width > i32::MAX as u32 || self.height > i32::MAX as u32 {
            return Err(EncodeError::InvalidConfig("dimension exceeds i32".into()));
        }
        if self.frame_rate.num() <= 0 {
            return Err(EncodeError::InvalidConfig("non-positive frame rate".into()));
        }
        if self.gop == 0 {
            return Err(EncodeError::InvalidConfig("zero GOP".into()));
        }
        // 4:2:0 chroma requires even geometry (swscale does NOT fix caller
        // mistakes silently — that would be a hidden resample).
        if matches!(self.pixel_format, PixelFormat::Yuv420p | PixelFormat::Nv12)
            && (!self.width.is_multiple_of(2) || !self.height.is_multiple_of(2))
        {
            return Err(EncodeError::InvalidConfig(
                "4:2:0 pixel format requires even width and height".into(),
            ));
        }
        Ok(())
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct EncoderConfig {
    pub codec: VideoCodec,
    pub profile: VideoProfile,
    pub rate: RateControl,
}

// ---------------------------------------------------------------------------
// Encoded packets & muxer inputs
// ---------------------------------------------------------------------------

/// One compressed packet on the encoder axis. The checkpointing unit
/// (ENCODER_SPEC §2): {stream_id, pts, duration, keyframe, byte_range_hint}
/// feed the segment-sequential checkpoint manifests (Android FGS resume,
/// headless retries; E-6 lands at segment granularity in a later wave).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct EncodedPacket {
    pub stream_id: StreamId,
    /// Presentation timestamp on the encoder/source axis (exact seconds).
    pub pts: Rational,
    /// Decode timestamp; `None` = dts == pts (no B-frame reordering).
    pub dts: Option<Rational>,
    /// Exact packet duration (`None` = codec-inherent single-tick frame).
    pub duration: Option<Rational>,
    pub keyframe: bool,
    /// Source-file byte position this packet was copied from (stream-copy
    /// provenance for checkpoint manifests); `None` for freshly encoded data.
    pub byte_range_hint: Option<u64>,
    pub data: Vec<u8>,
}

/// Muxer track codec names (backend maps to container codec tags).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum TrackCodec {
    Mpeg4,
    Aac,
}

/// Per-track muxing spec. The timescale is the track's EXACT rational axis
/// (ENCODER_SPEC §5: video = frame-rate-derived, audio = sample rate); the
/// muxer rejects any packet whose pts does not divide exactly into it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TrackSpec {
    pub stream_id: StreamId,
    pub codec: TrackCodec,
    pub timescale: i64,
    pub kind: TrackKind,
    /// Codec global header (esds AudioSpecificConfig / mpeg4 extradata).
    pub extradata: Vec<u8>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum TrackKind {
    Video {
        width: u32,
        height: u32,
        pixel_format: PixelFormat,
        color: ColorTags,
        frame_rate: Rational,
    },
    Audio {
        sample_rate: u32,
        channels: u32,
        /// Honest string id of the sample format ("fltp", "s16", ...;
        /// same names as ove-media AudioDetails). Reconstructed into the
        /// container's codec parameters by the muxer adapter.
        sample_format: String,
    },
}

impl TrackSpec {
    /// Exact minimal timescale for a video track at `frame_rate`:
    /// T = num / gcd(num, den) — every frame lands on an integer tick.
    /// (24/1 → 24; 30000/1001 → 30000 with 1001-tick frame durations.)
    pub fn video_timescale(frame_rate: Rational) -> Result<i64, MuxError> {
        if frame_rate.num() <= 0 {
            return Err(MuxError::InvalidTrackSpec("non-positive frame rate".into()));
        }
        let (n, d) = (frame_rate.num(), frame_rate.den());
        let g = gcd(n, d);
        let t = n / g;
        if t == 0 {
            return Err(MuxError::InvalidTrackSpec("zero timescale".into()));
        }
        Ok(t)
    }

    /// Ticks for an exact rational pts on this track's timescale axis.
    /// Err when not exactly representable (E-002; never rounds).
    pub fn ticks_for(&self, pts: Rational) -> Result<i64, MuxError> {
        // ticks = pts * timescale  →  (num * T) / den must divide exactly.
        let numer = (pts.num() as i128) * (self.timescale as i128);
        let den = pts.den() as i128;
        if numer % den != 0 {
            return Err(MuxError::NonExactTimestamp {
                pts,
                timescale: self.timescale,
            });
        }
        i64::try_from(numer / den).map_err(|_| MuxError::Internal("tick count exceeds i64".into()))
    }
}

fn gcd(a: i64, b: i64) -> i64 {
    let (mut a, mut b) = (a.unsigned_abs(), b.unsigned_abs());
    while b != 0 {
        let t = a % b;
        a = b;
        b = t;
    }
    a as i64
}

/// Output sink (v1: file path; network/pipe sinks are adapter-stage).
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum OutputSink {
    File(std::path::PathBuf),
}

/// Output container. v1 MP4 options are explicit (ENCODER_SPEC §5):
/// faststart (moov first) and bitexact (strip muxer/encoder identity
/// metadata — reproducible exports and stable golden bytes).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Container {
    Mp4 { faststart: bool, bitexact: bool },
}

// ---------------------------------------------------------------------------
// Traits (ENCODER_SPEC §1)
// ---------------------------------------------------------------------------

/// Declared encoder capabilities (ENCODER_SPEC §6 capability honesty).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct EncoderCaps {
    pub codecs: Vec<VideoCodec>,
    /// Input pixel formats accepted WITHOUT conversion (converted inputs are
    /// declared separately by the adapter — conversion is visible, never free).
    pub memory_inputs: Vec<PixelFormat>,
    /// Input formats the adapter converts internally (e.g. RGBA → YUV420P
    /// via swscale inside the FFmpeg adapter).
    pub converted_inputs: Vec<PixelFormat>,
    pub threaded: bool,
    /// Deterministic given pinned libav versions (recorded in
    /// `versions`); hardware encoders must report false.
    pub deterministic: bool,
    /// libav version identity used for golden byte-hash keying (E-5).
    pub versions: String,
}

/// The encoder session (ENCODER_SPEC §1): configure → feed frames → drain.
/// Pull-model friendly; adapter-internal buffering is contained inside the
/// adapter (ADR-004 risk mitigation pattern, mirrored from decode).
pub trait Encoder: Send {
    /// Static constructor: caps-checked configuration.
    fn configure(cfg: EncoderConfig) -> Result<Self, EncodeError>
    where
        Self: Sized;

    fn capabilities(&self) -> &EncoderCaps;

    /// The muxing spec for this encoder's output track. Available after
    /// `configure` (global header/extradata is produced by encoder init).
    /// ENCODER_SPEC §1 extension (ADR-015): needed because MP4 muxing
    /// requires header-before-packets; recorded in the spec annotation.
    fn track_spec(&self) -> Result<TrackSpec, EncodeError>;

    /// Feed one frame (FRAME_CONTRACT §2 envelope). Exact-timestamp contract:
    /// `frame.pts` must land exactly on the configured frame-rate axis.
    fn feed(&mut self, frame: ove_media::FrameEnvelope) -> Result<(), EncodeError>;

    /// Flush: deliver all buffered packets then signal EOS. After `drain`
    /// the session is finished (one-shot per ENCODER_SPEC §1).
    fn drain(&mut self) -> Result<Vec<EncodedPacket>, EncodeError>;
}

/// The muxer session (ENCODER_SPEC §1/§5): open → write packets (any order
/// the muxer accepts) → finalize. `finalize` reports the output record that
/// feeds E-2/E-5 and the project-format render records.
pub trait Muxer: Send {
    fn open(
        sink: OutputSink,
        container: Container,
        tracks: Vec<TrackSpec>,
    ) -> Result<Self, MuxError>
    where
        Self: Sized;

    fn write(&mut self, pkt: EncodedPacket) -> Result<(), MuxError>;

    /// Close the container and report the output record. Consumes self.
    fn finalize(self) -> Result<OutputInfo, MuxError>
    where
        Self: Sized;
}

/// Output record (ENCODER_SPEC §5): per-track tables + artifact integrity
/// fingerprint. NOTE the hash policy (PROJECT_FORMAT_SPEC §2): this sha256
/// is an OUTPUT-ARTIFACT INTEGRITY FINGERPRINT for render records and golden
/// comparison — asset identity stays BLAKE3 (ove-media).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct OutputInfo {
    pub path: std::path::PathBuf,
    pub tracks: Vec<OutputTrackInfo>,
    pub file_sha256: String,
    pub file_size: u64,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct OutputTrackInfo {
    pub stream_id: StreamId,
    pub timescale: i64,
    /// Container-reported stream duration (exact rational seconds).
    pub duration: Option<Rational>,
    /// Container-reported frame/packet count (E-1 checks it against the plan).
    pub nb_frames: Option<u64>,
}
