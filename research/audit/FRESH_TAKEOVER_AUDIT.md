# FRESH TAKEOVER AUDIT — WAVE 0

- **Session**: 2026-09-27, fresh-session takeover per MISSION_DIRECTIVE §4
- **Auditor**: principal-engineer takeover agent (independent session, no prior memory)
- **Repo state at audit start**: `main` @ `33fde9d0eddd8ad143dee245ee2975b0bdb8e347`
  (matches the baseline named in the takeover directive — verified, not assumed)
- **Reading-depth legend**: [L] read line-by-line · [T] reviewed via full test-suite
  pass + spec cross-check · [S] skimmed/scoped

## 0. Method

Repository cloned fresh from GitHub. Every claim below was verified against
live evidence (git, logs, source, objdump output, API-fetched CI logs for runs
36263921060 / 36284778365 / 36284844952). No prior session summary was trusted.
Where this audit could not establish something, it says so explicitly.

---

## 1. Current repository inventory

| Item | Finding |
|---|---|
| Branch | `main` (single local + remote branch; no stray branches) |
| HEAD | `33fde9d0` — docs/DEV_ENV.md canonical recipe |
| Working tree | clean at clone |
| History | linear-ish with one reconciliation merge (5d0fae1c); no force-push evidence; 546 objects in pack |
| Top level | README.md · STATUS.md · LICENSE-MIT · LICENSE-APACHE · docs/ · engine/ · experiments/ · research/ · scripts/ · .github/ |
| engine/ | workspace of 4 crates: ove-time, ove-timeline, ove-media, ove-decode; Cargo.lock committed; deny.toml present |
| Rust source | 7,390 LOC across 20 .rs files (tests included) |
| Specs | docs/specs/: DECODER_SPEC, ENCODER_SPEC, FRAME_CONTRACT, MEDIA_ENGINE_SPEC, PROJECT_FORMAT_SPEC, RENDER_GRAPH_SPEC |
| ADRs | 11 files (ADR-001..011) + TEMPLATE + UPDATED_ADR_STATUS; ADR-012 added by this session (CI SIGILL fix) |
| research/ | adr/ audit/ (12 docs) claims/ experiments/ gates/ risks/ sources/ (ledger 137 sources) synthesis/ unresolved/ |
| docs/research/ | 45 numbered research documents (~8,000+ lines) |
| scripts/ | ci/ (libav confinement, + this session: portability guard + gcc wrapper), corpus/gen_corpus.py, env_local/libav_env.sh, experiments/E-006a_ipc_serialization crate |
| Corpus fixtures | ove-decode/tests/media/: cfr24, vfr, ntsc, vidaud, truncated, garbage (+ committed ffprobe/frames/packets golden JSON) |
| CI | .github/workflows/ci.yml — 3 jobs: rust (fmt+clippy+tests bundled), cargo-audit, libav-confinement |

## 2. Implementation status

| Crate | Classification | Notes |
|---|---|---|
| ove-time | **TESTED** [L] | 297 LOC, zero deps (serde optional). Exact rational; i128 intermediates; normalized den>0. Doc drift found — see §10. |
| ove-timeline | **TESTED** [T] | Verbs with exact inverses, batch atomicity, undo/redo, typed errors; augmented AVL primary per ADR-011; gap.rs kept as noted alternative; oracle.rs model-based reference |
| ove-media | **TESTED** [L: asset/pool; T: frame/probe] | BLAKE3 content addressing (streaming); FrameEnvelope + FramePool with generation staleness; probe types per DECODER_SPEC |
| ove-decode | **TESTED** [L: decoder; T: mod] | FfmpegSwDecoder: full open/decode/seek/flush/cancel path; exact pkt_timebase handling; D-4 drop-until; typed hw-accel rejection; libav* confined here (CI-enforced) |
| ove-render / ove-encode / ove-project / ove-engine / ove-cli | **PLANNED** | Not present (honest per STATUS; next waves) |

## 3. Test status

| Suite | Local result (this container) | CI result |
|---|---|---|
| ove-time unit | 6/6 PASS | PASS (green runs) |
| ove-time properties | 11/11 PASS | PASS |
| ove-timeline properties | 10/10 PASS | PASS |
| ove-media unit | 13/13 PASS | PASS |
| ove-decode probe | 6/6 PASS | PASS |
| ove-decode conformance D-1..D-12 | **14/14 PASS** (bundled AND system libav paths) | 14/14 on green runs; **SIGILL** on red runs (root cause §6) |
| Workspace total | **60/60 PASS** | — |
| fmt / clippy -D warnings | PASS | PASS (even on red runs) |
| cargo-audit (RustSec) | not rerun locally | PASS |
| libav-confinement | PASS | PASS |

Mutation-style spot check performed mentally against suite design: golden tables
compare exact rationals from committed ffprobe dumps; conformance asserts typed
error behavior (truncated/garbage corpus present). "Test the tests" (§17 of
directive) is not yet systematically implemented as explicit known-bad checks —
recorded as an open gap (§11).

## 4. Architecture status

