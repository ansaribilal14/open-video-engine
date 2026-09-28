//! WAV output (W7, BUILD_PLAN wave 7: "mix → WAV/PCM out").
//!
//! Pure-Rust RIFF/WAVE writer — no libav, deterministic bytes. The engine's
//! audio assembly delivers planar f32 (the decode leg's canonical surface);
//! the WAV file is PCM16 LE with the pinned conversion:
//!
//!   s16 = round(clamp(x, -1.0, 1.0) × 32768).clamp(-32768, 32767)
//!
//! (the libav fltp→s16 convention; −1.0 maps to −32768, +1.0 clamps to
//! +32767 — round-half-away-from-zero via f32::round).
//!
//! Exactness: WAV PCM is sample-count exact by construction — the file
//! duration IS data_len / (rate × channels × 2) seconds; no container-level
//! rounding exists (the A/V mux is where sample grids matter, not WAV).

use std::path::Path;

use crate::EncodeError;

/// Write `planar` (one f32 plane per channel, equal lengths) as a PCM16 LE
/// WAV file. Returns the number of sample frames written (per channel).
pub fn write_wav_s16(
    path: &Path,
    planar: &[Vec<f32>],
    sample_rate: u32,
) -> Result<u64, EncodeError> {
    if planar.is_empty() || planar.len() > 2 {
        // v1: mono/stereo (the engine's audio assembly surface; ADR-018)
        return Err(EncodeError::InvalidConfig(format!(
            "WAV v1 supports 1..=2 channels, got {}",
            planar.len()
        )));
    }
    let channels = planar.len() as u16;
    let n = planar[0].len();
    if planar.iter().any(|p| p.len() != n) {
        return Err(EncodeError::InvalidConfig(
            "unequal plane lengths in audio assembly".into(),
        ));
    }
    if sample_rate == 0 {
        return Err(EncodeError::InvalidConfig("zero sample rate".into()));
    }
    let data_len = (n * planar.len() * 2) as u32;
    let byte_rate = sample_rate * channels as u32 * 2;
    let block_align = channels * 2;

    let mut out = Vec::with_capacity(44 + data_len as usize);
    out.extend_from_slice(b"RIFF");
    out.extend_from_slice(&(36 + data_len).to_le_bytes());
    out.extend_from_slice(b"WAVE");
    out.extend_from_slice(b"fmt ");
    out.extend_from_slice(&16u32.to_le_bytes()); // fmt chunk size
    out.extend_from_slice(&1u16.to_le_bytes()); // PCM
    out.extend_from_slice(&channels.to_le_bytes());
    out.extend_from_slice(&sample_rate.to_le_bytes());
    out.extend_from_slice(&byte_rate.to_le_bytes());
    out.extend_from_slice(&block_align.to_le_bytes());
    out.extend_from_slice(&16u16.to_le_bytes()); // bits per sample
    out.extend_from_slice(b"data");
    out.extend_from_slice(&data_len.to_le_bytes());

    // interleaved PCM16
    for i in 0..n {
        for p in planar {
            let x = p[i].clamp(-1.0, 1.0) * 32768.0;
            let s = (x.round() as i32).clamp(-32768, 32767) as i16;
            out.extend_from_slice(&s.to_le_bytes());
        }
    }

    std::fs::write(path, &out).map_err(|e| EncodeError::Io(e.to_string()))?;
    Ok(n as u64)
}
