//! ove-engine — the headless engine session (WAVE 5; ENGINE_BUILD_PLAN
//! wave 5: MEDIA+TIMELINE+PROJECT+RENDER+EXPORT cooperating).
//!
//! Architecture position (ADR-017): this crate is the INTEGRATION layer —
//! the only non-adapter crate that depends on the adapter crates (ove-decode,
//! ove-encode, which hold the libav linkage). The pure core (ove-time/
//! timeline/media/render/project) stays adapter-free; the future platform
//! shells (desktop/android/browser) sit at this same layer, never lower.
//!
//! Session shape (single-writer, v1):
//!   * a [`Project`] owns persistence (write-through; crash-safe per ADR-016);
//!   * probe records (`ProbeInfo`) are cached per asset for the session and
//!     persisted as sidecars at import;
//!   * timeline edits are ove-timeline Commands with EXPLICIT ids (E-012);
//!   * the render path: timeline walk → ADR-013 seam mapping → exact
//!     decoder seek (D-4/D-5) → YUV→RGBA boundary conversion (the engine's
//!     declared single source→working-space conversion) → ove-render
//!     compile+execute → RGBA output;
//!   * the export paths: stream-copy (planner + packet passthrough) and
//!     re-encode (render → RGBA → encoder-declared swscale → MP4).

use std::collections::BTreeMap;
use std::path::Path;

use ove_decode::ffmpeg::FfmpegSwDecoder;
use ove_decode::{DecodeConfig, Decoder, SeekMode};
use ove_encode::ffmpeg::{FfmpegAacEncoder, FfmpegCopySource, FfmpegMuxer, FfmpegSwEncoder};
use ove_encode::{
    AudioEncoder, Container, Encoder, EncoderConfig, Muxer, OutputInfo, OutputSink, RateControl,
    VideoCodec, VideoProfile,
};
use ove_media::{
    AssetRef, BackendId, ColorTags, FrameEnvelope, FrameMemory, MatrixCoeffs, PixelFormat,
    Primaries, ProbeInfo, Range, Transfer,
};
use ove_project::{Owner, Project, ProjectError};
use ove_render::{
    compile_frame, FrameSource, Placement, RenderInput, SoftwareRenderer, TrackInput,
};
use ove_time::Rational;
use ove_timeline::mapping::ClipWindow;
use ove_timeline::{Clip, Command, TrackId, TrackKind};

// ---------------------------------------------------------------------------
// Errors
// ---------------------------------------------------------------------------

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum EngineError {
    Project(ProjectError),
    /// Media rejected at import (unreadable, no video stream, ...). Named.
    Import(String),
    /// Unknown asset id/hash for this session.
    UnknownAsset(String),
    Timeline(ove_timeline::TimelineError),
    Render(ove_render::RenderError),
    Compile(ove_render::CompileError),
    Encode(ove_encode::EncodeError),
    Mux(ove_encode::MuxError),
    /// The requested timeline time maps outside the source (ADR-013 seam).
    Seam(ove_timeline::mapping::SeamError),
    /// No placement covers the requested output time.
    NoPlacement {
        at: Rational,
    },
    /// The imported sources carry no audio stream (W7 export with audio).
    NoAudioStream,
    /// A requested audio cut does not land on a sample boundary at the
    /// source rate (sample-exact contract, ADR-018).
    NonExactSampleCut {
        at: Rational,
        rate: u32,
    },
    /// Retimed clips are video-only in v1; audio retiming (resampling) is a
    /// named gap (ADR-018).
    AudioRetimeUnsupported {
        speed: Rational,
    },
    /// A keyframed geometry value rounds outside i32 pixel range at render
    /// time (ADR-019: the document stores exact rationals; the renderer
    /// consumes i32 translation). Fail-fast, never saturate.
    KeyframeValueOutOfRange {
        clip_id: u64,
        property: String,
        value: Rational,
    },
    Internal(String),
}

impl std::fmt::Display for EngineError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            EngineError::Project(e) => write!(f, "project: {e}"),
            EngineError::Import(d) => write!(f, "import: {d}"),
            EngineError::UnknownAsset(d) => write!(f, "unknown asset: {d}"),
            EngineError::Timeline(e) => write!(f, "timeline: {e:?}"),
            EngineError::Render(e) => write!(f, "render: {e:?}"),
            EngineError::Compile(e) => write!(f, "compile: {e:?}"),
            EngineError::Encode(e) => write!(f, "encode: {e:?}"),
            EngineError::Mux(e) => write!(f, "mux: {e:?}"),
            EngineError::Seam(e) => write!(f, "seam: {e:?}"),
            EngineError::NoPlacement { at } => write!(f, "no placement covers t={at}"),
            EngineError::NoAudioStream => write!(f, "no audio stream in the imported sources"),
            EngineError::NonExactSampleCut { at, rate } => {
                write!(f, "audio cut t={at} is not sample-exact at {rate} Hz")
            }
            EngineError::AudioRetimeUnsupported { speed } => {
                write!(f, "audio retime x{speed} unsupported in v1 (named gap)")
            }
            EngineError::KeyframeValueOutOfRange {
                clip_id,
                property,
                value,
            } => write!(
                f,
                "clip {clip_id} keyframed {property} value {value} leaves i32 pixel range"
            ),
            EngineError::Internal(d) => write!(f, "internal: {d}"),
        }
    }
}

impl std::error::Error for EngineError {}

impl From<ProjectError> for EngineError {
    fn from(e: ProjectError) -> Self {
        EngineError::Project(e)
    }
}

// ---------------------------------------------------------------------------
// Engine session
// ---------------------------------------------------------------------------

/// One imported media source, keyed by content hash.
#[derive(Clone, Debug)]
pub struct SourceMedia {
    pub hash: ove_media::ContentHash,
    pub probe: ProbeInfo,
}

