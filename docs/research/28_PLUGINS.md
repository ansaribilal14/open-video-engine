# 28 — PLUGINS: PLUGIN BOUNDARY ARCHITECTURE (Track O)

> Status: PARTIAL (v0.1 — wave-3 depth pass 2026-09-24).

## 1. Scope & method

Survey of real plugin systems with verified licenses/APIs, failure-mode analysis, and a
three-tier proposal for the engine (feeds future ADR-025). Verified by fetch on
2026-09-24: OBS plugin/scripting docs (S-3c2), GStreamer PWG from gst-docs clone
(S-3c3), VST3/CLAP READMEs+LICENSE (S-3c4), WASM component model + WASI 0.3 (S-3c0),
crates.io for wasmtime/wasmer/extism (S-3c1), Adobe CEP/UXP/AE (S-3c6), Kdenlive effect
XML + Audacity (S-3c8), Figma/Deno sandbox docs (S-3c9). No engine code written.
Related: 26_AI_AGENTS (approval), 27_MCP (out-of-process transport), 06_GSTREAMER,
07_MLT, 08_GPU (render graph), 09_WGPU (wgpu-on-wasm), 12_WASM (perf).

## 2. Survey A — in-process native plugin systems

| System | Plugin unit | Language/ABI | Trust domain | License (verified) |
|---|---|---|---|---|
| GStreamer (S-3c3, S-2c3..c5) | element (GObject subclass) | C, dynamic .so | in-process, full host privileges | LGPL-2.1 core+plugins |
| MLT (S-2b2, S-2b3) | service: producer/filter/transition/consumer | C, properties bag | in-process | LGPL-2.1+ |
| OBS Studio (S-3c2) | module registering obs_source_info etc. | C/C++, dynamic lib | in-process | host GPL-2.0+ |
| VST3 / CLAP (S-3c4) | audio processor + editor | C++ / stable C ABI | in-process | VST3 MIT (3.8.x); CLAP MIT |

- **GStreamer**: PWG preface — "GStreamer adheres to the GObject programming model";
  elements registered via `GST_ELEMENT_REGISTER_DEFINE` + `plugin_init` returning
  `GST_ELEMENT_REGISTER` (S-3c3 boiler.md). Heavy per-element GObject boilerplate;
  caps negotiation is the contract (S-2c4). A crash in an element kills the host
  pipeline — the documented reason several editors avoid in-process GST for editing
  (06_GSTREAMER §B).
- **MLT**: extension shield is the properties bag — new services attach data without
  ABI break; but 8-bit YUV422 orientation + consumer-centric threading limit it
  (S-2b2/S-2b3, doc 07).
- **OBS**: modules loaded in-process via `obs_open_module`/`obs_load_all_modules`;
  typed registration structs (`obs_source_info` id/version/type); official CMake
  plugin template (S-3c2). Host is GPL-2.0+, so plugin copyleft is absorbed; an
  MIT/Apache host would NOT be — license contagion is a boundary decision.
- **VST3/CLAP — the audio lesson**: CLAP = stable ABI where "a plugin binary compiled
  with CLAP 1.x can be loaded by any other CLAP 1.y", with capability negotiation:
  host asks `host->extension(host, CLAP_EXT_LOG)`; plugin exposes
  `clap_plugin_params` etc. only if implemented (S-3c4). VST3 SDK 3.8.x is now MIT —
  "Licensing under GPLv3 and the Steinberg proprietary license is no longer
  available" (S-3c4 README §700; corrects the common GPLv3-dual belief).
  **Lesson**: versioned ABI + optional extension queries + host/plugin handshakes
  survive decades; our effect ABI should copy the CLAP extension-negotiation shape.

## 3. Survey B — declarative/scripted effect definitions (cheapest tier)

- **Kdenlive**: every effect is a small XML file in `data/effects/` (91 files listed
  this pass: audiobalance.xml, box_blur.xml, brightness.xml, charcoal.xml, …) declaring
  parameters/limits; UI is generated from the XML; the actual processing stays an MLT
  filter (S-3c8, S-2b4). Zero code plugin path for simple effects.
- **Audacity**: Macro = "a sequence of pre-configured commands (mainly effects) in a
  set order that can be applied automatically to projects or audio files" — can embed
  any menu-listed effect (LADSPA/LV2/Nyquist/VST/AU) plus export commands; "Macros
  follow a fixed sequence of instructions" (no branching). Nyquist = Lisp-based
  plugin scripting language (S-3c8).
- **Lesson**: a declarative param schema + a small pure-transform runtime covers the
  long tail of simple effects without native ABI surface.

## 4. Survey C — host-embedded scripting/panel systems (crash + security domain)

