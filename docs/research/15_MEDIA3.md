# 15 — MEDIA3 / EXOPLAYER (androidx.media): PLAYBACK + TRANSFORMER AS EDIT BACKEND?

> Status: PARTIAL (v0.1).

Scope: verify the Media3 API surface for (a) playback architecture and (b) Transformer
edit operations (trim/effects/transcode/compositing), then judge suitability as the
Android editing backend for the Open Video Engine. Evidence priority: source code of
androidx/media (S-2e0) > official guides (S-2e1) > community.

## 1. Library facts (verified)

- Repo: https://github.com/androidx/media (Google, Apache-2.0). Latest tags observed:
  **1.11.1** (1.9.x..1.11.x present on main). Version claim = git ls-remote, 2026 session.
- Modules relevant here: `media3-exoplayer` (playback), `media3-transformer` (offline
  edit/export), `media3-effect` (GL effects), `media3-common` (MediaItem, Effect),
  `media3-muxer` (in-app muxing).
- Most Transformer surface is annotated `@UnstableApi` (verified on Transformer.java,
  Composition.java); API churn must be expected even in stable releases.

## 2. Playback architecture (ExoPlayer)

- `Player` interface + `ExoPlayer` implementation: timeline of `MediaItem`s, Renderer
  pipeline (video/audio/text), `MediaSource`s, `DefaultTrackSelector`, adaptive
  streaming (HLS/DASH/SS), DRM via `MediaItem.DrmConfiguration`.
- Media3 playback model = URI + (optional) ClippingConfiguration + (optional)
  static metadata; there is no concept of overlay/track composition at playback time
  except via `CompositionPlayer` (below) or custom Renderers.
- Key verified naming: `MediaItem.Builder().setClippingConfiguration(...)` is the
  current trim API; per-field `setClip*PositionMs` builders are `@Deprecated`
  (MediaItem.java source, S-2e0). Feed docs 19/20 (project format/timeline) so our
  engine never hard-codes deprecated names.

## 3. Transformer: edit-operation model (all verified in source, S-2e0)

- Entry: `Transformer.Builder(context)`; notable verified Builder setters:
  setAudioMimeType / setVideoMimeType / setPortraitEncodingEnabled /
  setNativeHardwareBufferHelpers / setFrameProcessorFactory /
  setVideoFrameProcessorFactory / setAssetLoaderFactory / setAudioMixerFactory /
  setEncoderFactory (Codec.EncoderFactory) / setMuxerFactory (Muxer.Factory,
  androidx.media3.muxer) / setLooper / setDebugViewProvider / setClock /
  setUsePlatformDiagnostics.
- Export entry points: `start(Composition, path)`, `start(EditedMediaItem, path)`,
  `start(MediaItem, path)`; listeners via `addListener/removeListener
  (Transformer.Listener)`; result types `ExportResult` (+ per-track stats) and
  `ExportException`.
- Composition model (the "sequence" question answered):
  `Composition.Builder(EditedMediaItemSequence, ...sequences)` — a Composition holds
  N parallel `EditedMediaItemSequence`s (concatenation within a sequence, mixing
  across sequences), plus `VideoCompositorSettings`, composition-level `Effects`,
  `hdrMode`, and transmux flags. So: multitrack = multiple sequences composited
  by `VideoCompositorSettings` (androidx.media3.common.VideoCompositorSettings);
  it is NOT a per-clip arbitrary-position overlay graph like desktop NLEs.
- `EditedMediaItemSequence.Builder`: `addItem(s)`, `addGap(durationUs)` (silence/
  black gap), `setIsLooping(boolean)`, trackTypes set (TRACK_TYPE_AUDIO/VIDEO);
  helpers `withAudioFrom(...)` / `withVideoFrom(...)` to pull A/V from separate items
  (verified source). Effects attach per `EditedMediaItem` and per `Composition`.
- Trim: `MediaItem.ClippingConfiguration` (start/end ms, relative to default position,
  startsAtKeyFrame). Transformer has a trim fast-path: source shows
  `trimOptimizationEnabled` + `mp4EditListTrimEnabled` — single-asset trimming can be
  done via MP4 edit lists (no re-encode) when enabled. VERIFIED flags in Transformer.java.
