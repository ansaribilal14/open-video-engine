# Waves 10–12 — Platform legs report (evidence collection, hardware-allowed subset)

Date: 2026-09-29 · Directive: BUILD_PLAN wave 8 / directive waves 10–12 ·
Authority: CROSS_PLATFORM_CONFORMANCE_PLAN (§1 legs, §3 invariants, §5 gates, §1 honesty rule
"VALIDATION levels ascend only on real evidence")

## What this wave delivers (honest scope)

This container has no Android device/NDK, no browser, no Tauri/webkit stack, and no GPU.
Per the plan's own rule, NOTHING here claims DEVICE / REAL_GPU / CROSS_PLATFORM status.
What IS delivered: the §3 invariant table executed for real across the WASM boundary
(L-3 core evidence), Android compile evidence (L-2, compile-level only), and two REAL
portability findings fixed in the engine (not papered over).

## §3 invariant execution — native (L-0) vs wasm32-wasip1 (L-3)

| Invariant | Suite | Evidence | Result |
|---|---|---|---|
| Command semantics | ME-1/ME-2 scenario via ove-project fold | canonical batch/split/move/keyframe/remove scenario, explicit ids (E-012) | **state_hash identical** `805d3e65…a0c17e` |
| Replay determinism | P-2 (no kill drill on WASI) | drop → reopen → log replay | **REOPEN_HASH identical, HASH_PARITY=true** |
| Undo/redo | ME-2 | undo×2 → redo×2 pre-reopen; undo→redo post-reopen | **hash-equal roundtrip on both targets** |
| Project bytes | P-1 | manifest+log written, reopened | **identical state hash via log replay** |
| Command schema | ME-7 | payload floats: none — exact rationals only, keyframe eval printed as exact `num/den` | **OPACITY_AT_12=1/2, X_AT_12=-10/1 identical** |
| Time arithmetic | ove-time spots | exact add/mul/half + P13 round-half-up | **TIME_SPOT/ROUND_SPOT/HALF_SPOT identical** |

Runner: `scripts/ci/run_platform_conformance.sh` — native run, wasm32-wasip1 build,
wasmtime v36.0.1 run, `diff` REQUIRED empty (CI job `platform-conformance`).

## Real findings (fixed in the engine — the platform wave's actual purpose)

1. **wasmtime does not honor POSIX O_APPEND on persistent handles** — every
   `write_all` landed at offset 0, silently destroying the project log (first 10
   entries lost, torn tail). Found by the L-3 reopen leg (`LogCorruption: line 1,
   seq 11 out of order`), reproduced minimized (11 appends → 1 line). FIX:
   `LogWriter::open` now uses create+write-no-truncate with an EXPLICIT
   `seek(SeekFrom::End(0))` — byte-identical semantics on POSIX for the single
   writer, correct on WASI; flush-per-entry (kill-9 model) unchanged; the P-suite
   (157/157) stays green. Documented at the site (ove-project/src/log.rs).
2. **blake3's C build needs the NDK clang on Android** — the project/media/render
   crates could not even `cargo check` for aarch64-linux-android without an NDK.
   FIX: target-scoped `blake3 = { features = ["pure"] }` for `target_os = "android"`
   in ove-media / ove-project / ove-render — same algorithm, identical digests
   (upstream-tested), no C toolchain. Native builds keep the default backend.

## Leg status after this wave (§5 vocabulary — honest)

| Leg | Level now | Evidence | Not claimed (hardware/deps-bound) |
|---|---|---|---|
| L-0 headless/CLI | SOFTWARE (unchanged) | W6 milestone + suites, CI green | — |
| L-1 desktop | SOFTWARE + (W9) GPU parity on software Vulkan | E-006b transport bench pending; wgpu golden parity landed (PR #11) | REAL_GPU, Tauri transport |
| L-2 android | NONE → **COMPILE-LEVEL core evidence** | 6 pure-Rust crates `cargo check` clean for aarch64-linux-android (CI job) | DEVICE (E-004c on-device JNI, MediaCodec D-suite subset, FGS drill) |
| L-3 browser | SOFTWARE(engine) **core evidence** | §3 table native≡WASI (above); wasmtime O_APPEND finding fixed | browser shells (WebCodecs D-suite subset, OPFS, feature-detection matrix) |

CROSS_PLATFORM (top-level): NOT claimed — requires L-0 + two of {L-1, L-2, L-3}
passing their full suites with recorded subsets; this wave contributes recorded
subsets toward L-2/L-3 and keeps every claim inside the plan's evidence rules.

## Files

- `engine/ove-conformance/` (new workspace member): `platform_core` — the canonical
  scenario (structural commands + W8 keyframes + undo/redo + reopen + exact time
  spots), byte-stable `KEY=VALUE` output.
- `scripts/ci/run_platform_conformance.sh` + CI job `platform-conformance`.
- `engine/ove-project/src/log.rs` — O_APPEND position policy fix.
- `engine/{ove-media,ove-project,ove-render}/Cargo.toml` — android-scoped pure blake3.
