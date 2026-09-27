// E-001 final attempt: spawn Chrome for Testing directly with --headless=new,
// connect over CDP (bypasses Playwright launcher flags), run full probe.
const { chromium } = require("playwright");
const { spawn } = require("child_process");
const fs = require("fs");
const http = require("http");

const CHROME = "/home/z/my-project/.browsers/chrome/linux-154.0.8037.57/chrome-linux64/chrome";
const H264 = fs.readFileSync(__dirname + "/../../experiments/E-001_testsrc.h264").toString("base64");
const W = 160, H = 90, FPS = 24;

// minimal static server: about:blank via CDP is NOT a secure context; 127.0.0.1 is
const SERVER_PORT = 8765;
const srv = http.createServer((req, res) => {
  res.writeHead(200, { "content-type": "text/html" });
  res.end("<html><body>probe</body></html>");
});
const serverReady = new Promise((r) => srv.listen(SERVER_PORT, "127.0.0.1", r));

const waitPort = (port, tries = 50) => new Promise((res, rej) => {
  const ping = (n) => http.get({ host: "127.0.0.1", port, path: "/json/version" }, (r) => { r.resume(); res(); }).on("error", () => n <= 0 ? rej(new Error("no cdp")) : setTimeout(() => ping(n - 1), 200));
  ping(tries);
});

