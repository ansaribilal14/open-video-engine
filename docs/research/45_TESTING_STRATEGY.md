# 45 · TESTING STRATEGY — the engine's testing constitution

> Status: **PARTIAL → first full draft (2026-09-24)**. Everything here is grounded in
> committed evidence: E-002, E-002c, E-003, E-009, E-005, E-007/E-007b, the ove-time
> property suite, and the disaster histories in doc 19. This doc resolves the GATE-14
> residual ("dedicated testing-strategy doc not yet written") and encodes ADR-009's
> per-verb property-test rule as standing engineering law.

## 1. Why a constitution, not a test plan

The mission directive's core rule — no fake completion — is only enforceable if every
"TESTED/BENCHMARKED" claim in STATUS.md is reproducible by a command anyone can run.
That property is what turns the status ledger from prose into an audit system. Three
committed results demonstrate the pattern this doc generalizes:

- **E-002 (13/13)** falsified fp time by *construction of the right property*, not by
  debugging: 3×(1/3) hit the round-to-even trap, and an empirical 48 kHz/29.97 sample
  census found 178 boundary mismatches. The lesson: the property suite is the product,
  the test code is its expression.
- **E-002c** proved that a benchmark without **cross-structure correctness gates** is
  noise: hash equality across all candidate structures (all workloads × reps) is what
  made the timing numbers meaningful. Two of our own bad test constructions were caught
  by this discipline (round-to-even trap; addition-monotonicity for mixed signs).
- **E-003 (9/9)** proved replay-determinism is testable at the schema level before the
  engine exists: log round-trip, replay == apply, snapshot + suffix equivalence, undo
  hash-equality — all on plain JSON fixtures.

A fourth lesson is negative and equally important: E-001's GPU import legs are blocked
by SwiftShader, and E-007's first timing leg was honestly *invalidated* (micro-scale).
Testing strategy must classify what a given environment can and cannot prove.

## 2. Test pyramid for the engine

| Layer | What it proves | Mechanism | Existing exemplar |
|---|---|---|---|
| L1 unit | primitive semantics | `cargo test` per crate | ove-time 4 unit tests |
| L2 property | invariants over generated space (seeded, deterministic) | seeded pseudo-random loops, fixed seeds committed | ove-time 11 property tests; E-003 schema properties |
| L3 cross-implementation | two independent impls agree | cross-structure/leg hash equality | E-002c hash gates; E-002c2 addendum |
| L4 golden | byte/pixel-exact pipeline stages | golden files, tolerance=0 for integer-math stages | E-005 composite readback (tolerance 0); E-007 keyframe-exact cuts 48/48, 72/72 |
| L5 conformance | same contract across platforms/backends | shared corpus, per-backend runner | planned: cross-backend seek suite (ADR-004) |
| L6 perf-ratio | no perf regression vs *ratio* gates | ratios (never absolute times) vs committed baselines | E-002c structure ratios; E-007b copy/re-encode 6.8–11.9× |
| L7 chaos/crash | persistence survives kills | kill -9 during save/append, reopen from snapshot+log | planned (ADR-008 crash-only rule) |

Rules of the pyramid: a layer may never fake the layer above it. A golden frame proves
this build draws these pixels; it does not prove the seek was correct (L5) or fast
enough (L6). STATUS.md upgrades must name the layer that justifies them.

## 3. Standing rules (numbered, citable)

**T-1 · Every command verb ships an inverse + two properties.** For each verb V in the
command surface (ADR-010): `apply(V); apply(V⁻¹) == identity` (hash) and
`replay(log[0..=k]) == apply(k)` on every fixture. This is ADR-009's CI rule made
concrete; E-003/E-009 demonstrated both properties at schema level (9/9, 36/36).

**T-2 · Exact time or it doesn't ship.** Any code path that touches a timestamp must
consume/produce ove-time rationals (or i64 ticks + rate). No f64 may appear in a
persisted or logged payload — the schema rule from E-003 (float payloads diverged
300/300 in replay). Every time-touching crate's suite must include: NTSC
(24000/1001) + PAL (25/1) + 48 kHz sample-boundary cases, and the tick-axis overflow
guard (ove-time P3b regression).

