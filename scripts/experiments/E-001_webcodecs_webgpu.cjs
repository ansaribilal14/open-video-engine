// E-001: WebCodecs → WebGPU browser pipeline experiment (headless Chromium).
// QUESTION: Can a browser decode real H.264 via WebCodecs and feed frames into a WebGPU
//           compositor (importExternalTexture) with per-frame GPU output?
// HYPOTHESIS: Chromium headless supports the full path under software rendering flags.
// CLAIMS: C-005 (WebCodecs viability), browser column of the capability matrix (doc 18).
const { chromium } = require("playwright");
const fs = require("fs");

const H264 = fs.readFileSync(__dirname + "/../../experiments/E-001_testsrc.h264").toString("base64");
const W = 160, H = 90, FPS = 24;

const CHROME = "/home/z/my-project/.browsers/chrome/linux-154.0.8037.57/chrome-linux64/chrome";
const FLAGSETS = [
  { name: "cft:default", channel: null, args: ["--no-sandbox", "--enable-unsafe-webgpu"] },
  { name: "cft:swiftshader-vulkan", channel: null, args: ["--no-sandbox", "--enable-unsafe-webgpu", "--use-vulkan", "--use-vulkan-swiftshader", "--enable-features=Vulkan"] },
  { name: "cft:angle-swiftshader", channel: null, args: ["--no-sandbox", "--enable-unsafe-webgpu", "--use-angle=swiftshader", "--disable-gpu-sandbox"] },
];

