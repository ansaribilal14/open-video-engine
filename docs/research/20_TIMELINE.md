# 20 — TIMELINE: DATA MODEL, TIME, AND EDIT OPERATIONS

> Status: PARTIAL (v0.1).
> Owner: agent 3-b. Track: C (timeline/NLE).
> Sources: fetched primary sources cited inline as (S-2bx); see
> `research/sources/SOURCE_LEDGER.md`. Mathematical arguments are standard
> computer-science reasoning with concrete numbers; nothing here requires
> trust in folklore.

## PART 1 — TIME REPRESENTATION (deep-dive)

### 1.1 The candidates

| Representation | Exactness | Mixed rates | Equality/hashing | Text round-trip | Used by (verified) |
|---|---|---|---|---|---|
| Floating-point seconds | none | poor | no | lossy (17 digits needed) | media-player APIs (e.g. HTMLMediaElement-style `currentTime`); UI *display* layers |
| Integer frame index in one sequence profile rate | exact at profile rate | requires a conformance rule per foreign-rate clip | yes | exact | MLT: `mlt_position` is `int32_t` (or double only if `MLT_DOUBLE_POSITION` is compiled in), profile carries `frame_rate_num/den` (mlt_types.h, mlt_profile.h, S-2b3) |
| (value, rate) pair | value exact in units of 1/rate; rescale is a float multiply | good (rate travels with the value) | only with care | ok | OTIO `RationalTime` (rationalTime.h: "a measure of time defined by a value and rate"; `rescaled_to`, `almost_equal` vs `strictly_equal`, S-2b1) |
| True rational num/den (int64/int32, reduced) | exact | exact | yes | exact | Olive `rational` wrapping `AVRational`, reduced and sign-normalized (olive-core rational.h, S-2b6); FCPXML: "Time values are expressed as a rational number of seconds with a 64-bit numerator and a 32-bit denominator" (Apple doc text via S-2b8) |
| Per-frame timing map (VFR) | exact per frame | native | yes | exact | camera/HLS media; editors typically *conform* VFR to CFR at import (see 1.4) |

### 1.2 Why floating-point seconds fail as the authoritative clock

This is mathematical reasoning (standard CS), with concrete numbers:

1. **NTSC rates are not decimals.** The rate usually written "23.976 fps" is
   exactly 24000/1001 fps = 23.976023976... Storing the decimal 23.976 loses
   the denominator: the relative error is (23.976023976 - 23.976)/23.976 ≈
   1.0e-6, i.e. the clock drifts ≈ 3.6 ms per hour (measured honestly: small,
   but *systematic* — it never cancels, it compounds through every re-timing
   and per-clip conversion, and it silently flips floor() comparisons near
   boundaries). The denominator 1001 must be carried. Every professional
   format does: FCPXML frame durations are rationals like 1001/30000s
   (S-2b8); MLT profiles are num/den (S-2b3).
2. **Boundary off-by-one is the practical killer.** IEEE 754 cannot represent
   1/3, 1/30, or 1001/30000 exactly. Canonical example: three clips of 1/3 s
   each sum to 0.99999999999999989, not 1.0; likewise 30000 frames of
   30000/1001-fps media is exactly 1001 s in rationals but computes as
   1000.9999999999999 (or 1001.0000000000002, depending on operation order)
   in doubles. Any `floor(seconds x rate)`-style quantization then lands on
   the *wrong frame* at exact boundaries — precisely where trims, ripple
   boundaries and media-end conditions live. Epsilon fixes move the failure;
   they do not remove it.
3. **Accumulation (secondary but real).** Sums of N additions of a rounded
   quantity carry worst-case error growing linearly in N (randomly ~sqrt(N)).
   At 1/30 s per add the practical magnitude is small, which is exactly why
   fp-based timelines *appear* to work in demos and fail later: the error is
   systematic (one direction) and interacts with (2) via floor().
4. **Order-dependence.** Floating addition is not associative:
   (a+b)+c != a+(b+c). "Clip start = sum of previous durations" computed in
   fp is therefore path-dependent — the same layout yields different starts
   depending on insertion order, loop direction, or compiler FMA decisions.
   Consequence: two of our renderers (browser and Android) can disagree
   bit-wise on the same document, and exact-equality snapping comparisons
   fail non-deterministically. Frame-accurate cuts must not depend on that.
