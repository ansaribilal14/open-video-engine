╔══════════════════════════════════════════════════════════════════════════════╗
║                                                                            ║
║                 PROJECT: ULTIMATE OPEN VIDEO ENGINE                        ║
║                                                                            ║
║             Z SUPER — MASTER RESEARCH → ARCHITECTURE → BUILD               ║
║                         EXECUTION DIRECTIVE                                ║
║                                                                            ║
╚══════════════════════════════════════════════════════════════════════════════╝


MISSION
=======

You are being assigned a long-term systems-engineering mission.

Your task is to research, understand, architect, engineer, test, benchmark,
and ultimately build an open-source video editing platform whose underlying
engine can become a reusable video/media infrastructure layer for applications
ranging from extremely simple mobile editing tools to complex professional
creative applications and AI-agent-controlled media systems.

This is NOT primarily a UI project.

This is NOT primarily a CapCut clone.

This is NOT primarily a Tauri application.

This is NOT primarily an Android application.

This is NOT primarily an AI wrapper around FFmpeg.

The real project is:

                    A UNIVERSAL VIDEO ENGINE

with:

                    EDITOR + MEDIA ENGINE + COMPOSITOR
                    + TIMELINE ENGINE + AUDIO ENGINE
                    + PROJECT ENGINE + RENDER ENGINE
                    + AI ENGINE + AGENT API
                    + PLUGIN SYSTEM + SCRIPTING SYSTEM
                    + HEADLESS AUTOMATION ENGINE

The eventual system must be capable of serving:

- simple mobile editors
- social-media editors
- browser editors
- desktop NLEs
- professional editing applications
- automated content pipelines
- AI video agents
- media-processing backends
- batch rendering systems
- research applications
- developer tools
- future applications that do not yet exist


======================================================================
                         ABSOLUTE PRINCIPLE
======================================================================

DO NOT START BY BUILDING THE APP.

DO NOT START BY GENERATING UI.

DO NOT START BY SELECTING A STACK.

DO NOT ASSUME THE USER-PROPOSED ARCHITECTURE IS CORRECT.

DO NOT ASSUME RUST IS CORRECT.

DO NOT ASSUME TAURI IS CORRECT.

DO NOT ASSUME KOTLIN IS CORRECT.

DO NOT ASSUME REACT IS CORRECT.

DO NOT ASSUME WGPU IS CORRECT.

DO NOT ASSUME FFMPEG IS CORRECT FOR EVERY LAYER.

DO NOT ASSUME WEBASSEMBLY IS THE CORRECT BROWSER STRATEGY.

DO NOT ASSUME A SINGLE RENDERING ENGINE CAN DO EVERYTHING.

DO NOT ASSUME EXISTING OPEN-SOURCE EDITORS HAVE SOLVED THE PROBLEM.

DO NOT BUILD FEATURES BECAUSE THEY SOUND IMPRESSIVE.

DO NOT COPY ARCHITECTURES WITHOUT UNDERSTANDING THEM.

DO NOT LET AN LLM GENERATE A LARGE CODEBASE BEFORE THE SYSTEM MODEL IS
UNDERSTOOD.

The operating principle is:

                    LEARN
                      ↓
                   VERIFY
                      ↓
                  COMPARE
                      ↓
                 EXPERIMENT
                      ↓
                 BENCHMARK
                      ↓
                 ARCHITECT
                      ↓
                 PROTOTYPE
                      ↓
                 IMPLEMENT
                      ↓
                   TEST
                      ↓
                 BENCHMARK
                      ↓
                   AUDIT
                      ↓
                    FIX
                      ↓
                   REPEAT


The main objective is NOT:

"build the application as quickly as possible."

The main objective is:

"become sufficiently knowledgeable about video systems engineering that
the resulting architecture is based on evidence rather than assumptions."


======================================================================
                         THE END STATE
======================================================================

The eventual platform should conceptually be capable of:

                 ┌───────────────────────────────┐
                 │       VIDEO ENGINE CORE       │
                 │                               │
                 │ Timeline                      │
                 │ Media                         │
                 │ Compositor                    │
                 │ Effects                       │
                 │ Keyframes                     │
                 │ Audio                         │
                 │ Color                         │
                 │ Captions                      │
                 │ Rendering                     │
                 │ Export                        │
                 │ Project                       │
                 │ Cache                         │
                 │ Undo/Redo                     │
                 └───────────────┬───────────────┘
                                 │
              ┌──────────────────┼──────────────────┐
              │                  │                  │
           ANDROID             WEB              DESKTOP
              │                  │                  │
           Kotlin             WASM              Tauri/
           Compose           WebGPU              Native
           Media APIs       WebCodecs           GPU APIs
              │                  │                  │
              └──────────────────┼──────────────────┘
                                 │
                         SHARED PROJECT MODEL
                                 │
                ┌────────────────┼────────────────┐
                │                │                │
               AI              MCP            SCRIPTING
                │                │                │
                └────────────────┼────────────────┘
                                 │
                           PLUGIN SYSTEM
                                 │
                        HEADLESS RENDERING
                                 │
                         BATCH AUTOMATION


But this diagram is ONLY an initial hypothesis.

You must determine the actual architecture through research.


======================================================================
                         YOUR FIRST JOB
======================================================================

Before implementing meaningful application functionality:

1. inspect the available project/repository state
2. inspect all provided project material
3. research the current ecosystem
4. research academic literature
5. research open-source implementations
6. research engineering talks
7. retrieve YouTube transcripts
8. study source code
9. compare competing architectures
10. identify unresolved engineering problems
11. perform experiments where necessary
12. produce an evidence-backed architecture
13. only then produce the implementation plan
14. only then begin implementation


======================================================================
                    DO NOT LOSE THE RESEARCH
======================================================================

Create persistent research state.

Use a dedicated research directory such as:

research/
├── sources/
├── papers/
├── github/
├── youtube/
├── transcripts/
├── conferences/
├── experiments/
├── benchmarks/
├── architecture/
├── adr/
├── comparisons/
├── licenses/
├── risks/
├── unresolved/
└── synthesis/

Maintain machine-readable records where useful.

Every important source must be recoverable.

Every major architectural decision must be traceable back to evidence.


======================================================================
                     SOURCE OF TRUTH SYSTEM
======================================================================

Create a source ledger.

Each source must contain:

SOURCE_ID
TITLE
TYPE
AUTHOR_OR_ORGANIZATION
DATE
URL
REPOSITORY
VERSION_OR_COMMIT_WHERE_RELEVANT
LICENSE
SOURCE_QUALITY
PRIMARY_OR_SECONDARY
TOPICS
RELEVANCE
KEY_FINDINGS
ARCHITECTURAL_LESSONS
PERFORMANCE_FINDINGS
LIMITATIONS
CONTRADICTORY_EVIDENCE
POTENTIAL_APPLICATION_TO_PROJECT


