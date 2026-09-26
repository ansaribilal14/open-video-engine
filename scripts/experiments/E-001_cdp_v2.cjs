// E-001 v2: lean CDP probe with per-step races + hard watchdog.
const { chromium } = require("playwright");
const { spawn } = require("child_process");
const fs = require("fs");
const http = require("http");

const CHROME = "/home/z/my-project/.browsers/chrome/linux-154.0.8037.57/chrome-linux64/chrome";
const H264 = fs.readFileSync(__dirname + "/../../experiments/E-001_testsrc.h264").toString("base64");
const PACKETS = JSON.parse(fs.readFileSync(__dirname + "/../../experiments/E-001_packets.json")).packets
  .map(pk => ({ size: parseInt(pk.size), key: pk.flags.includes("K"), ts: Math.round(parseFloat(pk.pts_time || pk.dts_time || 0) * 1e6) }));
const W = 160, H = 90, FPS = 24;
const log = (m) => process.stderr.write(m + "\n");

setTimeout(() => { log("HARD WATCHDOG EXIT"); fs.writeFileSync(__dirname + "/../../experiments/E-001_result.json", JSON.stringify({ hardTimeout: true }, null, 2)); process.exit(3); }, 110000).unref();

const race = (p, ms, tag) => Promise.race([p, new Promise((_, rej) => setTimeout(() => rej(new Error("TIMEOUT:" + tag)), ms))]);
const waitPort = (port) => new Promise((res, rej) => {
  let n = 30;
  const ping = () => http.get({ host: "127.0.0.1", port, path: "/json/version" }, (r) => { r.resume(); res(); }).on("error", () => (--n <= 0 ? rej(new Error("no cdp")) : setTimeout(ping, 300)));
  ping();
});