/// Assembled timeline audio (W7): the decode leg's canonical surface —
/// planar f32, one plane per channel, exactly `samples` samples per plane.
#[derive(Clone, Debug)]
pub struct AudioAssembly {
    pub planes: Vec<Vec<f32>>,
    pub sample_rate: u32,
    pub channels: u32,
    pub samples: u64,
}

pub struct Engine {
    project: Project,
    sources: BTreeMap<String, SourceMedia>,
}

impl Engine {
    // -- lifecycle ----------------------------------------------------------

    pub fn create(dir: &Path, tick_axis: (i64, i64)) -> Result<Self, EngineError> {
        Ok(Engine {
            project: Project::create(dir, tick_axis)?,
            sources: BTreeMap::new(),
        })
    }

    pub fn open(dir: &Path) -> Result<Self, EngineError> {
        let project = Project::open(dir)?;
        // re-hydrate the probe cache from sidecars (hash-addressed)
        let mut sources = BTreeMap::new();
        for entry in project.assets() {
            let probe_path = project.dir().join(
                entry
                    .probe
                    .clone()
                    .unwrap_or_else(|| format!("assets/{}/probe.json", entry.content_hash)),
            );
            if let Ok(raw) = std::fs::read_to_string(&probe_path) {
                if let Ok(probe) = serde_json::from_str::<serde_json::Value>(&raw) {
                    if let Ok(probe) = serde_json::from_value::<ProbeInfo>(probe) {
                        // round-trip the digest (from_hex), never re-hash it
                        if let Some(hash) = ove_media::ContentHash::from_hex(&entry.content_hash) {
                            sources.insert(entry.content_hash.clone(), SourceMedia { hash, probe });
                        }
                    }
                }
            }
        }
        Ok(Engine { project, sources })
    }

    pub fn dir(&self) -> &Path {
        self.project.dir()
    }

    pub fn state_hash(&self) -> String {
        self.project.state_hash()
    }

    pub fn uuid(&self) -> &str {
        self.project.uuid()
    }

    pub fn project(&self) -> &Project {
        &self.project
    }

    // -- import ---------------------------------------------------------------

    /// Import media: full probe (keyframe index + VFR) via the decode
    /// adapter, content-hash-addressed copy into the project, probe sidecar.
    pub fn import_media(&mut self, path: &Path) -> Result<String, EngineError> {
        let asset = AssetRef::from_path(path).map_err(|e| EngineError::Import(format!("{e}")))?;
        let probe = ove_decode::ffmpeg::FfmpegProbe
            .probe_full(&asset)
            .map_err(|e| EngineError::Import(format!("probe: {e}")))?;
        let probe_json = serde_json::to_value(&probe)
            .map_err(|e| EngineError::Import(format!("probe serialization: {e}")))?;
        let hash = self.project.import_asset(path, Some(&probe_json))?;
        let hex = hash.hex();
        self.sources
            .insert(hex.clone(), SourceMedia { hash, probe });
        Ok(hex)
    }

    pub fn source(&self, hex: &str) -> Option<&SourceMedia> {
        self.sources.get(hex)
    }

    // -- timeline commands (explicit ids; E-012) ------------------------------

    pub fn add_track(&mut self, id: TrackId, kind: TrackKind) -> Result<(), EngineError> {
        self.project
            .add_track(id, kind)
            .map_err(EngineError::Project)
    }

    /// Allocate a fresh clip id and INSERT it appended to the track.
    /// Returns the id used (allocation happens BEFORE command construction
    /// — E-012). v1: per-clip asset binding is a session note (the first
    /// imported source renders — ADR-017); the binding rides the log at W6.
    pub fn add_clip(
        &mut self,
        track: TrackId,
        asset_hex: &str,
        duration: Rational,
        source_in: Rational,
    ) -> Result<u64, EngineError> {
        if !self.sources.contains_key(asset_hex) {
            return Err(EngineError::UnknownAsset(asset_hex.to_string()));
        }
        let len = self
            .project
            .timeline()
            .track_len(track)
            .map_err(EngineError::Timeline)?;
        let clip_id = self.project.timeline_mut().alloc_id();
        self.project
            .execute_insert_asset(
                track,
                len,
                Clip::new(clip_id, duration, source_in),
                asset_hex.to_string(),
                Owner::Human,
            )
            .map_err(EngineError::Project)?;
        Ok(clip_id)
    }

    pub fn split(&mut self, track: TrackId, id: u64, at: Rational) -> Result<u64, EngineError> {
        let new_id = self.project.timeline_mut().alloc_id();
        self.execute(Command::Split {
            track,
            id,
            at,
            new_id,
        })?;
        Ok(new_id)
    }

    pub fn resize(
        &mut self,
        track: TrackId,
        id: u64,
        duration: Rational,
    ) -> Result<(), EngineError> {
        self.execute(Command::Resize {
            track,
            id,
            duration,
        })
    }

    pub fn move_clip(
        &mut self,
        id: u64,
        from_track: TrackId,
        to_track: TrackId,
        to_index: usize,
    ) -> Result<(), EngineError> {
        self.execute(Command::Move {
            id,
            from_track,
            to_track,
            to_index,
        })
    }

    pub fn remove(&mut self, track: TrackId, id: u64) -> Result<(), EngineError> {
        self.execute(Command::Remove { track, id })
    }

    pub fn execute(&mut self, cmd: Command) -> Result<(), EngineError> {
        self.project
            .execute(cmd, Owner::Human)
            .map_err(EngineError::Project)
    }

    pub fn undo(&mut self) -> Result<bool, EngineError> {
        self.project
            .undo(Owner::Human)
            .map_err(EngineError::Project)
    }

    pub fn redo(&mut self) -> Result<bool, EngineError> {
        self.project
            .redo(Owner::Human)
            .map_err(EngineError::Project)
    }

    // -- render path ----------------------------------------------------------