SOURCE QUALITY ORDER:

1. source code
2. official documentation/specification
3. original academic paper
4. maintainer/author technical discussion
5. conference presentation
6. engineering blog
7. long-form technical video
8. technical tutorial
9. community discussion
10. generic article
11. AI-generated material

Lower-quality material may reveal problems but must not become the sole
basis for major architecture decisions.


======================================================================
                    CLAIM / EVIDENCE LEDGER
======================================================================

For every important technical belief create:

CLAIM
EVIDENCE
SOURCE
COUNTER-EVIDENCE
CONFIDENCE
EXPERIMENT
RESULT
DECISION

Example:

CLAIM:
WebGPU can provide the common browser GPU rendering abstraction.

EVIDENCE:
...

COUNTER-EVIDENCE:
...

EXPERIMENT:
...

RESULT:
...

DECISION:
...


This prevents architecture from being constructed from intuition.


======================================================================
                         MULTI-AGENT MODE
======================================================================

Use specialized agents aggressively.

Do NOT ask one generic agent to research everything.

Create specialized research and engineering roles.

At minimum:

01. PRINCIPAL SYSTEM ARCHITECT
02. VIDEO ENGINE ARCHITECT
03. NLE ARCHITECT
04. MEDIA PIPELINE ENGINEER
05. FFMPEG ENGINEER
06. GSTREAMER ENGINEER
07. MLT/NLE RESEARCHER
08. GPU ARCHITECT
09. WGPU ENGINEER
10. WEBGPU ENGINEER
11. WEBCODECS ENGINEER
12. WASM ENGINEER
13. RUST SYSTEMS ENGINEER
14. ANDROID/KOTLIN ENGINEER
15. MEDIA3 ENGINEER
16. MEDIACODEC ENGINEER
17. JNI/UNIFFI ENGINEER
18. TAURI ENGINEER
19. WEB APPLICATION ENGINEER
20. GRAPHICS ENGINEER
21. COMPOSITOR ENGINEER
22. COLOR SCIENCE ENGINEER
23. AUDIO ENGINEER
24. CAPTION/TRANSCRIPTION ENGINEER
25. COMPUTER VISION ENGINEER
26. AI VIDEO RESEARCHER
27. AI AGENT ARCHITECT
28. MCP ENGINEER
29. PLUGIN SYSTEM ARCHITECT
30. SCRIPTING ENGINEER
31. PROJECT FORMAT ARCHITECT
32. STORAGE/DATABASE ENGINEER
33. PERFORMANCE ENGINEER
34. MOBILE PERFORMANCE ENGINEER
35. BROWSER PERFORMANCE ENGINEER
36. SECURITY ENGINEER
37. SANDBOXING ENGINEER
38. LICENSE/LEGAL-COMPATIBILITY RESEARCHER
39. OPEN-SOURCE REPOSITORY RESEARCHER
40. ACADEMIC PAPER RESEARCHER
41. YOUTUBE RESEARCHER
42. TRANSCRIPT ANALYST
43. UX/NLE INTERACTION RESEARCHER
44. BENCHMARK SCIENTIST
45. TEST ENGINEER
46. RED TEAM ENGINEER
47. RELEASE ENGINEER
48. DOCUMENTATION ENGINEER
49. COMPETITIVE ARCHITECTURE ANALYST
50. FINAL INDEPENDENT AUDITOR


Agents must challenge each other.

If Agent A recommends an architecture:

Agent B must attempt to break it.

If Agent B rejects it:

Agent C must investigate the evidence.

The Principal Architect resolves disagreements using evidence,
not popularity.


======================================================================
                    RESEARCH ORCHESTRATION
======================================================================

Divide research into tracks.

TRACK A
VIDEO EDITOR ARCHITECTURE

TRACK B
MEDIA/CODEC ENGINEERING

TRACK C
TIMELINE/NLE SYSTEMS

TRACK D
GPU/COMPOSITOR SYSTEMS

TRACK E
WEB/WASM

TRACK F
ANDROID/KOTLIN

TRACK G
DESKTOP/TAURI

TRACK H
AUDIO

TRACK I
COLOR

TRACK J
CAPTIONS

TRACK K
COMPUTER VISION

TRACK L
AI VIDEO EDITING

TRACK M
AI AGENTS

TRACK N
MCP

TRACK O
PLUGIN ARCHITECTURE

TRACK P
PROJECT FORMAT

TRACK Q
STORAGE

TRACK R
HEADLESS RENDERING

TRACK S
PERFORMANCE

TRACK T
SECURITY

TRACK U
UX

TRACK V
TESTING

TRACK W
LICENSING

TRACK X
OPEN-SOURCE ECOSYSTEM

TRACK Y
ACADEMIC RESEARCH

TRACK Z
ENGINEERING VIDEOS


Each track must produce findings.

Then perform cross-track synthesis.


======================================================================
                  OPEN-SOURCE REPOSITORY RESEARCH
======================================================================

Study complete repositories where practical.

Do NOT limit yourself to README files.

For each important repository inspect:

- architecture
- source tree
- core abstractions
- media pipeline
- rendering pipeline
- timeline
- state management
- command architecture
- project format
- caching
- proxies
- GPU
- hardware acceleration
- plugins
- scripting
- AI
- MCP
- testing
- benchmarks
- CI
- build system
- packaging
- licensing
- unresolved issues
- architectural compromises


HIGH-PRIORITY EDITORS:

OpenCut
OpenCut Classic
Clypra
Cutlass
Kerf
Velocut
OpenReelio
OpenTake
Frontstage
Katana
Kdenlive
Shotcut
Olive
Flowblade
LosslessCut


Do not blindly copy any of them.

Extract engineering principles.

For each project determine:

WHAT THEY GOT RIGHT
WHAT THEY GOT WRONG
WHAT THEY SIMPLIFIED
WHAT THEY OPTIMIZED FOR
WHAT THEIR ARCHITECTURE CANNOT DO
WHAT WE CAN REUSE
WHAT WE SHOULD NOT COPY


======================================================================
                         OPENCUT STUDY
======================================================================

Study OpenCut particularly deeply.

Investigate:

- current architecture
- rewrite
- Rust core
- Editor API
- plugin architecture
- MCP
- scripting
- headless rendering
- desktop
- mobile
- browser
- project format
- command architecture
- rendering strategy

Do not assume its direction is automatically correct.

Use it as an important architectural reference.


======================================================================
                         CLYPRA STUDY
======================================================================

Study:

- Rust
- Tauri
- React
- FFmpeg
- hardware acceleration
- decoding
- timeline
- frame-accurate editing
- caching
- rendering

Determine which architectural ideas generalize beyond desktop.


======================================================================
                         CUTLASS STUDY
======================================================================

Investigate:

- Rust engine
- timeline
- command architecture
- undo/redo
- AI editing
- natural-language editing
- Android strategy
- native media systems

