# 29 — SCRIPTING: USER SCRIPTING & AUTOMATION (Track O)

> Status: PARTIAL (v0.1 — wave-3 depth pass 2026-09-24).

## 1. Scope & method

Survey of verified scripting/automation surfaces in real editors, the
scripting-surface question for THIS engine (one command bus, ADR-010/E-003), runtime
sandbox options, and headless/batch story. Verified by fetch 2026-09-24: Blender bpy
docs + PyPI wheel (S-3c7), Audacity macros/Nyquist (S-3c8), OBS scripting docs
(S-3c2), Resolve evidence (S-3c5), AE expressions (S-3c6), melt CLI (S-2b2+docs),
FFmpeg `-/filter` file syntax (S-2c0-era docs, re-fetched), mlua/LuaJIT/QuickJS-ng/
rquickjs/Deno/Figma (S-3c9, S-3c6). No engine code written.
Related: 26_AI_AGENTS (same loop, agent client), 27_MCP (transport), 28_PLUGINS
(effect tier vs script tier), 30_HEADLESS (CLI adjacency), 19/20 (project format).

## 2. Survey — verified scripting surfaces

| System | Script surface | Language | What it can do (verified) |
|---|---|---|---|
| Blender (S-3c7) | bpy module, embedded interpreter | Python | full editor automation; PyPI wheel 5.2.2 "Blender as a Python module" (GPL-3.0, CPython ==3.13.*); docs flag "Python Threads are Not Supported" |
| DaVinci Resolve (S-3c5) | scripting API via fusionscript bridge | Python/Lua | project/timeline/media-pool/grade automation; community MCP wrapper maps "complete" API: Project 25, Timeline 59, Media Pool 27 tools |
| OBS (S-3c2) | scripting API | Python/Lua | script_* exports (description/load/save/defaults/update/properties/tick), timer_add; "Other Differences From the C API" — deliberately restricted vs C plugins |
| Audacity (S-3c8) | Macros + Nyquist | command list + Lisp | "sequence of pre-configured commands (mainly effects) in a set order" incl. export; Nyquist plugins for synthesis/analysis |
| Kdenlive (S-2b4) | MLT XML document + render profiles | XML | the project file IS the render input; render-profile file location NOT verified this pass |
| Shotcut (S-2b5) | MLT XML + melt CLI | XML/CLI | project = MLT XML; headless render via melt (below) |
| MLT (S-2b2) | melt CLI | CLI | "melt [options] [producer name=value]*" with -attach filters → free headless consumer |
| FFmpeg (S-2c0..c2) | filtergraph as file | CLI | "-/filter:v filter.script … will load a filtergraph description from the file" (option-value-from-file syntax) |
| After Effects (S-3c6) | expressions per property | JavaScript | see §3 |

- **Blender is the gold standard** (S-3c7): scripting is not a side API — every panel
  operation is exposed, the UI is largely expressible through operators, and the
  interpreter is the same runtime add-ons target. Cost: embedding Python couples the
  host to GPL (wheel license GPL-3.0) and to exact CPython versions.
- **OBS is the two-tier precedent** (S-3c2): C plugins get the full API; Lua/Python
  scripts get a documented RESTRICTED subset ("Other Differences From the C API").
