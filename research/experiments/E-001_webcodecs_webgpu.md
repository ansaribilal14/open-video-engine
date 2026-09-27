# E-001 — WebCodecs → WebGPU browser pipeline (headless Chrome 154)

> Status: PARTIAL-RUN (decode leg PASSED with real media; GPU-import legs BLOCKED by
> software rendering; harness committed and re-runnable on real-GPU hardware).

- **QUESTION**: Can the browser decode real H.264 via WebCodecs and feed frames into a
  WebGPU compositor (`importExternalTexture`, `copyExternalImageToTexture`), with per-frame
  GPU output readable back?
- **HYPOTHESIS**: Full path works in Chromium headless under software rendering flags.
- **IMPLEMENTATION**: `scripts/experiments/E-001_cdp_v2.cjs` (Playwright over raw CDP,
  `--headless=new`) + `E-001_testsrc.mp4/.h264` (ffmpeg-generated 160×90 24fps Constrained
  Baseline, 48 frames) + ffprobe packet table for exact chunk framing. Readback attempts:
  buffer-map, canvas→2D.
- **HARDWARE**: container CPU only, SwiftShader Vulkan, no discrete GPU.
- **RESULT**:
  1. **Secure-context discovery**: `VideoDecoder`/`navigator.gpu` are SecureContext-only;
     on `about:blank` via CDP both are ABSENT. Serving over `http://127.0.0.1` exposes
     both. (Explains many "WebCodecs missing" reports in headless automation.)
  2. **Support matrix (Chrome 154 Linux)**: decode avc1.42E01E ✓, avc1.640028 ✓,
     vp9 ✓, av1 ✓, hvc1 ✗; encode avc1/vp9/av1 ✓, hevc ✗.
  3. **Real H.264 decode: 48/48 frames decoded** via VideoDecoder with annex-b chunks.
  4. `importExternalTexture(VideoFrame)`: **"Failed to import texture from video"** under
     software rendering.
  5. `copyExternalImageToTexture` path + buffer-map readback: fails with "A valid external
     Instance reference no longer exists" (Dawn/SwiftShader teardown); device lost =
     "destroyed". Fresh-device-per-frame did not help.
- **LIMITATIONS**: GPU legs need real-GPU hardware (macOS/Windows/any Vulkan+dGPU host);
  harness runs as-is. Playwright's bundled Chromium **lacks WebCodecs entirely** — Chrome
  for Testing required. Headless shell lacks WebGPU.
- **DECISION**: C-005 upgraded to MEDIUM+ (decode proven with real media on Linux Chrome).
  Browser capability matrix (doc 18) updated. New open question Q-08: verify both import
  paths on real GPU; WebGL2 fallback stays mandatory regardless (matrix holes + software
  rendering). Falsifies nothing; refines the layered-adapter thesis: the zero-copy
  import is an OPTIMIZATION behind the adapter, the copy path is the portable contract.
