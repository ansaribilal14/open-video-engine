//! ove-media ↔ ove-decode integration: probe + keyframe index + VFR report
//! against committed golden tables (DECODER_SPEC §4: ove-media owns the
//! probe/keyframe index data, the libav backend fills it).

use std::path::{Path, PathBuf};

use ove_decode::ffmpeg::FfmpegProbe;
use ove_media::{AssetRef, ContainerKind, ProbeBackend, StreamKind};
use ove_time::Rational;
use serde_json::Value;

fn media(name: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests/media")
        .join(name)
}

fn asset(name: &str) -> AssetRef {
    AssetRef::from_path(media(name)).expect("corpus file present")
}

fn golden(name: &str) -> Value {
    let p = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests/media/golden")
        .join(name);
    serde_json::from_str(&std::fs::read_to_string(p).expect("golden table committed")).unwrap()
}

fn parse_rate(s: &str) -> (i64, i64) {
    let (a, b) = s.split_once('/').expect("rate string");
    (a.parse().unwrap(), b.parse().unwrap())
}

#[test]
fn probe_cfr24_matches_ffprobe() {
    let info = FfmpegProbe.probe(&asset("cfr24.mp4")).unwrap();
    assert_eq!(info.container, ContainerKind::Mp4);
    assert_eq!(info.streams.len(), 1);
    let s = &info.streams[0];
    assert_eq!(s.kind, StreamKind::Video);
    assert_eq!(s.codec, "mpeg4");
    assert_eq!(s.time_base, Rational::new(1, 24000));
    assert_eq!(s.nb_frames_hint, Some(192));
    assert_eq!(s.avg_frame_rate, Some(Rational::new(24, 1)));
    let v = s.video.as_ref().unwrap();
    assert_eq!(v.width, 320);
    assert_eq!(v.height, 240);
    // serde roundtrip of the whole probe (cache serialization honesty)
    let text = serde_json::to_string(&info).unwrap();
    let back: ove_media::ProbeInfo = serde_json::from_str(&text).unwrap();
    assert_eq!(back, info);
}

#[test]
fn probe_vidaud_sees_audio_stream() {
    let info = FfmpegProbe.probe(&asset("vidaud.mp4")).unwrap();
    assert_eq!(info.streams.len(), 2);
    let a = &info.streams[1];
    assert_eq!(a.kind, StreamKind::Audio);
    let ad = a.audio.as_ref().unwrap();
    let v = golden("vidaud.ffprobe.json");
    // compare against the committed ffprobe table, not hardcoded guesses
    let sr: u64 = v["streams"][1]["sample_rate"]
        .as_str()
        .unwrap()
        .parse()
        .unwrap();
    assert_eq!(ad.sample_rate as u64, sr);
}

#[test]
fn keyframe_index_matches_packet_truth() {
    let idx = FfmpegProbe.keyframe_index(&asset("cfr24.mp4")).unwrap();
    let p = golden("cfr24.packets.json");
    let tb = parse_rate(
        golden("cfr24.ffprobe.json")["streams"][0]["time_base"]
            .as_str()
            .unwrap(),
    );
    let expected: Vec<Rational> = p["packets"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|pk| pk["flags"].as_str().unwrap_or("").contains('K'))
        .map(|pk| {
            let t = pk["pts"].as_i64().unwrap();
            Rational::new(t * tb.0, tb.1)
        })
        .collect();
    let got: Vec<Rational> = idx.entries.iter().map(|e| e.pts).collect();
    assert_eq!(got, expected, "keyframe index == packet K flags");
    assert!(idx.entries.iter().all(|e| e.byte_pos.is_some()));
    // deterministic across scans
    let idx2 = FfmpegProbe.keyframe_index(&asset("cfr24.mp4")).unwrap();
    assert_eq!(idx, idx2);
}

#[test]
fn probe_full_vfr_report() {
    let info = FfmpegProbe.probe_full(&asset("vfr.mp4")).unwrap();
    let vfr = info.vfr.as_ref().expect("vfr report present");
    assert!(vfr.is_vfr);
    assert_eq!(vfr.frame_count, 10);
    assert!(vfr.distinct_deltas.contains(&Rational::new(1, 30)));
    assert!(vfr.distinct_deltas.contains(&Rational::new(1, 15)));
    let idx = info.keyframe_index.as_ref().unwrap();
    assert!(!idx.is_empty());
    // vfr.mp4 keyframes: pts 0 and 21000/90000 = 7/30 s (g=5 at output frame 5)
    assert_eq!(
        idx.floor(Rational::new(3, 2)).unwrap().pts,
        Rational::new(7, 30)
    );
    // floor() landing on cfr24 (keyframes every second): the E-007 1.5s trap
    let idx24 = FfmpegProbe.keyframe_index(&asset("cfr24.mp4")).unwrap();
    assert_eq!(
        idx24.floor(Rational::new(3, 2)).unwrap().pts,
        Rational::new(1, 1)
    );
}

#[test]
fn probe_garbage_is_typed_error() {
    let e = FfmpegProbe
        .probe(&asset("garbage.mp4"))
        .expect_err("garbage fails");
    assert!(
        matches!(
            e,
            ove_media::ProbeError::Unreadable | ove_media::ProbeError::Corrupt(_)
        ),
        "got {e:?}"
    );
}

#[test]
fn metadata_cache_end_to_end() {
    let dir = std::env::temp_dir().join(format!("ove_decode_cache_{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    let cache = ove_media::MetadataCache::new(&dir);
    let a = asset("cfr24.mp4");
    let backend = FfmpegProbe;
    let p1 = cache.get_or_probe(&a, &backend).unwrap();
    let p2 = cache.get_or_probe(&a, &backend).unwrap();
    assert_eq!(p1, p2, "second read served from cache");
    // cache file keyed by content hash
    assert!(dir
        .join(format!("{}.probe.json", a.content_hash.hex()))
        .exists());
    let _ = std::fs::remove_dir_all(&dir);
}