- **Adobe CEP→UXP migration** (S-3c6): CEP 12 HTML engine = Chromium Embedded
  Framework 3 **embedded in the host process**, bundling Node.js (table: Node 8.6.0 in
  CEP 9 → 17.7.1 in CEP 12). Panels are ZXP-packaged HTML/JS talking to the host via
  CSInterface.js. In-process JS = the host's crash and security domain: a panel can
  take down the app, and host automation is exposed to panel JS. Adobe's own UXP
  pivot ("the modern way to create plugins and scripts for Adobe Creative Cloud",
  Photoshop UXP 2022, S-3c6) is an admission that the CEP trust model needed a
  narrower runtime. **Lesson for us**: never expose the command bus raw to in-process
  JS from third parties.
- **DaVinci Resolve scripting** (S-3c5): a vendor RPC surface — community MCP server
  documents the chain "AI Assistant <-> MCP <-> Server <-> fusionscript <-> DaVinci
  Resolve" and splits the "complete scripting API" into tool groups (Project 25,
  Timeline 59, Media Pool 27, Gallery 14, Storage 7). Resolve returns live native
  objects that cannot be serialized, so wrappers keep an in-memory ID registry.
  **Lesson**: object-handle RPC is stateful and leaky; our boundary should carry
  explicit ids (E-003) and commands, not object references.

## 5. Survey D — sandboxed plugin models

- **Figma** (S-3c9): plugin code runs on the main thread "in a sandbox… a minimal
  JavaScript environment [that] does not expose browser APIs" (no fetch/DOM/setTimeout);
  UI lives in a separate `<iframe>` (`figma.showUI()`); the two communicate only via
  message passing; network access is manifest-declared per-domain and enforced with
  CSP errors. Smallest credible sandbox: language-level, no OS processes.
- **Deno** (S-3c9): "secure by default… no access to sensitive APIs, such as file
  system access, network connectivity, or environment access" unless explicitly
  granted by flags or a runtime prompt; permission broker; "executing untrusted code"
  is a documented first-class scenario.
- **WASM component-model plugin systems** (S-3c0, S-3c1):
  - WASI has shipped three milestones: 0.1/0.2/0.3; "WASI 0.3 adds native async
    support to the Component Model and refactors WASI interfaces to take advantage of
    async primitives like stream<T> and future<T>"; open standard under the W3C WASM
    CG; wasi.dev explicitly pitches WASI as what "every project with a plugin model"
    should use.
  - Runtimes (crates.io, 2026-09-24): **wasmtime 49.0.0** (Apache-2.0 WITH
    LLVM-exception — NOT the 33.x the task brief guessed), **wasmer 7.4.2** (MIT),
    **extism 1.30.0** (BSD-3-Clause) whose README targets exactly our use case:
    "execute arbitrary, untrusted code from your users", with runtime limiters &
    timers and secure host-controlled HTTP without WASI.
  - Precedent in an editor: OpenReelio runs Wasmtime plugins (S-106).
- **wgpu-on-wasm**: wgpu supports WebGL2/WebGPU backends on wasm (S-2d1), so a
  browser-hosted engine can run GPU effects in wasm via WebGPU — but in NATIVE
  wasmtime hosts there is no WASI GPU/zero-copy-frame interface in the 0.3 docs
  reviewed (UNVERIFIED absence, §11).

## 6. Failure-mode analysis

| Model | Crash blast radius | Latency | Serialization cost | Determinism | GPU/hw access |
|---|---|---|---|---|---|
| Native in-process (GStreamer/OBS/VST3) | host dies | lowest | none | no (fp, UB, threads) | full |
| Declarative XML + pure transforms | engine catches | low | none | yes (if pure) | via engine shaders only |
| In-process JS/Lua embed | exception; interpreter UB risk; CPU loops can starve render thread | low | none | partial (no fp-int misuse; gc noise) | via host only |
| Out-of-process helper | contained; restartable | +IPC per call | frame copies | no (arbitrary code) | own context |
| WASM (wasmtime) | contained by sandbox | 1.45–1.55x native avg, ~4x SIMD-heavy (S-2d9) | linear-memory copies | yes (wasm semantics; no ambient clock/random) | none natively; WebGPU only in browser hosts |

Key trade: **crash containment vs frame throughput**. Frame data crossing a process
boundary costs copies; crossing a WASM boundary costs a linear-memory write; crossing
no boundary risks the host. OBS/GStreamer chose throughput-and-trust; Figma chose
isolation-with-message-passing; Adobe was forced to re-platform (CEP→UXP).

## 7. Proposal — three tiers for the Open Video Engine (ADR-025 seed)