- Overlays: `androidx.media3.effect.OverlayEffect` (verified class, implements GlEffect)
  composites TextureOverlay/BitmapOverlay/TextOverlay — overlays are GL-composited
  onto the sequence, not "video tracks with alpha in the timeline".
- Custom GL effects: implement `GlEffect` -> gets a `GlMatrixTransformation`/
  shader hook inside `DefaultVideoFrameProcessor`/`DefaultGlFrameProcessor` chain;
  `androidx.media3.common.video.FrameProcessor` + new `androidx.media3.common.video
  .Frame` types are present on main; Transformer also imports a
  `HardwareBufferJniWrapper` (evidence of HardwareBuffer-based zero-copy plumbing).
- Preview: `CompositionPlayer` (experimental, `@ExperimentalApi`) renders a
  `Composition` through `SimpleBasePlayer` with `FrameProcessor.Factory` support and
  `VideoFrameAggregationParameters` (experimental) — i.e., Google is building
  WYSIWYG preview of exactly the Composition that Transformer exports. Caveat:
  active issue tracker (e.g., issue #2866 overlapped-sequence playback bugs).

## 4. Capabilities vs full-NLE needs

| NLE need | Transformer status |
|---|---|
| Trim/split, A/B-roll in one video track | yes (sequences + ClippingConfiguration) |
| Multitrack compositing (arbitrary layers, transforms per layer, alpha) | partial: N sequences with VideoCompositorSettings; per-clip overlay matrices limited; no per-layer z-order + keyframe graph API |
| Overlays (text/image) | yes (OverlayEffect) but GL-composited onto frames |
| Custom GPU effects | yes, GlEffect chain (GL ES only; no Vulkan effect path) |
| Speed/tempo | audio SpeedChangingAudioProcessor + video speed effects (verified imports); frame-rate agnostic retiming not exposed |
| Audio mixing across sequences | yes (AudioMixer.Factory, ChannelMixingAudioProcessor) |
| Export control (segmented, resumable) | no public API; start() is fire-once (setSpeed/resumption TODO in source) |
| Codec choice | via setAudioMimeType/setVideoMimeType + EncoderFactory; hardware encoder quality varies per OEM |
| Project persistence / undo | none (engine's job) |

## 5. Verdict (for ADR-002/ADR-005)

- Transformer is a solid, hardware-accelerated "render farm primitive" for the Android
  app: trim, concatenate, mix, overlay, apply effects, export MP4 — with Google-
  maintained device-compat handling (that is its real value).
- It is NOT a timeline engine: no keyframed layer graph, no arbitrary compositing
  model, no resumable exports, `@UnstableApi` churn. A full NLE core cannot be built
  by composing Transformer calls; at best Transformer is one export backend the core
  can target (and the fastest path to MVP on Android).
- `CompositionPlayer` is the strategic API to watch: preview==export parity is exactly
  the human+AI shared-command requirement of the directive; track it per release.
- Decision recommendation for phase 1 Android: core timeline in engine core
  (doc 03/20), with a Transformer-backed renderer for phone-export MVP; revisit
  custom MediaCodec pipeline (doc 16) when Transformer limits are hit.

## 6. Open questions / depth remaining (v0.2)

- Q-M3-1: minSdk of media3 1.11 ( believed 21; verify from build.gradle).
- Q-M3-2: CompositionPlayer stability timeline; issue-tracker triage of compositing bugs.
- Q-M3-3: Can multiple sequences each carry >1 video item layered with per-sequence
  matrices, or is one video track per sequence the hard limit (read compositor source).
- Q-M3-4: HDR modes (hdrMode / retainHdrFromUltraHdrImage) device support matrix.
- Q-M3-5: Benchmarks E-00x: 1080p30 trim+overlay+export realtime factor on
  mid-tier + flagship devices vs. custom MediaCodec path.

## Sources
- S-2e0 androidx/media source: Transformer.java, Composition.java,
  EditedMediaItemSequence.java, MediaItem.java, OverlayEffect.java, CompositionPlayer.java.
- S-2e1 developer.android.com supported-formats (codec mandates referenced by doc 16).
- Secondary: Media3 Transformer overview guide (developer.android.com/media/media3/transformer).
