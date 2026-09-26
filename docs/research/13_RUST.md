# 13 — RUST AS CORE-LANGUAGE CANDIDATE (NEUTRAL EVIDENCE SCAN)

> Status: PARTIAL (v0.1).

Scope: neutral, evidence-based scan of Rust as the candidate language for the Open Video
Engine core (the cross-platform media/timeline/graph layer), NOT an advocacy document.
Directive requires hypotheses verified, not assumed; this doc lists evidence FOR and
AGAINST with sources. Decision deferred to ADR-001 (language) / ADR-002 (core architecture).

## 1. Verification log (this scan)

| Claim | Verification method | Result |
|---|---|---|
| crates.io latest versions/dates | crates.io REST API queried live (2026 session) | jni 0.22.4, uniffi 0.32.1, symphonia 0.6.1, ffmpeg-next 9.0.0, gstreamer 0.25.3, rav1e 0.8.1 |
| UniFFI production use at Mozilla | mozilla/uniffi-rs README (fetched) | "used extensively by Mozilla in Firefox mobile and desktop browsers" |
| gstreamer-rs activity | crates.io | gstreamer 0.25.3, updated 2026-06 |
| rav1e activity | crates.io | 0.8.1 (2025-06), 46.5M downloads |
| Symmetrio (suggested project) | not found in search | UNVERIFIED — likely confusion; do not cite |
| iron-oxide (suggested project) | checked context | NOT a media library — IronOxide is a data-encryption SDK; exclude |
| retake (suggested project) | not found as Rust media project | UNVERIFIED — exclude |

## 2. Evidence FOR Rust as engine core