// Same probe as E-001 but standalone; returns support matrix + decode + GPU render.
const PROBE = async ({ h264b64 }) => {
  const out = { ua: navigator.userAgent, gpu: null, matrix: null, decode: null, gpurender: null, errors: [] };
  const hasWC = typeof VideoDecoder !== "undefined";
  out.webcodecs = hasWC ? "present" : "absent";
  if (!navigator.gpu) { out.errors.push("navigator.gpu undefined"); }
  else {
    const ad = await navigator.gpu.requestAdapter({ powerPreference: "high-performance" });
    out.gpu = ad ? { info: JSON.parse(JSON.stringify(ad.info || {})), features: [...ad.features].slice(0, 12) } : null;
    if (!ad) out.errors.push("requestAdapter null");
  }
  // support matrix regardless
  const codecs = ["avc1.42001E", "avc1.42E01E", "avc1.4D401E", "avc1.64001E", "avc1.640028",
    "vp09.00.10.08", "av01.0.04M.08", "hvc1.1.6.L93.B0", "hev1.1.6.L93.B0", "vp8"];
  if (hasWC) {
    const matrix = { decode: {}, encode: {} };
    for (const c of codecs) {
      try { matrix.decode[c] = (await VideoDecoder.isConfigSupported({ codec: c })).supported; } catch { matrix.decode[c] = "err"; }
      try { matrix.encode[c] = (await VideoEncoder.isConfigSupported({ codec: c, width: 320, height: 180, bitrate: 500000, framerate: 24 })).supported; } catch { matrix.encode[c] = "err"; }
    }
    out.matrix = matrix;
  }
  if (!hasWC) return out;

  // decode annex-b h264
  const raw = atob(h264b64);
  const bytes = new Uint8Array(raw.length);
  for (let i = 0; i < raw.length; i++) bytes[i] = raw.charCodeAt(i);
  const nalList = []; let start = -1;
  for (let i = 0; i < bytes.length - 2; i++) {
    if (bytes[i] === 0 && bytes[i + 1] === 0 && bytes[i + 2] === 1) {
      if (start >= 0) nalList.push(bytes.subarray(start, i - (bytes[i - 1] === 0 ? 1 : 0)));
      start = i + 3; i += 2;
    }
  }
  if (start >= 0) nalList.push(bytes.subarray(start));
  const sps = nalList.find(n => (n[0] & 0x1f) === 7);
  const codec = sps ? "avc1." + [sps[1], sps[2], sps[3]].map(b => b.toString(16).padStart(2, "0")).join("") : "avc1.42E01E";
  const frames = []; let acc = [];
  for (const n of nalList) {
    const t = n[0] & 0x1f;
    if ((t === 1 || t === 5) && acc.some(x => { const xt = x[0] & 0x1f; return xt === 1 || xt === 5; })) { frames.push(acc); acc = []; }
    acc.push(n);
  }
  if (acc.length) frames.push(acc);
  out.decode = { codec, nalCount: nalList.length, frameCount: frames.length };
  try {
    const framesOut = [];
    await new Promise((resolve, reject) => {
      const dec = new VideoDecoder({
        output: (vf) => framesOut.push(vf),
        error: (e) => reject(e),
      });
      dec.configure({ codec, optimizeForLatency: true });
      frames.forEach((f, i) => {
        const len = f.reduce((a, n) => a + n.length + 4, 0);
        const buf = new Uint8Array(len); let o = 0;
        for (const n of f) { buf[o] = 0; buf[o+1]=0; buf[o+2]=0; buf[o+3]=1; o += 4; buf.set(n, o); o += n.length; }
        const isKey = f.some(n => (n[0] & 0x1f) === 5);
        dec.decode(new EncodedVideoChunk({ type: isKey ? "key" : "delta", timestamp: Math.round(i * 1e6 / FPS), data: buf }));
      });
      // watchdog: resolve with whatever decoded so far
      const watchdog = setTimeout(() => resolve(), 25000);
      dec.flush().then(() => { clearTimeout(watchdog); resolve(); }).catch((e) => { clearTimeout(watchdog); reject(e); });
    });
    out.decode.decodedFrames = framesOut.length;

    async function gpuRender(vf) {
      const device = (window.__d ||= (async () => (await navigator.gpu.requestAdapter()).requestDevice())());
      const dev = await device;
      const module = dev.createShaderModule({ code: `
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
      const bgl = dev.createBindGroupLayout({ entries: [
        { binding: 0, visibility: GPUShaderStage.FRAGMENT, externalTexture: {} },
        { binding: 1, visibility: GPUShaderStage.FRAGMENT, sampler: { type: "filtering" } }] });
      const pipe = dev.createRenderPipeline({ layout: dev.createPipelineLayout({ bindGroupLayouts: [bgl] }),
        vertex: { module, entryPoint: "vs" }, fragment: { module, entryPoint: "fs", targets: [{ format: "rgba8unorm" }] } });
      const tex = dev.createTexture({ size: [W, H], format: "rgba8unorm", usage: GPUTextureUsage.RENDER_ATTACHMENT | GPUTextureUsage.COPY_SRC });
      const ext = dev.importExternalTexture({ source: vf });
      const bind = dev.createBindGroup({ layout: bgl, entries: [
        { binding: 0, resource: ext },
        { binding: 1, resource: dev.createSampler({ magFilter: "linear", minFilter: "linear" }) }] });
      const enc = dev.createCommandEncoder();
      const pass = enc.beginRenderPass({ colorAttachments: [{ view: tex.createView(), loadOp: "clear", storeOp: "store", clearValue: { r: 1, g: 0, b: 1, a: 1 } }] });
      pass.setPipeline(pipe); pass.setBindGroup(0, bind); pass.draw(3); pass.end();
      const bpr = Math.ceil(W * 4 / 256) * 256;
      const rb = dev.createBuffer({ size: bpr * H, usage: GPUBufferUsage.COPY_DST | GPUBufferUsage.MAP_READ });
      enc.copyTextureToBuffer({ texture: tex }, { buffer: rb, bytesPerRow: bpr, rowsPerImage: H }, [W, H]);
      dev.queue.submit([enc.finish()]);
      await rb.mapAsync(GPUMapMode.READ);
      const u8 = new Uint8Array(rb.getMappedRange());
      let sum = 0; for (let i = 0; i < u8.length; i++) sum = (sum + u8[i]) & 0xffffffff;
      const res = { sum, mid: [u8[(H >> 1) * bpr + (W >> 1) * 4], u8[(H >> 1) * bpr + (W >> 1) * 4 + 1], u8[(H >> 1) * bpr + (W >> 1) * 4 + 2]] };
      rb.unmap(); rb.destroy(); tex.destroy();
      return res;
    }
    if (navigator.gpu && framesOut.length >= 21) {
      const r0 = await gpuRender(framesOut[0]);
      const r20 = await gpuRender(framesOut[20]);
      out.gpurender = { frame0: r0, frame20: r20, framesDiffer: r0.sum !== r20.sum, importExternalTexture: "OK" };
    } else out.gpurender = { skipped: navigator.gpu ? "<21 frames" : "no webgpu" };
  } catch (e) { out.decode.error = e.message; }
  return out;
};

(async () => {
  const results = [];
  const combos = [
    ["--headless=new", "--no-sandbox", "--enable-unsafe-webgpu"],
    ["--headless=new", "--no-sandbox", "--enable-unsafe-webgpu", "--use-vulkan", "--use-vulkan-swiftshader", "--enable-features=Vulkan"],
    ["--headless=new", "--no-sandbox", "--enable-unsafe-webgpu", "--use-angle=swiftshader", "--enable-features=Vulkan"],
  ];
  const withTimeout = (promise, ms, tag) => Promise.race([
    promise,
    new Promise((_, rej) => setTimeout(() => rej(new Error("TIMEOUT:" + tag)), ms)),
  ]);
  for (const extra of combos) {
    const port = 9222 + Math.floor(Math.random() * 500);
    const proc = spawn(CHROME, [...extra, `--remote-debugging-port=${port}`, "--user-data-dir=/tmp/cdp-" + port, "--disable-dev-shm-usage", "about:blank"], { stdio: "ignore" });
    let entry = { flags: extra.join(" "), ok: false };
    try {
      await withTimeout(waitPort(port), 15000, "cdp-port");
      const b = await chromium.connectOverCDP(`http://127.0.0.1:${port}`);
      await serverReady;
      const page = await (b.contexts()[0] || b.newContext()).newPage();
      await page.goto(`http://127.0.0.1:${SERVER_PORT}/`);
      const r = await withTimeout(page.evaluate(PROBE, { h264b64: H264 }), 60000, "probe");
      entry.result = r;
      entry.ok = !!(r.gpu && r.gpurender && r.gpurender.framesDiffer);
      await b.close();
    } catch (e) { entry.error = e.message.slice(0, 200); }
    try { proc.kill("SIGKILL"); } catch {}
    results.push(entry);
    console.log("combo:", extra.slice(1, 4).join(" "), "=> ok:", entry.ok, "| gpu:", entry.result?.gpu ? "YES" : "no", "| webcodecs:", entry.result?.webcodecs, "| decoded:", entry.result?.decode?.decodedFrames ?? "-", "| err:", (entry.result?.errors || []).concat(entry.error || []));
    if (entry.ok) break;
  }
  fs.writeFileSync(__dirname + "/../../experiments/E-001_result.json", JSON.stringify(results, null, 2));
  const good = results.find(r => r.ok);
  console.log("\n==== E-001 SUMMARY ====");
  if (good) {
    const r = good.result;
    console.log("WINNING FLAGS:", good.flags);
    console.log("GPU:", JSON.stringify(r.gpu && r.gpu.info));
    console.log("CODEC:", r.decode.codec, "| decoded:", r.decode.decodedFrames, "frames");
    console.log("GPU IMPORT+RENDER:", JSON.stringify(r.gpurender));
    console.log("VERDICT: full WebCodecs -> importExternalTexture -> WebGPU pipeline WORKS headless");
  } else {
    console.log("VERDICT: WebGPU unavailable in this container; see E-001_result.json for the support matrix + decode evidence");
  }
})();