    /// Build the compiler input from the live timeline + probe records,
    /// with keyframe animation EVALUATED at timeline time `t` (WAVE 8,
    /// ADR-019): each placement's opacity/x/y key tracks are evaluated at
    /// the clip's LOCAL time (t − start, speed 1 in v1); properties without
    /// animation fall back to the static defaults (alpha 1, offset 0).
    /// Track order = TrackId ascending (bottom-up layer order).
    pub fn build_render_input(
        &self,
        output: &ove_render::OutputSpec,
        t: Rational,
    ) -> Result<RenderInput, EngineError> {
        let mut tracks = Vec::new();
        for tid in self.project.timeline().track_ids() {
            let mut placements = Vec::new();
            let track = self
                .project
                .timeline()
                .track_ref(tid)
                .map_err(EngineError::Timeline)?;
            // walk is a void visitor — evaluation errors are captured and
            // surfaced after the walk (fail-fast on the first bad value).
            let mut eval_err: Option<EngineError> = None;
            track.walk(&mut |pos, start, clip: &Clip| {
                if eval_err.is_some() {
                    return;
                }
                // resolve the placement's source by clip id → asset (v1:
                // every clip maps to the session's FIRST imported source —
                // the per-clip asset binding rides the log at W6; recorded
                // in ADR-017 as the v1 single-source note)
                let Some((hex, media)) = self.sources.iter().next() else {
                    return;
                };
                let Some(video) = media
                    .probe
                    .streams
                    .iter()
                    .find(|s| s.kind == ove_media::StreamKind::Video)
                else {
                    return;
                };
                let window = ClipWindow::from_clip(clip, start);
                // ADR-019 evaluation: exact rational at the frame's local
                // time, deterministic round-half-up to i32 for geometry.
                let local = t.sub(start);
                let props = &clip.properties;
                let alpha = props.opacity.evaluate(local).unwrap_or(Rational::new(1, 1));
                let ox = match eval_geometry(&props.x, local, clip.id, "x") {
                    Ok(v) => v,
                    Err(e) => {
                        eval_err = Some(e);
                        return;
                    }
                };
                let oy = match eval_geometry(&props.y, local, clip.id, "y") {
                    Ok(v) => v,
                    Err(e) => {
                        eval_err = Some(e);
                        return;
                    }
                };
                placements.push(Placement {
                    clip_id: clip.id,
                    window,
                    source: source_id_of(hex),
                    alpha,
                    offset: (ox, oy),
                    // The plan must declare the tags the FETCH will actually
                    // deliver: the boundary conversion (yuv420p_to_rgba) emits
                    // RGBA stamped {src primaries/transfer, Bt709, Full}.
                    // Declaring the raw probe tags here made the plan omit the
                    // ColorConvert stamp pass whenever the probe range happened
                    // to equal the working-space range — and exec then failed
                    // with TagMismatch on ANY real source carrying real color
                    // metadata (found by the real-world reference media test;
                    // the synthetic corpus probes tag-Unknown and could not
                    // see it). See REALWORLD-BUG-1 in docs/REALWORLD_VALIDATION.md.
                    src_color: video
                        .video
                        .as_ref()
                        .map(|v| boundary_rgba_stamp(v.color))
                        .unwrap_or_else(all_unknown_color),
                });
                let _ = pos;
            });
            if let Some(e) = eval_err {
                return Err(e);
            }
            tracks.push(TrackInput { placements });
        }
        Ok(RenderInput {
            tracks,
            output: output.clone(),
        })
    }

    /// Render one output frame at timeline time `t` (exact): decode at the
    /// mapped source pts → RGBA boundary conversion → compile → execute.
    pub fn render_frame(
        &mut self,
        output: &ove_render::OutputSpec,
        t: Rational,
    ) -> Result<FrameEnvelope, EngineError> {
        let input = self.build_render_input(output, t)?;
        let plan =
            compile_frame(&input, frame_index_of(output, t)).map_err(EngineError::Compile)?;
        // FrameSource: decode-on-demand per placement source
        let mut sources: std::collections::HashMap<u64, DecodeSource<'_>> =
            std::collections::HashMap::new();
        for (hex, media) in &self.sources {
            let sid = source_id_of(hex);
            sources.insert(sid, DecodeSource::new(self.project.dir(), media));
        }
        let refs: std::collections::HashMap<u64, &dyn FrameSource> = sources
            .iter()
            .map(|(k, v)| (*k, v as &dyn FrameSource))
            .collect();
        let renderer = SoftwareRenderer::new(refs);
        renderer.execute_frame(&plan).map_err(EngineError::Render)
    }

    // -- export paths -----------------------------------------------------------