**T-3 · Structure swaps need cross-implementation gates.** Before any data-structure
replacement (ADR-011's revisit trigger), run the old and new side-by-side over the
same seeded op script with hash equality on start-prefix + total + count. This caught
the time-keyed rekey collision bug (17 silent clip losses) and the negative-position
cast bug in E-002c/E-002c2.

**T-4 · Golden frames with declared tolerances.** Integer-math stages (YUV→RGB
coefficient paths, nearest-neighbor geometry, alpha-over with opaque inputs) are
tolerance=0 and must match a host reference exactly — E-005's composite readback is
the template. Float/GPU-dependent stages (dithering, tone mapping, resampling filters)
must declare a numeric tolerance in the golden test and justify it. Color tags
(matrix/primaries/transfer/range) must be asserted to *propagate* alongside pixels
(doc 22), not just be present.

**T-5 · Cross-backend seek conformance (ADR-004 gate).** One shared media corpus
(small, redistributable: own-generated clips at 24/25/30/29.97, VFR sample, audio
offsets) + one contract: `seek(pts_exact) → (pts, keyframe_flag)` answers identical
across FFmpeg-SW / WebCodecs / MediaCodec legs, with per-backend accuracy deltas
*documented*, not hidden. Gate for flipping ADR-004's conformance row.

**T-6 · Replay determinism is a versioned contract.** The E-003 log corpus becomes a
committed fixture set; every engine release must replay old corpora hash-identically.
This is the migration path Olive's per-version serializers implement (doc 19) and the
direct defense against Kdenlive's dual-store and locale disasters.

**T-7 · Perf gates are ratios, never absolute times.** Container CPUs vary; E-007b's
"copy ≈ 10× faster than re-encode" survives hardware change, "copy = 119 ms" does not.
CI gates: (a) structure ops ≤ N× naive-Vec baseline at fixed seed (from E-002c
medians), (b) copy/re-encode ratio ≥ 5× on the CI corpus, (c) ove-time rational-add
ns/op within 3× committed baseline (140 ns).

**T-8 · Crash-only persistence tests.** Kill the process during snapshot write and
log append (SIGKILL, not graceful); reopen must succeed from manifest + prefix-of-log
with an explicitly detected torn tail (append-only log + checksummed snapshot, ADR-008).
Kdenlive's documented dual-store corruption is the failure being designed against.

**T-9 · Schema migration tests from day one.** Every schema version bump adds a
fixture in the old format + migration test (Olive pattern). No fixture, no bump.

**T-10 · Adversarial command payloads are fuzzed.** The E-009 validation rules
(unknown id, (num,den) injection, den=0, i64 overflow, stale-undo) become a fuzz
corpus run in CI against the command parser. Doc 33's rules extend E-003's schema
validation; both are code-under-test, not prose.

## 4. What runs where

| Suite | Runs | Cadence | Layer |
|---|---|---|---|
| ove-time unit + property (15) | GitHub Actions (ci.yml) | every push | L1/L2 |
| fmt + clippy -D warnings | GitHub Actions | every push | hygiene |
| cargo-audit (RustSec) | GitHub Actions | every push | supply chain (doc 33) |
| E-003/E-009 schema properties | promoted into engine crate tests when ove-commands lands | per push | L2/L4 |
| E-002c structure bench | manual / perf-CI profile | weekly + pre-merge | L6 |
| E-005 golden composite | local (GPU) / CI (lavapipe, correctness only) | per push once ove-compositor exists | L4 |
| cross-backend seek | manual matrix (device farm when available) | per backend landing | L5 |
| crash-only persistence | CI (Linux) | per push once ove-project lands | L7 |

CI limitation, stated honestly: GitHub Actions runners have no GPU, so all GPU legs
run as *software Vulkan correctness legs* (lavapipe) or stay local/device-bound. Perf
ratios that depend on real GPU/driver characteristics (E-007b-style) can only be
validated on operator hardware; CI records, never asserts them.

## 5. Residuals and limits (honest)

- Property tests are **seeded and deterministic** (reproducible, CI-safe) — that makes
  them *not* exhaustive fuzzing. Deep fuzzing (cargo-fuzz for the command parser,
  media demuxer edge corpora) is scheduled with ove-commands/ove-decode, not promised
  here.
- Golden frames currently validate synthetic patterns (E-005 procedural frames). A
  real-media golden corpus (permissive-license clips: Blender open movies) is required
  before compositor color claims can graduate beyond synthetic correctness.
- The crash-only suite (T-8) and cross-backend seek corpus (T-5) do not exist yet;
  they are gated on the respective crates landing (ove-project, ove-decode).
- Coverage of the *UI layer* is explicitly out of scope for this doc (doc 34 owns the
  engine↔UI contract; UI tests are the shell apps' responsibility).

## 6. Links

- Gates: research/gates/GATE_STATUS.md (GATE-14 row cites this doc)
- ADRs: 004 (T-5), 006/011 (T-3), 007 (T-2), 008 (T-6/T-8/T-9), 009 (T-1), 010 (T-1/T-10)
- Experiments: research/experiments/*.md (E-002, E-002c, E-003, E-005, E-007b, E-009)
- CI: .github/workflows/ci.yml (fmt, clippy, tests, audit)
- Disaster histories: doc 19 (Kdenlive dual-store/locale), doc 02 (OpenCut no-format)