| # | Argument | Evidence (quality tier) | Confidence |
|---|---|---|---|
| F1 | Memory safety without GC suits long-lived media pipelines (frame buffers, shared GPU resources); data-race freedom at compile time enables fearless multi-threaded frame queues | Rust language model (official, tier 1) | High |
| F2 | Single core compiles to all platform targets: Linux/macOS/Windows/Android (aarch64-linux-android etc. via cargo-ndk 4.1.2, verified), iOS, and wasm32-unknown-unknown for browser/WASM track (track E) | cargo-ndk repo (tier 1), rustc platform docs | High |
| F3 | Cross-language FFI is a first-class use case: UniFFI 0.32.1 generates Kotlin + Swift (+Python/Ruby; 3rd-party C#/Go) bindings from one UDL/proc-macro interface — exactly the Android/iOS bridge shape needed | mozilla/uniffi-rs README (tier 1 repo, tier 2 prose) | High |
| F4 | UniFFI is production-proven: Mozilla ships Rust components to Firefox for Android via UniFFI-generated Kotlin (official README claim); Glean/Nimbus components are the canonical examples | uniffi-rs README (tier 2); mozilla/glean repo | High |
| F5 | Raw JNI also viable and maintained: jni crate 0.22.4 (crates.io, 188M downloads) — needed anyway for MediaCodec/Surface interop where UniFFI cannot express callbacks over raw Android types | crates.io API (tier 2) | High |
| F6 | GPU abstraction: wgpu (gfx-rs) targets Vulkan/Metal/DX12/GL + WebGPU — one shader/compositor codebase for desktop GPU track (D) and potential web track (E) | gfx-rs/wgpu repo (tier 1, agent 3-d owns detail) | High |
| F7 | Pure-Rust audio demux/decode exists: symphonia 0.6.1 (13.9M downloads) — no C dependency for audio path | crates.io (tier 2) | High |
| F8 | Production Rust media software exists to study: rav1e (AV1 encoder, 46M downloads), gstreamer-rs (official GLib/GStreamer bindings), ffmpeg-next 9.0 (7.2M downloads, bindings to FFmpeg C API), ruffle (Ruffle Flash emulator, ships on Android with wgpu rendering) | crates.io + project repos (tier 1-2) | High |
| F9 | FFmpeg/GStreamer integration possible from Rust: gstreamer-rs is an official GStreamer subproject; ffmpeg-next wraps libav* C libraries — i.e., Rust core CAN sit above C media stacks | gstreamer-rs docs, ffmpeg-next crate (tier 2) | High |
| F10 | Team capability signal (mission context): operator profile lists shipped Kotlin apps and Rust repos — Rust/Kotlin pairing matches existing skills; Kotlin interop (F3/F5) is the strongest single argument given Android-first ownership | worklog Task 1,2 (internal) | Medium (soft factor) |

## 3. Evidence AGAINST Rust as engine core

| # | Argument | Evidence | Confidence |
|---|---|---|---|
| A1 | The best-in-class media C/C++ libraries (FFmpeg for demux/filter/encode breadth, libdav1d for AV1 decode, x264, GStreamer core/plugins) are C/C++; from Rust they are reached via bindings (ffmpeg-next) or hand-written -sys crates — the same C dependency exists as from C++, with one more binding layer to maintain | ffmpeg-next crate, -sys convention (tier 2) | High |
| A2 | Real-time video pipelines depend on vendor/C APIs everywhere: MediaCodec (Java/JNI), AHardwareBuffer (NDK C), Core Video/Metal (ObjC/Swift), DX11/DX12 (C++). Rust does not remove these; every one is an FFI boundary to own | platform docs (tiers 2) | High |
| A3 | Rust does not compile to the JVM/Android-runtime side: an Android app is still Kotlin/Java at the surface; Rust is a .so loaded per-ABI (arm64-v8a, armeabi-v7a, x86_64...) with JNI init, panic-handling discipline, and 4x build matrix cost | Android NDK docs (tier 2) | High |
| A4 | GUI ecosystem for Rust is immature relative to the video-editor front-end needs; the pragmatic front-ends are web-tech (Tauri doc 17) or native Kotlin — Rust UI toolkits are not at parity for complex pro-app UI (docking, scrubbing, i18n) | ecosystem scan (tier 3-9) | Medium |
| A5 | Compile times: large Rust monoliths (esp. with gstreamer-rs/wgpu dependency trees) have materially longer incremental builds than Kotlin or C++ with precompiled libs — affects iteration speed on the core | community consensus (tier 9; measure in E-00x before deciding) | Medium (unquantified here) |
| A6 | Hiring pool smaller than C++/Kotlin for media-domain engineers; contributors to classic NLE codebases (Kdenlive, Olive, Shotcut — docs 02/07) are C++/Qt/QML people | observed ecosystem (tier 9) | Medium |
| A7 | Trait-object / async-heavy designs can fight the borrow checker in graph-scheduling code; dynamic plugin graphs sometimes want runtime reflection Rust deliberately lacks | design-experience argument (tier 9) | Low-Medium |
| A8 | Rust crates for pro NLE features are partial: no full-featured color grading library, no subtitle/TTML ecosystem, audio DAW crates immature vs. JUCE (C++). Gaps mean writing more core from scratch | crates ecosystem scan (tier 9) | Medium |

## 4. Neutral comparison snapshot (core-language candidates)

| Criterion | Rust | C++ | Kotlin/JVM core |
|---|---|---|---|
| Memory safety in pipeline code | compile-time, no GC | manual / sanitizers | GC pauses a risk for frame loops |
| FFI to FFmpeg/GStreamer | via -sys crates (A1) | native (strongest) | via JNI over C libs (worst) |
| Single-codebase multi-platform | yes incl. wasm (F2) | yes minus wasm ergonomics | Android-first only |
| Android bridge maturity | UniFFI 0.32 + jni 0.22 (F3/F5) | mature NDK C++ | native |
| Existing NLE codebase reuse | ~none | high (MLT, Olive, Kdenlive ecosystem) | none |
| Iteration speed / toolchain | slower builds (A5) | variable | fastest (Gradle hot paths) |
| Desktop shell fit | Tauri 2 = Rust core already (doc 17) | Qt | Compose Desktop (young) |

## 5. Existing Rust media projects to study (tracked for later deep-dives)

| Project | What it proves | Link |
|---|---|---|
| rav1e | Rust AV1 encoder used in production (Facebook/Meta deployments) | https://github.com/xiph/rav1e |
| gstreamer-rs | official bindings; idiomatic pipeline API over C GStreamer | https://gitlab.freedesktop.org/gstreamer/gstreamer-rs |
| ffmpeg-next | thin bindings to FFmpeg 9.x C API | https://github.com/zmwangx/rust-ffmpeg |
| symphonia | pure-Rust demux/decode (audio) — pattern for a Rust-first demuxer | https://github.com/pdeljanov/Symphonia |
| Ruffle | large Rust GPU app (wgpu) shipping to Android — desktop-class Rust app on Android precedent | https://github.com/ruffle-rs/ruffle |
| mozilla/glean + Nimbus | UniFFI Kotlin/Swift components shipped at Firefox scale | https://github.com/mozilla/glean |
| image-rs / ravif / zune-* | pure-Rust image codecs (JPEG/PNG/APNG decode; AVIF encode via ravif) | https://github.com/image-rs |
| wgpu (gfx-rs) | one GPU abstraction across Vulkan/Metal/DX12/GL/WebGPU; Ruffle proves it works in a shipped media-adjacent app | https://github.com/gfx-rs/wgpu |
| rust-native media utilities (study set) | rodio/cpal (audio I/O), video-rs (ffmpeg wrapper crate), opus/ogg crates — small parts, lower risk to adopt than full stacks | crates.io scan |

Counter-examples to keep the table honest (things Rust does NOT have yet, verify per
quarter): no production pro NLE written in Rust exists today (closest are
embeddings like Ruffle or encoders like rav1e); no mature subtitle/TTML toolkit;
no color-management library at OpenColorIO depth (ocip-rs bindings are early).

Excluded after verification: Symmetrio (not found), iron-oxide (not media), retake (not found).

## 6. Open questions feeding ADR-001

1. Q-RUST-1: Can the engine core keep FFmpeg behind a narrow Rust trait boundary so the
   C dependency is swappable (pure-Rust demuxers later)? Needs experiment E-00x.
2. Q-RUST-2: Exact incremental-build cost of a core crate with gstreamer-rs + wgpu trees
   (measure on CI shapes, tier-9 claim A5 must be quantified).
3. Q-RUST-3: UniFFI vs raw JNI split for Android: UniFFI for domain objects/commands,
   raw JNI (jni 0.22) for MediaCodec/Surface/HardwareBuffer plumbing (UniFFI cannot
   express AHardwareBuffer lifetimes directly) — prototype needed.
4. Q-RUST-4: Does WASM track (E) truly share the core, or only a subset (no threads,
   no SIMD-accelerated decode parity)?

### 6.1 How this doc relates to other tracks

- Desktop shell (Tauri 2) is itself Rust — see doc 17 §8 for where the core sits.
- Android bridge detail (UniFFI/jni/cargo-ndk) lives in doc 14 Part B.
- GPU abstraction overlap (wgpu) is owned by agent 3-d (docs 09/10).
- Decision artifact: ADR-001 must cite F1-F10 / A1-A8 rows by ID, not vibes.

## 7. Risk register pointers

- R-RUST-1: binding-layer maintenance for ffmpeg-next (upstream API churn across
  FFmpeg majors — same risk class as S-005 already records for doc 05).
- R-RUST-2: per-ABI Android build matrix slows CI (feeds doc 32 performance/
  tooling plans).
- R-RUST-3: two-language core (Rust core + Kotlin shell) raises contribution
  friction for Android-focused contributors; mitigation is the UniFFI-generated
  Kotlin layer being idiomatic rather than a raw FFI dump.

## 8. Depth remaining for v0.2

- Benchmark: decode-filter-encode pipeline in (a) Rust+ffmpeg-next vs (b) C++ FFmpeg
  vs (c) Kotlin+MediaCodec on one Android device and one desktop.
- Survey how gstreamer-rs and rav1e handle panics/callbacks across FFI (feeds doc 14).
- Study Ruffle's wgpu-on-Android surface integration (feeds docs 08/09/14).
- Quantify A5/A6 with real numbers; check `cargo build` profiles with `cargo-ndk`
  for 4-ABI Android CI cost.
- Poll the Rust media ecosystem quarterly: new demuxers (matroska/mp4 crates),
  caption toolkits, color crates; update Section 5 table with dates.
- Interview/check how 2-3 UniFFI consumers structure their Kotlin-facing API
  (naming, enums, async) to shape our core's foreign API contract.

## Sources

- crates.io API (live): jni, uniffi, symphonia, ffmpeg-next, gstreamer, rav1e
  versions/downloads (S-2e7 covers uniffi; sibling crates listed above).
- mozilla/uniffi-rs README + tags v0.32 (fetched, S-2e7).
- bbqsrc/cargo-ndk Cargo.toml v4.1.2 (fetched, S-2e7).
- Project repos for Section 5 (existence verified; depth studies pending).