    /// Re-encode export: render every output frame → encoder (RGBA input,
    /// declared swscale conversion) → MP4. Video-only in v1 (W7 adds audio).
    pub fn export_reencode(
        &mut self,
        out: &Path,
        output: &ove_render::OutputSpec,
        n_frames: i64,
    ) -> Result<OutputInfo, EngineError> {
        // output geometry comes from the first video source (v1)
        let (w, h, color) = self.first_video_geometry()?;
        let cfg = EncoderConfig {
            codec: VideoCodec::Mpeg4,
            profile: VideoProfile {
                width: w,
                height: h,
                frame_rate: Rational::new(output.rate_num, output.rate_den),
                pixel_format: PixelFormat::Rgba,
                color,
                gop: 12,
                bitexact: true,
            },
            rate: RateControl::Crf { quality: 6 },
        };
        let mut enc = FfmpegSwEncoder::configure(cfg).map_err(EngineError::Encode)?;
        let track = enc.track_spec().map_err(EngineError::Encode)?;
        for k in 0..n_frames {
            let t = output.frame_pts(k);
            let mut frame = self.render_frame(output, t)?;
            frame.pts = t;
            frame.duration = Rational::new(output.rate_den, output.rate_num);
            enc.feed(frame).map_err(EngineError::Encode)?;
        }
        let packets = enc.drain().map_err(EngineError::Encode)?;

        // W7 audio leg: when the sources carry audio, assemble the same
        // timeline span sample-exactly and re-encode AAC (ENCODER_SPEC §3.3
        // video re-encode route pairs with audio re-encode).
        let mut audio_tracks: Vec<ove_encode::TrackSpec> = Vec::new();
        let mut audio_packets: Vec<ove_encode::EncodedPacket> = Vec::new();
        if self.first_audio_stream().is_some() {
            let span = Rational::new(n_frames * output.rate_den, output.rate_num);
            let assembly = self.assemble_timeline_audio(span)?;
            let acfg = ove_encode::AudioEncoderConfig {
                codec: ove_encode::AudioCodec::Aac,
                sample_rate: assembly.sample_rate,
                channels: assembly.channels,
                rate: RateControl::Cbr { bitrate: 128_000 },
                bitexact: true,
            };
            let mut aenc = FfmpegAacEncoder::configure(acfg).map_err(EngineError::Encode)?;
            let atrack = aenc.track_spec().map_err(EngineError::Encode)?;
            let rate = assembly.sample_rate as i64;
            let ch = assembly.channels as usize;
            const CHUNK: u64 = 4096;
            let mut cursor: u64 = 0;
            while cursor < assembly.samples {
                let len = CHUNK.min(assembly.samples - cursor) as usize;
                let stride = len * 4;
                let mut data = Vec::with_capacity(stride * ch);
                for plane in &assembly.planes {
                    for v in &plane[cursor as usize..cursor as usize + len] {
                        data.extend_from_slice(&v.to_le_bytes());
                    }
                }
                let env = FrameEnvelope::audio(
                    Rational::new(cursor as i64, rate),
                    ove_media::StreamId(1),
                    assembly.sample_rate,
                    assembly.channels,
                    len,
                    ove_media::FrameBytes {
                        data,
                        strides: vec![stride; ch],
                    },
                    BackendId::FFmpegSw,
                    0,
                );
                aenc.feed(env).map_err(EngineError::Encode)?;
                cursor += len as u64;
            }
            audio_packets = aenc.drain().map_err(EngineError::Encode)?;
            audio_tracks.push(atrack);
        }

        let mut mux = FfmpegMuxer::open(
            OutputSink::File(out.to_path_buf()),
            Container::Mp4 {
                faststart: true,
                bitexact: true,
            },
            vec![track].into_iter().chain(audio_tracks).collect(),
        )
        .map_err(EngineError::Mux)?;
        for p in packets.into_iter().chain(audio_packets) {
            mux.write(p).map_err(EngineError::Mux)?;
        }
        mux.finalize().map_err(EngineError::Mux)
    }

    /// Stream-copy export of a source range (keyframe-aligned via the pure
    /// planner; snaps reported). Single video source, video-only in v1.
    pub fn export_copy(
        &mut self,
        asset_hex: &str,
        start: Rational,
        end: Rational,
        out: &Path,
    ) -> Result<(OutputInfo, Vec<ove_encode::planner::SnapRecord>), EngineError> {
        let media = self
            .sources
            .get(asset_hex)
            .ok_or_else(|| EngineError::UnknownAsset(asset_hex.to_string()))?
            .clone();
        let probe = &media.probe;
        let Some(vstream) = probe
            .streams
            .iter()
            .find(|s| s.kind == ove_media::StreamKind::Video)
        else {
            return Err(EngineError::Import("no video stream".into()));
        };
        let kfs: Vec<Rational> = probe
            .keyframe_index
            .as_ref()
            .map(|idx| idx.entries.iter().map(|e| e.pts).collect())
            .unwrap_or_default();
        let media_end = vstream.duration.unwrap_or_else(|| Rational::zero(1));
        let input = ove_encode::planner::TrackInput {
            stream_id: ove_media::StreamId(vstream.id.0),
            kind: ove_encode::planner::TrackKindTag::Video,
            keyframes: kfs,
            media_end,
            audio_grid: None,
            reencode_available: false,
        };
        let plan = ove_encode::planner::plan_export(
            &[input],
            ove_encode::planner::TimeRange::new(start, end)
                .map_err(|e| EngineError::Internal(e.to_string()))?,
            ove_encode::planner::CopyPolicy::KeyframeAlignedOnly,
        )
        .map_err(|e| EngineError::Internal(e.to_string()))?;
        let span = plan
            .copy_span_of(ove_media::StreamId(vstream.id.0))
            .ok_or_else(|| EngineError::Internal("plan produced no copy span".into()))?;
        let snaps = plan.snaps().into_iter().map(|(_, s)| s).collect();

        // execute: demux source packets in the snapped span, shift to 0, mux
        let asset_path = self
            .project
            .dir()
            .join(format!("assets/{}/src.mp4", media.hash.hex()));
        let asset =
            AssetRef::from_path(&asset_path).map_err(|e| EngineError::Import(format!("{e}")))?;
        let mut source = FfmpegCopySource::open(&asset).map_err(EngineError::Mux)?;
        let parsed = source
            .stream_by_source_index(vstream.id.0 as usize)
            .ok_or_else(|| EngineError::Import("video stream vanished".into()))?
            .clone();
        let track = parsed
            .track_spec(ove_media::StreamId(0))
            .map_err(EngineError::Mux)?;
        let packets = source
            .read_packets(
                &span,
                &[vstream.id.0 as usize],
                &[(vstream.id.0 as usize, ove_media::StreamId(0))],
            )
            .map_err(EngineError::Mux)?;
        let mut mux = FfmpegMuxer::open(
            OutputSink::File(out.to_path_buf()),
            Container::Mp4 {
                faststart: true,
                bitexact: true,
            },
            vec![track],
        )
        .map_err(EngineError::Mux)?;
        let n = packets.len();
        for p in packets {
            mux.write(p).map_err(EngineError::Mux)?;
        }
        let _ = n;
        let info = mux.finalize().map_err(EngineError::Mux)?;
        Ok((info, snaps))
    }

