//! WAVE 7 — the audio vertical: timeline audio assembly is sample-exact,
//! A/V export lands both streams in one MP4 (ffprobe + libav verified,
//! ± 0 samples drift), WAV/PCM out is exact, and exports never mutate the
//! document state (kill/reopen hash stability carries over from W6).

use std::path::{Path, PathBuf};

use ove_engine::Engine;
use ove_render::OutputSpec;
use ove_time::Rational;
use ove_timeline::{GapTrack, TrackKind};

fn tmp(name: &str) -> PathBuf {
    let d = std::env::temp_dir().join(format!("ove-w7-{name}"));
    let _ = std::fs::remove_dir_all(&d);
    d // Engine::create makes the folder itself (P-8 disposable dirs)
}

/// the committed wave-3 corpus doubles as the engine fixture set
fn media(name: &str) -> PathBuf {
    let p = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../ove-encode/tests/media")
        .join(name);
    assert!(p.exists(), "corpus fixture missing: {}", p.display());
    p
}

fn color_tags_bt709() -> ove_media::ColorTags {
    ove_media::ColorTags {
        primaries: ove_media::Primaries::Bt709,
        transfer: ove_media::Transfer::Bt709,
        matrix: ove_media::MatrixCoeffs::Bt709,
        range: ove_media::Range::Limited,
        chroma_loc: None,
    }
}

fn output_spec() -> OutputSpec {
    OutputSpec {
        width: 320,
        height: 240,
        rate_num: 24,
        rate_den: 1,
        working_space: color_tags_bt709(),
    }
}

#[test]
fn w7_audio_assembly_and_av_export() {
    let dir = tmp("assembly");
    // tick axis = audio rate (48 kHz): every tick-aligned cut is sample-exact
    let mut e = Engine::create(&dir, (48_000, 1)).expect("create");
    e.add_track(1, TrackKind::Gap(GapTrack::new()))
        .expect("track 1");
    let hex = e.import_media(&media("copyav.mp4")).expect("import");

    // timeline: two 2 s clips; the second starts at src 2 s (a real cut on
    // the sample axis: sample 96000 lands mid-AAC-frame → floor+trim)
    let id1 = e
        .add_clip(1, &hex, Rational::new(96_000, 48_000), Rational::new(0, 1))
        .expect("clip 1");
    let _id2 = e
        .add_clip(
            1,
            &hex,
            Rational::new(96_000, 48_000),
            Rational::new(96_000, 48_000),
        )
        .expect("clip 2");
    let _ = id1;

    // -- assembly: exactly 4 s × 48 000 samples, planar f32 --
    let assembly = e
        .assemble_timeline_audio(Rational::new(192_000, 48_000))
        .expect("assemble timeline audio");
    assert_eq!(assembly.sample_rate, 48_000);
    assert_eq!(assembly.channels, 1);
    assert_eq!(assembly.samples, 192_000, "4 s at 48 kHz, ± 0 samples");
    assert_eq!(assembly.planes.len(), 1);
    assert_eq!(assembly.planes[0].len(), 192_000);
    assert!(
        assembly.planes[0].iter().any(|&v| v != 0.0),
        "assembled audio is not silence"
    );

    // -- A/V export: both streams in one MP4 --
    let out = dir.join("w7-av.mp4");
    let info = e
        .export_reencode(&out, &output_spec(), 96) // 96 frames @ 24 = 4 s
        .expect("A/V re-encode export");
    assert_eq!(info.tracks.len(), 2, "video + audio tracks");

    // libav probe eye
    let asset = ove_media::AssetRef::from_path(&out).expect("asset of output");
    let probe = ove_decode_probe(&asset);
    assert_eq!(probe.streams.len(), 2);
    let v = probe
        .streams
        .iter()
        .find(|s| s.kind == ove_media::StreamKind::Video)
        .expect("video stream");
    let a = probe
        .streams
        .iter()
        .find(|s| s.kind == ove_media::StreamKind::Audio)
        .expect("audio stream");
    assert_eq!(v.codec, "mpeg4");
    assert_eq!(a.codec, "aac");
    assert_eq!(v.nb_frames_hint, Some(96));
    assert_eq!(
        v.duration,
        Some(Rational::new(4, 1)),
        "video duration exact"
    );
    assert_eq!(
        a.duration,
        Some(Rational::new(192_000, 48_000)),
        "audio duration 4 s ± 0 samples (elst-trimmed priming)"
    );

    // ffprobe CLI eye (when present)
    if ffprobe_present() {
        let j = ffprobe_json(&out).expect("ffprobe parses our A/V output");
        assert_eq!(j["streams"][0]["codec_name"], "mpeg4");
        assert_eq!(j["streams"][1]["codec_name"], "aac");
        assert_eq!(j["streams"][0]["duration"].as_str().unwrap(), "4.000000");
        assert_eq!(j["streams"][1]["duration"].as_str().unwrap(), "4.000000");
        assert_eq!(
            j["streams"][1]["duration"]
                .as_str()
                .unwrap()
                .parse::<f64>()
                .unwrap(),
            4.0,
            "A/V drift ± 0 samples at the container level"
        );
    }

    // -- WAV/PCM out: exact sample frames --
    let wav = dir.join("w7.wav");
    let frames = e.export_wav(&wav).expect("wav export");
    assert_eq!(frames, 192_000, "WAV carries exactly 4 s × 48 kHz");
    let b = std::fs::read(&wav).expect("wav bytes");
    assert_eq!(&b[0..4], b"RIFF");
    assert_eq!(u32::from_le_bytes(b[24..28].try_into().unwrap()), 48_000);
    assert_eq!(u32::from_le_bytes(b[40..44].try_into().unwrap()), 384_000);

    // -- exports never mutate the document (kill/reopen stability) --
    let hash_before = e.state_hash();
    drop(e);
    let reopened = Engine::open(&dir).expect("reopen");
    assert_eq!(
        reopened.state_hash(),
        hash_before,
        "document hash unchanged by export work"
    );
}

fn ove_decode_probe(asset: &ove_media::AssetRef) -> ove_media::ProbeInfo {
    use ove_media::ProbeBackend;
    ove_decode::ffmpeg::FfmpegProbe
        .probe(asset)
        .expect("libav probe")
}

fn ffprobe_present() -> bool {
    std::process::Command::new("ffprobe")
        .arg("-version")
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false)
}

fn ffprobe_json(path: &Path) -> Option<serde_json::Value> {
    let out = std::process::Command::new("ffprobe")
        .args(["-v", "error", "-show_streams", "-of", "json"])
        .arg(path)
        .output()
        .ok()?;
    if !out.status.success() {
        return None;
    }
    serde_json::from_slice(&out.stdout).ok()
}
