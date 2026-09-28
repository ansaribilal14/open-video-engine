# VERTICAL_SLICE_TRACE — the three chains at source level (W6, 2026-09-29)

> Directive deliverable: the vertical slice is proven by tracing three chains
> through REAL code paths (file → crate → function), not by narrative. Every
> hop names the fn; every claim is pinned by a named test. The chains were
> executed on the committed corpus (copy24.mp4 / copyntsc.mp4) in
> `engine/ove-engine/tests/integration.rs::w6_vertical_slice_milestone`.

Legend: `crate::path::fn` — calls flow downward. Exactness notes (E-002)
mark where floats could leak and why they cannot.

---

## Chain 1 — IMPORT (media in → registry + probe + content address)

```
ove_engine::Engine::import_media(path)                       [ove-engine/src/lib.rs]
├─ ove_media::AssetRef::from_path                            [ove-media/src/asset.rs]
│  └─ ContentHash::from_file — BLAKE3-256 streaming over the bytes
│     (identity = the hash; R-13: relink-by-path forbidden)
├─ ove_decode::ffmpeg::FfmpegProbe::probe_full               [ove-decode/src/ffmpeg/mod.rs]
│  ├─ avformat_open_input + avformat_find_stream_info        (libav, adapter-confined)
│  ├─ per-stream facts → ProbeInfo {codec, time_base (exact AVRational→Rational),
│  │                                dims/color tags, durations}
│  └─ one packet scan → KeyframeIndex (packet K flags + byte pos) + VfrReport
├─ serde_json::to_value(&probe) → project sidecar payload
└─ ove_project::Project::import_asset                        [ove-project/src/lib.rs]
   ├─ ContentHash::from_file (again — independent verification)
   ├─ copy → assets/<content-hash>/src.<ext>   (dedupe: identical bytes → same dir)
   ├─ write assets/<content-hash>/probe.json  (sidecar; disposable-regenerable)
   └─ manifest.assets += {id, content_hash, path, probe}   (manifest.store: tmp+rename)
```

Gate: `w5_*` tests open a session that imported through this chain; the
sidecar round-trip is what `Engine::open` re-hydrates
(`ContentHash::from_hex` — the digest is a round-trip, never re-hashed;
see ADR-017 §4 for the bug that naming prevents).

## Chain 2 — COMMAND + REPLAY (edit → log → kill → reopen → identical state)

```
ove_engine::Engine::add_clip(track, asset, duration, src_in) [ove-engine/src/lib.rs]
├─ Timeline::alloc_id — cursor advanced BEFORE command construction (E-012)
├─ ove_project::Project::execute_insert_asset                [ove-project/src/lib.rs]
│  ├─ Timeline::apply(&Command::Insert) → returns the EXACT inverse
│  │  (pre-state derived; validated before mutation)
│  ├─ LogEntry { seq, owner, payload: Insert{.., asset: Some(hash)},
│  │             undo: Some(inverse), nid: cursor-after }     [ove-project/src/log.rs]
│  ├─ record_entry_bindings — clip→asset enters the document state
│  ├─ LogWriter::append — one JSON line + flush to OS
│  │  (kill -9 afterwards = the on-disk state is exactly this)
│  └─ touch_manifest — hint updated (state_hash = BLAKE3 of the mirror)
│
│  ... process killed (acceptance: real subprocess abort, P-2a) ...
│
ove_engine::Engine::open(dir)
├─ ove_project::Project::open
│  ├─ Manifest::load (hint; schema_version gates the reader)
│  ├─ SnapshotMeta::load — the snapshot authority; state-<seq>.json verified
│  │  against meta.state_hash → StateMirror::to_timeline
│  │  (next_id + used_ids restored — allocation state is document state)
│  ├─ log::load(commands.jsonl, expected_first = snapshot_seq+1)
│  │  — anchored seq contiguity; ANY gap = typed LogCorruption (hard stop)
│  └─ replay_entry × suffix                                  [ove-project/src/lib.rs]
│     ├─ exec:  Timeline::apply(cmd) → undo_stack.push (rebuilt, ADR-008)
│     ├─ undo:  Timeline::apply(embedded inverse) — self-contained suffix
│     ├─ redo:  Timeline::apply(embedded forward)
│     └─ nid:   Timeline::set_next_id (monotonic; regression = typed error)
│  → reconciliation: stale manifest rewritten from the replayed truth
└─ Engine.state_hash() == pre-kill hash
   (BLAKE3-256 over the canonical StateMirror — PROJECT_FORMAT_SPEC §2)
```