- **Tier A — native Rust trait effects (compile-time, in-core)**: `trait VideoEffect`
  operating on planar frame views + effect state; GPU effects ship WGSL registered
  into the pass graph (doc 08 pass-graph + hooks pattern of libplacebo, S-2d5).
  Params are schema'd and keyframeable at exact rational ticks (ADR-007 — no float
  seconds in the param track). `catch_unwind` at every plugin call; panics degrade
  the effect to a bypass, never the host. Not a stable ABI — ships with the engine
  version (like MLT's in-tree services, S-2b2).
- **Tier B — WASM sandboxed effects/analysis (third-party)**: wasmtime +
  component-model (WIT) ABI, versioned; extism-style manifest with permissions and
  runtime limits (fuel/epoch). Inputs: frame bytes into linear memory (or reduced
  scanline tiles), params snapshot at time t; outputs: modified frame bytes OR
  analysis records (JSON). No ambient I/O, no clock, no threads → deterministic
  replay (E-003-compatible). Suited to color/transform/analysis, NOT heavy per-pixel
  DSP (perf tax, S-2d9).
- **Tier C — out-of-process helper services (heavy AI models)**: stdio/HTTP JSON +
  frame-hash receipts, MCP-wrappable (doc 27; kinocut receipts precedent S-4f8).
  Own GPU context; restartable; results are records that become commands only after
  user approval (26_AI_AGENTS §5). OpenReelio's FFmpeg sidecar is the same shape
  (S-106).

Declarative tier (Kdenlive-XML-like, §3) is Tier B0: metadata + pure transform
reference, no sandbox needed beyond determinism of the built-in ops.

## 8. What crosses the boundary — frames or commands?

- **Frames cross** into Tier A/B effect functions (pixels in, pixels out) — effects
  are stateless-per-frame plus explicit param evaluation at time t. Effects NEVER
  emit timeline commands: only UI, scripts, agents, and MCP clients produce commands
  (ADR-010 one command bus). An analysis plugin's findings enter the project as
  proposed commands through the normal validate→approve→apply ladder (doc 26).
- **Commands never cross** into effect code. This keeps render replay (E-003) and
  plugin code orthogonal: a project replay re-derives the same frames; plugin bugs
  cannot corrupt history.
- **ABI shapes**: Tier A = Rust trait (semver-coupled to engine releases, no dylib
  stability promise); Tier B = WIT component interface `ove:effect@X.Y` (params,
  frame buffers as `(ptr,len)` region handles, error codes); Tier C = JSON schema
  per tool family + content-addressed frame hashes (kinocut "typed tools, not
  invented FFmpeg flags", S-4f8).

## 9. Security & permissions

- Manifest-declared capabilities per plugin (Tier B/C): file paths, network
  domains, model downloads — **default DENY**, explicit grants, user-visible consent
  at install and at first use (Figma manifest+CSP precedent; Deno grant flow; extism
  host-controlled HTTP without WASI — S-3c9, S-3c1).
- Permission broker in host mediates every privileged call (Deno pattern, S-3c9);
  grants are per-plugin-version — update = re-consent (26_AI_AGENTS rug-pull rule).
- Signing/notarization: adopt macOS-style signed packages (UNVERIFIED this pass —
  no primary doc fetched; see §11). Distribution rule until then: signed engine
  releases bundle Tier A; Tier B/C load only from user-approved manifests.
- License firewall: Tier A/Tier B catalogs must record each plugin's license
  (doc 35 owner); OBS shows the GPL-host case, VST3's MIT shift shows vendor
  incentives change (S-3c2, S-3c4).

## 10. Versioning & stable-contract rules

- Tier B interface versioned per component-model package version; host keeps
  N-1 compatibility like CLAP 1.x/1.y (S-3c4).
- Param schemas carry per-field versions; effect XML migration = per-generation
  serializers (Olive pattern, S-2b6) — avoid Kdenlive's multi-generation drift
  disasters (S-2b4).
- Every plugin invocation recorded in the command/audit log with plugin id+version
  (E-003 explicit ids; kinocut receipts S-4f8) so replays can prove which binary
  produced which frame hash.

## 11. UNVERIFIED (honest)

- macOS notarization/signing requirements for plugin packages (not fetched).
- Absence of a native WASI GPU interface: reviewed wasi.dev 0.3 summary + component
  model AST explainer only; a full WASI proposals scan may find a GPU/graphics
  proposal.
- OBS actual crash reports from in-process plugins (community knowledge, no primary
  bug URL fetched).
- Resolve scripting transport details (local TCP port etc.) — community repo only.
- Adobe UXP permission/manifest specifics; AE engine-switch release history.
- Excalidraw plugin isolation model (not attempted this pass).
- wasmtime epoch-interruption/fuel exact API names (README silent; crate docs unread).

## 12. Depth remaining (v0.2)

- Prototype Tier B: compile one grayscale effect to wasm32-wasip2 component, drive
  from wasmtime 49, measure per-frame overhead vs Tier A (E-00x candidate).
- Read component-model high-level docs + WIT tooling (cargo-component) end-to-end.
- Design `ove:effect` WIT package draft incl. param-keyframe snapshot struct
  (rational ticks, ADR-007).
- Survey host crash-containment practice: OBS out-of-process proposals, GStreamer
  out-of-process elements (gst-subsystems), DaVinci OFX component isolation.
- Permissions UX pass with doc 34 owner.
