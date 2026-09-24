# W3a_REPORT — Task 14-a (wave-3 research agent W3-a, doc 21 AUDIO)

Date: 2026-09-24 · Agent: W3-a · Scope: research + docs only (no engine code touched).

## Docs written

| file | lines | status |
|---|---|---|
| docs/research/21_AUDIO.md | 241 | v0.1 PARTIAL (replaces 9-line PLANNED stub; header `> Status: PARTIAL (v0.1 — wave-3 depth pass 2026-09-24).`) |
| research/sources/LEDGER_W3a.md | addendum | 10 rows, S-3a0..S-3a9 (SOURCE_LEDGER.md untouched) |
| research/sources/W3a_REPORT.md | this file | — |

## Key findings (verified unless noted)

1. **Core question resolved (§4 of doc 21): sample-accurate clock vs ADR-007 is a false
   conflict.** A sample clock is the rational tick axis projected onto the 1/fs lattice:
   `s = floor(ticks·fs/R)` in exact integer arithmetic; remainder = exact sub-sample offset.
   Rule R-AUDIO-2: fix project tick rate `R = multiple of lcm(fs_project, frame-rate numerators)`
   — verified examples: 48k+{24,25,30,50,60,23.976,29.97} → R=240,000; 44.1k+23.976 →
   R=3,528,000 (+29.97 → 17,640,000); i64 horizon ≈16k years.
2. **Frame↔sample alignment is rate physics, not software**: exact whole samples/frame iff
   `numf | fs` — verified table: 23.976@48k = 2002 ✓ but 23.976@44.1k = 1839.34 ✗ and
   29.97@48k = 1601.6 ✗. Engine consequence: one floor policy + exact remainders; never
   decimal-ize rates (matches E-002).
3. **Industry evidence for the hazard**: MLT's `mlt_audio_calculate_frame_samples(float fps, …)`
   carries float fps in its public interface (master header) [S-3a9]; Olive wraps AVRational
   exactly [S-2b6]. Media3's audio clock is reconstructed (AudioTrack.getTimestamp + smoothing)
   [S-3a9] → treat framework timestamps as estimates; master clock = our own frame counters.
4. **v1 Rust audio stack is permissive and maintained** (crates.io API, 2026-09-24): symphonia
   0.6.1 MPL-2.0 (decode; AAC-LC via flag, **HE-AAC absent**), cpal 0.18.2 Apache-2.0 (WASAPI/
   CoreAudio/ALSA + optional JACK/PipeWire/Pulse/ASIO + WASM WebAudio/audioworklet backends),
   rubato 5.0, ebur128 0.1.10 MIT (pure-Rust libebur128 port), rustfft, hound. dasp stale (2020).
5. **Licensing blockers identified (decision-critical, license files fetched)**: Rubber Band =
   GPL-2.0+ **plus mandatory commercial licence for non-GPL distribution** [S-3a5]; JUCE 9 =
   AGPLv3/commercial [S-3a4]; essentia = AGPL-3.0; aubio + audiowaveform = GPL-3.0;
   waveform-data.js = **LGPL-3.0** (not Apache as commonly assumed). Permissive winners:
   **signalsmith-stretch MIT** (time-stretch), rnnoise BSD-3 (denoise), DeepFilterNet MIT OR
   Apache-2.0 (voice isolation — model weights terms UNVERIFIED), ebur128 MIT (R128 loudness).
6. **Browser leg**: AudioDecoder/Encoder = Safari 26+ (BCD re-verified; Firefox Android absent);
   spec mandates AudioWorkletNode for sample-accurate disk/network playback (AudioBufferSourceNode
   is in-memory-only); render quantum = 128 frames; worklet `currentFrame` is an exact ULL
   sample-frame index — the natural bridge to ADR-007. OfflineAudioContext = adapter/sink only
   (cross-engine determinism UNVERIFIED, experiment proposed).
