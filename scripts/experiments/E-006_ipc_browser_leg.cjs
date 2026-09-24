// E-006 (browser leg): IPC transport cost approximation for the desktop shell.
//
// Tauri v2 offers three transports: (1) invoke() — JSON-serialized command
// payloads; (2) Channel<T> — streaming serialized messages; (3) custom
// URI-protocol responses — raw bytes (binary path). This harness measures the
// WEBVIEW-SIDE serialization/message costs of those patterns in headless
// Chromium (the same class of webview engine). It does NOT run real Tauri
// (blocked: no webkit2gtk/display in container) — recorded honestly as a
// browser-leg PARTIAL proxy: numbers are lower bounds / relative comparators.
//
// Payload tiers model engine traffic:
//   S  ~128B   single timeline command (e.g. move clip)
//   M  ~8KB    command batch (e.g. scrub tick batch, 50 commands)
//   L  ~256KB  project snapshot delta
//   XL ~2MB    asset metadata / thumbnail manifest
//
// Frame budget context: 60fps => 16.7ms; interactive scrubbing issues
// command bursts — anything above ~1ms per burst is editor-relevant.
const { chromium } = require("playwright");
const fs = require("fs");

const SHELL = "/home/z/.cache/ms-playwright/chromium_headless_shell-1243/chrome-headless-shell-linux64/chrome-headless-shell";
const OUT = "/home/z/my-project/open-video-engine/experiments/E-006_result.json";

function makePayload(bytes) {
  // structured object with string+numbers+array mix (realistic command shape)
  const items = Math.max(1, Math.floor(bytes / 64));
  return {
    v: 1, type: "cmd.batch", ids: Array.from({ length: items }, (_, i) => `clip-${i}`),
    ops: Array.from({ length: items }, (_, i) => ({ k: i, t: 1000 + i, d: (i * 7919) % 997 })),
    blob: "x".repeat(Math.max(0, bytes - items * 64)),
  };
}

const BENCH = async ({ sizes }) => {
  const out = { ua: navigator.userAgent, results: {} };
  function makePayload(bytes) {
    const items = Math.max(1, Math.floor(bytes / 64));
    return {
      v: 1, type: "cmd.batch", ids: Array.from({ length: items }, (_, i) => `clip-${i}`),
      ops: Array.from({ length: items }, (_, i) => ({ k: i, t: 1000 + i, d: (i * 7919) % 997 })),
      blob: "x".repeat(Math.max(0, bytes - items * 64)),
    };
  }
  const timeIt = (fn, iters) => {
    // warmup then measure with performance.now()
    for (let i = 0; i < Math.min(20, iters); i++) fn();
    const t0 = performance.now();
    for (let i = 0; i < iters; i++) fn();
    return (performance.now() - t0) * 1000 / iters; // us per op
  };
  for (const [tag, bytes] of Object.entries(sizes)) {
    const p = makePayload(bytes);
    const s = JSON.stringify(p);
    const buf = new TextEncoder().encode(s);
    const cloneBuf = new Uint8Array(buf.length); // destination for binary copy leg
    const iters = bytes < 16384 ? 2000 : (bytes < 1048576 ? 200 : 50);
    out.results[tag] = {
      bytes: s.length,
      json_roundtrip_us: +timeIt(() => { JSON.parse(JSON.stringify(p)); }, iters).toFixed(2),
      structured_clone_us: +timeIt(() => { structuredClone(p); }, iters).toFixed(2),
      text_encode_us: +timeIt(() => { new TextEncoder().encode(JSON.stringify(p)); }, iters).toFixed(2),
      json_parse_only_us: +timeIt(() => { JSON.parse(s); }, iters).toFixed(2),
      binary_copy_us: +timeIt(() => { cloneBuf.set(buf); }, iters).toFixed(2),
    };
  }
  // MessageChannel round-trip latency (includes task hop; approximates
  // webview<->host message plumbing beyond pure serialization)
  const mc = (bytes, iters) => new Promise((resolve) => {
    const ch = new MessageChannel();
    const p = makePayload(bytes);
    let n = 0; const t0 = performance.now();
    ch.port1.onmessage = () => {
      if (++n >= iters) { resolve((performance.now() - t0) * 1000 / iters); ch.port1.close(); }
      else ch.port2.postMessage(p);
    };
    ch.port2.postMessage(p);
  });
  out.messagechannel_roundtrip_us = {
    s_128b: +(await mc(128, 500)).toFixed(2),
    m_8kb: +(await mc(8192, 500)).toFixed(2),
    l_256kb: +(await mc(262144, 100)).toFixed(2),
    xl_2mb: +(await mc(2097152, 30)).toFixed(2),
  };
  return out;
};

(async () => {
  const browser = await chromium.launch({ executablePath: SHELL, args: ["--no-sandbox"] });
  const page = await browser.newPage();
  const sizes = { s_128b: 128, m_8kb: 8192, l_256kb: 262144, xl_2mb: 2097152 };
  const res = await page.evaluate(BENCH, { sizes });
  res.note = "browser-leg only; real Tauri (webkit2gtk) not runnable in container; " +
             "values are webview-side lower bounds, used as RELATIVE comparators";
  res.frame_budget_context = { fps60_budget_ms: 16.7, scrub_burst_target_ms: 1.0 };
  fs.writeFileSync(OUT, JSON.stringify(res, null, 2));
  console.log(JSON.stringify(res.results, null, 2));
  console.log("messagechannel_roundtrip_us:", JSON.stringify(res.messagechannel_roundtrip_us));
  await browser.close();
})().catch((e) => { console.error("FATAL", e); process.exit(1); });
