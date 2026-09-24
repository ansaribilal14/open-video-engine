# 21_AUDIO — Audio

> Status: PARTIAL (v0.1 — wave-3 depth pass 2026-09-24).

> Owner: agent W3-a · Track: H · Date: 2026-09-24
> Scope: Rust audio crate survey; per-platform legs; the core question — sample-accurate clock vs
> ADR-007 rational ticks (mapping rule + exactness analysis); DSP feature matrix; decision-critical
> licensing; ove-audio sketch; leg comparison. Every claim is VERIFIED (source ID) or UNVERIFIED.

## 1. Frame of reference

Fixed upstream: ADR-007 (exact i64 rational ticks; fp display-only), E-002 (fp drift proven;
sample-boundary census 178 floor mismatches), E-002c (fixed per-project tick axis; free rationals
overflow after ~8–10 coprime-den additions), arch C (Rust core + Tauri/Android/WASM adapters)
[ADR-007, docs 18/20]. Audio slots into that shape, not against it.

## 2. Rust audio crate survey (crates.io API, fetched 2026-09-24) [S-3a0]

| crate | ver | license | last release | role for OVE |
|---|---|---|---|---|
| symphonia | 0.6.1 | MPL-2.0 | 2026-08-13 | pure-Rust decode: MP3, AAC-LC (flag), FLAC, Vorbis, ALAC, ADPCM; demux MP4/MKV/OGG/WAV |
| cpal | 0.18.2 | Apache-2.0 | 2026-08-16 | device I/O (hosts in §3.3) — the device layer |
| rodio | 0.22.2 | MIT OR Apache-2.0 | 2026-03-05 | playback graph; convenient, coarse control — thin apps |
| rubato | 5.0.0 | MIT OR Apache-2.0 | 2026-08-10 | async resampler — the ingest path (44.1↔48k) |
| fundsp | 0.23.0 | MIT OR Apache-2.0 | 2026-01-07 | DSP graph/synth; unit-based, research-flavored |
| dasp | 0.11.0 | MIT OR Apache-2.0 | 2020-05-29 | sample/Signal primitives; **stale (6 yrs)** — borrow ideas, don't depend |
| hound | 3.5.1 | Apache-2.0 | 2023-09-25 | WAV read/write — peaks cache + test fixtures |
| rustfft | 6.4.1 | MIT OR Apache-2.0 | 2025-09-18 | FFT for spectrogram/analysis |
| ebur128 | 0.1.10 | MIT | 2024-10-26 | **pure-Rust port** of libebur128 (C, MIT), same results [S-3a8] — R128 loudness |
| oxisynth | 0.1.0 | LGPL-2.1 | 2025-05-25 | Rust FluidSynth port — only if MIDI soundtracks; LGPL + young |
| fluidlite | 0.2.1 | LGPL-2.1 | 2021-08-21 | alternative SF2 synth; stale |
| timestretch | 0.15.0 | MIT | 2026-09-02 | pure-Rust time-stretch exists (EDM-optimized); maturity unassessed |
| soundio / libsoundio-sys | 0.2.1 / 0.3.0 | MIT | 2020 | libsoundio bindings, stale; libsoundio itself MIT [S-3a8] — cpal wins |

VERIFIED absence: **no `libsoundio` crate** exists (crates.io 404) — only stale bindings [S-3a0];
game-audio graphs kira 0.12.4 / oddio 0.7.4 (MIT OR Apache-2.0) = pattern reference only [S-3a0].
Verdict: decode=symphonia, device=cpal, resample=rubato, loudness=ebur128, FFT=rustfft — a
permissive, maintained v1 stack; the DSP mix graph goes in-house on plain f32 buffers.

## 3. Per-platform legs

### 3.1 Browser — WebAudio / AudioWorklet [S-3a1, S-3a2]

- Render quantum: default **128 frames**; `"hardware"` mode lets the UA pick (fingerprinting note
  in spec) [S-3a1]; native-graph scheduling granularity quantizes to this. Clock: `currentTime` is
  a **double** in seconds, advanced per render block; the exact counter is
  `AudioWorkletGlobalScope.currentFrame` — an **unsigned long long sample-frame index** equal to
  the context's `[[current frame]]` slot [S-3a1]. The web therefore exposes an integer sample-frame
  clock inside worklets — maps 1:1 onto §4's projection rule.