- **Resolve is the object-RPC anti-pattern to avoid** (S-3c5): live object handles
  need ID registries in wrappers (verified from the MCP server's design notes).

## 3. After Effects expressions — the keyframe-relevant model (S-3c6)

Verified wording: "An expression is a small piece of JavaScript code that you can
plug into animated properties… that evaluate to a single value for a single layer
property at a specific point in time. Unlike a script, which tells the application to
do something, an expression tells a property to do something." The docs carry a
dedicated page "Syntax differences between expression engines" (modern JS vs legacy
ExtendScript).

Engine mapping: AE expressions = **per-property, per-time, pure functions** evaluated
by the renderer — exactly the shape our param system needs if we allow scripted
param sources: `value(t)` at exact rational tick t (ADR-007), no state, no IO,
evaluated deterministically during render. Script vs expression distinction maps to
our distinction: scripts emit COMMANDS; expressions COMPUTE VALUES.

## 4. The engine-relevant question: one command bus (no side doors)

- Scripts are just another client of the SAME command API as UI and AI agents
  (ADR-010; doc 26 canonical loop; doc 27 MCP shell; C-003). A script that mutated
  model state directly would fork the truth: undo (inverse commands, E-003),
  autosave, diff, and agent replay all break.
- Contract: `script -> [command batch] -> validate -> preview -> apply -> receipt`
  — identical to the agent ladder in 26_AI_AGENTS §2; kinocut's "typed tools, not
  invented FFmpeg flags" applies verbatim to user scripts (S-4f8): a script may only
  call declared, schema'd commands with engine-issued ids.
- Read surface: scripts need queries (timeline structure, metadata, analysis
  results) — provide read-only snapshots, never writable handles (26_AI_AGENTS §2
  step 1; Resolve's object-registry mess shows why, S-3c5).
- Recording: script-emitted batches land in the command log with provenance =
  script id + hash (E-003 explicit ids), so a replay reproduces exactly what the
  script did.
- Example batch (shape follows E-003 schema lessons: explicit ids, (num,den) time,
  no floats):

```json
{ "batch_id": "b_01J9…", "schema": "ove.cmd@1",
  "provenance": {"kind": "script", "id": "scripts/intro-shuffle.lua",
                 "content_hash": "sha256-…"},
  "commands": [
    {"op": "timeline.trim",     "target": {"clip_id": "c_42"},
     "args": {"new_in": {"num": 4800, "den": 1001}, "edge": "in"}},
    {"op": "timeline.insert_clip", "target": {"track_id": "t_main", "at":
     {"num": 9600, "den": 1001}}, "args": {"asset_id": "a_7"}}
  ] }
```

Validation happens engine-side (schema -> refs -> time -> policy); the script
never constructs ids or time values outside engine vocabulary.

## 5. Deterministic vs non-deterministic script parts

| Part | Example | Determinism rule |
|---|---|---|
| Command emission | trim, insert, tag | must be pure function of (snapshot, script input); no wall-clock/random ids — ids come from engine allocation (E-003) |
| Expression/param functions | `value(t)` easing | pure, exact-time input (rational tick), no IO — render-replay safe |
| Transform functions | per-frame pixel math | pure (Tier B plugin rules, 28_PLUGINS §7-8) |
| IO parts | fetch transcript, read a file, call a model | quarantined: explicit capability grants; results enter as commands/records, never silent state (26_AI_AGENTS §7 media-data-is-data rule) |

Design rule: a script FILE has two declared sections — `plan()` (pure, runs on
snapshot, emits commands) and `effects/analysis hooks` (pure per-frame/per-time
functions). Anything else (network, filesystem, clock) requires permissions and is
excluded from deterministic replay; its outputs must be materialized as logged
commands or content-hashed records.

## 6. Sandboxing options for user scripts (verified state, 2026-09-24)

- **Lua via mlua 0.12.1** (MIT; crates.io + README, S-3c9): features lua54, luajit,
  luau (+luau-jit), vendored builds, async/await, `send` — the most complete Rust
  Lua binding set. Lua sandboxing is a known pattern but interpreter bugs = host
  memory risk (in-process).
- **LuaJIT status — CORRECTION**: the working assumption "repo dormant" is stale.
  luajit.org/status.html states "LuaJIT is actively developed and maintained", uses
  rolling releases from versioned branches (v2.1 maintained; old master pinned to
  v2.0); README copyright "2005-2026 Mike Pall", MIT (S-3c9). Still: one-maintainer
  bus factor; LuaJIT disables some hardening (UNVERIFIED detail).
- **QuickJS → QuickJS-ng**: Bellard's QuickJS "went dormant" (qjs-ng README's own
  words, S-3c9); quickjs-ng is active (v0.17.0 releases feed), MIT (retains Bellard
  copyright line). Rust bindings: rquickjs 0.14.0 (MIT). Embedding JS matches the
  AE/UXP skill ecosystem but brings ECMAScript feature pressure.
- **QuickJS/Lua compiled to WASM inside wasmtime**: maximum isolation + determinism
  at the cost of an interpreter-inside-interpreter (bench required; UNVERIFIED perf).
- **Deno-style capability permissions** (S-3c6): "secure by default… no access to
  file system, network, environment" without explicit grants; runtime prompt;
  permission broker — the model for our script permission manifest (28_PLUGINS §9).
- **Figma lesson** (S-3c6): even "sandboxed" main-thread JS needs the UI in a
  separate frame with message-passing only — dual-world design; our analog: script
  world vs engine world communicate only via command batches and value snapshots.

## 7. Headless/batch — artifacts, not APIs

- **Precedents**: melt renders MLT XML headless (verified usage line, S-2b2 + melt
  doc); FFmpeg loads a filtergraph from a file (`-/filter:v filter.script`, S-2c2
  docs re-fetched); Audacity macros batch-apply command lists; Shotcut's project IS
  the render artifact (S-2b5). Lesson: the durable automation artifact is the
  DOCUMENT + a command/operation list, not a hosted IDE.
- **ove-cli (v1 proposal)**: `ove-cli render project.ove --out out.mp4`,
  `ove-cli apply project.ove batch.json`, `ove-cli validate batch.json`.
  - JSON command batches = v1 scripting surface (no embedded interpreter): schema-
    versioned commands, explicit ids, batch = one transaction (E-003, ADR-010).
    Generators: UI export, agent plans (doc 26), Python/JS emit-batches from any
    external toolchain.
  - Project file (.ove) + command log replay = free headless render (doc 30
    adjacency), exactly MLT-XML/melt's trick.
- Concrete precedents of "document file = automation artifact":

```console
# MLT: render the project document headless (no editor process)
melt project.mlt -consumer avformat:out.mp4 vcodec=libx264 crf=18
# FFmpeg: filtergraph loaded from a script file (option-value-from-file syntax)
ffmpeg -i INPUT -/filter:v filter.script OUTPUT
```

  (melt usage verified from mltframework.org/docs/melt; ffmpeg `-/opt` syntax from
  ffmpeg(1) docs, fetched 2026-09-24.) Our equivalent one-liner must render the
  project file identically to the UI path — hash-determinism is the acceptance bar
  (E-003).
- **v2 proposal**: embedded scripting runtime for in-app user scripts, gated on
  benchmarks + sandbox audit (§9).

## 8. Proposal summary & decision gates

1. v1 ships: JSON command batches + ove-cli + read-only snapshot query API +
   expression-shaped param functions (pure, exact-time) in the param system.
2. v2 candidates ranked: (a) Lua 5.4 via mlua (small, MIT, sandbox-friendly, send/
   async); (b) QuickJS-ng via rquickjs (JS ecosystem, active fork); (c) both
   compiled to WASM in wasmtime if audit demands memory isolation.
   Luau (via mlua, sandbox-minded Roblox dialect) is the dark-horse option (S-3c9).
3. Decision gates: G1 bench — 100k-command batch parse+validate+apply latency;
   G2 sandbox — fuzz interpreter boundary (no host-memory escapes); G3 replay —
   hash-identical render with scripts in the loop (E-003 harness reuse);
   G4 license — no GPL interpreter linked into MIT/Apache core (bpy lesson, S-3c7).

## 9. UNVERIFIED (honest)

- Kdenlive render-profile storage location and any official Kdenlive Python API
  (D-Ogi/kdenlive-api exists but depth not verified; docs.kdenlive.org fetch failed).
- Shotcut's own CLI job-control flags (FAQ only documents QT_SCALE_FACTOR env var);
  melt CLI verified instead.
- Resolve scripting transport details (port/handshake) — community repo only;
  official API text lives in a 209MB manual PDF (reachable, not parsed).
- QuickJS/Lua-in-WASM interpreter nesting perf; LuaJIT hardening gaps.
- wasmtime fuel/epoch API names (docs unread); mlua sandbox API specifics
  (`set_memory_limit` etc. unverified).
- Excalidraw plugin model; Audacity macro branching limits beyond "fixed sequence".
- Blender add-on distribution policy details (extensions.blender.org) this pass.

## 10. Depth remaining (v0.2)

- Prototype E-00x: 10-command JSON batch -> ove-cli -> hash-equal state vs UI path
  (E-003 harness reuse); measure batch overhead.
- mlua sandbox audit: memory limits, instruction budget, removed stdlib surface —
  write the actual sandbox checklist.
- bpy study v0.2: operator/undo integration (how Blender maps scripts to undo steps)
  as the reference for script-in-undo semantics.
- Expression evaluator spec: grammar subset, rational tick input, pure-function
  guarantees, WGSL parity for GPU-side param animation.
- Survey Nyquist/Lisp sandboxing history (audacity security page) for IO-quarantine
  patterns.