// Probe page JS: support matrix + real decode + GPU import + readback
const PROBE = async ({ h264b64, W, H, FPS }) => {
  const out = { url: location.href, ua: navigator.userAgent, gpu: null, matrix: null, decode: null, gpurender: null, errors: [] };
  try {
    if (!navigator.gpu) { out.errors.push("navigator.gpu undefined"); return out; }
    const adapter = await navigator.gpu.requestAdapter({ powerPreference: "high-performance" });
    if (!adapter) { out.errors.push("requestAdapter null"); return out; }
    const info = adapter.info || {};
    out.gpu = { vendor: info.vendor || "?", architecture: info.architecture || "?", device: info.device || "?", description: info.description || "?" };
  } catch (e) { out.errors.push("gpu-probe: " + e.message); return out; }

  // --- WebCodecs support matrix ---
  const codecs = ["avc1.42001E", "avc1.42E01E", "avc1.4D401E", "avc1.64001E", "avc1.640028",
    "vp09.00.10.08", "av01.0.04M.08", "hvc1.1.6.L93.B0", "hev1.1.6.L93.B0", "vp8"];
  const matrix = { decode: {}, encode: {} };
  try {
    for (const c of codecs) {
      try { const r = await VideoDecoder.isConfigSupported({ codec: c }); matrix.decode[c] = r.supported; } catch (e) { matrix.decode[c] = "err"; }
      try { const r = await VideoEncoder.isConfigSupported({ codec: c, width: 320, height: 180, bitrate: 500000, framerate: 24 }); matrix.encode[c] = r.supported; } catch (e) { matrix.encode[c] = "err"; }
    }
    out.matrix = matrix;
  } catch (e) { out.errors.push("matrix: " + e.message); }

  // --- real decode of annex-b H.264 ---
  const raw = atob(h264b64);
  const bytes = new Uint8Array(raw.length);
  for (let i = 0; i < raw.length; i++) bytes[i] = raw.charCodeAt(i);
  // split NALs on 3-byte start codes (00 00 01); annex-b may use 4-byte too — normalize
  const nals = [];
  let cur = null;
  for (let i = 0; i + 2 < bytes.length; i++) {
    if (bytes[i] === 0 && bytes[i + 1] === 0 && bytes[i + 2] === 1) {
      if (cur) nals.push(cur.subarray(0, cur.length - (i - cur.start - 3 >= 0 ? (i - 3 - cur.start >= 0 && bytes[i - 1] === 0 ? 1 : 0) : 0)));
      cur = { start: i + 3 };
      i += 2;
    }
  }
  // simpler robust re-split:
  const nalList = [];
  let start = -1;
  for (let i = 0; i < bytes.length - 2; i++) {
    if (bytes[i] === 0 && bytes[i + 1] === 0 && bytes[i + 2] === 1) {
      if (start >= 0) nalList.push(bytes.subarray(start, i - (bytes[i - 1] === 0 ? 1 : 0)));
      start = i + 3; i += 2;
    }
  }
  if (start >= 0) nalList.push(bytes.subarray(start));
  const vcl = nalList.filter(n => (n[0] & 0x1f) === 1 || (n[0] & 0x1f) === 5);
  const sps = nalList.find(n => (n[0] & 0x1f) === 7);
  const codec = sps ? "avc1." + [sps[1], sps[2], sps[3]].map(b => b.toString(16).padStart(2, "0")).join("") : "avc1.42E01E";
  out.decode = { codec, nalCount: nalList.length, vclFrames: vcl.length };

  // group NALs into frames: a frame = NALs from one VCL to before the next VCL
  const frames = [];
  let acc = [];
  for (const n of nalList) {
    const t = n[0] & 0x1f;
    if ((t === 1 || t === 5) && acc.some(x => { const xt = x[0] & 0x1f; return xt === 1 || xt === 5; })) {
      frames.push(acc); acc = [];
    }
    acc.push(n);
  }
  if (acc.length) frames.push(acc);

  let decoded = 0, decodeErr = null;
  try {
    const framesOut = [];
    await new Promise((resolve, reject) => {
      let done = 0, total = frames.length;
      const dec = new VideoDecoder({
        output: (vf) => { framesOut.push(vf); decoded++; },
        error: (e) => { decodeErr = e.message; reject(e); },
      });
      dec.configure({ codec, optimizeForLatency: true });
      const ts = (i) => Math.round(i * 1e6 / FPS);
      frames.forEach((f, i) => {
        const len = f.reduce((a, n) => a + n.length + 4, 0); // 4-byte lengths (annexb ok as data)
        const buf = new Uint8Array(len);
        let o = 0;
        for (const n of f) { buf[o] = 0; buf[o+1]=0; buf[o+2]=0; buf[o+3]=1; o += 4; buf.set(n, o); o += n.length; }
        const isKey = f.some(n => (n[0] & 0x1f) === 5);
        dec.decode(new EncodedVideoChunk({ type: isKey ? "key" : "delta", timestamp: ts(i), data: buf }));
      });
      dec.flush().then(() => { resolve(); }).catch(reject);
    });
    out.decode.decodedFrames = framesOut.length;
    out.decode.firstTs = framesOut[0]?.timestamp;
    out.decode.lastTs = framesOut[framesOut.length - 1]?.timestamp;

    // --- GPU import + render + readback for frame 0 and frame 20 ---
    async function gpuRender(vf) {
      const adapter = await navigator.gpu.requestAdapter();
      const device = await adapter.requestDevice();
      const module = device.createShaderModule({ code: `
        @vertex fn vs(@builtin(vertex_index) i: u32) -> @builtin(position) vec4<f32> {
          var p = array<vec2<f32>,3>(vec2(-1.0,-1.0), vec2(3.0,-1.0), vec2(-1.0,3.0));
          return vec4(p[i], 0.0, 1.0);
        }
        @group(0) @binding(0) var t: texture_external;
        @group(0) @binding(1) var s: sampler;
        @fragment fn fs(@builtin(position) pos: vec4<f32>) -> @location(0) vec4<f32> {
          let uv = pos.xy / vec2(${W}.0, ${H}.0);
          return textureSampleBaseClampToEdge(t, s, uv);
        }` });
      const bgl = device.createBindGroupLayout({ entries: [
        { binding: 0, visibility: GPUShaderStage.FRAGMENT, externalTexture: {} },
        { binding: 1, visibility: GPUShaderStage.FRAGMENT, sampler: { type: "filtering" } }] });
      const pipe = device.createRenderPipeline({ layout: device.createPipelineLayout({ bindGroupLayouts: [bgl] }),
        vertex: { module, entryPoint: "vs" }, fragment: { module, entryPoint: "fs", targets: [{ format: "rgba8unorm" }] } });
      const tex = device.createTexture({ size: [W, H], format: "rgba8unorm", usage: GPUTextureUsage.RENDER_ATTACHMENT | GPUTextureUsage.COPY_SRC });
      const ext = device.importExternalTexture({ source: vf });
      const bind = device.createBindGroup({ layout: bgl, entries: [
        { binding: 0, resource: ext },
        { binding: 1, resource: device.createSampler({ magFilter: "linear", minFilter: "linear" }) }] });
      const enc = device.createCommandEncoder();
      const pass = enc.beginRenderPass({ colorAttachments: [{ view: tex.createView(), loadOp: "clear", storeOp: "store", clearValue: { r: 1, g: 0, b: 1, a: 1 } }] });
      pass.setPipeline(pipe); pass.setBindGroup(0, bind); pass.draw(3); pass.end();
      const bytesPerRow = Math.ceil(W * 4 / 256) * 256;
      const rb = device.createBuffer({ size: bytesPerRow * H, usage: GPUBufferUsage.COPY_DST | GPUBufferUsage.MAP_READ });
      enc.copyTextureToBuffer({ texture: tex }, { buffer: rb, bytesPerRow, rowsPerImage: H }, [W, H]);
      device.queue.submit([enc.finish()]);
      await rb.mapAsync(GPUMapMode.READ);
      const u8 = new Uint8Array(rb.getMappedRange());
      let sum = 0; for (let i = 0; i < u8.length; i++) sum = (sum + u8[i]) & 0xffffffff;
      const result = { sum, first: [u8[0], u8[1], u8[2]], mid: [u8[(H >> 1) * bytesPerRow + (W >> 1) * 4], u8[(H >> 1) * bytesPerRow + (W >> 1) * 4 + 1], u8[(H >> 1) * bytesPerRow + (W >> 1) * 4 + 2]] };
      rb.unmap(); rb.destroy(); tex.destroy();
      return result;
    }
    if (framesOut.length >= 21) {
      const r0 = await gpuRender(framesOut[0]);
      const r20 = await gpuRender(framesOut[20]);
      out.gpurender = { frame0: r0, frame20: r20, framesDiffer: r0.sum !== r20.sum, importExternalTexture: "OK" };
    } else {
      out.gpurender = { skipped: "decoded frames < 21" };
    }
  } catch (e) { out.decode.error = decodeErr || e.message; }
  return out;
};