7. **Android leg**: OpenSL ES officially deprecated → Oboe (AAudio with OpenSL fallback);
   AAudio performance modes + burst-buffer latency model; NDK audio edge is integer frames —
   clean fit for the projection rule. Export stays Media3/Transformer (doc 15).
8. No dominant Rust-native beat/onset tracker exists (crates.io searches return pre-1.0 toys;
   aubio GPL-3.0, essentia AGPL-3.0) → beat analysis = out-of-core service.

## Decisions relevant to ADRs

- **Feed for a future ADR-012 (audio architecture)**: engine core = ove-audio (deterministic,
  device-free, f32 planar blocks, exact-time scheduler implementing §4 rules) + ove-audio-device
  (cpal) + platform adapters (browser AudioWorklet over WASM core; Android AAudio via JNI;
  desktop Tauri event-out for meters only — no samples over IPC). Preview and export share one
  mixer; OfflineAudioContext is never a second mixer.
- **R-AUDIO-1/2/3 proposed as the audio appendix of ADR-007** (no change to ADR-007 itself —
  this doc only adds the audio projection rules and the exactness table).
- **Licensing track (doc 35) inputs**: Rubber Band (commercial-only path), JUCE (AGPL),
  essentia (AGPL), aubio/audiowaveform (GPL) flagged BLOCKED/out-of-core; signalsmith-stretch +
  rnnoise + DeepFilterNet + ebur128 flagged permissive-OK. cpal ASIO SDK terms flagged for
  verification before any Windows low-latency promise.
- **E-008 proposed** (device-latency probe matrix: cpal hosts, AAudio, AudioWorklet) and a
  WebAudio cross-engine OfflineAudioContext determinism experiment.

## NOT-FOUND / failures logged (honesty log)

- `libsoundio` crate: crates.io 404 — only stale bindings (`soundio` 0.2.1, `libsoundio-sys`).
- `bliss-audio-aubio` crate name: crates.io 404 (crate exists under a different name; not pursued).
- DeepFilterNet at `breizhn/DeepFilterNet`: GitHub 404 (my initial owner guess was wrong) —
  located via GitHub search as **Rikorose/DeepFilterNet** (4,746★); license fetched there.
- SoundTouch COPYING: 3 wrong codeberg paths (source/SoundTouch/COPYING.TXT, branch main,
  root COPYING) before hitting `raw/branch/master/COPYING.TXT`.
- signalsmith-stretch: LICENSE/COPYING at repo root 404; found `LICENSE.txt` via contents API.
- JUCE: `LICENSE` (no ext) 404 on master/develop; `LICENSE.md` on master = 200 (JUCE 9, AGPLv3).
- AAudio guide at `/ndk/guides/audio/aaudio`: 404; correct nested URL `/ndk/guides/audio/aaudio/aaudio`.
- Olive audio file `core/audio/audiovisualwaveform.h` @master: 404 → Olive audio-chunking
  specifics left UNVERIFIED (Olive rational-time claim already covered by S-2b6).
- developer.android.com/ndk/guides/audio/aaudio first fetch returned 404-with-HTML (redirect
  target found from audio-latency page nav).
- essentia COPYING at `COPYING` (no ext): 404 → `COPYING.txt` = 200.

## Source IDs used

New rows: S-3a0 (crates.io sweep), S-3a1 (Web Audio spec), S-3a2 (MDN BCD audio), S-3a3
(Android audio docs), S-3a4 (cpal README + JUCE 9 LICENSE), S-3a5 (Rubber Band), S-3a6
(SoundTouch + signalsmith-stretch), S-3a7 (rnnoise + DeepFilterNet), S-3a8 (essentia/aubio/
audiowaveform/waveform-data.js/libebur128/libsoundio), S-3a9 (MLT mlt_audio.h + Media3
AudioTrackPositionTracker).
Cross-referenced existing ledger rows (not modified): S-2b6 (Olive rational.h), S-2d0 (MDN BCD,
3-d), S-2e0 (Media3, 3-e), S-4f-x (librosa ISC, 3-f); ADR-007; docs 03/05/06/07/11/15/17/20;
E-002/E-002c.