Critical question:

Can AI and human editing use the SAME command system?

If yes, understand how.

If no, understand why.


======================================================================
                           KERF STUDY
======================================================================

Study deeply:

- MCP
- agent-controlled editing
- project state
- observation
- preview
- staged edits
- rollback
- non-destructive editing
- command architecture
- EDL
- FFmpeg filter generation

Determine how AI can become a first-class operator without bypassing the
editor's safety model.


======================================================================
                         VELOCUT STUDY
======================================================================

Study:

- Rust
- WASM
- WebGPU
- WebCodecs
- browser rendering
- project runtime
- JSON command protocol
- SDK
- MCP
- browser storage
- undo
- local-first architecture

Pay special attention to the browser architecture.


======================================================================
                       OPENREELIO STUDY
======================================================================

Study:

- Tauri 2
- Rust
- React
- FFmpeg
- SQLite
- WASM plugins
- AI
- event-sourced commands
- captions
- transitions
- audio
- GPU export
- QC
- proxies
- caching


======================================================================
                         OPENTAKE STUDY
======================================================================

Study:

- Rust
- Tauri
- React
- FFmpeg
- wgpu
- Whisper
- MCP
- semantic search
- workflow plugins
- AI context
- GPU compositor
- frame publication
- caching


======================================================================
                         FRONTSTAGE STUDY
======================================================================

Study:

- headless core
- timeline model
- commands
- undo
- ripple editing
- color
- captions
- WebGPU
- WebCodecs
- WebAudio
- MP4 export
- AI
- MCP


======================================================================
                  MATURE NLE RESEARCH
======================================================================

Study deeply:

Kdenlive
Shotcut
Olive
Flowblade

Also research professional systems conceptually:

DaVinci Resolve
Adobe Premiere Pro
Final Cut Pro
Avid Media Composer

The goal is NOT to copy proprietary systems.

The goal is to understand mature NLE engineering.

Study:

- timeline
- sequences
- source ranges
- tracks
- nested sequences
- compound clips
- adjustment layers
- transitions
- effects
- keyframes
- masks
- proxies
- media management
- render pipelines
- color
- audio
- caching
- undo/redo
- project serialization


======================================================================
                        MEDIA ENGINE RESEARCH
======================================================================

MASTER:

FFmpeg
GStreamer
MLT
libplacebo
mpv
OBS

Study:

- demuxing
- decoding
- encoding
- filtering
- timestamps
- PTS
- DTS
- B-frames
- GOP
- keyframes
- seeking
- VFR
- CFR
- synchronization
- stream copy
- transcoding
- remuxing
- scaling
- pixel formats
- color conversion
- hardware acceleration


======================================================================
                     HARDWARE ACCELERATION
======================================================================

Study:

NVIDIA
NVDEC
NVENC

Intel
Quick Sync
oneVPL

AMD
AMF
VAAPI

Apple
VideoToolbox
Metal

Linux
VAAPI
Vulkan

Windows
Media Foundation
DirectX

Android
MediaCodec
Media3

Determine:

- decode paths
- encode paths
- availability
- fallback behavior
- performance
- device detection
- error handling
- quality differences
- platform abstraction


======================================================================
                       GPU RESEARCH
======================================================================

Study:

wgpu
WebGPU
Vulkan
Metal
DirectX
OpenGL
OpenGL ES

Research:

- compositor design
- textures
- shaders
- compute
- render passes
- frame synchronization
- GPU/CPU transfers
- video textures
- zero-copy opportunities
- color conversion
- scaling
- effects
- masks
- blur
- transitions
- particles
- LUTs


Do not assume the entire media pipeline should be GPU-native.

Determine where CPU and GPU boundaries should exist.


======================================================================
                         BROWSER RESEARCH
======================================================================

Study:

WebCodecs
WebGPU
WebAssembly
WASI
OPFS
IndexedDB
File System Access API
SharedArrayBuffer
Web Workers
AudioWorklet

Research:

- browser media decoding
- encoding
- seeking
- GPU access
- memory limits
- storage
- threading
- SIMD
- mobile browser limitations
- Safari
- Firefox
- Chromium

Determine exactly which features can be shared with desktop/mobile
and which cannot.


======================================================================
                        ANDROID RESEARCH
======================================================================

Study deeply:

Kotlin
Jetpack Compose
Media3
ExoPlayer
MediaCodec
MediaExtractor
MediaMuxer
Android NDK
JNI
UniFFI
Vulkan
OpenGL ES
Android GPU systems

Research:

- hardware decoding
- hardware encoding
- preview
- Surface
- SurfaceTexture
- ImageReader
- native memory
- ARM64
- thermal throttling
- battery
- RAM
- storage
- background execution
- device fragmentation
- GPU differences

Do not treat Android as a small desktop browser.


======================================================================
                     RUST AND ANDROID BRIDGE
======================================================================

Research:

- JNI
- UniFFI
- cargo-ndk
- Kotlin/Rust bindings
- ABI packaging
- native library distribution
- callbacks
- async communication
- threading
- memory ownership
- error propagation

Determine the correct boundary between:

Kotlin
and
Rust.


======================================================================
                          TAURI RESEARCH
======================================================================

Study Tauri 2 deeply.

Compare against:

Electron
Wails
Flutter
Qt
native Rust GUI

Study:

- IPC
- commands
- events
- state
- filesystem
- shell
- permissions
- security
- sidecars
- FFmpeg distribution
- GPU
- WebView
- packaging
- updates
- crash recovery


======================================================================
                     TIMELINE ENGINE RESEARCH
======================================================================

This is one of the most important subsystems.

Research:

- timeline data structures
- clips
- tracks
- sequences
- source ranges
- transitions
- effects
- keyframes
- markers
- snapping
- ripple
- roll
- slip
- slide
- trim
- split
- nesting
- compound clips
- adjustment layers
- linked media
- time remapping
- speed curves

Determine the mathematically correct timeline representation.


======================================================================
                       TIME REPRESENTATION
======================================================================

Research deeply:

- frame numbers
- rational frame rates
- timecode
- milliseconds
- microseconds
- nanoseconds
- source timestamps
- project timestamps
- PTS
- DTS

DO NOT use floating-point seconds as the primary authoritative timeline
representation without proving that it is safe.

Study professional approaches.

Determine:

- precision
- overflow
- rounding
- frame accuracy
- VFR
- mixed frame rates
- audio sample alignment


======================================================================
                       PROJECT FORMAT
======================================================================

Study:

FCPXML
EDL
AAF
OTIO
Kdenlive format
OpenCut formats
other open-source project formats

Research:

JSON
SQLite
binary formats
event sourcing
command logs
hybrid packages

The project format must support:

- portability
- versioning
- migrations
- autosave
- crash recovery
- undo/redo
- asset references
- proxies
- cache references
- plugins
- effects
- keyframes
- metadata
- AI operations
- future schema evolution