    // -- W7 audio: assembly + WAV/AAC export (ADR-018) --------------------------

    /// The first imported source's first audio stream, if any.
    fn first_audio_stream(&self) -> Option<ove_media::ProbeStream> {
        for media in self.sources.values() {
            if let Some(a) = media
                .probe
                .streams
                .iter()
                .find(|s| s.kind == ove_media::StreamKind::Audio)
            {
                return Some(a.clone());
            }
        }
        None
    }

    /// The first track's timeline span (end of its last placement).
    fn first_track_span(&self) -> Result<Rational, EngineError> {
        use ove_timeline::mapping::ClipWindow;
        let Some(tid) = self.project.timeline().track_ids().min() else {
            return Err(EngineError::NoAudioStream);
        };
        let track = self
            .project
            .timeline()
            .track_ref(tid)
            .map_err(EngineError::Timeline)?;
        let mut end = Rational::zero(1);
        track.walk(&mut |_pos, start, clip: &Clip| {
            let w = ClipWindow::from_clip(clip, start);
            let e = w.timeline_start + w.dur;
            if e > end {
                end = e;
            }
        });
        Ok(end)
    }

    /// Assemble the timeline audio for the FIRST track's clips (v1: single
    /// audio lane following the video placements; multi-track mixing is a
    /// named gap — ADR-018). Sample-exact: every cut lands on a sample
    /// boundary of the source rate and the assembly is the concatenation of
    /// per-clip sample ranges. NO resampling — speed ≠ 1 is a typed error.
    ///
    /// `timeline_span` is the span authority: the result is EXACTLY
    /// span × rate samples (sources must cover it; short sources are a
    /// typed error, never a silent pad).
    pub fn assemble_timeline_audio(
        &mut self,
        timeline_span: Rational,
    ) -> Result<AudioAssembly, EngineError> {
        use ove_timeline::mapping::ClipWindow;

        let Some(audio) = self.first_audio_stream() else {
            return Err(EngineError::NoAudioStream);
        };
        let rate = audio.audio.as_ref().map(|a| a.sample_rate).unwrap_or(0);
        let channels = audio.audio.as_ref().map(|a| a.channels).unwrap_or(0);
        if rate == 0 || channels == 0 {
            return Err(EngineError::NoAudioStream);
        }
        let span_r = timeline_span * Rational::new(rate as i64, 1);
        if span_r.den() != 1 || span_r.num() < 0 {
            return Err(EngineError::NonExactSampleCut {
                at: timeline_span,
                rate,
            });
        }
        let want_total: u64 = span_r.num() as u64;

        // placements of the first track (the v1 audio lane), validated
        // BEFORE any decoding (typed errors, never mid-walk panics)
        let Some(tid) = self.project.timeline().track_ids().min() else {
            return Err(EngineError::NoAudioStream);
        };
        let track = self
            .project
            .timeline()
            .track_ref(tid)
            .map_err(EngineError::Timeline)?;
        let mut placements: Vec<(ClipWindow, u64, u64)> = Vec::new(); // (window, n0, n1)
        track.walk(&mut |_pos, start, clip: &Clip| {
            let window = ClipWindow::from_clip(clip, start);
            if window.speed != Rational::new(1, 1) {
                // propagate after the walk (the closure cannot return Err)
                placements.push((window, u64::MAX, u64::MAX));
                return;
            }
            let n0r = window.src_in * Rational::new(rate as i64, 1);
            let n1r = (window.src_in + window.src_dur()) * Rational::new(rate as i64, 1);
            placements.push((
                window,
                if n0r.den() == 1 {
                    n0r.num() as u64
                } else {
                    u64::MAX
                },
                if n1r.den() == 1 {
                    n1r.num() as u64
                } else {
                    u64::MAX
                },
            ));
        });
        for (window, n0, n1) in &placements {
            if window.speed != Rational::new(1, 1) {
                return Err(EngineError::AudioRetimeUnsupported {
                    speed: window.speed,
                });
            }
            if *n0 == u64::MAX || *n1 == u64::MAX {
                return Err(EngineError::NonExactSampleCut {
                    at: window.src_in,
                    rate,
                });
            }
        }

        // per-clip cuts, concatenated (clips are sequential on the track)
        let mut planes: Vec<Vec<f32>> = vec![Vec::new(); channels as usize];
        let mut produced: u64 = 0;
        for (_window, n0, n1) in &placements {
            if produced >= want_total {
                break;
            }
            let want = (*n1 - *n0).min(want_total - produced);
            let piece = self.cut_source_samples(&audio, *n0, want)?;
            if piece.samples != want {
                return Err(EngineError::Internal(format!(
                    "audio source exhausted: {} of {want} samples",
                    piece.samples
                )));
            }
            produced += piece.samples;
            for (plane, target) in piece.planes.into_iter().zip(planes.iter_mut()) {
                target.extend_from_slice(&plane);
            }
        }
        if produced != want_total {
            return Err(EngineError::Internal(format!(
                "audio assembly produced {produced} of {want_total} samples"
            )));
        }
        Ok(AudioAssembly {
            planes,
            sample_rate: rate,
            channels,
            samples: produced,
        })
    }

