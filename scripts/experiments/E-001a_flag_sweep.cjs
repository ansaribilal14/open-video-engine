// E-001a: minimal navigator.gpu flag sweep
const { chromium } = require("playwright");
const COMBOS = [
  ["--enable-unsafe-webgpu", "--no-sandbox", "--use-vulkan", "--use-vulkan-swiftshader", "--enable-features=Vulkan", "--disable-gpu-sandbox"],
  ["--enable-unsafe-webgpu", "--no-sandbox", "--use-webgpu-adapter=swiftshader", "--enable-features=Vulkan"],
  ["--enable-unsafe-webgpu", "--no-sandbox", "--use-angle=vulkan", "--use-vulkan-swiftshader", "--enable-features=Vulkan,SkipVulkanVerification"],
  ["--enable-unsafe-webgpu", "--no-sandbox", "--use-angle=swiftshader", "--use-webgpu-adapter=swiftshader"],
  ["--enable-features=Vulkan", "--enable-unsafe-webgpu", "--no-sandbox", "--ignore-gpu-blocklist", "--enable-gpu"],
];
(async () => {
  for (const channel of ["chromium", null]) {
    for (const args of COMBOS) {
      const tag = (channel || "shell") + " | " + args.filter(a => a !== "--no-sandbox").join(" ");
      try {
        const opts = { headless: true, args };
        if (channel) opts.channel = channel;
        const b = await chromium.launch(opts);
        const p = await b.newPage();
        const r = await p.evaluate(async () => {
          if (!navigator.gpu) return { presence: "undefined" };
          const ad = await navigator.gpu.requestAdapter();
          if (!ad) return { presence: "defined", adapter: null };
          const d = await ad.requestDevice();
          return { presence: "defined", adapter: (ad.info && (ad.info.vendor + "/" + ad.info.architecture)) || "unknown", deviceOk: !!d };
        });
        console.log(tag, "=>", JSON.stringify(r));
        await b.close();
        if (r.adapter) { console.log("WORKING COMBO FOUND:", tag); return; }
      } catch (e) { console.log(tag, "=> LAUNCH-ERR:", e.message.slice(0, 120)); }
    }
  }
  console.log("NO WORKING COMBO");
})();