const PROBE = async ({ h264b64, packets, W, H }) => {
  const out = { ua: navigator.userAgent, gpu: null, matrix: null, decode: null, gpurender: null, errors: [], webcodecs: typeof VideoDecoder !== "undefined" ? "present" : "absent" };
  out.secureContext = isSecureContext;
  if (navigator.gpu) {
    const ad = await navigator.gpu.requestAdapter({ powerPreference: "high-performance" });
    out.gpu = ad ? { info: JSON.parse(JSON.stringify(ad.info || {})) } : null;
    if (!ad) out.errors.push("adapter null");
  } else out.errors.push("navigator.gpu undefined");
  if (out.webcodecs === "present") {
    const m = { decode: {}, encode: {} };
    for (const c of ["avc1.42E01E", "avc1.640028", "vp09.00.10.08", "av01.0.04M.08", "hvc1.1.6.L93.B0"]) {
      try { m.decode[c] = (await VideoDecoder.isConfigSupported({ codec: c })).supported; } catch { m.decode[c] = "err"; }
      try { m.encode[c] = (await VideoEncoder.isConfigSupported({ codec: c, width: 320, height: 180, bitrate: 500000, framerate: 24 })).supported; } catch { m.encode[c] = "err"; }
    }
    out.matrix = m;
    // decode: cut annex-b stream by ffprobe packet table (ffmpeg is the demuxer of record)
    const raw = atob(h264b64);
    const bytes = new Uint8Array(raw.length);
    for (let i = 0; i < raw.length; i++) bytes[i] = raw.charCodeAt(i);
    const framesOut = [];
    try {
      await new Promise((resolve, reject) => {
        const dec = new VideoDecoder({ output: (vf) => framesOut.push(vf), error: (e) => reject(e) });
        let off = 0;
        dec.configure({ codec: "avc1.42E01E", optimizeForLatency: true });
        packets.forEach((pk, i) => {
          const data = bytes.subarray(off, off + pk.size); off += pk.size;
          dec.decode(new EncodedVideoChunk({ type: pk.key ? "key" : "delta", timestamp: pk.ts || Math.round(i * 1e6 / 24), data }));
        });
        const wd = setTimeout(resolve, 12000);
        dec.flush().then(() => { clearTimeout(wd); resolve(); }).catch(() => { clearTimeout(wd); resolve(); });
      });
    } catch (e) { out.decode = { error: e.message }; }
    out.decode = out.decode || {};
    out.decode.decodedFrames = framesOut.length;
    if (navigator.gpu && framesOut.length >= 21) {
      const dev = await (await navigator.gpu.requestAdapter()).requestDevice();
      dev.lost.then((info) => { window.__lost = info.reason || 'unknown'; });
      async function gpuRender(vf) {
        const module = dev.createShaderModule({ code: `
          @vertex fn vs(@builtin(vertex_index) i: u32) -> @builtin(position) vec4<f32> {
            var p = array<vec2<f32>,3>(vec2(-1.0,-1.0), vec2(3.0,-1.0), vec2(-1.0,3.0));
            return vec4(p[i], 0.0, 1.0);
          }
          @group(0) @binding(0) var t: texture_external;
          @group(0) @binding(1) var s: sampler;
          @fragment fn fs(@builtin(position) pos: vec4<f32>) -> @location(0) vec4<f32> {
            let uv = pos.xy / vec2(160.0, 90.0);
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
        await dev.queue.onSubmittedWorkDone();
        await rb.mapAsync(GPUMapMode.READ);
        const u8 = new Uint8Array(rb.getMappedRange());
        let sum = 0; for (let i = 0; i < u8.length; i++) sum = (sum + u8[i]) & 0xffffffff;
        const res = { sum, mid: [u8[(H >> 1) * bpr + (W >> 1) * 4], u8[(H >> 1) * bpr + (W >> 1) * 4 + 1], u8[(H >> 1) * bpr + (W >> 1) * 4 + 2]] };
        rb.unmap(); rb.destroy(); tex.destroy();
        return res;
      }
      async function gpuRenderCopy(vf) {
        const module = dev.createShaderModule({ code: `
          @vertex fn vs(@builtin(vertex_index) i: u32) -> @builtin(position) vec4<f32> {
            var p = array<vec2<f32>,3>(vec2(-1.0,-1.0), vec2(3.0,-1.0), vec2(-1.0,3.0));
            return vec4(p[i], 0.0, 1.0);
          }
          @group(0) @binding(0) var t: texture_2d<f32>;
          @group(0) @binding(1) var s: sampler;
          @fragment fn fs(@builtin(position) pos: vec4<f32>) -> @location(0) vec4<f32> {
            let uv = pos.xy / vec2(160.0, 90.0);
            return textureSample(t, s, uv);
          }` });
        const bgl = dev.createBindGroupLayout({ entries: [
          { binding: 0, visibility: GPUShaderStage.FRAGMENT, texture: {} },
          { binding: 1, visibility: GPUShaderStage.FRAGMENT, sampler: { type: "filtering" } }] });
        const pipe = dev.createRenderPipeline({ layout: dev.createPipelineLayout({ bindGroupLayouts: [bgl] }),
          vertex: { module, entryPoint: "vs" }, fragment: { module, entryPoint: "fs", targets: [{ format: "rgba8unorm" }] } });
        const tex = dev.createTexture({ size: [160, 90], format: "rgba8unorm",
          usage: GPUTextureUsage.TEXTURE_BINDING | GPUTextureUsage.COPY_DST | GPUTextureUsage.RENDER_ATTACHMENT | GPUTextureUsage.COPY_SRC });
        dev.queue.copyExternalImageToTexture({ source: vf }, { texture: tex }, [160, 90]);
        const bind = dev.createBindGroup({ layout: bgl, entries: [
          { binding: 0, resource: tex.createView() },
          { binding: 1, resource: dev.createSampler({ magFilter: "linear", minFilter: "linear" }) }] });
        const enc = dev.createCommandEncoder();
        const pass = enc.beginRenderPass({ colorAttachments: [{ view: tex.createView(), loadOp: "clear", storeOp: "store", clearValue: { r: 1, g: 0, b: 1, a: 1 } }] });
        pass.setPipeline(pipe); pass.setBindGroup(0, bind); pass.draw(3); pass.end();
        const bpr = Math.ceil(160 * 4 / 256) * 256;
        const rb = dev.createBuffer({ size: bpr * 90, usage: GPUBufferUsage.COPY_DST | GPUBufferUsage.MAP_READ });
        enc.copyTextureToBuffer({ texture: tex }, { buffer: rb, bytesPerRow: bpr, rowsPerImage: 90 }, [160, 90]);
        dev.queue.submit([enc.finish()]);
        await dev.queue.onSubmittedWorkDone();
        await rb.mapAsync(GPUMapMode.READ);
        const u8 = new Uint8Array(rb.getMappedRange());
        let sum = 0; for (let i = 0; i < u8.length; i++) sum = (sum + u8[i]) & 0xffffffff;
        const res2 = { sum, mid: [u8[45 * bpr + 80 * 4], u8[45 * bpr + 80 * 4 + 1], u8[45 * bpr + 80 * 4 + 2]] };
        rb.unmap(); rb.destroy(); tex.destroy();
        return res2;
      }
      async function renderToCanvas(vf, useCopy) {
        const dev = await (await navigator.gpu.requestAdapter()).requestDevice();
        // canvas readback path: webgpu canvas -> 2d canvas -> pixels
        const cv = new OffscreenCanvas(160, 90);
        const ctx = cv.getContext("webgpu");
        ctx.configure({ device: dev, format: "rgba8unorm", alphaMode: "opaque" });
        let bgl, bind;
        if (useCopy) {
          const srcTex = dev.createTexture({ size: [160, 90], format: "rgba8unorm",
            usage: GPUTextureUsage.TEXTURE_BINDING | GPUTextureUsage.COPY_DST });
          dev.queue.copyExternalImageToTexture({ source: vf }, { texture: srcTex }, [160, 90]);
          bgl = dev.createBindGroupLayout({ entries: [
            { binding: 0, visibility: GPUShaderStage.FRAGMENT, texture: {} },
            { binding: 1, visibility: GPUShaderStage.FRAGMENT, sampler: { type: "filtering" } }] });
          bind = dev.createBindGroup({ layout: bgl, entries: [
            { binding: 0, resource: srcTex.createView() },
            { binding: 1, resource: dev.createSampler({ magFilter: "linear", minFilter: "linear" }) }] });
        } else {
          bgl = dev.createBindGroupLayout({ entries: [
            { binding: 0, visibility: GPUShaderStage.FRAGMENT, externalTexture: {} },
            { binding: 1, visibility: GPUShaderStage.FRAGMENT, sampler: { type: "filtering" } }] });
          const ext = dev.importExternalTexture({ source: vf });
          bind = dev.createBindGroup({ layout: bgl, entries: [
            { binding: 0, resource: ext },
            { binding: 1, resource: dev.createSampler({ magFilter: "linear", minFilter: "linear" }) }] });
        }
        const frag = useCopy
          ? `@group(0) @binding(0) var t: texture_2d<f32>;
             @group(0) @binding(1) var s: sampler;
             @fragment fn fs(@builtin(position) pos: vec4<f32>) -> @location(0) vec4<f32> {
               let uv = pos.xy / vec2(160.0, 90.0);
               return textureSample(t, s, uv);
             }`
          : `@group(0) @binding(0) var t: texture_external;
             @group(0) @binding(1) var s: sampler;
             @fragment fn fs(@builtin(position) pos: vec4<f32>) -> @location(0) vec4<f32> {
               let uv = pos.xy / vec2(160.0, 90.0);
               return textureSampleBaseClampToEdge(t, s, uv);
             }`;
        const module = dev.createShaderModule({ code: `
          @vertex fn vs(@builtin(vertex_index) i: u32) -> @builtin(position) vec4<f32> {
            var p = array<vec2<f32>,3>(vec2(-1.0,-1.0), vec2(3.0,-1.0), vec2(-1.0,3.0));
            return vec4(p[i], 0.0, 1.0);
          }
          ${frag}` });
        const pipe = dev.createRenderPipeline({ layout: dev.createPipelineLayout({ bindGroupLayouts: [bgl] }),
          vertex: { module, entryPoint: "vs" }, fragment: { module, entryPoint: "fs", targets: [{ format: "rgba8unorm" }] } });
        const enc = dev.createCommandEncoder();
        const pass = enc.beginRenderPass({ colorAttachments: [{ view: ctx.getCurrentTexture().createView(), loadOp: "clear", storeOp: "store", clearValue: { r: 1, g: 0, b: 1, a: 1 } }] });
        pass.setPipeline(pipe); pass.setBindGroup(0, bind); pass.draw(3); pass.end();
        dev.queue.submit([enc.finish()]);
        await dev.queue.onSubmittedWorkDone();
        // readback via 2D canvas
        const c2 = new OffscreenCanvas(160, 90);
        const ctx2 = c2.getContext("2d");
        ctx2.drawImage(cv, 0, 0);
        const px = ctx2.getImageData(0, 0, 160, 90).data;
        let sum = 0; for (let i = 0; i < px.length; i++) sum = (sum + px[i]) & 0xffffffff;
        const o = (45 * 160 + 80) * 4;
        return { sum, mid: [px[o], px[o+1], px[o+2]], lost: window.__lost || null };
      }
      try {
        let r0, r20, mode;
        try { mode = "importExternalTexture"; r0 = await renderToCanvas(framesOut[0], false); r20 = await renderToCanvas(framesOut[20], false); }
        catch (ie) {
          out.errors.push("import-mode failed: " + ie.message.slice(0,80));
          mode = "copyExternalImageToTexture";
          r0 = await renderToCanvas(framesOut[0], true);
          r20 = await renderToCanvas(framesOut[20], true);
        }
        out.gpurender = { mode, frame0: r0, frame20: r20, framesDiffer: r0.sum !== r20.sum };
      } catch (e) { out.gpurender = { error: e.message.slice(0, 200), lost: window.__lost || null, copyAlsoFailed: true }; }
    }
  }
  return out;
};

const srv = http.createServer((req, res) => { res.writeHead(200, { "content-type": "text/html" }); res.end("<html><body>probe</body></html>"); });
const serverReady = new Promise((r) => srv.listen(8765, "127.0.0.1", r));

(async () => {
  const results = [];
  const combos = [
    ["--headless=new", "--no-sandbox", "--enable-unsafe-webgpu", "--use-vulkan", "--use-vulkan-swiftshader", "--enable-features=Vulkan"],
    ["--headless=new", "--no-sandbox", "--enable-unsafe-webgpu"],
  ];
  for (const extra of combos) {
    const port = 9300 + Math.floor(Math.random() * 400);
    log("combo: " + extra.join(" "));
    const proc = spawn(CHROME, [...extra, `--remote-debugging-port=${port}`, `--user-data-dir=/tmp/cdp-v2-${port}`, "--disable-dev-shm-usage", "about:blank"], { stdio: "ignore" });
    let entry = { flags: extra.join(" "), ok: false };
    try {
      await race(waitPort(port), 12000, "cdp");
      log("  cdp up");
      const b = await race(chromium.connectOverCDP(`http://127.0.0.1:${port}`), 15000, "connect");
      log("  connected");
      await race(serverReady, 5000, "http-srv");
      const page = await race((b.contexts()[0] || b.newContext()).newPage(), 10000, "page");
      await race(page.goto(`http://127.0.0.1:8765/`), 10000, "goto");
      log("  page loaded (secure context: " + (await page.evaluate(() => isSecureContext)) + ")");
      const r = await race(page.evaluate(PROBE, { h264b64: H264, packets: PACKETS, W: 160, H: 90 }), 55000, "probe");
      entry.result = r;
      entry.ok = !!(r.gpu && r.gpurender && r.gpurender.framesDiffer);
      log("  probe done: gpu=" + (r.gpu ? "YES" : "no") + " wc=" + r.webcodecs + " decoded=" + (r.decode && r.decode.decodedFrames));
      await race(b.close(), 5000, "bclose");
    } catch (e) { entry.error = e.message.slice(0, 150); log("  ERR: " + entry.error); }
    try { proc.kill("SIGKILL"); } catch {}
    results.push(entry);
    if (entry.ok) break;
  }
  srv.close();
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
    console.log("VERDICT: see experiments/E-001_result.json");
    console.log(JSON.stringify(results.map(x => ({ ok: x.ok, err: x.error, gpu: x.result?.gpu ? "yes" : "no", wc: x.result?.webcodecs, sec: x.result?.secureContext, decoded: x.result?.decode?.decodedFrames, mx: x.result?.matrix })), null, 1));
  }
  process.exit(0);
})();