Gate: `w5_save_reopen_hash_equal`, `w5_kill_reopen_continue`,
`w6_vertical_slice_milestone` (reopen mid-milestone), plus the project-layer
P-1..P-4/P-7 (P-2a does the real subprocess kill).

## Chain 3 — AI/CLIENT → COMMAND → RENDER → EXPORT (owner-agnostic session)

```
any client (human UI, agent, script — Owner::Human|Agent|Script)
│  The command surface is IDENTICAL for every owner (E-009):
│  explicit ids in, exact rationals in, one Command per intent.
│  Agents construct the SAME ove_timeline::Command values — there is no
│  second API (that is the AI-is-just-a-client rule made structural).
│
ove_engine::Engine::add_clip(.., Owner::Human)               [ove-engine/src/lib.rs]
├─ (see chain 2 through Project::execute → commands.jsonl, owner recorded)
│
ove_engine::Engine::render_frame(&OutputSpec, t)             [ove-engine/src/lib.rs]
├─ Engine::build_render_input — timeline walk (TrackOps::walk, derived starts)
│  ├─ ClipWindow::from_clip — ADR-013 seam (exact timeline↔source mapping)
│  ├─ Placement { clip_id, window, source: hash-derived id, alpha, offset,
│  │             src_color } — probe-record tags, never per-frame
│  └─ ove_render::compile_frame (PURE; layer order = track order)
│     → RenderPlan (canonical hash; culling only when provably identical)
├─ DecodeSource::fetch (ove_render::FrameSource impl)         [ove-engine/src/lib.rs]
│  ├─ FfmpegSwDecoder::seek(target, Exact) — keyframe-floor + forward drop (D-4/D-5)
│  ├─ Decoder::next → FrameEnvelope (YUV420P, exact pts/duration)
│  └─ yuv420p_to_rgba — integer BT.601/709, once (the engine's declared
│     source→working-space conversion; ADR-017 §2)
└─ SoftwareRenderer::execute_frame(plan)
   → output FrameEnvelope (Cpu RGBA8, opaque, working-space tags)
│
ove_engine::Engine::export_reencode(out, &output, n)         [ove-engine/src/lib.rs]
├─ per frame k: render_frame(output.frame_pts(k)) → Encoder::feed (RGBA input;
│  encoder-adapter swscale = the ONE working-space→codec conversion)
├─ Encoder::drain → EncodedPacket stream (checkpoint fields present)
├─ FfmpegMuxer::open/write (exact ticks on the EFFECTIVE post-header axis)
└─ FfmpegMuxer::finalize → OutputInfo {nb_frames, duration, file_sha256}
```

Gate: `w6_vertical_slice_milestone` — two multi-rate sources, renders from
both, 120-frame export, `nb_frames == 120` and `duration == 5/1` exact
(ffprobe + libav verified), then save → reopen → SAME state hash →
re-export byte-identical (`file_sha256` equality — the deterministic
software pipeline).

---

## What this slice deliberately does NOT claim (named, not hidden)

* **Audio** — video-only until W7 (the AAC seam + sample-exact mux path).
* **Per-clip asset binding edits** — bound at insert, permanent for the
  log's lifetime (re-bind verb lands with W8 keyframes' value-tracking).
* **GPU** — the software renderer is the reference; GPU legs must match
  these bytes (wave 9).
* **Multi-source decode scheduling** — one decoder per source per render
  pass (correct, unoptimized; perf wave).
* **Agent transport** — the AI→command chain is the COMMAND surface itself;
  MCP/WS transports (wave 15) sit ABOVE it and change nothing here.