5. **Equality is load-bearing.** Timeline logic constantly needs exact
   adjacency: "does clip A's out meet clip B's in?", "is the playhead on a cut
   for snapping?", "do these two clips touch for a roll?" Floating-point
   answers require epsilons, and epsilon tuning leaks bugs at rate
   conversions (23.976 to 25 to 29.97 to 48 kHz). Exact rationals/integers
   make equality a machine operation. (Honest counter-note: doubles are fine
   for *display* and for tolerance-based UI hit-testing — the claim is only
   that they must not be authoritative.)
6. **Audio alignment.** At 25 fps and 48 kHz, one video frame = exactly 1920
   audio samples. At 30000/1001 fps, one video frame = 48000 x 1001/30000 =
   1601.6 samples: frame boundaries fall *between* samples, and the mapping is
   only well-defined with rational math (sample position of frame n =
   n x 16016/10). fp gives no canonical rounding rule; every engine invents
   its own and they diverge after cuts and speed changes.
7. **Serialization.** A double round-trip through text needs 17 significant
   digits and care; a rational round-trips exactly as "16016/5005".

Summary of magnitudes (honest): pure accumulation error of doubles over
editorial-scale timelines is small; the failures that actually bite are (a)
wrong-frame floor() at exact boundaries, (b) order-dependence breaking exact
comparisons, (c) cross-platform non-determinism, (d) systematic decimal-rate
drift, (e) audio-sample alignment. All five are eliminated by exact integer
or rational time.

Conclusion supporting C-001: the authoritative representation must be exact
(integers/rationals). Floating-point seconds are acceptable only as a
presentation-layer derivative (display, scripting convenience), recomputed
from the authoritative value. This matches every verified mature system
(MLT int positions with rational profile rates; OTIO value/rate; Olive num/den;
FCPXML rational seconds).

Nuance recorded honestly: OTIO's `RationalTime` is *not* a big-rational; it is
a (double value, double rate) pair. It is exact enough for editorial cut lists
because values are typically integers at a rate (frame counts), and rescaling
is a single multiply; but Olive's maintainers went further to num/den
rationals. The ASWF mailing list shows an active debate titled "Time
Representations in OpenTimelineIO" about the float/float representation
(lists.aswf.io otio-discussion, S-2b9; thread content UNVERIFIED — not
fetched). Open question for our engine: int64 frame counts at a rational rate
covers editorial; audio-sample-precise mixing may need 1/48000s-grid rationals
(both are rationals — same type, different canonical rates).

### 1.3 SMPTE timecode and drop-frame (basics, standard reasoning + vendor refs)

- Timecode rates supported by OTIO are enumerated: `is_smpte_timecode_rate`
  in rationalTime.h; conversion via `to_timecode(rate, IsDropFrameRate)`.
- NTSC: color TV forced 30000/1001 fps. Timecode counts frame *numbers* at
  nominal 30 fps, so the counter runs 1.001x ahead of wall-clock: one hour of
  real time contains 3600 x 30000/1001 ≈ 107892.1 frames, but a 30-count
  counter reaches 108000 — ≈ 3.6 s ahead. Drop-frame timecode drops frame
  numbers 00 and 01 at the start of every minute except minutes divisible by
  ten: 2 x 9 = 18 numbers per 10 minutes, 108 per hour. 108 x 1001/30000 s ≈
  3.604 s, which realigns the counter with wall-clock at hour boundaries
  (residual ≈ 3.6 ms per hour remains; that is standard drop-frame behavior,
  not an error). Frame *durations* are never dropped — only numbers are, so
  total recorded media length is unaffected. (Standard SMPTE ST 12-1
  mechanism; exact clause citations TODO for v0.2. The arithmetic above is
  self-contained reasoning.) The OTIO implementation is fetched evidence:
  `to_timecode(rate, IsDropFrameRate)` with `InferFromRate`, and
  `is_smpte_timecode_rate` in rationalTime.h (S-2b1).
- Engine rule: timecode is a *string encoding* of an exact rational time at a
  rate; it must never be the stored value.

### 1.4 Variable frame rate (VFR)

- VFR is common from phone screen-recorders and webcams. The Shotcut project's
  forum guidance: "VFR may have unexpected results when editing with any video
  editor, which is why it's best to convert each VFR recording to CFR"
  (forum.shotcut.org, S-2b9). Adobe has an ongoing product effort around VFR
  editing experience (community.adobe.com, S-2b9) — evidence that even large
  vendors find VFR-in-timeline hard.
- Architectural position: the timeline document stays CFR + exact rational;
  VFR is a *source media property* carried as a per-frame timestamp map
  (PTS list) used by the decoder layer and by source-trim math. Conform at the
  media layer, not in the document. This is how MLT/Shotcut practical guidance
  works (convert/normalize), generalized.

