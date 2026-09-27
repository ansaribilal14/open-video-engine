# 14 — ANDROID/KOTLIN: GPU STACK, EXPORT LIFECYCLE, AND RUST BRIDGE

> Status: PARTIAL (v0.1).

Scope (agent 3-e): (a) Android GPU/compositing stack reality for a video engine,
(b) long-render job lifecycle (foreground services, Doze, process death, thermal),
(c) Rust-on-Android bridge options with verified versions. Kotlin app-shell knowledge
is assumed (owner has shipped Kotlin apps); this doc focuses on engine-relevant reality.

## Part A — Android GPU stack for compositing

### A1. The buffer backbone: Surface/BufferQueue/AHardwareBuffer
- Every decode->render->encode chain on Android is glued by `Surface`/`SurfaceTexture`
  over gralloc buffers; `AHardwareBuffer` is the NDK handle for cross-API sharing.
- SurfaceTexture sampling uses the `GL_TEXTURE_EXTERNAL_OES` target defined by the
  `GL_OES_EGL_image_external` extension; shaders must declare the extension, bind to
  EXTERNAL target (not GL_TEXTURE_2D), and apply the queried transform matrix.
  VERIFIED: android.graphics.SurfaceTexture reference, quoted 2026 session (S-2e5).
- Implication for the engine: our compositor must accept EXTERNAL_OES textures as a
  first-class input type (decoder output), i.e., a shader system with a pluggable
  "source texture" abstraction, not a naive GL_TEXTURE_2D pipeline.

### A2. OpenGL ES vs Vulkan on Android
- OpenGL ES 3.x remains the universal, low-boilerplate path; Media3's own effect chain
  (`androidx.media3.effect`, `DefaultGlFrameProcessor`) is built on GL ES + EGL.
  VERIFIED via androidx/media source imports (S-2e0).
- Vulkan is NOT universally mandated by CDD: Android 15 CDD requires Vulkan support
  only conditionally; handheld devices supporting Vulkan must satisfy the
  "Android Baseline 2021" profile (which includes Vulkan 1.1 on qualifying launches).
  VERIFIED: CDD 7.1.4.2/H-1-1 text pulled from android-15-cdd (S-2e6).
  UNVERIFIED detail: exact RAM/bitness preconditions of Baseline 2021 — check
  source.android.com Baseline profile page before citing in ADR.
- Practical verdict for v1 engine: GL ES for Android compositing (interop with
  MediaCodec/AHardwareBuffer is mature); Vulkan only if/when wgpu core needs it.
  Cross-check with docs 08/09 (GPU architecture, wgpu).

### A3. Surface plumbing choices
| Pattern | Use | Risk |
|---|---|---|
| GLSurfaceView | toy previews; owns EGL + UI thread | render loop coupled to UI thread; not for pro scrubbing |
| Custom EGL + SurfaceView/TextureView | correct choice for preview | must own EGL context loss, multithread sharing |
| SurfaceTexture -> OES texture | decoder output into compositor | transform matrix + EXTERNAL constraints (A1) |
| ImageReader | CPU-side frames (analysis, thumbnails, screenshot export) | YUV_420_888 plane handling; latency adds a copy |
| AHardwareBuffer + EGLImage / Vulkan ext-memory | zero-copy across decode->GL->encode | API 26+; driver variance; JNI/NDK glue needed |

- GLSurfaceView vs custom EGL: engine should use a dedicated render thread with its own
  EGL context (pbuffer + shared contexts for upload), never GLSurfaceView. Media3 does
  exactly this internally (its GL processors run on a private thread + EGL surface).
  VERIFIED pattern via media3.effect sources (S-2e0).

### A4. Thermal throttling on long renders
- Android exposes thermal status via `PowerManager.getCurrentThermalStatus()` (API 29+)
  and `addThermalStatusListener(OnThermalStatusChangedListener)`; statuses run
  NONE -> LIGHT -> MODERATE -> SEVERE -> CRITICAL -> EMERGENCY -> SHUTDOWN.
  UNVERIFIED enum quotes this session (from reference knowledge; verify against
  android.os.PowerManager reference before ADR citation).
- SoC-thermal-driven DVFS means a 4K export that starts at realtime can slow 2-5x on
  mid-tier phones; engine must support resumable chunked export (state checkpoints)
  rather than assume monotonic speed. Feed into docs 32 (performance) and 42 (risks).

### A5. Export-job lifecycle constraints (VERIFIED highlights)
- Android 14 introduced a dedicated foreground service type `mediaProcessing`
  (`FOREGROUND_SERVICE_MEDIA_PROCESSING`) for "time-consuming operations on media
  assets, like converting media to different formats".
- Budget: "under normal circumstances ... 6 hours out of every 24", SHARED across all
  of the app's mediaProcessing FGS. On timeout the system calls `Service.onTimeout()`;
  the app must call `stopForeground()`/`stopSelf()`.
  VERIFIED: developer.android.com fgs-service-types page, quoted 2026 session (S-2e4).
- Consequences for the engine: (1) export orchestrator must checkpoint progress and be
  killable/resumable at sample-accurate boundaries; (2) >6h jobs must be segmented into
  multiple runs; (3) on Android <14 fall back to `dataSync`-style FGS or WorkManager
  with expedited constraints (verify: WorkManager "expedited work" quota).