(async () => {
  const results = [];
  for (const fs2 of FLAGSETS) {
    let entry = { flags: fs2.name, ok: false };
    try {
      const launchOpts = { headless: true, args: fs2.args, executablePath: CHROME };
      const browser = await chromium.launch(launchOpts);
      const page = await browser.newPage();
      await page.goto("about:blank");
      const r = await page.evaluate(PROBE, { h264b64: H264, W, H, FPS });
      entry.result = r;
      entry.ok = !!(r.gpu && r.gpurender && r.gpurender.framesDiffer);
      await browser.close();
    } catch (e) { entry.error = e.message; }
    results.push(entry);
    if (entry.ok) break;
  }
  fs.writeFileSync(__dirname + "/../../experiments/E-001_result.json", JSON.stringify(results, null, 2));
  const good = results.find(r => r.ok);
  console.log("==== E-001 SUMMARY ====");
  if (good) {
    const r = good.result;
    console.log("WINNING FLAGS:", good.flags);
    console.log("GPU ADAPTER:", JSON.stringify(r.gpu));
    console.log("CODEC STRING:", r.decode.codec, "| NALs:", r.decode.nalCount, "| decoded frames:", r.decode.decodedFrames);
    console.log("GPU IMPORT:", JSON.stringify(r.gpurender));
    console.log("VERDICT: full WebCodecs->importExternalTexture->WebGPU path WORKS headless");
  } else {
    console.log("VERDICT: pipeline NOT proven headless — see errors", JSON.stringify(results.map(x => x.result?.errors || x.error), null, 1));
  }
})();
