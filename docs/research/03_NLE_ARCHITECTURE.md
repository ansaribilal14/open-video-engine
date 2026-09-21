# 03 — NLE ARCHITECTURE

> Status: PARTIAL (v0.1).
> Owner: agent 3-b. Tracks: A (editor architecture), C (timeline/NLE).
> Method: primary sources first (source code, official docs). Sources are listed
> per section and cross-referenced in `research/sources/SOURCE_LEDGER.md`
> (S-2b0..S-2b9). Anything not verified against a fetched source is marked
> UNVERIFIED.

## 1. Scope and method

This document surveys how mature non-linear editors (NLEs) are actually
architected: how the application model relates to the media engine, where the
timeline document lives, how undo is implemented, and what this implies for the
Open Video Engine. Focus is on open-source systems whose source we inspected
directly (Kdenlive, Shotcut, Olive, Flowblade, MLT, OpenTimelineIO), with
commercial systems referenced only for vocabulary (Apple Final Cut Pro docs for
trim terminology, FCPXML for time representation).

Depth still required (v0.2+): Blender VSE sequencer internals; Premiere/Resolve
data model via vendor docs only; OpenShot/libopenshot timeline class; AAF SDK.

## 2. A taxonomy of NLE architectures

Observed pattern classes:

1. **UI + separate engine (engine-delegating NLE).** The application owns a
   document model (its own timeline objects) and delegates rendering to an
   engine library it does not consider its own code. Kdenlive, Shotcut and
   Flowblade all sit on MLT this way. Kdenlive's own architecture doc states:
   "The most important library is MLT which is responsible for the core video
   editing functionality ... Kdenlive provides the user interface for this
   functionality" (dev-docs/architecture.md, S-2b4).
2. **Self-contained engine with a node graph.** Olive (0.2, "master" branch)
   builds everything — effects, generators, *and* timeline clips — out of a
   single Node abstraction connected in a graph (app/node/node.h: "A single
   processing unit that can be connected with others to create intricate
   processing systems", S-2b6).
3. **Closed commercial monoliths.** Premiere Pro, Final Cut Pro, Resolve.
   Only vendor docs / file formats are observable. They are included for
   vocabulary and time representation, not internals (internals UNVERIFIED).

## 3. OpenTimelineIO: interchange model, not an editing engine

OTIO (AcademySoftwareFoundation, Apache-2.0, verified from repository headers)
explicitly defines itself as "an interchange format and API for editorial cut
information. OTIO contains information about the order and length of cuts and
references to external media. It is not however, a container format for media"
(README.md, S-2b0/S-2b1).

Canonical structure (docs/tutorials/architecture.html):

```
Timeline
  tracks: Stack
    Track (kind: Video / Audio)
      items: Clip | Gap | Stack | Track | Transition
```

Key structural properties verified from source headers and docs:

- `Clip` timing = `media_reference.available_range` (what the media offers)
  intersected with `source_range` (what is cut in). Queries like
  `trimmed_range_in_parent()` / `trimmed_range_of_child()` compute a child's
  time in a parent's space, applying parent `source_range` trims along the way.
- `Track` is a *sequential* composition (`range_of_child_at_index` walks the
  child list); `Stack` is a *parallel* composition (all children same range).
  Both are the same base class `Composition` with different composition kinds.
- Every object carries a free-form `metadata` dictionary.
- Nesting is recursion, not a special case: a Stack or Track can contain
  Stacks/Tracks.

The critical architectural lesson: **OTIO deliberately separates the editorial
interchange model from any application's internal editing model.** Kdenlive has
an `OtioImport` class (forward-declared in timelinemodel.hpp) but does not use
OTIO objects as its live timeline; OTIO is read/imported. Our engine should
make the same separation: a canonical interchange mapping (OTIO-compatible)
plus an internal model optimized for interactive editing and command logs.

## 4. Kdenlive (GPL-3.0, C++/Qt, MLT backend)

Sources: repo README, dev-docs/architecture.md, dev-docs/mlt-intro.md,
src/timeline2/model/timelinemodel.hpp, src/doc/docundostack.hpp,
dev-docs/fileformat.md (S-2b4).

### 4.1 Model

- `TimelineModel` (timeline2) is the backend entry point for every
  modification. It "holds pointers to all tracks and clips, and gives them
  unique IDs on creation. These Ids are used in any interactions with the
  objects and have nothing to do with Melt IDs." — i.e. the application model
  is independent of the engine graph identity.
- It derives from `QAbstractItemModel` (two levels: tracks as rows, clips as
  sub-rows) so the QML GUI reads state directly from the model. Clip order in
  the model is *not* chronological: "the clips are rendered based on their
  positions rather than their row order".
- Mapping to MLT: Kdenlive builds/updates an MLT tractor graph (the header
  includes `mlt++/MltTractor.h`); effects are MLT filters, transitions are MLT
  transitions (mlt-intro.md walks producers/filters/transitions/tractor).

### 4.2 Editing API and undo (highly relevant to claim C-003/C-007)

The timelinemodel.hpp header comment is one of the best primary statements of
an NLE command API design we have found:

- All modifications enter through `request*Action` functions
  (`requestClipMove`, ...) that return bool success and "when they return
  false they should guarantee than nothing has been modified".
- Undo/redo is built from **lambda pairs accumulated during the operation**:
  "Each time a function executes an elementary change to the model, it writes
  the corresponding operation and its reverse, respectively in the redo and
  the undo lambdas." Composite operations compose the lambdas; failure reverts
  what was done so far instead of pre-simulating the edit.
- Group behavior lives in the model layer: `requestClipMove` "checks whether
  the clip belongs to a group and in that case actually moves the full group".
- `DocUndoStack : public QUndoStack` (docundostack.hpp) is the Qt undo stack
  wrapper; the lambda-pairs are pushed as one command per user-level edit.

### 4.3 Known pain points (from project docs and community, quality 4-9)

- Community reports of MLT/Kdenlive render and memory issues (discuss.kde.org,
  SourceForge melt issues; S-2b9): effects on 4K sources re-decode and scale in
  software; GPU rendering has had long-standing instability reports
  (libmovit). UNVERIFIED as current status, treated as directional.
- File-format generations caused real damage: gen-1 duplicated project data
  between "outer MLT XML" and inner Kdenlive data and "had the habit of getting
  the outer MLT data out of sync with the inner Kdenlive project data" (dev-
  docs/fileformat.md). gen-2 fixed it by making MLT XML the single source of
  truth. gen-4 was forced by a decimal-separator (comma vs point) bug "causing
  many crashes". Lesson: duplication of project state across two serializers
  is a corruption generator; locale-sensitive serialization is a hazard.