Do not choose the format until research is complete.


======================================================================
                         COMMAND MODEL
======================================================================

Investigate whether every edit should become a command.

Example:

AddClip
MoveClip
TrimClip
SplitClip
DeleteClip
AddTrack
DeleteTrack
SetKeyframe
ApplyEffect
RemoveEffect
AddTransition
ChangeSpeed
ChangeVolume
AddCaption
GenerateCaptions
CreateSequence

Research:

Command Pattern
Event Sourcing
Immutable State
Transactions
Revision IDs
Snapshots
Undo/Redo
Diffs
Optimistic Concurrency

The goal is eventually:

HUMAN
  ↓
COMMAND API
  ↓
EDITOR ENGINE
  ↑
COMMAND API
  ↑
AI AGENT


AI must not have a secret alternate editing system.


======================================================================
                       AI AGENT ARCHITECTURE
======================================================================

Research how an AI agent can safely operate the editor.

Desired conceptual loop:

OBSERVE
 ↓
UNDERSTAND
 ↓
PLAN
 ↓
GENERATE EDIT PLAN
 ↓
VALIDATE
 ↓
PREVIEW
 ↓
ASK FOR APPROVAL WHEN NECESSARY
 ↓
APPLY COMMANDS
 ↓
RENDER
 ↓
VERIFY
 ↓
REPORT


The AI should be able to perform tasks such as:

"Create a 60-second highlight."

"Remove silent sections."

"Create a vertical version."

"Add captions."

"Sync cuts to the beat."

"Find the strongest moments."

"Create three social-media versions."

"Make this section faster."

"Replace background music."

"Find every mention of X."

"Create a montage from these clips."

But every operation must resolve into deterministic editor operations.


======================================================================
                             MCP
======================================================================

Research Model Context Protocol deeply.

Potential tools:

create_project
open_project
save_project
import_media
inspect_media
list_assets
inspect_clip
search_media
find_scene
detect_scenes
detect_silence
detect_beats
add_clip
remove_clip
split_clip
trim_clip
move_clip
duplicate_clip
add_track
add_text
add_caption
add_audio
add_effect
remove_effect
add_transition
set_keyframe
create_sequence
create_compound_clip
render_preview
export_video
inspect_timeline
get_frame
get_waveform
get_project_state
undo
redo
create_snapshot
compare_revisions


But do NOT blindly expose all internal functions.

Research:

- capability-based permissions
- tool schemas
- transactions
- revision control
- approval
- rollback
- resource limits
- sandboxing


======================================================================
                         HEADLESS ENGINE
======================================================================

The engine must eventually work without a GUI.

Example:

video-engine create-project
video-engine import
video-engine edit
video-engine render
video-engine export


Example conceptual workflow:

project.json
     ↓
headless engine
     ↓
timeline
     ↓
compositor
     ↓
encoder
     ↓
output.mp4


This enables:

- CI
- batch rendering
- AI automation
- server rendering
- desktop automation
- mobile background jobs
- developer tooling


======================================================================
                         SCRIPTING
======================================================================

Research scripting systems.

Potential APIs:

JavaScript
TypeScript
Lua
WASM
Rust SDK
Python
JSON command language

The scripting API must operate through the same public editor API.

Example:

const project = editor.open("project");

for (const clip of project.timeline.clips()) {
    if (clip.duration < 1) {
        clip.remove();
    }
}

await project.captions.generate();

await project.export({
    width: 1080,
    height: 1920
});


Do not implement scripting until the underlying API is stable.


======================================================================
                         PLUGIN SYSTEM
======================================================================

Research:

WASM plugins
WASI
Wasmtime
Rust plugins
JavaScript plugins
sandboxing
permission models
plugin manifests
versioning
dependency resolution

Potential plugin categories:

effects
transitions
generators
audio processors
importers
exporters
AI providers
panels
tools
scripts
templates


Determine what API must remain stable.

Do not expose unstable internals unnecessarily.


======================================================================
                           AUDIO
======================================================================

Research:

PCM
sample rates
bit depth
channels
mixing
gain
normalization
EQ
compression
noise reduction
voice isolation
reverb
delay
time stretching
pitch shifting
waveforms
spectrograms
beat detection

Study:

FFmpeg audio
GStreamer
Web Audio
AudioWorklet
CPAL
JUCE


======================================================================
                           COLOR
======================================================================

Research:

RGB
YUV
YCbCr
Rec.709
Rec.2020
sRGB
P3
HDR10
HLG
PQ
gamma
transfer functions
linear light
tone mapping
LUTs
ACES
OpenColorIO
bit depth
chroma subsampling

Determine where color processing belongs.


======================================================================
                         CAPTIONS
======================================================================

Research:

Whisper
whisper.cpp
faster-whisper
WhisperX
Vosk

Study:

- word timestamps
- speaker diarization
- SRT
- VTT
- ASS
- karaoke timing
- animated captions
- caption layout
- caption rendering


======================================================================
                    COMPUTER VISION
======================================================================

Research:

OpenCV
MediaPipe
ONNX Runtime
PyTorch
TensorFlow

Tasks:

- face detection
- face tracking
- object detection
- object tracking
- segmentation
- background removal
- pose detection
- OCR
- shot detection
- scene detection
- optical flow
- motion estimation
- semantic search
- image/video embeddings


======================================================================
                     AI VIDEO RESEARCH
======================================================================

Research the entire field.

Start from:

wentianli/awesome-video-editing

Then branch outward.

Research:

- automatic video editing
- text-guided editing
- instruction-driven editing
- cinematic compilation
- highlight generation
- video summarization
- scene detection
- shot detection
- video retrieval
- multimodal retrieval
- video-language models
- video agents
- LLM video editing
- generative video editing
- video inpainting
- object removal
- background replacement
- automatic reframing
- beat synchronization
- temporal consistency
- multi-agent editing


Study relevant papers discovered through:

arXiv
OpenReview
CVF
ACL
Semantic Scholar
Papers With Code
institutional repositories


For every important paper record:

- problem
- contribution
- architecture
- model
- dataset
- training
- inference
- compute
- latency
- benchmark
- limitations
- code
- license
- reproducibility
- relevance


Important starting research:

EditDuet
CineAgents
Generative Timelines for Instructed Visual Assembly
JoyAI-Video-Edit

Do not assume these are directly production-ready.


======================================================================
                    ACADEMIC RESEARCH DOMAINS
======================================================================

Research adjacent fields.

VIDEO EDITING

VIDEO UNDERSTANDING

COMPUTER VISION

COMPUTER GRAPHICS

GPU COMPUTING

MULTIMEDIA SYSTEMS

VIDEO CODECS

HCI

INTERACTION DESIGN

MULTIMODAL AI

AGENT SYSTEMS

PROGRAM SYNTHESIS

HUMAN-COMPUTER COLLABORATION