- Spec (relevant verbatim): AudioBufferSourceNode targets in-memory assets; "If sample-accurate
  playback of network- or disk-backed assets is required, an implementer should use
  AudioWorkletNode to implement playback" [S-3a1]. → OVE preview audio = AudioWorklet fed with
  PCM from the WASM core; AudioBufferSourceNode is a fallback only.
- `OfflineAudioContext.startRendering() -> Promise<AudioBuffer>` (`length` in frames) is the
  built-in mixdown render [S-3a1]; our deterministic export must run the same core DSP as desktop —
  OfflineAudioContext is an adapter/sink, never a second mixer (determinism UNVERIFIED — §10).
- Safari gap (BCD, re-verified today): `AudioDecoder`/`AudioEncoder` = **Safari 26+ only**
  (Chrome 94+, Firefox 130+, Firefox Android absent); `decodeAudioData` = everywhere (Safari 6+);
  `AudioWorkletNode` = Chrome 66 / FF 76 / Safari 14.1 [S-3a2; cross-check S-2d0/doc 11].
  → Safari <26 bridge = WebAudio `decodeAudioData`; no WebCodecs audio at all there. Firefox
  Android also lacks WebCodecs audio; its mobile-web path = decodeAudioData + worklet (latency
  on old Android WebView UNVERIFIED).

### 3.2 Android — AAudio / OpenSL ES / AudioTrack [S-3a3, S-3a9]

- **OpenSL ES is deprecated** — NDK page warns verbatim: "OpenSL ES is deprecated. Developers
  should use the open source Oboe library… Oboe calls AAudio when AAudio is available, and falls
  back to OpenSL ES if AAudio is not available" [S-3a3].
