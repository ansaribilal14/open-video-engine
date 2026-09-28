//! DECODER_SPEC §2 audio conformance suite (A-1..A-4, WAVE 7, ADR-018).
//!
//! The decoded-audio contract: canonical planar-f32 ("fltp") delivery at the
//! SOURCE rate/layout (swresample format conversion only — never a resample,
//! so the sample count is preserved exactly), per-frame duration =
//! nb_samples/rate EXACT, contiguous pts (no gaps), and the audio floor+trim
//! seek policy (frames straddling an Exact target are delivered whole; the
//! caller trims the exact sample in-point).
//!
//! Corpus: `vidaud.mp4` (mpeg4 video + AAC 44.1 kHz mono, 1.0 s, committed by
//! `scripts/corpus/gen_corpus.py`). Decoded AAC facts pinned here are
//! deterministic across libav 7.1.x (same major.minor pin as the decode
//! goldens; see DEV_ENV.md) and carry the libav identity in failure messages.

use std::path::{Path, PathBuf};

use ove_decode::ffmpeg::FfmpegSwDecoder;
use ove_decode::{DecodeConfig, Decoder, SeekMode};
use ove_media::{AssetRef, FrameKind, StreamId};
use ove_time::Rational;

const RATE: i64 = 44100;

fn media(name: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests/media")
        .join(name)
}

fn audio_decoder() -> FfmpegSwDecoder {
    let asset = AssetRef::from_path(media("vidaud.mp4")).expect("corpus present");
    FfmpegSwDecoder::open(&asset, StreamId(1), DecodeConfig::default())
        .expect("audio stream opens (W7)")
}

/// Decode the whole audio stream; return (frames, frames as
/// (pts, nb_samples) tuples).
fn decode_all() -> Vec<(Rational, i64)> {
    let mut dec = audio_decoder();
    let mut out = Vec::new();
    while let Some(f) = dec.next().expect("decode is a value, never a panic") {
        let nb = f.duration * Rational::new(RATE, 1);
        assert_eq!(nb.den(), 1, "nb_samples must be an exact integer count");
        out.push((f.pts, nb.num()));
    }
    out
}

// ---------------------------------------------------------------------------
// A-1 — sample-count authority: decode total, priming trim, tail padding
// ---------------------------------------------------------------------------

#[test]
fn a1_sample_count_authority() {
    let frames = decode_all();
    let total: i64 = frames.iter().map(|(_, nb)| *nb).sum();
    // AAC: every decoded frame carries exactly frame_size (1024) samples for
    // this corpus; the container says 1.0 s = 44100 samples.
    // * priming is trimmed by the demuxer/decoder (skip_samples): first pts 0
    // * the tail is NOT sample-trimmed: the last frame extends past 44100 up
    //   to the AAC frame grid — callers trim to the exact duration (ADR-018).
    assert_eq!(first_pts(&frames), Rational::new(0, 1), "priming trimmed");
    assert_eq!(total, 44 * 1024, "44 AAC frames × 1024 samples");
    assert!(
        total >= 44100 && total - 44100 < 1024,
        "decoded total {total} exceeds container 44100 by tail padding only (< frame_size)"
    );
}

fn first_pts(frames: &[(Rational, i64)]) -> Rational {
    frames.first().expect("non-empty stream").0
}

// ---------------------------------------------------------------------------
// A-2 — per-frame invariants: exact duration, contiguity, monotonicity
// ---------------------------------------------------------------------------

#[test]
fn a2_frame_continuity_and_exact_duration() {
    let frames = decode_all();
    assert_eq!(frames.len(), 44);
    let mut prev_end: Option<Rational> = None;
    for (i, (pts, nb)) in frames.iter().enumerate() {
        // duration = nb/rate EXACT (the envelope constructor derives it; the
        // container's per-frame duration field is never trusted for audio)
        let expected_dur = Rational::new(*nb, RATE);
        let dur = expected_dur;
        if let Some(end) = prev_end {
            assert_eq!(*pts, end, "gap before frame {i}: pts must be contiguous");
        }
        assert!(dur.num() > 0, "positive duration");
        let _ = dur;
        prev_end = Some(*pts + expected_dur);
    }
    // strict monotonicity
    for w in frames.windows(2) {
        assert!(w[1].0 > w[0].0, "pts strictly increasing");
    }
}

// ---------------------------------------------------------------------------
// A-3 — audio seek semantics: floor frame STRADDLES the Exact target; the
// caller-computed sample in-point is an exact integer
// ---------------------------------------------------------------------------

#[test]
fn a3_exact_seek_floor_and_trim_inpoint() {
    let mut dec = audio_decoder();
    let target = Rational::new(1, 2); // 0.5 s = 22050 samples — mid-frame
    let landed = dec
        .seek(target, SeekMode::Exact)
        .expect("mid-stream seek lands");
    assert_eq!(landed, target, "Exact seek reports the requested pts");
    let f = dec.next().expect("decode").expect("frame at/after target");
    let end = f.pts + f.duration;
    assert!(
        f.pts <= target && target < end,
        "floor frame must STRADDLE the target (pts {} ≤ 1/2 < end {})",
        f.pts,
        end
    );
    // the in-point inside the straddling frame is an exact sample index
    let sample_in = (target - f.pts) * Rational::new(RATE, 1);
    assert_eq!(
        sample_in.den(),
        1,
        "sample in-point must be an exact integer, got {sample_in}"
    );
    assert!(sample_in.num() >= 0);
    // and decoding continues contiguously from the straddling frame
    let f2 = dec.next().expect("decode").expect("next frame");
    assert_eq!(f2.pts, end, "contiguity after seek");
}

// ---------------------------------------------------------------------------
// A-4 — canonical payload: planar f32 layout + committed decode golden
// ---------------------------------------------------------------------------

#[test]
fn a4_canonical_fltp_payload_and_golden() {
    let mut dec = audio_decoder();
    let f = dec.next().expect("decode").expect("first frame");
    assert!(matches!(
        f.kind,
        FrameKind::Audio {
            sample_rate: 44100,
            channels: 1
        }
    ));
    let fb = f.cpu_bytes().expect("cpu payload");
    let nb = f.duration * Rational::new(RATE, 1);
    assert_eq!(fb.strides.len(), 1, "one plane per channel (planar)");
    assert_eq!(fb.strides[0], nb.num() as usize * 4, "compact plane stride");
    assert_eq!(fb.data.len(), fb.strides[0], "mono: single plane");

    // committed decode golden: the first four decoded f32 samples, pinned
    // bit-exactly (deterministic across libav 7.1.x for the committed
    // corpus; the payload is the lossy DECODED waveform — decode-level
    // determinism, not input reconstruction).
    let mut got: Vec<u32> = Vec::with_capacity(4);
    for i in 0..4 {
        let c: [u8; 4] = fb.data[i * 4..i * 4 + 4].try_into().expect("4 bytes");
        got.push(u32::from_le_bytes(c));
    }
    let golden: [u32; 4] = [0x3ada_a2a1, 0x3beb_e8f1, 0x3c74_eb0d, 0x3cbf_9d38];
    assert_eq!(
        &got[..4],
        &golden[..],
        "decoded sample bits drifted (libav {:?}) — decode is version-pinned",
        libav_identity()
    );
}

fn libav_identity() -> String {
    // best-effort identity string for failure messages only
    std::process::Command::new("ffmpeg")
        .arg("-version")
        .output()
        .ok()
        .map(|o| {
            String::from_utf8_lossy(&o.stdout)
                .lines()
                .next()
                .map(String::from)
                .unwrap_or_default()
        })
        .unwrap_or_else(|| "unknown".into())
}
