//! Probe data model (MEDIA FILE→PROBE→ASSET chain, DECODER_SPEC §4) and the
//! content-hash-keyed metadata cache.
//!
//! `ProbeBackend` implementations live OUTSIDE this crate (libav linkage is
//! confined to ove-decode; DECODER_SPEC §5 gate). ove-media owns the TYPES and
//! the CACHE so browser/WASM legs can implement the same trait in pure Rust.

use std::path::{Path, PathBuf};

use ove_time::Rational;
use serde::{Deserialize, Serialize};

use crate::asset::{AssetRef, StreamKind};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum ContainerKind {
    Mp4,
    Mov,
    Mkv,
    WebM,
    Avi,
    Other,
}

/// Full static description of a media file, built once at import.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ProbeInfo {
    pub container: ContainerKind,
    /// Container-level duration (exact if the container carries it exactly).
    pub duration: Option<Rational>,
    pub streams: Vec<ProbeStream>,
    /// Decoding-entry-point index for the FIRST video stream, if built.
    /// pts are exact container pts; byte_pos = demuxer offset for seek.
    pub keyframe_index: Option<KeyframeIndex>,
    /// Variable-frame-rate analysis of the first video stream, if built.
    pub vfr: Option<VfrReport>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ProbeStream {
    pub id: crate::asset::StreamId,
    pub kind: StreamKind,
    /// Codec name as reported by the probe backend (e.g. "mpeg4", "aac").
    pub codec: String,
    /// Container time base for this stream's timestamps (exact).
    pub time_base: Rational,
    pub start_time: Option<Rational>,
    pub duration: Option<Rational>,
    /// Average rate hint — informational only. Scheduling never assumes CFR
    /// (DECODER_SPEC D-12): per-frame pts/duration are authoritative.
    pub avg_frame_rate: Option<Rational>,
    pub nb_frames_hint: Option<u64>,
    pub video: Option<VideoDetails>,
    pub audio: Option<AudioDetails>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct VideoDetails {
    pub width: u32,
    pub height: u32,
    pub pixel_format: crate::frame::PixelFormat,
    pub bit_depth: crate::frame::BitDepth,
    /// All five color tags, unknown-as-value (FRAME_CONTRACT §4).
    pub color: crate::frame::ColorTags,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct AudioDetails {
    pub sample_rate: u32,
    pub channels: u32,
    pub sample_format: String,
}

/// One decoding entry point (keyframe) of a video stream.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct KeyframeEntry {
    /// Exact container pts of the keyframe.
    pub pts: Rational,
    /// Demuxer byte offset (av_seek_frame byte target), if the backend has it.
    pub byte_pos: Option<u64>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct KeyframeIndex {
    /// Sorted ascending by pts.
    pub entries: Vec<KeyframeEntry>,
}

impl KeyframeIndex {
    /// Last keyframe with pts <= t (the Snap/Exact seek landing point).
    /// Returns None if t is before the first keyframe.
    pub fn floor(&self, t: Rational) -> Option<&KeyframeEntry> {
        // entries sorted; partition_point keeps this O(log n)
        let idx = self.entries.partition_point(|e| e.pts <= t);
        idx.checked_sub(1).map(|i| &self.entries[i])
    }

    pub fn len(&self) -> usize {
        self.entries.len()
    }

    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }
}

/// VFR detection result for a video stream (DECODER_SPEC D-12).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct VfrReport {
    /// true if more than one distinct inter-frame delta was observed.
    pub is_vfr: bool,
    /// Observed distinct deltas (exact, capped sample for huge streams).
    pub distinct_deltas: Vec<Rational>,
    pub frame_count: u64,
}

