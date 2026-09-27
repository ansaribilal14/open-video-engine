# MEDIA ENGINE SPEC — vertical-slice engine contract (v1, 2026-09-26)

> The slice-scoped definition of the engine's public behaviour. Parent: ADR-002 (provisional C),
> FINAL_ARCHITECTURE_WHITEPAPER. This spec is the contract the integration vertical slice
> must satisfy; it deliberately excludes everything the anti-overbuild rules exclude.

## 1. Engine surface (v1)

```
ove-engine (façade, v1 = library + ove-cli binary)
 ├─ ProjectSession::open(path) / create(spec)
 ├─ execute(cmd: Command) -> Receipt            // the ONE API (ADR-010)
 ├─ undo() / redo()                             // inverse commands (ADR-009)
 ├─ snapshot() -> StateHash + version           // fold verification
 ├─ render(frame_range, target: FrameSink)      // via ove-render
 └─ export(profile: ExportProfile) -> Output    // via ove-encode (+copy route)
```

- **No UI types, no async runtime dependency in the façade signature surface v1** (sync
  calls, engine-internal threads; platform shells wrap).
- Every command: schema-versioned, explicit ids, exact rationals, no floats (E-003/E-009
  schema rules). Floats are rejected at the boundary — never rounded.

## 2. Session semantics

1. `open` = load manifest → rebuild state from snapshot + log suffix (OVE-PROJECT-SPEC §4).
2. `execute` = validate → apply → log append → receipt {state_hash, inverse_ref}. Receipt
   is the only feedback channel (no callbacks v1).
3. Crash anywhere = reopen + replay; acceptance is hash-equality (save/kill/reopen suite).
4. Single-writer (ADR-008): one session mutates a project; concurrent read-only render is
   allowed via state snapshot clone.

## 3. Media flow (v1, desktop/headless software path)

```
MEDIA FILE → probe (ove-media) → asset {content_hash, streams, keyframe index, color tags}
           → decoder session (ove-decode, FFmpeg-SW adapter)
           → FrameEnvelope (FRAME_CONTRACT) at timeline-remapped pts
           → render graph passes (ove-render, software raster)
           → encoder/muxer or stream-copy route (ove-encode) → MP4
```

- Timeline↔media time mapping: clip in/out are project-axis rationals; source pts are
  source-rate rationals; mapping is exact rational arithmetic (no fp), conversion at
  ingest is exact (ADR-007 refinement). Q-06 (map-in-format vs normalize) stays open —
  v1 stores source pts provenance in the asset record, mapping at query time.
- Keyframe index is mandatory for copy routes (E-007: mid-GOP copy cuts select wrong
  content; boundaries snap to keyframes unless re-encode is requested).

## 4. Behavioural contracts (testable)

| ID | Contract | Test |
|---|---|---|
| ME-1 | Every command applied via API == replayed from log (hash-equal) | property, per verb |
| ME-2 | undo/redo exact: apply→undo→apply == identity; undo after batch == pre-batch | property |
| ME-3 | Export duration exactness: project duration == ffprobe duration ± 0 frames | golden MP4 suite |
| ME-4 | Frame identity: decode(clip@t) == decode via seek+flush at t (conformance) | DECODER_SPEC suite |
| ME-5 | Render determinism: same project → same frame bytes (software path) | golden frames |
| ME-6 | Kill-safety: kill -9 during save/execute → reopen → hash-equal | crash drill |
| ME-7 | No floats in commands: fp payload → typed rejection at both API edges | E-009 pattern |
| ME-8 | Copy route correctness: keyframe-aligned cut == demux-remux of that GOP range (content hash) | E-007 72/72 pattern |

## 5. Error model

- Typed errors at every boundary (no panics across FFI/lib boundaries; ove-time panics
  are internal invariants, contained per E-004a pattern).
- Media errors classify: {Unreadable, Unsupported{feature}, Corrupt{where}, Cancelled} —
  enough for shells to surface; details in logs, never swallowed.
- Command validation errors are pre-execution and leave no log entry; execution errors
  leave the log untouched (command either applies or not — no partial states).

## 6. Threading model (v1)

- One command-executor thread (ordered, serial) — matches single-writer project rule.
- Decode/render/export worker pool behind the façade; cancellation is cooperative
  (checked between frames/passes; DECODER_SPEC cancel semantics).
- No shared mutable state across threads without ownership transfer; FrameEnvelope
  ownership rules (FRAME_CONTRACT §5) are the discipline.

## 7. Out of scope v1 (binding)

UI/preview windows · hardware decode/encode (adapters designed for, not implemented) ·
audio output devices (sample-exact file audio only in wave 7) · effects beyond opacity/
transform/geometry · transitions beyond none/cut · titles/text · plugins · scripting ·
MCP server binary · cloud · OTIO/MLT interchange · multi-editor concurrency.

## 8. Acceptance = ENGINE_BUILD_PLAN Wave-5 gate

The slice is done when ME-1..ME-8 are green in-repo (CI where software-runnable) and the
directive's quality-gate sentence is literally satisfied: MEDIA, TIMELINE, PROJECT,
RENDER, EXPORT cooperating end-to-end with an ffprobe-inspectable MP4.