| Decision | State | Confidence |
|---|---|---|
| Rust-centered shared core, clients-not-engines | ADR-002 ACCEPTED | 0.90 |
| Exact rational authoritative time (no fp) | ADR-007 ACCEPTED, encoded in ove-time | 0.95 |
| Augmented AVL primary timeline structure | ADR-011 REVISED at its own gate (bench: AVL p99 0.52ms vs gap 3.62ms @1ms budget) | 0.90 |
| Command system / exact inverses / no implicit ID allocation | ADR-008/009/010 ACCEPTED; E-012 10/10 | 0.90 |
| Decoder abstraction + capability honesty + no hidden HW downgrade | ADR-004 + D-9 typed rejection in code [L] | 0.90 |
| libav isolation to ove-decode | DECODER_SPEC §5 + CI-enforced (2 checks) | 0.95 |
| BLAKE3 content-addressed assets | asset.rs [L]; PROJECT_FORMAT_SPEC §5 | 0.90 |
| FrameEnvelope + pool generation staleness | FRAME_CONTRACT §5.2; pool.rs [L] | 0.88 |
| Bundled-vs-system FFmpeg equivalence | Same 7.1.x major.minor; conformance corpus identical both paths (this session) | 0.85 |
| CI portability of bundled build | **BROKEN → FIXED this session (ADR-012)** | 0.92 |

## 5. Documentation status