    /// Cut exactly `want` samples of the (single) source starting at sample
    /// index `n0` (floor frame + sample trim per ADR-018). `n0` and `want`
    /// are sample-exact integers; the source must cover the range.
    fn cut_source_samples(
        &mut self,
        audio: &ove_media::ProbeStream,
        n0: u64,
        want: u64,
    ) -> Result<AudioAssembly, EngineError> {
        let rate = audio.audio.as_ref().map(|a| a.sample_rate).unwrap_or(0);
        let channels = audio.audio.as_ref().map(|a| a.channels).unwrap_or(0) as usize;
        let media = self
            .sources
            .values()
            .next()
            .ok_or(EngineError::NoAudioStream)?
            .clone();
        let hex = media.hash.hex();
        let asset_path = self.project.dir().join(format!("assets/{hex}/src.mp4"));
        let asset =
            AssetRef::from_path(&asset_path).map_err(|e| EngineError::Import(format!("{e}")))?;
        let mut dec = FfmpegSwDecoder::open(&asset, audio.id, DecodeConfig::default())
            .map_err(|e| EngineError::Import(format!("audio decode open: {e}")))?;
        if n0 > 0 {
            let src_in = Rational::new(n0 as i64, rate as i64);
            dec.seek(src_in, SeekMode::Exact)
                .map_err(|e| EngineError::Import(format!("audio seek: {e}")))?;
        }
        let mut planes: Vec<Vec<f32>> = vec![Vec::new(); channels];
        let mut collected: u64 = 0;
        while collected < want {
            let Some(frame) = dec
                .next()
                .map_err(|e| EngineError::Import(format!("audio decode: {e}")))?
            else {
                break; // source exhausted; caller reports the shortfall
            };
            let nb_r = frame.duration * Rational::new(rate as i64, 1);
            if nb_r.den() != 1 {
                return Err(EngineError::Internal("non-integer audio frame".into()));
            }
            let nb = nb_r.num() as u64;
            let frame_start_r = frame.pts * Rational::new(rate as i64, 1);
            if frame_start_r.den() != 1 {
                return Err(EngineError::NonExactSampleCut {
                    at: frame.pts,
                    rate,
                });
            }
            let frame_start = frame_start_r.num() as u64;
            // cut window inside this frame: [max(start, n0), min(end, n0+want))
            let lo = frame_start.max(n0);
            let hi = (frame_start + nb).min(n0 + want);
            if hi > lo {
                let fb = frame
                    .cpu_bytes()
                    .ok_or_else(|| EngineError::Internal("audio frame without payload".into()))?;
                let stride = fb.strides[0];
                for (ch, target) in planes.iter_mut().enumerate() {
                    let plane = &fb.data[ch * stride..(ch + 1) * stride];
                    let floats: &[f32] = unsafe {
                        std::slice::from_raw_parts(plane.as_ptr().cast::<f32>(), nb as usize)
                    };
                    target.extend_from_slice(
                        &floats[(lo - frame_start) as usize..(hi - frame_start) as usize],
                    );
                }
                collected += hi - lo;
            }
            if frame_start + nb >= n0 + want {
                break;
            }
        }
        Ok(AudioAssembly {
            planes,
            sample_rate: rate,
            channels: channels as u32,
            samples: collected,
        })
    }

    /// WAV/PCM export of the timeline audio (BUILD_PLAN wave 7: "mix →
    /// WAV/PCM out"). Returns the sample frames written.
    pub fn export_wav(&mut self, out: &Path) -> Result<u64, EngineError> {
        let span = self.first_track_span()?;
        let assembly = self.assemble_timeline_audio(span)?;
        ove_encode::wav::write_wav_s16(out, &assembly.planes, assembly.sample_rate)
            .map_err(EngineError::Encode)
    }

    fn first_video_geometry(&self) -> Result<(u32, u32, ColorTags), EngineError> {
        for media in self.sources.values() {
            if let Some(v) = media
                .probe
                .streams
                .iter()
                .find(|s| s.kind == ove_media::StreamKind::Video)
            {
                if let Some(vd) = &v.video {
                    return Ok((vd.width, vd.height, vd.color));
                }
            }
        }
        Err(EngineError::Import("no video source imported".into()))
    }
}

/// All-unknown tags (unknown-as-value; never invented tags).
fn all_unknown_color() -> ColorTags {
    ColorTags {
        primaries: Primaries::Unknown,
        transfer: Transfer::Unknown,
        matrix: MatrixCoeffs::Unknown,
        range: Range::Unknown,
        chroma_loc: None,
    }
}

fn source_id_of(hex: &str) -> u64 {
    // stable 64-bit source id from the content hash (explicit identity; not
    // a positional index)
    let bytes = decode_hex_32(hex).unwrap_or_default();
    u64::from_le_bytes(bytes[..8].try_into().expect("32 bytes"))
}

fn decode_hex_32(hex: &str) -> Option<[u8; 32]> {
    if hex.len() != 64 {
        return None;
    }
    let mut out = [0u8; 32];
    for (i, b) in out.iter_mut().enumerate() {
        *b = u8::from_str_radix(&hex[i * 2..i * 2 + 2], 16).ok()?;
    }
    Some(out)
}

fn frame_index_of(output: &ove_render::OutputSpec, t: Rational) -> i64 {
    // k = floor(t × rate) — frame k covers [k/rate, (k+1)/rate). Exact
    // i128 arithmetic; frame-aligned callers hit frame_pts(k) == t exactly.
    let numer = (t.num() as i128) * (output.rate_num as i128);
    let denom = (t.den() as i128) * (output.rate_den as i128);
    (numer / denom) as i64
}

/// Evaluate one keyframed geometry property at local time and convert to
/// the renderer's i32 pixel domain with the deterministic round-half-up
/// convention (ADR-019; ove-time P13 pins the arithmetic). Unanimated
/// properties evaluate to None → 0 (the static default). An exact value
/// rounding outside i32 is a typed error — never a silent saturation.
fn eval_geometry(
    track: &ove_timeline::PropertyTrack,
    local: Rational,
    clip_id: u64,
    property: &'static str,
) -> Result<i32, EngineError> {
    match track.evaluate(local) {
        None => Ok(0),
        Some(v) => {
            let r = v.round_half_up();
            i32::try_from(r).map_err(|_| EngineError::KeyframeValueOutOfRange {
                clip_id,
                property: property.to_string(),
                value: v,
            })
        }
    }
}