## 5. Shotcut (GPL-3.0, C++/Qt6, MLT backend)

Sources: README, src/models/multitrackmodel.h, src/commands/timelinecommands.h
(S-2b5).

- `MultitrackModel : public QAbstractItemModel` — "Two level model: tracks and
  clips on track", exposing roles `StartRole`, `DurationRole`, `InPointRole`,
  `OutPointRole`, `FramerateRole` etc. So Shotcut's timeline state is again an
  application-side model whose cells are MLT producer in/out points.
- Undo is classic Qt: one `QUndoCommand` subclass per edit operation in
  timelinecommands.h (Append/Insert/Overwrite/Lift/Remove/Group/Move/Trim/
  Split/FadeIn/FadeOut/AddTransition/AddTrack/Merge/DetachAudio/Replace/
  AlignClips/...). The command names are effectively the engine's verb set.
- Project file is MLT XML itself (shotcut.org "MLT XML Annotations" describes
  Shotcut's `shotcut:` properties embedded in MLT XML; S-2b9). There is no
  second, non-MLT document format — a deliberate contrast to Kdenlive gen-1.

## 6. Olive (GPL-3.0, C++/Qt, node-based, self-contained)

Sources: app/node/node.h, app/node/block/block.h, ext/core rational.h,
app/node/project/serializer/*.cpp, app/undo/ (S-2b6).

- Everything is a `Node` ("visual programming"): inputs/outputs, connectable.
  Timeline participants are also nodes: `Block : Node` — "A Node that
  represents a block of time, also displayable on a Timeline" — with
  subclasses clip / gap / subtitle / transition.
- `Track` is a node holding a sequence of Blocks; sequences are node graphs;
  the compositor is the same node system ("To render a frame, Olive will work
  through a node graph that can be infinitely customized by the user").
- Time: `Block::in()/out()` return `olive::core::rational`, a true
  numerator/denominator rational (wrapping FFmpeg `AVRational`, exactly
  reduced; ext/core/include/olive/core/util/rational.h). This is *stricter*
  than OTIO's `RationalTime` (double value + double rate). See 20_TIMELINE.md.
- Undo: custom `UndoStack` + `NodeUndo` (app/undo, app/node/nodeundo.*),
  not QUndoStack — because node-graph edits need their own command semantics.
- Project format: versioned XML with a **per-version serializer class per
  release** (serializer190219, 210528, 210907, 211228, 220403, 230220) and
  version sniffing on load ("Allows easy integer math for checking project
  versions"). A concrete, small-scale migration pattern worth copying.
- Branch structure: `0.1.x` (legacy, timeline-only model) vs `master` (the
  node-based rewrite). The rewrite exists *because* the fixed pipeline of
  0.1 could not express arbitrary compositing; the node model is their answer
  (README/wiki; the code structure itself is the evidence).
- Status: README says "Olive is alpha software and is considered highly
  unstable" — the node model is aspirational evidence, not proof of scale.

## 7. Flowblade (GPL-3.0, Python, MLT backend)

Source: repo README (S-2b7). Python/PySide application over MLT.

- Film-style editing idiom: "FILM STYLE WORKFLOW has the Insert tool as the
  default tool and employs insert style editing. Tools: Insert, Move, Trim,
  Roll, Slip, Spacer, Box" (release notes via search snippet; README confirms
  "Toolset with 6 editing tools", "4 methods to insert / overwrite / append").
- Two compositing workflows (standard track compositing with filters vs
  compositor objects), max 21 combined tracks, clip parenting / audio
  synchronizing.
- Lesson: the *same* MLT engine supports both an insert-driven film idiom
  (Flowblade) and a toolbox idiom (Shotcut). Editing idiom is a UI policy on
  top of the model, not a property of the engine.

## 8. Cross-system lessons for the Open Video Engine

1. **Document model and engine graph must be distinct layers.** Kdenlive
   explicitly decouples object IDs from MLT IDs; Kdenlive gen-1's dual
   serialization caused corruption; Shotcut avoided it by making MLT XML the
   document. Our engine: one authoritative document model; the render graph is
   a projection of it, never a second store.
2. **Every edit enters through one command API with success/failure semantics
   and built-in reverse.** Kdenlive's request*Action + lambda undo pairs and
   Shotcut's QUndoCommand-per-verb both show this; it is exactly the shape
   needed for claim C-003 (human and AI edits through one API).
3. **Track = ordered list of clips with in/out; effects attach to clips or
   tracks; transitions are objects between clips or track-level mixes.** All
   four OSS NLEs agree on this core; node graphs are used for the *effect/
   compositing* dimension (Olive, and as sub-model inside others).
4. **Rational/integer time everywhere in the model layer.** MLT positions are
   int32 in a profile with rational frame rate; Olive uses num/den rationals;
   OTIO uses value/rate pairs; FCPXML uses rational seconds. No mature NLE
   keeps floating-point seconds as the authoritative timeline time.
5. **Project format versioning must be designed-in from day one.** Olive's
   per-version serializers and Kdenlive's document-generation scheme (with its
   documented gen-1/gen-4 disasters) are the concrete evidence. See
   19_PROJECT_FORMATS.md.
6. **Engine choice shapes the app.** MLT-based editors inherited MLT's
   8-bit-YUV422 orientation, consumer-driven threading, and partial GPU story
   (see 07_MLT.md). The engine is a strategic commitment, not a swap-in.

## 9. Source list

- S-2b0 OpenTimelineIO docs (architecture tutorial, file format specification).
- S-2b1 OpenTimelineIO source headers (rationalTime.h, track.h, stack.h, README).
- S-2b2 MLT framework + MLT XML docs (mltframework.org/docs).
- S-2b3 MLT source headers (mlt_types.h, mlt_profile.h) and README (license).
- S-2b4 Kdenlive repo (README, dev-docs/architecture.md, dev-docs/mlt-intro.md,
  dev-docs/fileformat.md, src/timeline2/model/timelinemodel.hpp,
  src/doc/docundostack.hpp).
- S-2b5 Shotcut repo (README, src/models/multitrackmodel.h,
  src/commands/timelinecommands.h).
- S-2b6 Olive repo + core submodule (app/node/node.h, app/node/block/block.h,
  ext/core rational.h, app/node/project/serializer, app/undo).
- S-2b7 Flowblade repo README.
- S-2b8 Apple/vendor references (FCP trim docs; FCPXML rational-time quotes).
- S-2b9 Community/secondary evidence (Resolve SQLite forum, OpenCut IndexedDB
  issue, Shotcut VFR forum, OpenShot .osp JSON, Kdenlive/MLT render complaints).