| Document | Classification |
|---|---|
| README.md | CURRENT (minor drift: CI badge/description updated this session) |
| STATUS.md | CURRENT, v0.8 (this session appends the CI-fix entry) |
| docs/DEV_ENV.md | CURRENT (canonical container recipe; verified working end-to-end this session — rustup + extracted debs + libclang + env block reproduce 60/60) |
| docs/MISSION_DIRECTIVE.md, ENGINE_BUILD_PLAN.md, FINAL_ARCHITECTURE_WHITEPAPER.md, CROSS_PLATFORM_CONFORMANCE_PLAN.md | NORMATIVE planning docs (not contradicted by code findings) |
| docs/specs/* | NORMATIVE contracts; DECODER_SPEC/FRAME_CONTRACT match implementation [cross-checked] |
| research/audit/* (12 docs, 2026-09-26) | HISTORICAL snapshots of audit-day state (per their own headers) — superseded by this document for current state |
| research/gates/UPDATED_GATE_STATUS.md | HISTORICAL snapshot |
| research/sources/LEDGER_S5a.md | CURRENT ledger addition |
| experiments/ + research/experiments/ | EVIDENCE records (do not modify) |

No document was deleted or rewritten to look cleaner (directive §9 respected).

---

## 6. CI status — the SIGILL failure (WAVE 0.5) — ROOT-CAUSED AND FIXED

**Failure**: `cargo test --workspace --release --all --features ove-decode/bundled`
→ conformance binary SIGILL (signal 4) ~135 ms after `running 14 tests`.
Affected runs: 36284778365 (5d0fae1c), 36284844952 (33fde9d). Last green:
36263921060 (6af3ed97, 14/14 PASS).

**Evidence chain** (all verified this session):

| # | Fact | Source |
|---|---|---|
| 1 | ffmpeg-sys-next 7.1.3 build.rs:292 hardcodes `--extra-cflags=-march=native -mtune=native` for host builds; no opt-out | vendored crate source |
| 2 | Live local bundled build compiles libav C with `-march=graniterapids` (full AVX-512 set) | `ps` capture of gcc invocation |
| 3 | Resulting C objects contain `vmovdqu8 %zmm0` in `h264dec.o` (4), `mpegvideo.o` (2), `mpeg4videodec.o` (6) — compiler-inlined wide stores, NOT runtime-dispatched asm | `objdump -d` |
| 4 | Green run: ~0 MB cache restore → full build on runner A → 14/14 PASS. Red runs: 527 MB cache restore → "Finished in 7.83s" → same binary hash → SIGILL on runner B | CI logs (API) |
| 5 | Local single-CPU repro (build+run same AVX-512 host): 60/60 PASS including bundled conformance | this session |
| 6 | Rust code itself is baseline x86-64 (no RUSTFLAGS/target-cpu in CI) → fault layer is the cached native objects | workflow + logs |

**Fix (ADR-012, ACCEPTED)**: portable gcc wrapper (`scripts/ci/gcc_portable.sh`,
installed as `.ci-bin/gcc` first on PATH) rewrites `-march=native/-mtune=native`
→ `-march=x86-64/-mtune=generic` for the bundled build; `prefix-key:
v2-portable-ffmpeg` on rust-cache (poisoned caches unreachable);
`scripts/ci/check_bundled_portability.sh` regression guard (config.mak flag
check + objdump scan of pure-C objects) runs post-test in CI. libav hand-written
SIMD remains runtime-dispatched — decode coverage and conformance unchanged.

**What was deliberately NOT done**: bundled FFmpeg was not removed from CI
(directive §5); tests were not weakened; no conformance case was dropped.

## 7. Evidence status

| Claim | Recoverable chain? |
|---|---|
| E-001..E-012 experiment results | YES — research/experiments/ records + committed harnesses |
| 60/60 local green (v0.8 claim) | REPRODUCED this session (system + bundled paths) |
| libav confinement | YES — script is deterministic (cargo metadata + source grep), PASS |
| CI green history | YES — GitHub Actions runs 36069149793..36263921060 all success |
| CI SIGILL | YES — logs fetched + root-caused (§6) + fix + guard committed |
| Bench claims (AVL 0.52ms etc.) | YES — e012_bench binary committed; NOT re-benched this session (no perf claim re-verified — honest residual) |

## 8. Security status

| Area | Finding |
|---|---|
| unsafe FFI | confined to ove-decode (decoder.rs + ffmpeg/mod.rs); all libav returns checked; packet/frame unref paths audited [L] — no leak found on the audited paths |
| libav* linkage | CI-enforced confinement (graph + source scan) — PASS |
| Hostile media | truncated + garbage corpus fixtures present with typed-error expectations (D-8 path) |
| Supply chain | Cargo.lock committed; cargo-audit in CI; deny.toml present |
| Secrets | repo tree clean; **NOTE: a GitHub PAT was pasted into chat by the operator — rotated usage: token used for push only, never written into any repo file (per DEV_ENV standing rule); operator should revoke/rotate after this session** |
| Future AI injection | architecture keeps analysis→proposal→validation→approval→execution separation (directive §19) — not yet implemented, nothing to audit |

## 9. Performance status

- e012_bench committed and CI-covered (build), numbers recorded in ADR-011.
- This session made NO performance changes; bundled-FFmpeg C code moves to
  baseline ISA in CI (conformance correctness path, not a perf benchmark).
- No regression-ratio baselines exist yet for engine ops (directive §18
  infrastructure is a later wave; noted, not silently skipped).

## 10. Contradictions / drift found

| # | Drift | Class | Disposition |
|---|---|---|---|
| 1 | ove-time module doc says arithmetic "saturates on overflow" but code PANICS via `expect` on result/i64 conversion overflow | H (documentation drift) | Recorded; policy question (panic = intentional contractual vs doc bug) queued for a spec amendment decision — NOT silently changed this session |
| 2 | ove-time `gcd64`/`neg` hit `i64::MIN` edge (`.abs()` debug-panic; `neg` of MIN wraps/panics) | potential D/A | Same queue as #1: define the i64::MIN policy explicitly (panic with message is acceptable for an absurd tick value, but it must be contractual + tested) |
| 3 | `FramePool` reclaim path clones frame bytes (decoder.rs `reclaim` → `FrameBytes.data.clone()`) before release | performance note | Recorded (§6.3 of directive); renderer must not build on a `take()`-free reclaim API; optimization deferred with a marker |
| 4 | README CI-badge/description wording pre-dates the SIGILL episode | H | Updated this session alongside the fix |
| 5 | STATUS.md CI row claimed "TESTED-infra" while the test leg was red | H | STATUS entry amended this session with the fix + guard |
| 6 | Workflow `branches:` line — investigated as possibly corrupted (`ain]`); **disproven**: display artifact of `[m` sequences in this environment; HEAD already had `branches: [main]` | none | No change needed (lesson recorded: verify bracket content with boolean byte-checks, not rendered output) |

## 11. Open gaps (unchanged + new)

1. ove-render / ove-encode / ove-project / ove-engine / ove-cli do not exist (planned waves).
2. "Test the tests" (mutation-style known-bad checks) not yet systematic.
3. i64::MIN + saturate-vs-panic policy needs an explicit spec amendment (drift #1/#2).
4. Audio leg not started (per wave order — correct).
5. E-004c runtime / E-005 real-GPU / E-006b real-Tauri legs remain hardware-bound residuals.
6. No perf regression-ratio CI baselines yet.
7. Corpus is self-generated (mpeg4); broader real-world corpus + fuzz depth pending (per doc 45).

## 12. Risk ranking (top, this session)

| Rank | Risk | Mitigation state |
|---|---|---|
| 1 | Cross-runner native-artifact portability (SIGILL class) | FIXED + guarded (ADR-012) |
| 2 | ffmpeg-sys-next upgrade could change configure flags/behavior | guard (1) catches flag class; upgrade checklist lives in directive §32 |
| 3 | Cache-restored artifacts masking source drift | rust-cache keyed on lockfile+rustc; source changes recompile (observed) |
| 4 | ove-time overflow-policy ambiguity propagating into timeline/project layers | queued as wave-0.6 contract decision BEFORE ove-project lands |
| 5 | Single-fixture conformance corpus breadth | known; corpus generator committed for extension |

## 13. Immediate work order (this session)

1. ~~Root-cause CI SIGILL~~ → DONE (ADR-012)
2. ~~Fix without weakening conformance~~ → DONE (wrapper + cache prefix + guard)
3. ~~Verify fix locally end-to-end~~ → portable rebuild + objdump-clean + 60/60 (see §6)
4. Reconcile STATUS.md + README.md entries (this session)
5. Land ADR-012 + audit + MASTER_ENGINE_STATE on a branch; PR; CI must be green; merge
6. WAVE 0.6 continues: contract audit findings (§10) → spec amendments as their own commits