// ---------------------------------------------------------------------------
// Decode source — the FrameSource adapter (decode → RGBA boundary conversion)
// ---------------------------------------------------------------------------

/// One decode-backed source. Interior mutability = the decoder session +
/// its seek/decode cursor; single-writer (v1), so RefCell is sound here.
struct DecodeSource<'a> {
    dir: &'a Path,
    media: &'a SourceMedia,
    session: std::cell::RefCell<Option<VideoSession>>,
}

/// The live decode cursor for one source: where the decoder is landed (the
/// keyframe floor of the last seek) and the pts of the last floor delivered.
struct VideoSession {
    dec: FfmpegSwDecoder,
    landed: Rational,
    last_floor_pts: Rational,
}

/// Greatest keyframe pts <= target (the ADR-013 landing discipline); the
/// stream start (0) when no keyframe precedes the target.
fn keyframe_floor_of(media: &SourceMedia, target: Rational) -> Rational {
    media
        .probe
        .keyframe_index
        .as_ref()
        .and_then(|idx| {
            idx.entries
                .iter()
                .map(|e| e.pts)
                .rev()
                .find(|&k| k <= target)
        })
        .unwrap_or_else(|| Rational::zero(1))
}

impl<'a> DecodeSource<'a> {
    fn new(dir: &'a Path, media: &'a SourceMedia) -> Self {
        DecodeSource {
            dir,
            media,
            session: std::cell::RefCell::new(None),
        }
    }

    fn open_decoder(&self) -> Result<FfmpegSwDecoder, EngineError> {
        let hex = self.media.hash.hex();
        let asset_path = self.dir.join(format!("assets/{hex}/src.mp4"));
        let asset =
            AssetRef::from_path(&asset_path).map_err(|e| EngineError::Import(format!("{e}")))?;
        let vstream = self
            .media
            .probe
            .streams
            .iter()
            .find(|s| s.kind == ove_media::StreamKind::Video)
            .ok_or_else(|| EngineError::Import("no video stream".into()))?;
        FfmpegSwDecoder::open(&asset, vstream.id, DecodeConfig::default())
            .map_err(|e| EngineError::Import(format!("decode open: {e}")))
    }
}

impl FrameSource for DecodeSource<'_> {
    /// The D-5 floor rule: return the frame with the GREATEST pts ≤ target
    /// (exact hits are the common CFR case; multi-rate sources floor).
    ///
    /// REALWORLD-BUG-3 (found by the real-world reference media test): the
    /// previous fetch sought EXACTLY at the mapped target. On real NTSC
    /// media the mapped target usually falls BETWEEN frame pts, and the
    /// decoder's Exact seek forward-drops everything ≤ target (D-4) — so
    /// the floor frame was never even delivered and fetch returned None
    /// (SourceFrameMissing). The synthetic corpus always seeked to exact
    /// frame pts and could not see this. The ADR-013 discipline is the
    /// keyframe-FLOOR landing (plan_seek): land at the greatest keyframe
    /// ≤ target, then decode FORWARD keeping the last frame ≤ target.
    /// Sequential same-GOP targets reuse the decoder position; a target
    /// at/before the last delivered floor re-seeks (backward jumps).
    fn fetch(&self, target: Rational) -> Option<FrameEnvelope> {
        let land = keyframe_floor_of(self.media, target);
        let mut session = self.session.borrow_mut();
        if session.is_none() {
            let dec = self.open_decoder().ok()?;
            *session = Some(VideoSession {
                dec,
                landed: Rational::new(-1, 1),
                last_floor_pts: Rational::new(-1, 1),
            });
        }
        let s = session.as_mut().expect("just set");
        if s.landed != land || target <= s.last_floor_pts {
            s.dec.seek(land, SeekMode::Exact).ok()?;
            s.landed = land;
        }
        let mut floor: Option<FrameEnvelope> = None;
        // A decode error or EOF after a floor exists still returns the floor:
        // real edits can end on the source's last frame.
        while let Ok(Some(frame)) = s.dec.next() {
            if frame.pts > target {
                // decoded past the target: the last frame ≤ target
                // is the floor (D-5); None only when the target
                // precedes the first frame — the honest empty answer.
                break;
            }
            // pts ≤ target: keep it as the running floor; convert
            // at the engine boundary (the single declared swap).
            floor = yuv_to_rgba(&frame);
        }
        if let Some(f) = &floor {
            s.last_floor_pts = f.pts;
        }
        floor
    }
}