REAL-TIME SYSTEMS

DATABASE SYSTEMS

DISTRIBUTED RENDERING

EDGE COMPUTING

MOBILE COMPUTING


======================================================================
                        YOUTUBE RESEARCH
======================================================================

YouTube research is mandatory.

But DO NOT primarily study:

"AI builds a video editor in 10 minutes"

"Build an app with ChatGPT"

"Vibe coding"

"Clone CapCut with AI"

Those can provide ideas but are NOT authoritative engineering evidence.


PRIORITIZE:

- engineers building systems manually
- maintainers explaining implementations
- conference talks
- source-code walkthroughs
- graphics programming
- FFmpeg engineering
- Rust systems engineering
- GPU programming
- WebGPU
- WebCodecs
- Android media
- Kotlin
- Tauri
- WASM
- NLE engineering
- codec engineering
- performance profiling
- debugging
- architecture


Search topics including:

"build video editor from scratch"

"video editor from scratch"

"NLE from scratch"

"timeline editor from scratch"

"Rust video editor"

"Rust video engine"

"Rust FFmpeg"

"Rust compositor"

"Rust GPU video"

"wgpu compositor"

"WebGPU video editor"

"WebCodecs video editor"

"WASM video editor"

"Tauri video editor"

"Tauri FFmpeg"

"Kotlin video editor"

"Android MediaCodec video"

"Android Media3 video"

"Jetpack Compose timeline"

"Rust Android JNI"

"Rust Kotlin UniFFI"

"video codec engineering"

"GPU compositor"

"real time video rendering"

"video editor architecture"


======================================================================
                      YOUTUBE TRANSCRIPTS
======================================================================

For each important video:

retrieve:

title
channel
date
duration
description
chapters
transcript
GitHub links
technologies
speaker
engineering level


Then analyze the transcript.

Extract:

- architecture
- implementation decisions
- trade-offs
- bugs
- debugging
- performance
- failed approaches
- lessons
- practical implementation details


Classify:

A = engineer manually builds system
B = engineer explains existing system
C = strong technical tutorial
D = AI-assisted but technically informed
E = mostly AI-generated
F = superficial


For foundational learning:

A > B > C > D > E > F


If transcript unavailable:

TRANSCRIPT_UNAVAILABLE

Never fabricate one.


======================================================================
                       CONFERENCE RESEARCH
======================================================================

Research:

RustConf
KotlinConf
Google I/O
Android Developers
Chrome Developers
WWDC
FOSDEM
Demuxed
SIGGRAPH
Eurographics
GStreamer Conference
WASM I/O
Tauri
LLVM


Search topics such as:

video codec engineering
GPU rendering
WebGPU
WebCodecs
Android MediaCodec
Media3
Rust graphics
Rust media
video pipelines
NLE architecture


======================================================================
                       PROFESSIONAL THEORY
======================================================================

Master:

NLE
EDL
AAF
FCPXML
OTIO
timeline
sequence
track
clip
source range
nested sequence
compound clip
adjustment layer
transition
effect
keyframe
Bezier
easing
ripple
roll
slip
slide
snap
marker
timecode
VFR
CFR
audio synchronization


Study OpenTimelineIO deeply.

Repository:

AcademySoftwareFoundation/OpenTimelineIO


Determine whether OTIO should influence the internal representation.


======================================================================
                       PROJECT FORMAT
======================================================================

Research candidate architectures:

A. JSON
B. SQLite
C. JSON + asset directory
D. SQLite + asset directory
E. event-sourced project
F. command log + snapshots
G. hybrid package


Benchmark:

- loading
- saving
- autosave
- migration
- crash recovery
- undo
- redo
- branching
- versioning
- portability
- large projects


Do not select based on convenience.


======================================================================
                         STORAGE
======================================================================

Study:

SQLite
IndexedDB
OPFS
File System Access API
SQLite WASM
DuckDB
libSQL
RocksDB where relevant


Determine:

- project state
- metadata
- cache
- waveform
- thumbnails
- proxies
- undo history
- AI analysis
- asset metadata


======================================================================
                      PERFORMANCE RESEARCH
======================================================================

Study:

frame pacing
dropped frames
decode latency
encode latency
render latency
GPU utilization
CPU utilization
memory
cache
proxy generation
timeline virtualization
zero-copy
DMA
multithreading
worker pools
scheduling


Tools:

Rust:
perf
Tracy
cargo-flamegraph
Instruments
Windows Performance Analyzer


Android:
Perfetto
Android Studio Profiler
GPU Inspector


Web:
Chrome Performance
Chrome Memory
WebGPU tools


======================================================================
                    CROSS-PLATFORM REALITY
======================================================================

Create a capability matrix:

                    ANDROID | WEB | DESKTOP

Media decode
Media encode
GPU
Effects
Audio
File access
Storage
Background processing
AI inference
Plugins
Scripting
Headless
MCP
Rendering
Hardware acceleration


Every capability must be classified:

SHARED
PLATFORM-ADAPTER
PLATFORM-SPECIFIC
UNAVAILABLE
EMULATED
OPTIONAL


======================================================================
                     MOBILE ENGINEERING
======================================================================

Research low-end, mid-range and high-end Android.

Measure:

RAM
CPU
GPU
thermal behavior
battery
storage
hardware codecs
decode
encode
preview
export


The editor must degrade gracefully.

Example:

HIGH-END
→ real-time effects

MID-RANGE
→ selected GPU effects

LOW-END
→ proxies + reduced preview quality


======================================================================
                     BROWSER ENGINEERING
======================================================================

Determine behavior when:

WebGPU unavailable
WebCodecs unavailable
WASM threads unavailable
storage limited
memory limited
codec unavailable
Safari differs
Firefox differs
mobile browser differs


There must always be an explicit fallback strategy.


======================================================================
                       SECURITY RESEARCH
======================================================================

Threat model:

malicious media
malicious project
malicious plugin
malicious MCP tool
prompt injection
metadata injection
path traversal
shell execution
network access
resource exhaustion
FFmpeg vulnerabilities
WASM plugins
untrusted scripts
AI-generated commands


Design capability-based security.

AI should NOT automatically receive unrestricted:

filesystem
shell
network
plugin
project
render
credential


permissions.


======================================================================
                       LICENSE RESEARCH
======================================================================

Audit:

MIT
Apache-2.0
BSD
LGPL
GPL
AGPL
MPL
model licenses
dataset licenses
font licenses
media licenses


For every dependency determine:

- redistribution
- static linking
- dynamic linking
- attribution
- source obligations
- commercial compatibility
- patent terms
- model restrictions


Never copy code without understanding its license.


======================================================================
                     BENCHMARKING PHILOSOPHY
======================================================================

Do not say:

"fast"

"efficient"

"lightweight"

"professional"

"state of the art"

without measurements.


Benchmark:

