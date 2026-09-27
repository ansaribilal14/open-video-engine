# ADR-012: Portable CI build for bundled FFmpeg (SIGILL class)

- **Status**: ACCEPTED (2026-09-27)
- **Date**: 2026-09-27
- **Confidence**: 0.92 (mechanism reproduced locally end-to-end; the exact
  ISA delta between the two GitHub runner CPUs is not directly observable,
  but every link in the causal chain is evidenced)

## CONTEXT

GitHub Actions `ci.yml` runs `cargo test --workspace --release --all
--features ove-decode/bundled`. Starting at 5d0fae1c/33fde9d (2026-09-27)
the conformance binary (`running 14 tests`) terminated ~135 ms in with
`signal: 4, SIGILL: illegal instruction` — while fmt, clippy, audit and
libav-confinement jobs stayed green. The two red runs restored a 527 MB
`Swatinem/rust-cache` archive ("Finished release in 7.83s"); the last green
run (6af3ed97, 14/14 conformance PASS) had restored a ~0 MB cache and built
everything from scratch on its own runner.

## ROOT CAUSE (evidenced chain)

1. `ffmpeg-sys-next 7.1.3` `build.rs` line 292 hardcodes, for every host
   (non-cross) build:
   `configure.arg("--extra-cflags=-march=native -mtune=native")`
   There is no feature or env var to disable it.
2. `-march=native` makes gcc bake the BUILD machine's ISA into libav **C**
   objects. FFmpeg's `-fno-tree-vectorize` does not prevent this: gcc
   inlines memcpy/struct stores with the widest registers the target ISA
   allows. Verified by `objdump` on a live local build (AVX-512 host):
   `vmovdqu8 %zmm0` present in `libavcodec/h264dec.o`, `mpegvideo.o`,
   `mpeg4videodec.o` — exactly the decode path the conformance suite runs.
   These are compiler-chosen instructions; FFmpeg's runtime CPU dispatch
   (cpuid-based) covers only its hand-written `.asm`, not this code.
3. `Swatinem/rust-cache` caches `target/` including those native-compiled
   objects. GitHub's ubuntu-latest runner fleet has heterogeneous CPUs, so
   the cache moves objects compiled for CPU A onto runner B. Execution of
   an instruction B lacks ⇒ SIGILL.
4. Same binary metadata hash (`conformance-8f23c15d096f748b`) appears in
   the green and red runs; only the executing runner differs. Local
   reproduction on a single machine (build + run on the same AVX-512 CPU)
   is 60/60 GREEN — matching the CI "build on A, run on A = pass" case.

## OPTIONS

- **A. Portable-compiler wrapper (ADOPTED)** — a `gcc` wrapper on PATH
  rewrites `-march=native/-mtune=native` to `-march=x86-64/-mtune=generic`
  for the bundled build; cache prefix bumped to invalidate poisoned caches;
  portability guard added to CI.
- B. Fork/patch `ffmpeg-sys-next` build.rs — fixes it at the source but
  adds a permanent supply-chain fork to maintain and justify per upgrade.
- C. Drop rust-cache from the test job — removes the transfer mechanism but
  leaves the hazard (any artifact reuse across CPUs reintroduces it) and
  costs ~4 min per run.
- D. System FFmpeg only — abandons the pinned bundled build (reproducibility
  regression; runners lack FFmpeg 7 dev packages) and hides the class.

## DECISION

Option A, with three parts:

1. `scripts/ci/gcc_portable.sh` — installed as `.ci-bin/gcc` first on PATH
   for the CI build steps. The basename MUST be `gcc`: ffmpeg-sys-next
   derives FFmpeg's `--cross-prefix` from the compiler basename, and any
   other name would break the host build.
2. `prefix-key: v2-portable-ffmpeg` on `Swatinem/rust-cache` — pre-fix
   caches contain non-portable objects and must never be restored again.
3. `scripts/ci/check_bundled_portability.sh` — regression guard for the
   class: (1) no `-march=native/-mtune=native` in any generated
   `ffbuild/config.mak`; (2) objdump scan of known pure-C libav objects
   (`mem/rational/log/dict/time/h264dec/mpegvideo/mpeg4videodec.o`) for
   `%ymm/%zmm/%k` registers. Runs after the test step in CI.

## CONSEQUENCES

- Bundled-FFmpeg **C** code is compiled for the x86-64 baseline (SSE2) in
  CI: portable across every x86-64 runner. libav's hand-written SIMD is
  unaffected — it remains assembled per-ISA and runtime-dispatched, so
  decode coverage and SIMD performance paths are unchanged; only
  compiler-chosen scalar/memory-op generation is pinned to baseline.
- CI conformance runtime impact is negligible for the committed corpus.
- If a future ffmpeg-sys-next upgrade changes its configure flags, guard
  (1) fails loudly instead of silently producing non-portable objects.
- A future performance baseline for RELEASE/distribution artifacts of the
  bundled build (if ever shipped) should revisit this via `--cpu=` /
  per-arch distribution rather than reverting to `-march=native`.