/// YUV420P → RGBA8, integer-only (deterministic on every platform).
/// Range-aware expansion (declared Limited vs Full; declared-Unknown keeps
/// the limited assumption) with the matrix chosen from the frame's declared
/// tags; Unknown matrix → BT.601 (the SD convention). Chroma upsample =
/// nearest (replicate) — declared v1 limitation, ADR-017.
pub fn yuv420p_to_rgba(frame: &FrameEnvelope) -> Option<FrameEnvelope> {
    if frame.pixel_format != PixelFormat::Yuv420p || frame.memory != FrameMemory::Cpu {
        return None;
    }
    let fb = frame.cpu_bytes()?;
    if fb.strides.len() != 3 {
        return None;
    }
    let (w, h) = (frame.width as usize, frame.height as usize);
    let y_plane = &fb.data[..fb.strides[0] * h];
    let u_off = fb.strides[0] * h;
    let v_off = u_off + fb.strides[1] * (h.div_ceil(2));
    let u_plane = &fb.data[u_off..u_off + fb.strides[1] * (h.div_ceil(2))];
    let v_plane = &fb.data[v_off..];

    // Range-aware expansion (REALWORLD-BUG-2): the previous math hardwired
    // the LIMITED-range expansion even for declared Full-range sources —
    // real consumer media does declare Full, and the levels were crushed.
    // Declared-Unknown keeps the limited assumption (documented v1 policy;
    // unknown-as-value, never invented — but a guess must stay visible).
    let full_range = frame.color.range == ove_media::Range::Full;
    // Fixed-point s15:16 coefficients per matrix (BT.601 default).
    let (cr, cb, cu, cv): (i32, i32, i32, i32) = match frame.color.matrix {
        ove_media::MatrixCoeffs::Bt709 if !full_range => (298, 496, 55, 139),
        ove_media::MatrixCoeffs::Bt709 => (256, 454, 88, 183),
        _ if !full_range => (298, 516, 100, 208),
        _ => (256, 472, 86, 177),
    };
    let y_shift: i32 = if full_range { 0 } else { 16 };
    let c_shift: i32 = 128; // chroma always centers on 128

    let mut rgba = Vec::with_capacity(w * h * 4);
    for y in 0..h {
        let yrow = &y_plane[y * fb.strides[0]..][..w];
        let urow = &u_plane[(y / 2) * fb.strides[1]..][..w.div_ceil(2)];
        let vrow = &v_plane[(y / 2) * fb.strides[2]..][..w.div_ceil(2)];
        for x in 0..w {
            let yy = yrow[x] as i32 - y_shift;
            let uu = urow[x / 2] as i32 - c_shift;
            let vv = vrow[x / 2] as i32 - c_shift;
            let r = (cr * yy + cv * vv + 128) >> 8;
            let g = (cr * yy - cu * uu - cv * vv + 128) >> 8;
            let b = (cr * yy + cb * uu + 128) >> 8;
            rgba.push(r.clamp(0, 255) as u8);
            rgba.push(g.clamp(0, 255) as u8);
            rgba.push(b.clamp(0, 255) as u8);
            rgba.push(255);
        }
    }

    // RGBA output carries the BOUNDARY STAMP (see boundary_rgba_stamp): the
    // plan declares exactly these tags, so the single-conversion rule stays
    // honest end to end.
    Some(FrameEnvelope::video_cpu(
        frame.pts,
        frame.duration,
        frame.stream_id,
        frame.width,
        frame.height,
        PixelFormat::Rgba,
        frame.bit_depth,
        boundary_rgba_stamp(frame.color),
        ove_media::FrameBytes {
            data: rgba,
            strides: vec![w * 4],
        },
        frame.keyframe,
        frame.backend.clone(),
        frame.generation,
    ))
}

/// The tags the YUV→RGBA boundary conversion stamps onto its output: source
/// primaries/transfer carry over, matrix becomes Bt709 (RGBA is matrix-free,
/// Bt709 primaries assumed for the stamp), and 8-bit RGBA is definitionally
/// Full range. The plan (compile) and the executor (single-conversion check)
/// must both see THIS declaration — one source of truth, no drift.
pub fn boundary_rgba_stamp(src: ove_media::ColorTags) -> ove_media::ColorTags {
    ove_media::ColorTags {
        primaries: src.primaries,
        transfer: src.transfer,
        matrix: ove_media::MatrixCoeffs::Bt709,
        range: ove_media::Range::Full,
        chroma_loc: src.chroma_loc,
    }
}

/// Alias matching the crate-level name used in the session docs.
fn yuv_to_rgba(frame: &FrameEnvelope) -> Option<FrameEnvelope> {
    yuv420p_to_rgba(frame)
}

#[cfg(test)]
mod boundary_stamp_tests {
    use super::*;

    fn tags(range: Range, matrix: MatrixCoeffs) -> ColorTags {
        ColorTags {
            primaries: Primaries::Bt709,
            transfer: Transfer::Bt709,
            matrix,
            range,
            chroma_loc: Some(ove_media::ChromaLoc::Left),
        }
    }

    fn yuv_frame(color: ColorTags, y: u8, u: u8, v: u8) -> FrameEnvelope {
        // 2x2 YUV420P: one luma sample, one chroma sample of each plane.
        FrameEnvelope::video_cpu(
            Rational::new(0, 1),
            Rational::new(1, 24),
            ove_media::StreamId(0),
            2,
            2,
            PixelFormat::Yuv420p,
            ove_media::BitDepth::B8,
            color,
            ove_media::FrameBytes {
                // 2x2 YUV420P: four luma samples, one U, one V.
                data: vec![y, y, y, y, u, v],
                strides: vec![2, 1, 1],
            },
            true,
            BackendId::FFmpegSw,
            0,
        )
    }

    /// REALWORLD-BUG-1 pin: the stamp the boundary conversion emits is the
    /// EXACT declaration the plan must carry (compile sees the same fn).
    #[test]
    fn stamp_matches_converted_envelope() {
        let src = tags(Range::Limited, MatrixCoeffs::Bt709);
        let out = yuv420p_to_rgba(&yuv_frame(src, 128, 128, 128)).expect("converts");
        assert_eq!(out.color, boundary_rgba_stamp(src));
        assert_eq!(out.color.range, Range::Full);
        assert_eq!(out.pixel_format, PixelFormat::Rgba);
    }

    /// REALWORLD-BUG-2 pin: declared-Limited black (y=16) maps to 0.
    #[test]
    fn limited_black_maps_to_zero() {
        let out = yuv420p_to_rgba(&yuv_frame(
            tags(Range::Limited, MatrixCoeffs::Bt709),
            16,
            128,
            128,
        ))
        .expect("converts");
        assert_eq!(&out.cpu_bytes().unwrap().data[..3], &[0, 0, 0]);
    }

    /// REALWORLD-BUG-2 pin: declared-Full luma is IDENTITY (no studio swap):
    /// y=200, neutral chroma -> (200,200,200); the old math crushed it.
    #[test]
    fn full_range_luma_is_identity() {
        let out = yuv420p_to_rgba(&yuv_frame(
            tags(Range::Full, MatrixCoeffs::Bt709),
            200,
            128,
            128,
        ))
        .expect("converts");
        assert_eq!(&out.cpu_bytes().unwrap().data[..3], &[200, 200, 200]);
    }
}
