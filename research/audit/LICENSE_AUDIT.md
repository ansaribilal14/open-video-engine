# LICENSE AUDIT (forensic, 2026-09-26)

> Question answered: can the repo legally declare its license, and is the slice plan
> license-safe? Method: README licensing posture vs actual repo files; doc 35 verdict
> cross-check; dependency licenses (see DEPENDENCY_AUDIT).

## 1. Repo-state findings

| Item | State | Problem? |
|---|---|---|
| README "License" section | "TBD — pending the licensing audit (directive TRACK W)" | ACCEPTABLE for private research phase; must be resolved before first public release |
| LICENSE file | **ABSENT** | same TBD cause; no legal risk while private, but add before open-sourcing |
| source headers | none (research corpus) | not required for MIT/Apache-2.0; add SPDX headers with first crate wave for hygiene |
| doc 35 verdict | "MIT OR Apache-2.0" for the engine; GPL/LGPL codec stacks isolated to adapters; x264 excluded from distributed core; AV1 §1.3 defensive termination verified | consistent with DEPENDENCY_AUDIT |

## 2. License-compatibility posture of the planned slice

1. **Engine crates (ove-*)**: MIT OR Apache-2.0 dual — clean against all planned deps.
2. **FFmpeg linkage (desktop/headless decode/encode)**: LGPL-2.1+ build discipline:
   dynamic linking or --enable-lgpl builds only; no GPL components (x264 explicitly
   rejected in ADR-005). Contamination rules from doc 35's RED/GREEN table apply.
3. **Android**: platform MediaCodec — no license exposure; Media3 (Apache-2.0) optional
   export backend.
4. **Browser**: WebCodecs/mediabunny (MIT) — no exposure.
5. **Research corpus citations**: quoting findings from GPL-licensed codebases
   (Kdenlive/Shotcut/Olive) is fair use for architecture analysis; no code was copied —
   verified by the audit (all engine code is original).

## 3. Third-party content in-repo

- Experiment media: E-007b mp4s are **self-generated** (ffmpeg testsrc-class content) —
  no third-party copyright issue (they are local-only anyway, D-5).
- No vendored third-party code found in engine/ or scripts/.
- Generated Kotlin (E-004b) is UniFFI output of our own IDL — fine.

## 4. Required actions (cheap, scheduled)

1. Add `LICENSE-APACHE`/`LICENSE-MIT` + Cargo `license = "MIT OR Apache-2.0"` fields with
   the first crate wave (do not wait for public release; costs nothing, removes a gate).
2. SPDX headers in new crates from file one.
3. At ADR-003 ACCEPT time: record the exact FFmpeg build profile (lgpl-only flags) in the
   ADR so the license posture is reproducible.
4. Keep the annual patent-landscape check (doc 35 tracker) — AVC/HEVC pool royalties
   affect distribution, not development; noted in ENCODER_SPEC.

## 5. Verdict

No license violations exist today. The TBD is honest and correctly gated, but the
license files themselves are cheap to add now — scheduled into the slice wave 1.
