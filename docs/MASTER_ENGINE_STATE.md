# MASTER ENGINE STATE — single authoritative work order

> RULE (takeover directive §6): this is the ONE current engineering truth.
> Other status documents are historical/evidence records and stay untouched.
> Update this file at the end of every major wave using the §40 report format.

WAVE: 0 / 0.5 / 0.6 (takeover audit + CI SIGILL resolution + contract reconciliation start)
DATE: 2026-09-27
COMMIT: `b6f58c99c9d3` (main; PR #1 merged)

## CI CONFIRMATION (2026-09-27)

- PR #1 run 36303605420: ALL 3 JOBS GREEN (fresh portable build; wrapper
  marker present; guard scanned 1445 objects PASS; conformance 14/14).
- Post-merge main run 36304226310: ALL 3 JOBS GREEN — the original failure
  scenario (cache restored from another runner) now passes with portable
  artifacts; conformance 14/14; portability guard PASS.

## Current commit

- Takeover baseline: `33fde9d0eddd8ad143dee245ee2975b0bdb8e347` (main)
- This session lands: CI SIGILL fix (ADR-012) + portability guard + takeover
  audit + this file (branch → PR → merge; hash recorded in the merge commit)

## Current implementation inventory

| Crate | State | Evidence |
|---|---|---|
| ove-time | TESTED — 6 unit + 11 property | CI + local 60/60 (2026-09-27) |
| ove-timeline | TESTED — 10 properties, AVL primary (ADR-011) | CI + local; bench in ADR-011 |
| ove-media | TESTED — 13 unit (asset hash, FrameEnvelope, pool, probe types) | CI + local |
| ove-decode | TESTED — 14 conformance (D-1..D-12) + 6 probe; libav confined | CI + local, both system and bundled libav |
| ove-render / ove-encode / ove-project / ove-engine / ove-cli | PLANNED (waves 3–5 per ENGINE_BUILD_PLAN) | — |

## Current research / architecture status

- ADR-001/002/007/008/009/010/011 ACCEPTED; ADR-003/004/005/006 PROPOSED with
  named evidence legs; **ADR-012 ACCEPTED this session** (portable CI bundled build).
- 45 research docs + 137-source ledger + gates (13 UNDERSTOOD / 1 PARTIAL env-bound).
- Architecture residuals (named, not hidden): E-005 real-GPU perf, E-004c device
  runtime, E-006b real-Tauri transport — all hardware/environment-bound.

## Current validation status

- 60/60 workspace tests GREEN locally on BOTH libav paths:
  - system FFmpeg 7.1.5 (pkg-config, distro libs)
  - bundled FFmpeg 7.1 (ffmpeg-sys-next, through portable gcc wrapper)
- fmt GREEN · clippy -D warnings GREEN · libav-confinement GREEN (local)
- CI: last green 36263921060; SIGILL 36284778365/36284844952 root-caused
  (ffmpeg-sys-next `-march=native` + rust-cache cross-runner restore);
  fix + regression guard committed this session — **CI re-verification
  required on the PR** (gate for merge).

## Current CI state

- Workflow: fmt + clippy + tests (bundled) + portability guard + cargo-audit
  + libav-confinement. Cache prefix `v2-portable-ffmpeg` (poisoned caches
  unreachable).

## Current blockers

- None environmental. CI-green-on-PR is the merge gate for this session's branch.

## Current open gaps (top)

1. WAVE 0.6 remainder: ove-time overflow-policy spec amendment (saturate-vs-panic
   + i64::MIN contract — audit §10 #1/#2); hash-terminology single-policy check
   across PROJECT_FORMAT_SPEC/media (BLAKE3 is the code truth — verify no SHA-256
   wording implies a second algorithm).
2. WAVE 1 media↔timeline seam hardening (next implementation wave).
3. ove-render (software reference first) — directive §11, after 0.6.
4. Mutation-style "test the tests" not yet systematic.
5. Hardware-bound experiment residuals (E-004c/E-005/E-006b).

## Current wave order (directive §37, unchanged)

0 audit ✓ → 0.5 SIGILL ✓(pending CI confirm) → 0.6 reconcile (in progress) →
1 seam → 2 render → 3 encode+mux+export → 4 project → 5 engine+cli →
6 vertical slice → 7 audio → 8 keyframes → 9 GPU → 10–12 platforms →
13 conformance → 14 headless → 15 AI/MCP → 16 scripting → 17 plugins →
18 security → 19 perf → 20 docs/release → 21 production audit.

## Current acceptance gates

- Merge gate now: fmt + clippy -D warnings + 60/60 tests (bundled) + portability
  guard + cargo-audit + libav-confinement, all green on the PR run.
- Next wave gate (0.6 exit): spec amendments committed with property tests;
  MASTER_ENGINE_STATE updated; no doc contradicts code.

## Resolved decisions (registry)

- ADR-012 (2026-09-27): CI bundled-FFmpeg portability — wrapper + cache prefix
  + guard. Confidence 0.92. Reopen condition: ffmpeg-sys-next flag change (guard
  fails loudly) or CI SIGILL recurrence.

## Unresolved decisions (registry)

- ove-time overflow policy: intentional-panic contract vs saturate (doc says
  saturate, code panics) — decide at 0.6 with property tests; do NOT drift.
- Hash algorithm single-policy statement (BLAKE3) in PROJECT_FORMAT_SPEC §5
  wording — verify wording matches code exactly; amend if it implies SHA-256.
- NLE derived-verb surface (ripple/roll/slip/slide/insert/overwrite/extract/lift):
  record primitive-vs-derived table when timeline work resumes (directive §6.2) —
  not before.

## Explicit next action

Land this session's branch via PR (CI green → merge), then WAVE 0.6:
commit the ove-time overflow-policy amendment + hash-terminology reconciliation
as their own atomic commits, update this file, then start WAVE 1.
