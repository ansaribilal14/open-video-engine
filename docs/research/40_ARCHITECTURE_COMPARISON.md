# 40 — ARCHITECTURE COMPARISON (v0.1)

> Status: PARTIAL (v0.1). Three candidate architectures are framed and compared against
> gathered evidence. **No winner is ACCEPTED** — directive requires architecture gates
> GATE-1..14 and experiments E-001..E-005 before decision. A provisional leader is
> identified with stated falsification conditions.

Evidence base: research docs 02–27 + claim ledger C-001..C-007 + SOURCE_LEDGER (77 sources).

---

## ARCHITECTURE A — Rust-first unified engine ("one core everywhere")

Rust owns everything: media I/O (via ffmpeg-next/GStreamer bindings or native reimpl),
timeline, project, compositor (wgpu), encode. Platform shells (Tauri / Kotlin via UniFFI /
WASM) are thin.

| Criterion | Assessment |
|---|---|
| Performance | Best on desktop (zero-copy across layers is intra-process); Android via Surface path possible |
| Portability | Browser contradicts it: no FFmpeg, no native FS; media I/O must be re-implemented with WebCodecs/Mediabunny regardless (docs 11, 18) |
| Complexity | Highest single-codebase complexity; FFmpeg FFI churn risk (doc 05: yearly majors, avcodec 62 in 9.0) |
| Correctness | One timeline/one command model — strongest consistency story |
| Hardware access | Full on desktop/Android; browser limited to WebCodecs/WebGPU capabilities |
| Evidence | OpenCut rewrite (GPUI + Rust crates), Clypra (Tauri+wgpu), Cutlass (20 Rust crates), OpenReelio — new-gen convergence (doc 02) |
| Key weakness | In-browser: Rust core can hold timeline/commands compiled to WASM, but decode/encode/gpu-import cannot be Rust-owned → the "unified" claim is ~60% true at best |

## ARCHITECTURE B — Native platform media engines + shared timeline spec

Each platform uses its best native media stack (Media3/MediaCodec on Android, AVFoundation
on Apple, FFmpeg on desktop, WebCodecs+Mediabunny on web); the timeline model, project
format, and command API are shared as a specification (+ optional model library), not a
binary core.

| Criterion | Assessment |
|---|---|
| Performance | Best per-platform media performance (native hw paths, zero syscalls wasted) |
| Portability | Excellent — each platform idiomatic |
| Complexity | Distributed: N implementations of decode orchestration; render parity across platforms is the classic NLE failure mode |
| Correctness | Timeline shared; but effect/color/compositor parity across 4 media stacks is historically unachievable (Kdenlive/Shotcut diverge even on one backend — doc 03) |
| Hardware access | Best |
| Evidence | Mature-NLE reality (docs 03, 07): MLT unified media but each NLE still owns model; browser evidence (doc 18) forces platform media anyway |
| Key weakness | Semantic consistency across platforms ("THE SAME PROJECT MUST REMAIN SEMANTICALLY CONSISTENT" — directive) becomes a testing nightmare; AI/agent API must be re-bound per platform |

## ARCHITECTURE C — Hybrid layered engine (shared core at the model/graph layer, adapters at the media/GPU-import layer)

One Rust core = **timeline model, project model, command system, undo, pass-graph
compiler, WGSL shader library, color math** — exposed as: native lib (desktop/Android via
UniFFI/JNI), WASM (web), and CLI (headless). Media I/O (demux/decode/encode/mux) and
video-frame GPU import are **platform adapters** behind engine traits. Compositor executes
the compiled pass graph via wgpu where available, WebGL2 fallback, platform GL on Android.

| Criterion | Assessment |
|---|---|
| Performance | Desktop/Android: full native paths retained; web: WebCodecs + importExternalTexture (doc 11) |
| Portability | Matches the actual capability map (doc 18: SHARED = model/graph/shaders/color; ADAPTER = media I/O + GPU exec; PLATFORM-SPECIFIC = probing/storage) |
| Complexity | Two hard problems (model core + adapter traits) instead of N hard problems |
| Correctness | Determinism concentrated in the shared core (claim C-001 HIGH: exact time; C-007: command log) — cross-platform parity becomes "same commands + same pass graph" not "same native calls" |
| Hardware access | Full, via adapters with SW fallback chains (doc 04: per-stream hw probing) |
| Evidence | Convergent independent evidence: new-gen editors' Rust-core consensus (doc 02), browser layered-sharing analysis (doc 18), EditDuet/Timeline-Assembler show AI outputs timeline diffs → command model is the integration point (docs 25, 38) |
| Key weakness | Adapter trait design is where editors die (decode state, seeking, hw memory import); risk of leaky abstraction → requires E-001/E-004 experiments before ADR acceptance |

---

## Comparison matrix (evidence-linked)

| Criterion (directive) | A | B | C |
|---|---|---|---|
| performance | ++ desktop, – web | ++ each | + everywhere |
| portability | – (browser contradiction) | ++ | ++ |
| complexity | – | – (×N stacks) | + |
| correctness/determinism | + | – parity risk | ++ |
| hardware access | + | ++ | + |
| browser support | – | + | ++ |
| Android support | + | ++ | ++ |
| AI/agent integration | + one API | – per-platform | ++ one command API (C-003 MEDIUM) |
| plugin system | + | – per-platform | ++ (Wasmtime/WASM at core boundary — doc 02 OpenReelio evidence) |
| licensing | – FFmpeg LGPL/GPL discipline required (doc 05) | mixed per stack | same as A for desktop, browser stack MPL/Apache (Mediabunny) |
| maintenance | + single repo | – 4 stacks | + |
| developer experience | + | – | ++ (owner's Kotlin strength = shell layer; doc S-001 team profile) |

## Provisional leader (NOT A DECISION)

**C**, on convergent evidence (C-001 HIGH, C-002/C-005/C-006 MEDIUM, C-003 MEDIUM,
C-007 MEDIUM). Confidence: MEDIUM.

**Falsification conditions** (what would flip the decision):
1. E-004 (Rust→Android bridge) shows UniFFI/JNI overhead or lifetimes break the
   decode-surface path → drop to B for Android.
2. E-003 (command-log replay determinism) fails → project format falls back to
   snapshot-only; C weakens.
3. E-001 (WebCodecs→WebGPU import) shows importExternalTexture insufficient for
   multi-layer compositing → browser adapter needs copy path; cost re-measured.
4. Adapter traits cannot express MediaCodec EOS/reuse semantics without leaking
   platform types → trait redesign or per-platform split (toward B).

## Gates still open (directive GATE list)

1–4 media/timeline/render/GPU: research v0.1 done, depth pass pending ·
5 Android: pending E-004 · 6 browser: pending E-001 · 7 desktop: pending Tauri IPC E-006 ·
8 project format: pending E-003 · 9 AI command architecture: shape proposed (doc 26/27) ·
10 plugin boundary: not started · 11–14 performance/security/licensing/testing: not started.
