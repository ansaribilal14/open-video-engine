> Status: PARTIAL (v0.1).
# 12 — WASM (for media)

> Owner: agent 3-d · Tracks: E, B · Date: 2026-09-21
> Scope: SIMD, threads, memory (4GB/Memory64), streaming compilation, ffmpeg.wasm
> performance reality, and the correct WASM role in OVE (probe/demux/mux vs bulk decode).
> Feeds claim C-006.

## 1. Language/platform features (status verified 2026-09-21)

- **Core wasm**: shipped in all major engines since 2017 (Mozilla blog, Nov 2017).
- **SIMD (128-bit fixed-width)**: shipped Chrome 91 / Firefox 89 / Safari 16.4 (MDN BCD;
  Safari marked UNVERIFIED exact version at v0.1, widely reported 16.4). Note: 128-bit
  registers only — no 256/512-bit (AVX2/AVX-512) equivalents, which is a structural perf
  gap vs native SIMD code (nickb.dev benchmark: wasm ~4x slower than native at large
  payloads partly due to this).
- **Threads (SharedArrayBuffer + Atomics.wait)**: requires cross-origin isolation
  (COOP: same-origin; COEP: require-corp). caniuse sharedarraybuffer (verified today):
  Chrome 68+, Firefox 79+, Safari/iOS 15.2+ (with cross-origin-isolation notes). Practical
  consequence: shipping a multithreaded wasm module (pthread pool) *costs COOP/COEP headers*,
  which also constrain embeddability (e.g., being iframed by partners without those headers
  breaks it). Design OVE wasm to degrade to single-thread without rebuild.
- **Memory limits**: wasm32 memory caps at 4 GiB (WebKit raised addressable wasm memory to
  4GB in Safari 15.2 — webkit.org, Dec 2021). Practical native-vs-wasm gap: many native
  ffmpeg builds already exceed 4GB working sets for large exports unless the code is
  compiled with memory-saving strategies.
- **Memory64 (wasm64)**: phase 4/5 proposal; **shipped Chrome 133 and Firefox 134**
  (SpiderMonkey blog, Jan 2025; Chromium Intent to Ship M133). Safari status: UNVERIFIED at
  v0.1. Toolchain reality: Rust nightly `wasm64-unknown-unknown` (wgpu changelog, 2026,
  requires `-Z build-std`), so wasm64 is real but not yet a production default. OVE must
  keep wasm heap budget under 4GB until wasm64 is baseline.
- **Streaming compilation**: `WebAssembly.instantiateStreaming` compiles from the network
  response while it downloads (works with any `application/wasm` response; all engines).
  Loads ffmpeg-class multi-MB modules faster; use code-caching + `compileStreaming` and lazy
  module instantiation per feature (probe vs mux vs filter).

## 2. ffmpeg.wasm performance reality

Evidence assembled today:

- Academic measurement (ar5iv: "Analyzing the Performance of WebAssembly vs. Native Code"):
  wasm on average **1.55x slower than native in Chrome, 1.45x in Firefox** — but that is a
  mixed benchmark, not video-codec workloads.
- Codec/SIMD-heavy reality is worse: wasm SIMD is 128-bit vs native AVX2 (256-bit);
  nickb.dev (2024) measured **~4x slower than native at large payloads**; the ffmpeg.wasm
  project itself states plainly that it "won't perform as good as FFmpeg" (official perf
  page). The ffmpeg.wasm maintainer's writeups (jeromewu, 2022) identify SIMD intrinsics +
  multithreading as the only paths to competitiveness, with threads blocked behind
  cross-origin isolation.
- Practical consensus band for video encode/decode workloads: **~2-5x slower than native**,
  worse when single-threaded (browser pthreads depend on SAB/COOP-COEP) and for encoders
  tuned for wide vectors (x265/VP9/AV1). Mark: consistent with all sources above; exact
  per-codec numbers = E-001 experiment.
- Conclusion: **wasm decode of 1080p H.264 is feasible; 4K or AV1/x265 encode at speed is
  not competitive**. Bulk media throughput belongs to WebCodecs (HW/SW platform codecs) or
  native platform layers; wasm is for the flexible long tail.

## 3. Right role for WASM in OVE (draft)

| Stage | Verdict | Rationale |
|---|---|---|
| Container probe/metadata parse (any format) | YES wasm | Byte parsing; perf irrelevant; format breadth wins (FFmpeg/mediainfo-class code) |
| Demux to EncodedVideoChunk | YES wasm (or Mediabunny TS) | Cheap; needed to feed WebCodecs for exotic containers |
| Mux to MP4/WebM | NO wasm — use Mediabunny/mp4box.js (TS) | Native-JS muxers exist, fast, small |
| Decode | NO wasm (except exotic codecs) | WebCodecs HW/SW beats it; wasm only fallback for formats WebCodecs rejects (e.g. ProRes, DNxHD, unusual VBR/HDR) |
| Encode | NO wasm for bulk; wasm for niche | Same; WebCodecs encoder ladder first |
| Filters/analysis (histogram, scene detect) | MAYBE wasm/GPU compute | SIMD helps; GPU compute (WebGPU) better on desktop; keep algorithm shared |
| Full-FFmpeg filter graph emulation | NO (long term) | Cost too high; implement effect passes natively in shader land (doc 08) |

## 4. Packaging/toolchain notes (for the Rust-core hypothesis, C-004)

- Rust -> wasm32-unknown-unknown is first-class (wasm-bindgen/wasm-pack ecosystem);
  wgpu/web backend runs on WebGL2+WebGPU from the same crate (doc 09).
- Rust -> wasm64 needs nightly (`-Z build-std`) per wgpu changelog — not production-ready.
- Threads: Rust std threads compile to wasm pthreads (worker pool + SAB) under
  `-C target-feature=+atomics` — again COOP/COEP dependent.
- Keep the shared core (timeline/project/command logic) allocation-light so the wasm build
  fits the 4GB ceiling even with large undo snapshots; heavy media buffers must live in
  JS/GPU land, not in wasm linear memory.

## 5. Risks

- R1: multithreaded wasm unusable inside non-COOP/COEP embeds — single-thread fallback path
  mandatory.
- R2: 4GB ceiling vs large-format exports (need streaming design, not in-memory files).
- R3: ffmpeg.wasm binary size (tens of MB) — lazy-load per feature; consider trimming to a
  demux/probe-only build.
- R4: Memory64 in Safari unverified — cannot rely on it in the plan.

## Sources (fetched 2026-09-21)
- ar5iv "Analyzing the Performance of WebAssembly vs. Native Code" (1.45-1.55x averages).
- nickb.dev "The WebAssembly value proposition..." (Jan 3, 2024; 4x large-payload figure).
- ffmpegwasm.netlify.app Performance page; jeromewu.github.io SIMD post (Aug 2022);
  github.com/ffmpegwasm/ffmpeg.wasm issue #415 "Next steps" (Sep 2022).
- caniuse sharedarraybuffer.json (COOP/COEP-era notes); webkit.org Safari 15.2 post (4GB
  wasm memory); SpiderMonkey blog "Is Memory64 actually worth using?" (Jan 15, 2025);
  Chromium Intent to Ship Memory64 (M133); webassembly.org feature status.
- gfx-rs/wgpu CHANGELOG (wasm64-unknown-unknown support, 2026).

## Depth tracking (v0.1)
- TODO: run E-001 wasm-vs-WebCodecs decode/encode microbench; measure ffmpeg.wasm
  single- vs multi-thread under COOP/COEP; verify Safari Memory64; evaluate a trimmed
  FFmpeg demux/probe wasm build size.