- Doze/App Standby suspend network/wakelocks for idle apps but a RUNNING FGS is exempt;
  process death under memory pressure is the real threat -> persist project + export
  job state to disk (SQLite/pb) before each segment. UNVERIFIED: exact FGS exemption
  wording — verify in "Optimize for Doze and App Standby" doc.

## Part B — Rust on Android: bridge options (verified versions)

### B1. Toolchain
- cargo-ndk 4.1.2 (bbqsrc/cargo-ndk) automates `-Z build-std`-free cross builds and
  jniLibs packaging for arm64-v8a, armeabi-v7a, x86, x86_64; requires Android NDK
  + rustup targets per ABI. VERIFIED version via Cargo.toml (S-2e7 adjacent fetch).
- Output ABI packaging: one .so per ABI under `src/main/jniLibs/<abi>/libengine.so`;
  Play requires 16KB page alignment for arm64 targets on newer devices
  (UNVERIFIED exact policy date — verify before release engineering).

### B2. Bridge layers (decision space)
| Layer | Status (verified) | Best for | Avoid for |
|---|---|---|---|
| UniFFI 0.32.1 (mozilla) | Kotlin+Swift+Python+Ruby bindings generated from UDL/proc-macros; README: "used extensively by Mozilla in Firefox mobile and desktop browsers" | domain API surface: timeline objects, commands, project format, background task control | raw Android types (Surface, AHardwareBuffer, MediaCodec handles) |
| jni crate 0.22.4 | raw JNI; 188M downloads; maintained | codec/surface plumbing, byte[]/ByteBuffer bulk transfer, callbacks into Kotlin | mass API surface (boilerplate, unsafe) |
| JNI via NDK C | manual RegisterNatives in C or C++ shims | when C/C++ libs (FFmpeg) must call back into Java | general use |
| UniFFI callbacks | `#[uniffi::export]` traits -> Kotlin objects | progress/event reporting to app | high-rate frame callbacks (async machinery cost) |

### B3. Cross-boundary discipline (both bridges)
- Panic handling: unwinding across the JNI/FFI boundary is UB; Rust code at the
  boundary must `catch_unwind` and convert to error codes/foreign exceptions.
  UniFFI wraps exported calls and converts panics into foreign exceptions
  (UNVERIFIED exact current semantics — confirm in uniffi-rs docs).
- Callback threading: JNI attach/detach rules; MediaCodec async callbacks arrive on
  framework threads — any Kotlin->Rust->Kotlin event path must pin explicit threads;
  UniFFI async (0.30+ async trait support) exists but frame-rate event streams should
  stay raw-jni or shared-memory ring buffers. UNVERIFIED: uniffi async version gate.
- Memory ownership: keep frame memory Rust-side as `Vec<u8>`/memmap; pass Java side
  only direct ByteBuffer views; never let Kotlin own what Rust frees and vice versa.
  AHardwareBuffer lifetimes (acquire/release) must be explicit Rust RAII + JNI
  handoff — no garbage-collected ownership for GPU buffers.
- APK/AAB size: each Rust .so adds per-ABI payload; measure with a hello-engine crate
  (experiment E-00x) before committing.

### B4. Real projects shipping Rust on Android (for pattern study)
- Mozilla Firefox Android / android-components: UniFFI-generated Kotlin for storage,
  Nimbus, Glean (README-verified use; verify a concrete build.gradle import in
  mozilla-mobile/firefox-android for v0.2 — first grep found 0 in fenix/app/build.gradle,
  usage lives in android-components crates; do not cite fenix path until located).
- Ruffle: Flash emulator; wgpu rendering, ships Android builds — precedent for a large
  Rust+GPU app on Android (repo verified to exist; Android shipping claim UNVERIFIED
  this session — verify CI artifacts).
- Glean (mozilla/glean): Rust core + UniFFI Kotlin bindings shipped in many Android apps.
- NOT verified / excluded: Symmetrio (not found), iron-oxide (crypto SDK, not media).

## Part C — Implications for the engine architecture

1. Shell stays pure Kotlin (UI, permissions, FGS orchestration); engine core optional
   Rust behind UniFFI/jni boundary; MediaCodec/Surface glue is JNI-native either way.
2. Export pipeline must be: FGS mediaProcessing (14+) -> segment checkpoints ->
   PowerManager thermal listener -> resume-on-kill. Design the core export state
   machine around this now (feeds ADR-005 export jobs).
3. Compositor inputs are EXTERNAL_OES textures (A1); this constraint must appear in
   the core frame-abstraction API or Android needs a shader-path fork.
4. Open questions: Q-AND-1 minimal API 21 vs 24+ floor for engine (Media3 1.11
   minSdk to confirm in doc 15); Q-AND-2 16KB page alignment build flags; Q-AND-3
   UniFFI async suitability for our callback rates.

## Sources used here
- S-2e0 androidx/media (Transformer/Composition/effect sources, fetched)
- S-2e4 developer.android.com FGS types (mediaProcessing, fetched)
- S-2e5 android.graphics.SurfaceTexture reference (EXTERNAL_OES, fetched)
- S-2e6 Android 15 CDD (Vulkan conditional requirement, fetched)
- S-2e7 UniFFI repo/README + cargo-ndk (fetched) + crates.io versions
- Marked UNVERIFIED inline: PowerManager thermal enum quote, Doze exemption wording,
  Baseline 2021 preconditions, 16KB alignment policy, uniffi async gate, Ruffle Android.