impl VfrReport {
    /// Compute from the exact pts list of a stream. Pure function — testable
    /// without any media backend (D-12: nothing may assume CFR).
    pub fn from_pts(pts: &[Rational]) -> Self {
        let mut distinct: Vec<Rational> = Vec::new();
        let push = |d: Rational, distinct: &mut Vec<Rational>| {
            if distinct.len() < 16 && !distinct.contains(&d) {
                distinct.push(d);
            }
        };
        for pair in pts.windows(2) {
            let d = pair[1].sub(pair[0]);
            push(d, &mut distinct);
        }
        let is_vfr = distinct.len() > 1;
        // distinct_deltas must be empty when the stream is CFR (single delta)
        let distinct_deltas = if is_vfr { distinct } else { vec![] };
        VfrReport {
            is_vfr,
            distinct_deltas,
            frame_count: pts.len() as u64,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ProbeError {
    Unreadable,
    Corrupt(String),
    Unsupported(String),
    Io(String),
}

impl std::fmt::Display for ProbeError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ProbeError::Unreadable => write!(f, "unreadable media file"),
            ProbeError::Corrupt(d) => write!(f, "corrupt media: {d}"),
            ProbeError::Unsupported(d) => write!(f, "unsupported: {d}"),
            ProbeError::Io(d) => write!(f, "io error: {d}"),
        }
    }
}

impl std::error::Error for ProbeError {}

/// Static probe + keyframe index builder. Implemented per media backend.
pub trait ProbeBackend {
    fn probe(&self, asset: &AssetRef) -> Result<ProbeInfo, ProbeError>;
    /// Build the decoding-entry-point index (packet scan; E-007's lesson:
    /// boundaries must be decided from a real index, never guessed).
    fn keyframe_index(&self, asset: &AssetRef) -> Result<KeyframeIndex, ProbeError>;
}

/// Metadata cache keyed by content hash: one JSON file per asset.
/// A corrupted cache file is a MISS, never an error (self-healing).
pub struct MetadataCache {
    dir: PathBuf,
}

#[derive(Serialize, Deserialize)]
struct CacheEntry {
    content_hash: String,
    probe: ProbeInfo,
}

impl MetadataCache {
    pub fn new(dir: impl Into<PathBuf>) -> Self {
        MetadataCache { dir: dir.into() }
    }

    fn path_for(&self, asset: &AssetRef) -> PathBuf {
        self.dir
            .join(format!("{}.probe.json", asset.content_hash.hex()))
    }

    pub fn get(&self, asset: &AssetRef) -> Option<ProbeInfo> {
        let p = self.path_for(asset);
        let text = std::fs::read_to_string(&p).ok()?;
        let entry: CacheEntry = serde_json::from_str(&text).ok()?;
        if entry.content_hash != asset.content_hash.hex() {
            return None; // mismatched entry: treat as miss
        }
        Some(entry.probe)
    }

    pub fn put(&self, asset: &AssetRef, probe: &ProbeInfo) -> std::io::Result<()> {
        std::fs::create_dir_all(&self.dir)?;
        let entry = CacheEntry {
            content_hash: asset.content_hash.hex(),
            probe: probe.clone(),
        };
        // write-then-rename: a killed process leaves no half-written cache file
        let tmp = self.path_for(asset).with_extension("tmp");
        std::fs::write(&tmp, serde_json::to_vec(&entry).expect("serialize cache"))?;
        std::fs::rename(&tmp, self.path_for(asset))
    }

    /// Cached-or-computed probe (the whole point of the asset-as-cache-key).
    pub fn get_or_probe(
        &self,
        asset: &AssetRef,
        backend: &dyn ProbeBackend,
    ) -> Result<ProbeInfo, ProbeError> {
        if let Some(p) = self.get(asset) {
            return Ok(p);
        }
        let info = backend.probe(asset)?;
        self.put(asset, &info)
            .map_err(|e| ProbeError::Io(e.to_string()))?;
        Ok(info)
    }