IMPORT
DECODE
SEEK
PREVIEW
TIMELINE
EFFECT
COMPOSITE
AUDIO
EXPORT
SAVE
LOAD
AUTOSAVE
UNDO
REDO
PROXY
THUMBNAIL
WAVEFORM
AI ANALYSIS


Across:

low-end Android
mid-range Android
desktop CPU
desktop GPU
browser


======================================================================
                         EXPERIMENT ENGINE
======================================================================

When architecture is uncertain:

DO NOT ARGUE FOREVER.

BUILD A SMALL EXPERIMENT.

Examples:

- wgpu video texture experiment
- WebCodecs decoding experiment
- WASM FFmpeg experiment
- Rust/Android bridge experiment
- Tauri IPC experiment
- timeline benchmark
- project serialization benchmark
- GPU compositor benchmark
- MediaCodec benchmark
- browser storage benchmark


Each experiment records:

QUESTION
HYPOTHESIS
IMPLEMENTATION
HARDWARE
RESULT
LIMITATIONS
DECISION


======================================================================
                       ARCHITECTURE SYNTHESIS
======================================================================

After research, create at least 3 competing architectures.

For example:

ARCHITECTURE A
Rust-first unified engine

ARCHITECTURE B
Native platform media engines + shared timeline

ARCHITECTURE C
Hybrid Rust core + platform media adapters


Then compare:

performance
portability
complexity
correctness
hardware access
browser support
Android support
desktop support
AI integration
plugin system
maintenance
testing
licensing
developer experience


Do not select a winner merely because it sounds elegant.

Use evidence.


======================================================================
                         ARCHITECTURE GATES
======================================================================

Before final architecture:

GATE 1
Media pipeline understood.

GATE 2
Timeline model understood.

GATE 3
Rendering model understood.

GATE 4
GPU strategy understood.

GATE 5
Android strategy understood.

GATE 6
Browser strategy understood.

GATE 7
Desktop strategy understood.

GATE 8
Project format understood.

GATE 9
AI command architecture understood.

GATE 10
Plugin boundary understood.

GATE 11
Performance risks understood.

GATE 12
Security model understood.

GATE 13
Licensing understood.

GATE 14
Testing strategy understood.


======================================================================
                       ARCHITECTURE DECISION RECORDS
======================================================================

Create ADRs.

At minimum:

ADR-001 Core language
ADR-002 Core architecture
ADR-003 Media backend
ADR-004 Decode abstraction
ADR-005 Encode abstraction
ADR-006 Timeline representation
ADR-007 Time representation
ADR-008 Project format
ADR-009 Undo/redo
ADR-010 Command system
ADR-011 Event sourcing
ADR-012 GPU architecture
ADR-013 Browser architecture
ADR-014 Android architecture
ADR-015 Desktop architecture
ADR-016 WebAssembly
ADR-017 WebCodecs
ADR-018 WebGPU
ADR-019 Tauri
ADR-020 Media3
ADR-021 MediaCodec
ADR-022 Audio architecture
ADR-023 Color architecture
ADR-024 Caption architecture
ADR-025 Plugin architecture
ADR-026 Scripting
ADR-027 MCP
ADR-028 AI architecture
ADR-029 Headless rendering
ADR-030 Storage
ADR-031 Cache
ADR-032 Proxy system
ADR-033 Security
ADR-034 Licensing
ADR-035 Testing
ADR-036 Performance


Every ADR contains:

CONTEXT
OPTIONS
EVIDENCE
ADVANTAGES
DISADVANTAGES
PERFORMANCE
PORTABILITY
COMPLEXITY
SECURITY
LICENSE
MAINTENANCE
DECISION
REJECTED ALTERNATIVES
CONFIDENCE


======================================================================
                     FINAL RESEARCH DOCUMENTATION
======================================================================

Create:

docs/research/

01_RESEARCH_INDEX.md
02_EXISTING_EDITORS.md
03_NLE_ARCHITECTURE.md
04_VIDEO_ENGINEERING.md
05_FFMPEG.md
06_GSTREAMER.md
07_MLT.md
08_GPU_ARCHITECTURE.md
09_WGPU.md
10_WEBGPU.md
11_WEBCODECS.md
12_WASM.md
13_RUST.md
14_ANDROID_KOTLIN.md
15_MEDIA3.md
16_MEDIACODEC.md
17_TAURI.md
18_WEB_ARCHITECTURE.md
19_PROJECT_FORMATS.md
20_TIMELINE.md
21_AUDIO.md
22_COLOR.md
23_CAPTIONS.md
24_COMPUTER_VISION.md
25_AI_VIDEO.md
26_AI_AGENTS.md
27_MCP.md
28_PLUGINS.md
29_SCRIPTING.md
30_HEADLESS.md
31_STORAGE.md
32_PERFORMANCE.md
33_SECURITY.md
34_UX.md
35_LICENSES.md
36_YOUTUBE.md
37_TRANSCRIPTS.md
38_PAPERS.md
39_GITHUB_RESEARCH.md
40_ARCHITECTURE_COMPARISON.md
41_EXPERIMENTS.md
42_RISKS.md
43_OPEN_QUESTIONS.md
44_ARCHITECTURE_WHITEPAPER.md


======================================================================
                    KNOWLEDGE SYNTHESIS
======================================================================

At the end of the research phase produce:

ULTIMATE_VIDEO_ENGINE_KNOWLEDGE_BASE.md


It must explain:

WHAT A VIDEO ENGINE ACTUALLY IS

HOW MEDIA FLOWS THROUGH IT

HOW TIMELINE MATHEMATICS WORKS

HOW DECODING WORKS

HOW SEEKING WORKS

HOW COMPOSITING WORKS

HOW EFFECTS WORK

HOW GPU ACCELERATION WORKS

HOW AUDIO WORKS

HOW COLOR WORKS

HOW PROJECTS ARE REPRESENTED

HOW UNDO WORKS

HOW RENDERING WORKS

HOW HARDWARE ENCODING WORKS

HOW BROWSER VIDEO EDITING WORKS

HOW ANDROID VIDEO EDITING WORKS

HOW DESKTOP VIDEO EDITING WORKS

HOW AI CAN OPERATE THE ENGINE

HOW MCP CAN CONTROL IT

HOW PLUGINS SHOULD WORK

HOW HEADLESS RENDERING SHOULD WORK

HOW THE SYSTEM SHOULD SCALE


This should become the project's foundational engineering knowledge document.


======================================================================
                       ONLY NOW: ARCHITECTURE
======================================================================

Once the research is complete:

Design the actual system.

Produce:

ULTIMATE_VIDEO_ENGINE_ARCHITECTURE.md


It must contain:

1. system overview
2. module architecture
3. dependency graph
4. media architecture
5. timeline architecture
6. project architecture
7. compositor
8. GPU
9. audio
10. color
11. captions
12. AI
13. MCP
14. plugins
15. scripting
16. headless
17. Android
18. web
19. desktop
20. storage
21. cache
22. proxy
23. security
24. testing
25. benchmarks
26. build system
27. release system


