# ADR-024: Decoder input budgets — declared geometry caps at the media boundary

- Status: ACCEPTED
- Date: 2026-10-02 (RLW-9; closes the ADR-022 "decoder pixel-bomb" residual)
- Consumers: ove-decode (probe backend + decoder adapter), ove-media (typed error), ove-engine (import boundary), SECURITY.md

## Context

ADR-022 (W18) budgeted every untrusted *line/byte* reader but explicitly
left the decoder's untrusted *geometry* open: "libav `max_pixels`/dimension
caps are NOT set in this wave; a hostile media file with extreme dimensions
still relies on libav's own limits." The residual was deliberately parked
to ride the hostile/difficult real-media corpus conversation so the caps
could be designed against real difficult media, not synthetic assertions.

RLW-8 (REALWORLD_VALIDATION §10) provided exactly that evidence base: seven
real corpus items (406×720 portrait, 1280×720 CFR/VFR/long-GOP/audio
variants) established the honest scale the budgets must carry with ≥1000×
headroom, and the corpus discipline (typed errors, classified findings,
identity-gated media) set the enforcement idiom. The user commissioned
RLW-9 as the separate security wave (user-separated from RLW-8 by design:
RLW-8 = "what does real difficult media do", RLW-9 = "can deliberately
extreme decoder inputs be safely rejected").

## Decision

**1. Declared constants, not magic numbers** (the ADR-022 idiom):

| Constant | Value | Meaning |
|---|---|---|
| `ove_decode::DECODE_MAX_DIM` | 16 384 | hard cap per dimension (w or h) |
| `ove_decode::DECODE_MAX_PIXELS` | 33 554 432 (2²⁵) | hard cap on declared pixels per frame |

Legitimate scale is far below every cap: the certified corpus tops out at
1280×720 (0.92 MP), 4K UHD is 8.3 MP, and 8K UHD (7680×4320 = 33.2 MP) is
INSIDE the pixel cap — honest use never sees the budgets. A hostile header
declaring 40000×40000 (1.6 GP), 16384×16384 (268 MP), or 4000×9000
(36 MP) is rejected before any allocation scales with the declared
geometry.

**2. Typed rejection with its own variant.** `DecodeError::
BeyondDeclaredLimits(String)` and `ProbeError::BeyondDeclaredLimits
(String)` — a media PROPERTY (not a backend capability, hence not
`Unsupported`), carrying the constant name in the message. Both enums are
exhaustively matched at their boundaries; the new variant is mapped 1:1
through `pe2de`.

**3. Enforced at BOTH boundaries — read-early, allocate-never.**

- *Probe/import boundary* (`FfmpegProbe::stream_facts`): the header-level
  probe rejects before a probe result can drive any geometry-scaled
  allocation downstream. `import_media` therefore fails typed at the
  project door; hostile geometry never becomes an asset.
- *Decoder open boundary* (`FfmpegSwDecoder::open`): defense in depth — a
  caller that bypasses the probe is still rejected before
  `avcodec_open2`, pool-geometry computation, or any `av_image` buffer
  sizing.

**4. Pinned by committed conformance (no corpus media in CI).** New
`ove-decode/tests/limits_test.rs` runs in every CI suite:

- pure budget edges (degenerate 0-dims, per-dimension violation,
  per-pixel violation, honest scale incl. 8K UHD);
- a runtime-header-patched **v210/MOV** fixture (codecpar dimensions come
  straight from the sample entry — no codec-level config to spoof) whose
  container DECLARES 16400×64 → typed dimension-cap rejection, and
  4000×9000 → typed pixel-cap rejection, at BOTH boundaries;
- the negative control: the unpatched fixture opens through both
  boundaries (budgets invisible to honest media).

The hostile media is SYNTHESIZED at test runtime from a committed 49 KB
fixture — no hostile media in git, matching the corpus discipline.

**5. Zero-durations are not corruption (RLW-8-F3 fix riding the same
decode-boundary audit).** The corpus proved real benign media can declare
NO per-frame duration at all (`vfr_constructed`: 220/220 packets duration
N/A, decoded frames carry duration 0, ffprobe decode clean). The pre-fix
decoder typed every such frame `Corrupt("frame without duration")` —
render-blocking the whole VFR class. Fixed: a container-declared absence
is delivered as duration `0/1` (exact, never inferred from a rate); a
NEGATIVE duration stays typed corrupt. Pinned by a committed synthetic
B-frame VFR fixture (`vfr_zerodur.mp4`, 8.7 KB, 23/23 zero-duration
packets, `-xerror`-clean) whose whole stream must decode and deliver
zero-duration frames. The real-media proof: the corpus VFR item renders
and exports 96/96 frames after the fix (pre-fix typed-failure evidence
preserved in the RLW-8 records), while every previously certified export
hash stayed byte-identical (primary `baf23d2a…`, W17/W18 `ff5e67f8…`, all
6 prior corpus exports).

**6. VFR detection is presentation-order analysis (RLW-8-F1 fix, same
wave).** `VfrReport::from_pts` sorts packet-order pts into presentation
order before the delta pass — VFR is a property of presentation timing.
B-frame CFR media no longer false-positives; genuinely VFR media still
detects. Pinned by pure unit tests (CI) and the corpus probe-vs-ffprobe
verdict alignment (all 7 items, both directions).

## Consequences

- The ADR-022 pixel-bomb residual is CLOSED for the software decode path:
  no untrusted header can drive an allocation before a declared cap
  rejects it typed.
- Determinism and the certification loop are untouched: all three hash
  invariants and all six corpus export hashes were re-verified
  byte-identical with the budgets in place (they are invisible to honest
  media by construction).
- Workspace 210/210 (+7 budget/zero-duration conformance, +3 VFR unit
  pins); fmt/clippy GREEN; corpus 5/5 scenarios (VFR upgraded from
  typed-fail to certified).
- Honest-use behavior is unchanged: every cap carries ≥1000× legitimate
  headroom (8K UHD legal by declaration).

## Named residuals (not done, deliberate)

- **Audio-shape budgets** (absurd sample rates/channel counts): libav
  opens these fine and the W7 audio surface rate-limits nothing hostile —
  but no corpus evidence showed a real attack shape; deferred until the
  audio-hostile leg is commissioned (same pattern as the pixel residual:
  design against evidence, not speculation).
- **Non-libav probe backends** (browser/WASM legs): the budget check is
  implemented in the libav backend; the `ProbeBackend` trait does not
  enforce it. Each future backend re-implements the same declared check —
  recorded here so the requirement is explicit.
- **Segmentation-era caps** (per-clip decode regions beyond per-frame
  geometry): belongs with the multi-source/render waves.

## Confidence

0.90. The budgets are declared constants enforced at two boundaries with
committed hostile-header conformance and a negative control; the one
empirical pixel-bomb shape (header-declared extreme geometry) is
reproducibly converted from "reach libav's allocator" to a typed error.
What would reopen this ADR: a legitimate workload exceeding a cap; a
surface discovered that allocates from untrusted geometry before these
checks run; a codec/container where codecpar dimensions are not declared
by the container header (streaming formats) — the check then needs a
post-first-frame re-validation point.