    pub fn dir(&self) -> &Path {
        &self.dir
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::asset::{AssetRef, ContentHash, StreamId};
    use crate::frame::{
        BitDepth, ColorTags, MatrixCoeffs, PixelFormat, Primaries, Range, Transfer,
    };

    fn fake_asset(data: &[u8]) -> AssetRef {
        AssetRef {
            content_hash: ContentHash::from_bytes(data),
            locator: "memory://fake".into(),
        }
    }

    fn fake_probe() -> ProbeInfo {
        ProbeInfo {
            container: ContainerKind::Mp4,
            duration: Some(Rational::new(8, 1)),
            streams: vec![ProbeStream {
                id: StreamId(0),
                kind: StreamKind::Video,
                codec: "mpeg4".into(),
                time_base: Rational::new(1, 24000),
                start_time: Some(Rational::new(0, 1)),
                duration: Some(Rational::new(8, 1)),
                avg_frame_rate: Some(Rational::new(24, 1)),
                nb_frames_hint: Some(192),
                video: Some(VideoDetails {
                    width: 320,
                    height: 240,
                    pixel_format: PixelFormat::Yuv420p,
                    bit_depth: BitDepth::B8,
                    color: ColorTags::unknown(),
                }),
                audio: None,
            }],
            keyframe_index: None,
            vfr: None,
        }
    }

    struct CountingBackend {
        calls: std::cell::Cell<u32>,
        probe: ProbeInfo,
    }

    impl ProbeBackend for CountingBackend {
        fn probe(&self, _a: &AssetRef) -> Result<ProbeInfo, ProbeError> {
            self.calls.set(self.calls.get() + 1);
            Ok(self.probe.clone())
        }
        fn keyframe_index(&self, _a: &AssetRef) -> Result<KeyframeIndex, ProbeError> {
            Ok(KeyframeIndex { entries: vec![] })
        }
    }

    #[test]
    fn cache_roundtrip_and_miss() {
        let dir = std::env::temp_dir().join(format!("ove_media_cache_{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        let cache = MetadataCache::new(&dir);
        let a = fake_asset(b"asset-a");
        assert!(cache.get(&a).is_none(), "empty cache must miss");
        let info = fake_probe();
        cache.put(&a, &info).unwrap();
        let got = cache.get(&a).unwrap();
        assert_eq!(got, info);
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn cache_corrupt_file_is_miss_not_error() {
        let dir = std::env::temp_dir().join(format!("ove_media_crc_{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        let cache = MetadataCache::new(&dir);
        let a = fake_asset(b"asset-b");
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(cache.path_for(&a), "{{{ not json").unwrap();
        assert!(cache.get(&a).is_none(), "corrupt cache entry = miss");
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn get_or_probe_hits_backend_once() {
        let dir = std::env::temp_dir().join(format!("ove_media_orp_{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        let cache = MetadataCache::new(&dir);
        let a = fake_asset(b"asset-c");
        let be = CountingBackend {
            calls: std::cell::Cell::new(0),
            probe: fake_probe(),
        };
        let p1 = cache.get_or_probe(&a, &be).unwrap();
        let p2 = cache.get_or_probe(&a, &be).unwrap();
        assert_eq!(p1, p2);
        assert_eq!(be.calls.get(), 1, "second get must be a cache hit");
        // different asset bytes => different key => backend called again
        let b = fake_asset(b"asset-d");
        // give b a color-tagged probe to also exercise serde of color enums
        let mut info_b = fake_probe();
        if let Some(v) = info_b.streams[0].video.as_mut() {
            v.color = ColorTags {
                primaries: Primaries::Bt709,
                transfer: Transfer::Bt709,
                matrix: MatrixCoeffs::Bt709,
                range: Range::Limited,
                chroma_loc: None,
            };
        }
        let be2 = CountingBackend {
            calls: std::cell::Cell::new(0),
            probe: info_b.clone(),
        };
        let pb = cache.get_or_probe(&b, &be2).unwrap();
        assert_eq!(pb, info_b);
        assert_eq!(be2.calls.get(), 1);
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn keyframe_index_floor_finds_landing_point() {
        let idx = KeyframeIndex {
            entries: vec![0, 24, 48, 72]
                .into_iter()
                .map(|n| KeyframeEntry {
                    pts: Rational::new(n, 24),
                    byte_pos: None,
                })
                .collect(),
        };
        // E-007's 1.5s trap: floor(1.5s) = 1.0s keyframe, NOT 2.0s
        assert_eq!(
            idx.floor(Rational::new(3, 2)).unwrap().pts,
            Rational::new(1, 1)
        );
        assert_eq!(
            idx.floor(Rational::new(0, 1)).unwrap().pts,
            Rational::new(0, 1)
        );
        assert!(idx.floor(Rational::new(-1, 24)).is_none());
    }
}