## PART 2 — TIMELINE DATA STRUCTURES

### 2.1 What mature systems actually use

| System | Authoritative store | Track representation | Notes (source) |
|---|---|---|---|
| OTIO | ordered item list per Track | `Track : Composition` of Clip/Gap/Stack/Track; `range_of_child_at_index` walks the list | S-2b0, S-2b1 |
| MLT | service graph + playlist order | `mlt_playlist` = ordered entries with in/out; blank entries for gaps; tractor = multitrack + field | framework doc S-2b2 |
| Kdenlive | app-side `TimelineModel` (QAbstractItemModel) of tracks/clips with unique IDs; MLT graph is a projection | clips positioned by start time; model order not chronological | timelinemodel.hpp S-2b4 |
| Shotcut | app-side `MultitrackModel` exposing MLT clips as items with Start/In/Out roles | two-level tree (tracks, clips) | multitrackmodel.h S-2b5 |
| Olive | node graph; timeline participants are `Block` nodes (clip/gap/transition/subtitle) inside Track nodes | tracks hold Blocks linked in time; in/out are `rational` | node.h, block.h S-2b6 |
| Flowblade | Python object model (sequence/tracks/clips) projected to MLT | film-style insert editing | README S-2b7 |

The near-unanimous answer: **per-track ordered clip lists with in/out points,
plus a parallel effect layer** — not interval trees, not interval lists with
holes, not pure node graphs. Node graphs appear for the *compositing* domain
(Olive everywhere; adjustment/effect stacks elsewhere).

### 2.2 Options with trade-offs

| Option | Insert/overwrite | Hit-test/overlap queries | Memory | Serialization/diff | Verdict |
|---|---|---|---|---|---|
| A. Ordered clip list per track (durations imply starts) | O(n) shift on insert at head; O(1) at tail; O(n) rebuild of starts | O(n) scan, or O(log n) with derived index | minimal | trivial (list of {in,out,source,ref}) | industry default; correct base layer |
| B. Start-time-keyed map/interval tree per track | O(log n) locate; insert O(n) worst due to overlaps | O(log n + k) overlap/stabbing | index maintenance cost | derived index must be rebuilt, not stored | good *derived index*, bad authoritative store |
| C. Full node graph (blocks as nodes) | graph surgery per edit | same as A within a track | node objects everywhere | verbose | uniform effect model (Olive) but heavy; alpha-status evidence |
| D. Event-sourced command log as the store (commands replayed to state) | O(edit) apply; state = fold of log | via snapshot state | log growth, needs snapshots | excellent (log is the diff) | pairs with A; see 19_PROJECT_FORMATS.md and C-007 |

Recommended shape (candidate, not decision): **A as the authoritative model +
B as a rebuilt derived index + an effect graph (Olive-style) as a sub-model +
command-log persistence (D)**. Every verified system supports the A-layer;
nothing verified contradicts B as derived; C is a compositing pattern; D is
the open question of this mission (experiment E-003).

### 2.3 Canonical object model (synthesis of verified systems)

```
Project
  Sequences[] (each a Timeline; Kdenlive gen5 stores each in an MLT tractor)
  MediaPool[] (asset references + probe metadata; never embedded media)
Timeline (sequence)
  Stack? -> tracks[] (Track: kind Video|Audio, locked/mute/solo flags)
Track
  clips[] : ordered; each = Clip { id, source asset, source_range(in,dur),
            timeline_start (derived or stored; pick one, derive the other),
            speed (time map), enabled, effects[] }
  gaps are implicit (from layout) or explicit objects (OTIO Gap; MLT blank)
  transitions[] : between two clips on a track (MLT mix) or as objects (OTIO)
  effects[] : track-level effects
Nesting: a Clip may reference another Timeline (compound/nested) — recursion,
  not a special case (OTIO Stack/Track recursion; Kdenlive library clips =
  MLT rendering files; FCP compound clips UNVERIFIED internals)
Markers / ranges / captions: side tables keyed by rational time
```

Design tension recorded: storing `timeline_start` vs deriving from the
ordered list. Deriving avoids inconsistency (Kdenlive gen-1 dual-store bug);
storing speeds up random access (all UIs then keep an index anyway). Decision
deferred to ADR with E-003 data; the OTIO/Kdenlive evidence favors
*derived + rebuildable index*.

## PART 3 — EDIT OPERATIONS MATRIX

