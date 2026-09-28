# ADR-017: Engine integration layer v1 (ove-engine + ove-cli)

- **Status**: ACCEPTED (2026-09-29) — acceptance condition met: ENGINE_
  BUILD_PLAN wave-5 gate green (engine integration suite 6 tests + CLI
  smoke; workspace 131/131): MEDIA+TIMELINE+PROJECT+RENDER+EXPORT
  cooperating on real committed media, with save/reopen hash-equality and
  an ffprobe-verified duration-exact export.
- **Date**: 2026-09-29 · **Confidence**: 0.87 (layer placement and the
  boundary-conversion contract are the durable decisions; the v1
  single-source render note is a named, short-lived limitation)

## CONTEXT

WAVE 5 = ove-engine + ove-cli: the session layer that makes the five legs
cooperate — probe → asset import → timeline commands → project persistence
→ decode → render → export. Constraints: core crates stay libav-free;
explicit ids everywhere (E-012); exact rationals end-to-end; the software
renderer is the correctness reference; single conversion per stage (RG-7 /
FRAME_CONTRACT §4.2).

## DECISION

1. **Layer placement**: ove-engine is the INTEGRATION layer — the one
   non-adapter crate allowed to depend on the adapter crates (ove-decode,
   ove-encode). The libav confinement invariant is restated: pure core
   (time/timeline/media/render/project) never links libav; ove-engine and
   the future platform shells (desktop/android/browser) sit at this same
   layer. `check_libav_confinement.sh` (direct-dependency + source scan)
   continues to gate this.
2. **Boundary conversion contract (the single-conversion rule made
   concrete)**: the decode boundary converts source YUV420P → RGBA8 ONCE
   (integer fixed-point, matrix from the frame's declared tags, Unknown →
   BT.601; chroma upsample = nearest — both declared); the encode boundary
   converts RGBA8 → YUV420P ONCE (the encoder adapter's declared swscale
   path). Between the two, compositing runs in the renderer's RGBA working
   space. The converted frame is tagged in the working space so the plan's
   tag consistency check sees no hidden second conversion.
3. **Allocation state rides the log (E-012 completion)**: the W4 format
   carried `next_id`/`used_ids` in snapshots, but replay-from-log could not
   reproduce the cursor (apply() never advances it — only alloc_id does),
   so a live session that allocated ids diverged from its own replay on
   the state hash. Fix: every log entry now carries `nid` (the session
   cursor after execution); replay restores it (`Timeline::set_next_id`,
   monotonicity-guarded — a smaller value = lost-alloc divergence = typed
   error). Found by the engine test suite being the first caller that both
   allocates and replays; the project-layer tests had used literal ids and
   passed vacuously — the test is now mutation-strong.
4. **ContentHash::from_hex added** (round-trip constructor): the probe
   sidecar hydration re-parsed the digest with `from_bytes` — which HASHES
   its input — producing a different identity on reopen and a missing-asset
   failure. `from_hex` reinterprets the 64-char digest as the digest;
   naming now makes the two operations impossible to confuse.
5. **Export routes via the engine**: re-encode = render every output frame
   (ADR-013 seam mapping → exact decode → boundary conversion → compile →
   execute) → encoder (RGBA input declared) → MP4 (bitexact+faststart);
   copy = pure planner (keyframe index from the probe sidecar) → packet
   passthrough → MP4, snaps reported. Both ffprobe- and libav-verified in
   the integration suite.
6. **v1 session honesty (named limitations)**: per-clip asset binding is a
   session note (multi-source timelines render the first imported source);
   the binding rides the log at W6 together with the vertical-slice trace.
   Audio in exports: video-only until the W7 audio leg.

## REJECTED ALTERNATIVES

- swscale for the decode boundary (would drag a direct libav dependency
  into ove-engine or a new adapter; the integer conversion is 40 lines,
  deterministic, and test-owned).
- Persisting probe records inside commands.jsonl (probes are cacheable
  derived data — PROJECT_FORMAT_SPEC disposable rule; sidecars keep the
  log purely editorial).
- A clippy-style arg parser crate for the CLI (hand-rolled subcommands
  keep the dependency surface zero; the CLI is the engine's harness, not a
  product surface).

## CONFIDENCE & RISK

0.87. Risks: the boundary conversion's color fidelity (integer fixed-point)
is uncalibrated against reference converters — golden-frame color tests
land with W6's vertical slice; multi-source render scheduling (decode
buffering across N sources) is untested at this layer (single-source v1);
the `nid` log field is a v1 format addition (serde default keeps old logs
loadable — none exist in the wild).