======================================================================
                     THEN CREATE THE PLAN
======================================================================

The implementation plan must be generated FROM THE RESEARCH.

Not before it.

Divide it into:

PHASE 0
Research infrastructure

PHASE 1
Core media abstraction

PHASE 2
Timeline engine

PHASE 3
Project system

PHASE 4
Decode/playback

PHASE 5
Compositor

PHASE 6
Rendering

PHASE 7
Audio

PHASE 8
Effects/keyframes

PHASE 9
Desktop

PHASE 10
Android

PHASE 11
Browser

PHASE 12
Project portability

PHASE 13
Plugins

PHASE 14
Scripting

PHASE 15
Headless

PHASE 16
AI

PHASE 17
MCP

PHASE 18
Computer vision

PHASE 19
Performance

PHASE 20
Security

PHASE 21
Cross-platform verification

PHASE 22
Production hardening


You may change the sequence if dependency analysis proves another ordering
superior.


======================================================================
                     IMPLEMENTATION PRINCIPLE
======================================================================

When implementation finally begins:

DO NOT build 300 features.

First prove the engine.

The first vertical slice should prove:

IMPORT
  ↓
PROBE
  ↓
PROJECT
  ↓
TIMELINE
  ↓
CLIP
  ↓
TRIM
  ↓
PREVIEW
  ↓
SAVE
  ↓
REOPEN
  ↓
EXPORT


Then prove:

MULTITRACK
→ AUDIO
→ TEXT
→ KEYFRAMES
→ EFFECTS
→ TRANSITIONS
→ COMPOSITING


Then:

CAPTIONS
→ AI
→ MCP
→ SCRIPTING
→ PLUGINS
→ HEADLESS


======================================================================
                  HUMAN AND AI MUST SHARE THE ENGINE
======================================================================

This is one of the most important principles.

Do NOT create:

Human Editor
+
Separate AI Video Generator


Instead:

                    EDITOR ENGINE
                         ↑
              ┌──────────┴──────────┐
              │                     │
           HUMAN                  AI
              │                     │
              └──────── COMMAND API ┘


The AI must manipulate the same project model,
same timeline,
same commands,
same validation,
same undo system,
same renderer,
same export system.


======================================================================
                    NON-DESTRUCTIVE PRINCIPLE
======================================================================

Source media must remain untouched.

Editing should operate on:

REFERENCES
+
RANGES
+
TRANSFORMS
+
EFFECTS
+
KEYFRAMES
+
COMMANDS


The engine should render derived output.


======================================================================
                     SAFETY PRINCIPLE
======================================================================

AI edits should support:

snapshot
→ proposed changes
→ preview
→ diff
→ validation
→ apply
→ undo


Potentially:

transaction
→ commit
or
→ rollback


Never allow an AI agent to silently destroy a project.


======================================================================
                     UNIVERSAL BACKEND PRINCIPLE
======================================================================

The editor must expose a clean engine API.

A simple application should be able to use:

create project
import media
trim
add text
export


A complex application should be able to use:

timeline graph
render graph
custom effects
plugins
AI
MCP
scripting
headless rendering
batch processing


Do not force simple consumers to understand the entire engine.


======================================================================
                       API LAYERS
======================================================================

Design multiple layers:

LOW LEVEL
media
frames
audio
GPU
codec

CORE
timeline
project
commands
render

HIGH LEVEL
editor API
AI API
plugin API
script API

EXTERNAL
Rust SDK
TypeScript SDK
Kotlin bindings
CLI
MCP


======================================================================
                     TESTING REQUIREMENTS
======================================================================

Create:

unit tests
integration tests
timeline tests
codec tests
render tests
golden-frame tests
project migration tests
property tests
fuzz tests
cross-platform tests
GPU tests
Android tests
browser tests
security tests
performance tests
AI command tests
MCP tests
plugin sandbox tests


Critical invariant:

THE SAME PROJECT MUST REMAIN SEMANTICALLY CONSISTENT ACROSS PLATFORMS.


Where exact pixels cannot be identical because of hardware/rendering differences,
define measurable tolerances.


======================================================================
                       BENCHMARK SYSTEM
======================================================================

Create a permanent benchmark suite.

Measure:

IMPORT
PROBE
DECODE
SEEK
PREVIEW
TIMELINE OPERATIONS
COMPOSITING
EFFECTS
AUDIO
ENCODE
EXPORT
SAVE
LOAD
AUTOSAVE
UNDO
REDO
PROXY
THUMBNAILS
WAVEFORMS
AI ANALYSIS


Measure:

p50
p95
p99
throughput
memory
CPU
GPU
disk
battery where practical


======================================================================
                     FAILURE ENGINEERING
======================================================================

Test:

power loss
crash
partial render
corrupted project
missing media
media moved
unsupported codec
GPU failure
hardware encoder failure
plugin crash
AI failure
MCP timeout
storage exhaustion
memory exhaustion
background interruption
Android process death
browser tab crash


The engine must recover predictably.


======================================================================
                    NO FAKE COMPLETION
======================================================================

You are forbidden from declaring:

"production ready"
"world class"
"best"
"state of the art"
"complete"

unless evidence supports the claim.

Every subsystem must be classified:

IMPLEMENTED
TESTED
BENCHMARKED
EXPERIMENTAL
PARTIAL
PLANNED
BLOCKED
UNKNOWN


======================================================================
                  NO DOCUMENTATION-ONLY PROGRESS
======================================================================

Never replace missing engineering with documentation.

Never write:

"future implementation will..."

and treat that as implementation.

Documentation must accurately reflect the repository.


======================================================================
                    NO ARCHITECTURAL FASHION
======================================================================

Do not introduce:

microservices
Kubernetes
distributed systems
vector databases
LLMs
agents
event sourcing
graph databases
WASM plugins
cloud infrastructure

simply because they sound modern.

Every subsystem must justify itself.

Prefer the smallest architecture that satisfies the actual requirements.


======================================================================
                     RESEARCH COMPLETENESS
======================================================================

Before leaving research mode, verify that you have investigated:

[ ] open-source editors
[ ] mature NLEs
[ ] media engines
[ ] codecs
[ ] GPU systems
[ ] timeline systems
[ ] project formats
[ ] Android
[ ] Kotlin
[ ] Media3
[ ] MediaCodec
[ ] Rust Android
[ ] Tauri
[ ] browser media
[ ] WebCodecs
[ ] WebGPU
[ ] WASM
[ ] audio
[ ] color
[ ] captions
[ ] computer vision
[ ] AI video editing
[ ] AI agents
[ ] MCP
[ ] plugins
[ ] scripting
[ ] headless rendering
[ ] storage
[ ] caching
[ ] performance
[ ] security
[ ] licensing
[ ] testing
[ ] YouTube engineering
[ ] transcripts
[ ] academic papers
[ ] GitHub source code
[ ] competing architectures
[ ] real-world failure reports