- AAudio (NDK, C) is the native path: performance modes NONE / LOW_LATENCY ("smaller buffers and
  an optimized data path") / POWER_SAVING; latency tuned via buffer-size-in-frames over "bursts"
  [S-3a3]. AAudio exposes **integer frame positions** — clean fit for §4 (no fp at the NDK edge).
- Latency model (official guidance): output+input ≈ half the round-trip latency; "fast track"
  check via `dumpsys media.audio_flinger` (F flag); first-enqueue warmup exists — prime with
  silence [S-3a3]. Device spread is wide (20–90 ms round-trip class is a framing, UNVERIFIED).
- AudioTrack (framework) is what Media3 uses. Media3's clock, verified source:
  `AudioTrackPositionTracker` prefers `AudioTrack.getTimestamp` playback timestamps, else derives a
  **smoothed** position from track frame positions, with speed-change smoothing + latencyUs
  adjustment [S-3a9]. Lesson: Android's audio clock is *reconstructed and smoothed* — trust frame
  counters we write (AAudio), treat framework timestamps as smoothed estimates. Export stays
  Media3/Transformer for the Android MVP (doc 15); ove-audio serves preview + other exports.

### 3.3 Desktop — Tauri/wgpu shell [S-3a4]

- cpal host table (README, verified): default hosts = **WASAPI (Windows) / CoreAudio (macOS, iOS,
  tvOS, visionOS) / ALSA (Linux, BSD)**; optional JACK, PipeWire, PulseAudio (Linux), ASIO
  (Windows), JACK (macOS); Linux needs `libasound2-dev`; ALSA underlies even JACK/PipeWire/Pulse
  use [S-3a4]. cpal ≥0.18 also ships WASM backends: WebAudio, `audioworklet` (nightly; needs
  atomics+SAB), emscripten, wasip1 — one device API covers desktop + browser.
- Tauri 2 has no audio layer (doc 17): a cpal thread owned by the Rust core; UI gets
  metering/peaks via events, never samples over IPC (same rule as video frames, E-006).
- JUCE is the heavyweight alternative — **JUCE 9 is dual AGPLv3 / commercial EULA** (LICENSE.md,
  verified) [S-3a4] → study reference only, never linked into the engine.

## 4. Core question — sample-accurate clock vs ADR-007 rational ticks

**Resolution: no conflict. A sample-accurate clock is the exact projection of the rational tick
axis onto the 1/fs sample lattice.** Audio adds one integer lattice (samples) beside frames; ADR-007
already handles lattices. Rules (to encode in ove-time/ove-audio):

- **R-AUDIO-1 (projection)**: instant `t = ticks/R` (R = project tick rate, ticks/s) at sample
  rate `fs` → `s = floor(ticks · fs / R)`, computed in i128/i64 — exact integer floor division, no
  fp. The remainder `r = (ticks·fs) mod R` is the *exact* sub-sample offset `r/R` s (≤ 20.8 µs
  @48k); carry it as a rational where phase matters, drop it otherwise — always deterministically.
  Range rendering: `n = floor(t1·fs/R) − floor(t0·fs/R)`.
- **R-AUDIO-2 (tick-rate choice)**: fix the project tick rate
  `R := multiple of lcm(fs_project, {numf of every project frame rate})` (numf = fps numerator,
  e.g. 24000 for 24000/1001). Then every frame edge of those rates is an exact tick (frame k at
  `k·denf·R/numf` — integer iff `numf | R`, since gcd(numf,denf)=1), and sample boundaries are
  exact ticks (`s·R/fs` integer iff `fs | R`). VERIFIED examples: fs=48000 + {24, 24000/1001,
  25, 30, 30000/1001, 50, 60} → R = **240,000**; fs=44100 + 24000/1001 → **3,528,000**; adding
  30000/1001 → **17,640,000**. i64 overflow horizon at R=17.64M ≈ 16,000 years — non-issue.
- **R-AUDIO-3 (non-alignment is physical, not a bug)**: a frame spans `fs·denf/numf` samples —
  an integer iff `numf | fs`. VERIFIED exactness table (✓ = whole samples/frame):

| fps | 44100 | 48000 | 96000 |
|---|---|---|---|
| 24 | 1837.5 ✗ | 2000 ✓ | 4000 ✓ |
| 25 | 1764 ✓ | 1920 ✓ | 3840 ✓ |
| 30 | 1470 ✓ | 1600 ✓ | 3200 ✓ |
| 50 / 60 | 882 / 735 ✓ | 960 / 800 ✓ | 1920 / 1600 ✓ |
| 23.976 (24000/1001) | 1839.34 ✗ | 2002 ✓ | 4004 ✓ |
| 29.97 (30000/1001) | 1471.47 ✗ | 1601.6 ✗ | 3203.2 ✗ |

  Even with R per R-AUDIO-2, a 23.976 frame edge at 44.1k sits at tick 147,147 with ticks/sample
  = 80 → remainder 27: the edge is *not* on a sample — correct, since 1839.34 samples is not an
  integer. Consequence: floor policy + exact remainders; never "fix" by re-deriving rates as
  decimals (E-002's 23.976-vs-24000/1001 drift lesson).
- **Ingest**: audio at fs ≠ fs_project is resampled once (rubato) at import, then lives on the
  fixed axis (mirrors ADR-007's convert-at-ingest rule for video PTS).
- **Playback clock direction**: the audio device is master. Device frames played `s` map back
  exactly: `ticks = s·R/fs` (integer if `fs | R`; else the exact rational `(s·R, fs)` per ADR-007's
  source-provenance rule). Video/GPU schedules against it — never the reverse, never fp. WebAudio's
  fp `currentTime` is accepted only as a display/estimate; inside worklets use the integer
  `currentFrame` [S-3a1]. Media3's smoothed AudioTrack timestamps are the same idea in Java [S-3a9].
- Industry evidence: MLT's `mlt_audio_calculate_frame_samples(float fps, int frequency,
  int64_t position)` and `mlt_audio_calculate_samples_to_position(float fps, …)` — **float fps in
  the interface** (current master header) [S-3a9] — exactly the fp input ADR-007 forbids; Olive
  wraps AVRational exactly [S-2b6, doc 20]. Kdenlive/Shotcut delegate the whole question to MLT's
  consumer pull (docs 03/07).

## 5. DSP feature matrix

| feature | engine v1 | later | out-of-scope (engine core) |
|---|---|---|---|
| decode (compressed→f32) | symphonia + platform decoders (WebCodecs/decodeAudioData, MediaCodec) | exotic codecs via plugin | — |
| resample | rubato (ingest-only per §4) | device-rate adaptive | — |
| gain / pan / mute / solo / fades / equal-power crossfade | yes, in core | — | — |
| mix bus (sum + clip policy + peak/RMS metering) | yes, deterministic pull | loudness-guided | — |
| waveform peaks (min/max buckets, cached per clip+zoom) | yes, own bucketing | multi-res .dat-style cache | — |
| loudness normalize (EBU R128) | — | ebur128 crate (MIT) [S-3a8] | broadcast mastering |
| EQ / simple comp / limiter | — | biquad set in core (no license risk) | full dynamics suite |
| time-stretch / pitch-shift | — | signalsmith-stretch (MIT, C++11 header) [S-3a6] | — |
| noise reduction | — | rnnoise (BSD-3) optional module [S-3a7] | — |
| voice isolation / ML "Enhance"-class | — | sidecar: DeepFilterNet (MIT OR Apache-2.0) [S-3a7] | Adobe-class quality bar |
| spectrogram | — | rustfft → GPU texture (doc 08 pass-graph) | — |
| beat detection / music analysis | — | analysis service (librosa ISC per S-4f ledger, or essentia) | in-core realtime |

Rationale: v1 = everything a cut/edit needs, fully permissive + deterministic; license-clean DSP
(ebur128, signalsmith, rnnoise, DFN) lands as *modules* behind a narrow f32-block trait; ML-heavy
features are runtime-heavy and stay out of the deterministic core.

## 6. Licensing table (decision-critical — license files fetched 2026-09-24)

| library | license (verified) | source | engine implication |
|---|---|---|---|
| Rubber Band | GPL-2.0-or-later; **commercial licence mandatory for non-GPL distribution** (README §Licence; terms at breakfastquay.com/rubberband) | [S-3a5] | BLOCKED for permissive core; buy license or avoid |
| SoundTouch | LGPL-2.1 (codeberg COPYING.TXT) | [S-3a6] | OK as optional dynamic module; prefer MIT alternative |
| signalsmith-stretch | MIT ("Copyright (c) 2022 Geraint Luff / Signalsmith Audio Ltd.") | [S-3a6] | **preferred stretch engine** |
| rnnoise | BSD-3-style (Xiph/Valin/Mozilla/Borgerding; name-restriction clause) | [S-3a7] | OK, module |
| DeepFilterNet | dual MIT OR Apache-2.0 (Rikorose/DeepFilterNet LICENSE) | [S-3a7] | OK; model-weight terms UNVERIFIED (§9) |
| essentia | AGPL-3.0 (MTG COPYING.txt is plain AGPL) | [S-3a8] | OUT of core; analysis service only (MTG commercial offering UNVERIFIED) |
| aubio (+ aubio-rs 0.2.0, stale) | GPL-3.0 | [S-3a8, S-3a0] | OUT of core |
| audiowaveform (BBC) | GPL-3.0 | [S-3a8] | reference/CLI only; implement peaks in-house |
| waveform-data.js (BBC) | **LGPL-3.0** (not Apache as often assumed) | [S-3a8] | JS-side consumption caution; format doc fine to read |
| libebur128 (C) | MIT (Jan Kokemüller) | [S-3a8] | OK (crate = pure-Rust port, same results) |
| JUCE 9 | AGPLv3 OR commercial EULA | [S-3a4] | OUT of core |
| libsoundio (C) | MIT (Expat) | [S-3a8] | OK but stale-ish; cpal preferred |
| symphonia | MPL-2.0 | [S-3a0] | OK (file-level copyleft; separate crates) |
| cpal | Apache-2.0 | [S-3a0, S-3a4] | OK; `asio` feature pulls Steinberg ASIO SDK — terms UNVERIFIED (§9) |

## 7. ove-audio integration sketch

```
engine/ove-audio        (no device deps, deterministic, mirrors ove-time rules)
  buf:     f32 planar blocks, fixed max block frames (e.g. 1024), per (track, t-range)
  sched:   R-AUDIO-1 projection + exact remainder; pull(t0_ticks, t1_ticks) -> block
  mix:     gain/pan/fade/crossfade/sum + clip policy; peak/RMS meters out
  resamp:  rubato adapter (ingest-only); peaks: min/max buckets -> cache; later: ebur128 + stretch FFI
engine/ove-audio-device (cpal): output ring + device frame counter -> ove-time clock bridge
platform adapters:  browser: AudioWorkletProcessor calling wasm core, scheduled by currentFrame
                      [S-3a1]; decode via WebCodecs (Safari 26+) / decodeAudioData bridge [S-3a2]
                    android: AAudio stream (PERFORMANCE_MODE_LOW_LATENCY) via JNI; frames→ticks §4
                      [S-3a3]; export leg stays Media3 (doc 15)
```

Invariants: no fp seconds on any authoritative path (ADR-007); one floor policy (R-AUDIO-1); the
device thread never allocates/locks on the render path; preview and export share the core block math.

## 8. Leg comparison — FFmpeg-audio vs GStreamer vs WebAudio vs CPAL(+DSP) vs JUCE

| axis | FFmpeg filters [doc 05] | GStreamer [doc 06] | WebAudio [S-3a1] | CPAL + in-house DSP | JUCE [S-3a4] |
|---|---|---|---|---|---|
| decode | best coverage (LGPL/GPL split) | good, plugin-gated | browser codecs; Safari<26 via decodeAudioData only | symphonia (no HE-AAC; AAC-LC via flag) + platform | built-in |
| device I/O | none (files) | appsink/alsasrc etc. | host-managed | **WASAPI/CoreAudio/ALSA + wasm** | full |
| mix graph | filtergraph (string-built) | pipeline + caps negotiation | node graph, 128-frame quantum | own pull graph on exact ticks | node graph |
| exact time | µs rational stamps | pipeline clock, bridge drift | fp seconds + integer currentFrame | i64 rational ticks (native) | fp-ish |
| license | LGPL-2.1+ core | LGPL-2.1+ | n/a | Apache/MIT/MPL stack | **AGPLv3/commercial** |
| fit for OVE | export backend only (doc 05 pitfalls) | heavy for embedded core; adapter candidate | browser leg only | **engine core choice** | reference only |

## 9. UNVERIFIED items (honest)

- Rubber Band commercial price/terms — README points to breakfastquay.com; page not fetched.
- essentia commercial licensing via MTG — not fetched; COPYING.txt contains plain AGPL-3.0 only.
- DeepFilterNet **model weights** distribution terms (LICENSE covers code; weights page not fetched).
- cpal `asio` feature: Steinberg ASIO SDK redistribution terms — not fetched (feed doc 35).
- Safari <26 AudioDecoder codec coverage; OfflineAudioContext cross-engine bit-determinism;
  cpal per-host latency/xrun behavior; AAudio MMAP specifics; device latency spread — need E-008.
- MLT calculator implementation internals (header interface only) [S-3a9]; Olive audio render
  chunking (repo path 404 this pass); `timestretch` 0.15.0 quality (existence verified only).

## 10. Depth-remaining (next passes)

1. E-008 proposal: device-latency probe matrix (cpal WASAPI/CoreAudio/ALSA, AAudio, AudioWorklet)
   — startup, xruns, clock jitter vs ove-time expectations.
2. WebAudio determinism experiment: identical graph via OfflineAudioContext on Chrome/Firefox/
   Safari — bit-equality feeds the "one mixer" invariant.
3. Read MLT mlt_frame.c calculator + Olive audio pipeline sources; check whether MLT's float-fps
   calculator drifted in practice (audio-specific backing for doc 20's warnings).
4. Loudness: ebur128 vs FFmpeg loudnorm; spectrogram rustfft CPU vs GPU (doc 08); rnnoise WASM
   footprint; DFN runtime cost on Android.
5. MIDI: oxisynth vs fluidlite only if a MIDI soundtrack is scoped (likely out for v1).

## Sources (all fetched 2026-09-24 unless noted; full rows in research/sources/LEDGER_W3a.md)

- S-3a0 crates.io API sweep (15 crates) + symphonia README · S-3a1 Web Audio spec · S-3a2 MDN BCD
  audio · S-3a3 AAudio/audio-latency/OpenSL docs · S-3a4 cpal README + JUCE 9 LICENSE · S-3a5
  Rubber Band README+COPYING · S-3a6 SoundTouch COPYING + signalsmith LICENSE · S-3a7 rnnoise
  COPYING + DeepFilterNet LICENSE · S-3a8 essentia/aubio/audiowaveform/waveform-data/libebur128/
  libsoundio license files · S-3a9 MLT mlt_audio.h + Media3 AudioTrackPositionTracker.java.
  Cross-refs: S-2b6 (Olive rational.h), S-2d0 (BCD), S-2e0 (Media3), S-4f-x (librosa ISC); docs
  03/05/06/07/11/15/17/20; E-002/E-002c; ADR-007.