Vocabulary per Apple FCP docs (S-2b8): ripple, roll, slip, slide are the four
canonical trims; 3-point editing is standard editorial practice (UNVERIFIED
vendor page in this pass; vocabulary consistent across NLEs we inspected).

| Operation | Definition | Document mutation | Neighbor impact | Model requirements |
|---|---|---|---|---|
| Insert (film style) | push material downstream | splice clip into track list | all later clips shift +len | ordered list; group/ripple policy |
| Overwrite | replace range | place clip, truncate/trim overlapped | later clips unchanged | overlap resolution rules |
| Append | add at end | push_back | none | — |
| Ripple trim (head/tail) | trim and move downstream material | change clip in/out AND shift downstream starts | downstream shifts | layout derivation or explicit shift of many clips |
| Roll | move shared boundary | out(A)+in(B) change together | none outside pair | adjacency exactness |
| Slip | change content under fixed bounds | source_range shifts | none | source range independent of placement |
| Slide | move clip within neighbors | timeline position changes, duration fixed | neighbors' edges move | — |
| Lift / Delete | remove, gap vs close | remove item (keep gap) vs remove+ripple | none vs downstream shifts | explicit gap object or implicit layout |
| Split (razor) | cut one clip into two | one item -> two with shared edge | none | exact frame at cut; audio/video split rules |
| 3-point edit | fill between in/out marks from source | insert/overwrite bounded by marks | per mode | playhead+marks state in UI layer |
| Snapping | align to cuts/marks | quantize candidate positions during drag | none | exact equality of rationals (1.2.3) |
| Linked A/V | move/trim A+V together | group of two clips across tracks | both tracks | clip groups (Kdenlive GroupsModel) |
| Group | multi-clip unit | set membership; ops propagate | as group | GroupsModel S-2b4 |
| Nest / compound | sequence as clip | new asset type: timeline-ref | recursion | recursive model |
| Adjustment layer | effect applied over a range of tracks | special clip spanning range, rendered above | none | per-track override semantics; UNVERIFIED per-engine internals |
| Time remap / speed | non-uniform mapping timeline->source | time map on clip (OTIO `LinearTimeWarp`/`FreezeFrame` TimeEffects, `time_scalar`; MLT `timewarp` producer; keyframed maps in NLEs) | ripple recomputes effective duration | time maps must compose with trims |

Notes with evidence: MLT models speed as a `timewarp` producer (Kdenlive
mlt-intro.md lists it, S-2b4); OTIO models speed as TimeEffect schemas with
`time_scalar` (linearTimeWarp.h, S-2b1). Two different placements of the same
concept (media-layer vs document-layer) — our engine should put time maps in
the document layer (they must survive re-render and round-trip).

Ripple/slip/slide all reduce to two primitives on an ordered list:
`set_item_bounds(item, new_in, new_out)` and `shift_range(track, t, delta)`;
roll is both primitives on two items with an equality constraint; insert is
`shift_range + splice`. A minimal verified-capable primitive set:
1. `move/resize item` (with group propagation),
2. `splice/extract` (list ops),
3. `retime item` (time map edit),
4. `split item`,
5. `set/add/remove track/transition/effect`.
Kdenlive's request*Action set and Shotcut's QUndoCommand verb list (S-2b4,
S-2b5) map onto these one-to-one, which is strong evidence the primitive set
is complete for 2D-track NLE editing.

### 3.1 Undo/redo integration

- Kdenlive: one user operation = one undo entry built from lambda pairs
  recorded during execution; failures revert partial work (S-2b4).
- Shotcut: one user operation = one QUndoCommand subclass (S-2b5).
- Olive: custom UndoStack with node-specific commands (S-2b6).
Implication: if commands already carry their inverse (or are replayable),
the undo log and the project file's command log are the *same object*.
That is the core of claim C-007; risks (log growth, migration of log schema,
validation on load) are analyzed in 19_PROJECT_FORMATS.md.

## PART 4 — OPEN QUESTIONS (for v0.2)

1. Exact big-rational vs int64-frames-at-rate for the canonical time type
   (audio sample grid argues for rationals; benchmark E-003).
2. Stored vs derived `timeline_start` (index rebuild cost at 10^5 clips).
3. VFR: exact per-frame map schema in the document (or always conform at
   import?). Adobe's active work suggests first-class VFR support is a
   differentiator; OTIO has no VFR story in the schemas we inspected.
4. Adjustment-layer semantics (needs per-engine verification pass).
5. Blender VSE internals (scene-integrated sequencer) — not yet studied.