If an item is not sufficiently researched:

DO NOT MARK IT COMPLETE.


======================================================================
                     RESEARCH DEPTH REQUIREMENT
======================================================================

Do not stop when you find the first useful implementation.

For important subsystems aim to understand:

AT LEAST:
- multiple implementations
- multiple technical sources
- primary documentation
- competing approaches
- failure cases

For critical architecture decisions:

prefer:

3+ independent implementation references
+
primary documentation
+
research literature where applicable
+
experiment/benchmark where feasible.


======================================================================
                    RESEARCH → IMPLEMENTATION LOOP
======================================================================

For each major subsystem:

RESEARCH
   ↓
UNDERSTAND
   ↓
COMPARE
   ↓
DESIGN
   ↓
MINIMAL PROTOTYPE
   ↓
BENCHMARK
   ↓
REVIEW
   ↓
IMPLEMENT
   ↓
TEST
   ↓
DOCUMENT
   ↓
AUDIT


Do this repeatedly.


======================================================================
                    FINAL INDEPENDENT AUDIT
======================================================================

After implementation, create a fresh auditor agent.

The auditor must assume the project is WRONG.

Ask:

Does the engine actually work?

Does the timeline model survive complex editing?

Does seeking remain frame-accurate?

Does audio remain synchronized?

Does the compositor work?

Does hardware acceleration work?

Does software fallback work?

Does Android work?

Does browser work?

Does desktop work?

Does the project format migrate?

Does undo work?

Does AI use the same engine?

Does MCP use safe commands?

Can plugins be sandboxed?

Does headless rendering work?

Can large projects load?

Can low-end devices function?

Can corrupted projects recover?

Are benchmarks real?

Are claims supported?

Are dependencies legally compatible?

Are there hidden hacks?

Are there mocks pretending to be implementations?

Are there documentation-only features?

Are there security vulnerabilities?

Are cross-platform assumptions false?


Then:

FIX EVERYTHING CRITICAL.

Run the audit again.


======================================================================
                       FINAL DELIVERABLES
======================================================================

At the end of the complete cycle produce:

1. RESEARCH DATABASE

2. RESEARCH INDEX

3. SOURCE LEDGER

4. PAPER INDEX

5. GITHUB REPOSITORY ANALYSIS

6. YOUTUBE VIDEO INDEX

7. TRANSCRIPT INDEX

8. KNOWLEDGE SYNTHESIS

9. ARCHITECTURE OPTIONS

10. ARCHITECTURE DECISION RECORDS

11. FINAL ARCHITECTURE

12. IMPLEMENTATION PLAN

13. MONOREPO DESIGN

14. API DESIGN

15. PROJECT FORMAT SPECIFICATION

16. TIMELINE SPECIFICATION

17. RENDER ENGINE SPECIFICATION

18. PLUGIN SPECIFICATION

19. AI/MCP SPECIFICATION

20. SECURITY MODEL

21. TESTING STRATEGY

22. BENCHMARK SUITE

23. RISK REGISTER

24. FINAL AUDIT

25. ACTUAL IMPLEMENTATION


======================================================================
                         FINAL PRODUCT VISION
======================================================================

The final system should conceptually allow:

A HUMAN:

import video
→ edit
→ preview
→ export


A SCRIPT:

open project
→ manipulate timeline
→ render


AN AI AGENT:

inspect media
→ understand content
→ create edit plan
→ manipulate timeline
→ preview
→ validate
→ render


A SERVER:

receive project
→ render
→ export


AN ANDROID APP:

use native Kotlin UI
→ call shared engine
→ use Android hardware acceleration


A WEB APP:

use browser APIs
→ WASM where appropriate
→ WebCodecs
→ WebGPU
→ local storage


A DESKTOP APP:

use Tauri/native capabilities
→ shared engine
→ native codecs/GPU


A PLUGIN:

extend effects
→ transitions
→ generators
→ AI
→ import/export


All of these should ultimately interact with the SAME conceptual engine.


======================================================================
                         THE GREATER GOAL
======================================================================

Do not think of this as:

"another video editor."

Think of it as:

                    VIDEO INFRASTRUCTURE.


The editor UI is one consumer.

The Android application is another consumer.

The browser application is another consumer.

The desktop application is another consumer.

The AI agent is another consumer.

MCP is another interface.

The scripting engine is another interface.

The headless renderer is another interface.

Future applications should be able to build on the engine without needing
to understand its internal implementation.


======================================================================
                     THE ULTIMATE PRINCIPLE
======================================================================

BUILD THE ENGINE FIRST.

THE APPLICATIONS ARE CLIENTS OF THE ENGINE.

THE AI IS A CLIENT OF THE ENGINE.

THE SCRIPTING SYSTEM IS A CLIENT OF THE ENGINE.

MCP IS A CLIENT INTERFACE TO THE ENGINE.

PLUGINS EXTEND THE ENGINE THROUGH STABLE CONTRACTS.

THE RENDERER IS PART OF THE ENGINE.

THE TIMELINE IS PART OF THE ENGINE.

THE PROJECT FORMAT IS PART OF THE ENGINE.

THE MEDIA PIPELINE IS PART OF THE ENGINE.


The ultimate objective is not to create the largest video editor.

It is to create the deepest, most extensible, most interoperable,
most experimentally validated open video-engine architecture that can
serve both tiny applications and extremely sophisticated applications.


======================================================================
                       EXECUTION COMMAND
======================================================================

START NOW.

Do not ask me to design the architecture for you.

Do not ask me which technologies you should research.

Do not ask me which repositories to inspect.

Do not ask me which papers to read.

Do not ask me which YouTube videos to find.

That is YOUR responsibility.

Use the assigned agents.

Use parallel research where possible.

Use source code.

Use papers.

Use arXiv.

Use GitHub.

Use official documentation.

Use conference talks.

Use YouTube.

Retrieve transcripts.

Run experiments.

Benchmark competing approaches.

Maintain persistent research state.

Challenge your own conclusions.

Then synthesize.

Then architect.

Then build.

Then test.

Then benchmark.

Then independently audit.

Then improve.

DO NOT STOP AT THE PLAN.

DO NOT STOP AT THE RESEARCH.

DO NOT STOP AT THE ARCHITECTURE.

THE ULTIMATE OBJECTIVE IS THE ACTUAL ENGINE.


                    LEARN EVERYTHING NECESSARY.

                    BUILD WHAT THE EVIDENCE SUPPORTS.

                    MEASURE EVERYTHING IMPORTANT.

                    REMOVE WHAT DOES NOT WORK.

                    KEEP WHAT PROVES ITSELF.

                    REPEAT UNTIL THE ENGINE IS REAL.


                         START THE MISSION.