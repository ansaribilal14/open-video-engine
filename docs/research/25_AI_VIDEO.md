# 25 — AI VIDEO EDITING RESEARCH LANDSCAPE (Track L)

> Status: PARTIAL (v0.1).

## 1. Scope and method

This document maps the AI video editing research landscape as first-pass evidence for the
Open Video Engine mission. Everything below was fetched and verified against arXiv
abstract pages or the referenced repository on 2026-09-22 (v0.1 scan). Anything not
verifiable is marked NOT FOUND / UNVERIFIED. No claim in this document is load-bearing
for architecture until re-studied in v0.2.

Primary anchor: the community survey repo `wentianli/awesome-video-editing` (verified;
GitHub redirects from the capitalized form to this lowercase slug). Source: S-4f0.

## 2. Survey anchor: wentianli/awesome-video-editing

- Repo: https://github.com/wentianli/awesome-video-editing (HTTP 200, raw README fetched)
- Maintainer framing: "A paper list on video editing (in a cinematographic sense) and its
  related computer vision tasks." Explicitly EXCLUDES content manipulation (object
  removal, stylization) — this is about EDITING (assembly, cutting, sequencing), which is
  exactly our engine's problem domain.
- Tagging convention: application-scenario icons (talk/meeting, dance, sports, ads,
  movie), plus `venue year` prefixes. Useful as a venue signal, not a quality signal.

### 2.1 Taxonomy observed in the README (verified 2026-09-22)

1. **Text-based Video Editing**
   - LLM-powered / instruction-based editing (agents that take natural-language
     directives): CineAgents, EditDuet, Generative Timelines (Timeline Assembler), LAVE.
   - Text-to-sequence assembly from a video/image collection: SKALD, Transcript-to-Video,
     Write-A-Video, audio-visual slideshows, QuickCut.
   - Transcript editing (cut filler words, edit speech by editing transcript):
     text-based editing of speech videos.
2. **Cutting and Sequencing Shots** — shot boundary detection + shot order learning:
   DIRECT, Energy-Based Shot Assembly, Match Cutting (Netflix), Learning to Cut by
   Watching Movies, Video-Story Composition, movie trailer generation.
3. **Multi-Camera / Multi-Take Editing** — view/take selection: Smart Director,
   dialogue-driven scene editing (TOG 2017), multi-camera TV show editing.
4. **Video Summarization & Highlight Detection** — trailer moments (ECCV 2020), sports
   highlights, e-commerce short-video generation, home-video editing (AVE, MM 2003).
5. **Other Forms of Editing**
   - Fast-forwarding & retiming (semantic hyperlapse, Video-ReTime).
   - Music-driven editing (GLANCE, CutClaw, AutoMatch beat benchmark, Visual Rhythm and
     Beat) — an active 2026 sub-area.
   - Spatial editing (Reframe Anything LLM agent, GAZED, actionness crops).
   - Video editing style transfer (JAWS, Non-Linear Video Editing Transfer).
   - Virtual cinematography (GenDoP camera-trajectory generation).
6. **Datasets and more** — MovieNet, MovieCuts, Edit3K, VEU-Bench, AVE benchmark
   (Anatomy of Video Editing), camera movement / shot-type datasets.

### 2.2 Structural lesson for the engine

The survey's categories map almost one-to-one onto *analysis services that output editor
commands* (detect shots -> add-cut commands; transcript edit -> remove-segment commands;
highlight detection -> keep-ranges command). That supports claim C-003's design shape:
AI features should be command producers, not a parallel editing path.

## 3. The four named works (all verified REAL; none fabricated)

### 3.1 EditDuet: A Multi-Agent System for Video Non-Linear Editing

- Verified: arXiv https://arxiv.org/abs/2509.10761 (fetched, title+abstract match);
  SIGGRAPH 2025 Conference Papers (Vancouver, DOI 10.1145/3721238.3730761);
  project page https://mudtriangle.com/editduet (exists; Cloudflare interstitial on fetch).
- Authors: Marcelo Sandoval-Castañeda (TTI-Chicago), Bryan Russell (Adobe), Josef Sivic
  (Adobe / CTU Prague), Gregory Shakhnarovich (TTI-Chicago), Fabian Caba Heilbron (Adobe).
- Problem: automate the core NLE task itself (not retrieval or UI): produce an edited
  sequence from raw footage + natural-language instruction. Target scenario: A-roll
  (interview/voice-over) + B-roll coverage, i.e., documentary/social content.
- Architecture: TWO LLM agents + an NLE environment.
  - **Editor agent**: takes user instruction, clip collection, and A-roll; operates on a
    draft timeline through tools "commonly found in video editing software" — search,
    trim, add/remove clips on the timeline. Sequential decision making.
  - **Critic agent**: receives draft timeline + user request; returns natural-language
    feedback, or calls render when satisfied. Iterative editor<->critic loop.
  - **In-context learning for multi-agent communication**: exploration-based synthetic
    demonstration generation (BAGEL-style, extended to two agents) to teach agents how
    to emit/receive feedback; no fine-tuning required.
  - **NLE judge**: VLM-based LLM-as-a-judge metric over structure, relevance, aesthetic
    coherence, pacing; shown to correlate with human preference.
- Inputs: NL instruction, video collection, A-roll. Outputs: a finished timeline,
  rendered to video.
- Evaluation: real-world NLE projects from EditStock; beats baselines on failure rate,
  time-constraint satisfaction, human/automatic preference.
- Code: NO public code repo found (searched GitHub + web; only ACM/RG/project page).
  License: paper CC BY-NC-ND 4.0. Relevance: HIGHEST — it is literally an agent editing
  a timeline through editing-tool APIs, the closest published analog of our mission.

### 3.2 CineAgents (+ CineBench): Instruction-driven Cinematic Video Compilation

- Verified: arXiv https://arxiv.org/abs/2604.10456 (fetched, title+abstract match),
  arXiv HTML v1 fetched 2026-09-22. Affiliations: Tsinghua, BAAI, Tencent PCG (Online
  Video AI Tech Center), Peking University, BUPT. Note: the awesome list titles it
  "CineAgents: ..."; the arXiv title is "A Benchmark and Multi-Agent System for
  Instruction-driven Cinematic Video Compilation" (system name appears in abstract).
- Problem: adapting long-form cinematic content into short compilations under diverse
  user instructions. Prior work = "retrieve-and-rank" over shots -> two failure modes:
  *contextual collapse* (shot captions lose narrative context) and *temporal
  fragmentation* (isolated clips, no narrative flow).
- Contribution 1 — **CineBench**: first benchmark for instruction-driven cinematic
  compilation; 500+ instruction-video pairs from 70 films/series; ground truths
  re-authored by professional editors; 11 metrics in 3 aspects.
- Contribution 2 — **CineAgents**: "design-and-compose" multi-agent system inspired by a
  professional editing studio:
  - Script reverse-engineering: convert the source film into a structured script.
  - Hierarchical narrative memory: shot / event / story / character levels.
  - Iterative narrative planning: refine a creative blueprint into a final compiled
    script (plan-then-assemble, not per-shot retrieval).
- Inputs: long film/series + NL instruction. Outputs: a compiled short sequence
  (shot selection, order, assembly).
- Code: none found on GitHub (searches for CineAgents/CineBench repos returned nothing;
  only paper + aggregator pages). License: arXiv non-exclusive license.
- Relevance: HIGH — validates plan-then-assemble over retrieve-and-rank for long-form
  content; the hierarchical narrative memory is a candidate AI-side "project context"
  structure our engine could expose via the same command API.

### 3.3 Generative Timelines for Instructed Visual Assembly (Timeline Assembler)

- Verified: arXiv https://arxiv.org/abs/2411.12293 (fetched, title+abstract match);
  NeurIPS 2024 workshop per awesome list; project page
  https://sites.google.com/kaust.edu.sa/timeline-assembler (HTTP 200, title match).
- Authors: Alejandro Pardo, Jui-Hsien Wang, Bernard Ghanem, Josef Sivic, Bryan Russell,
  Fabian Caba Heilbron (KAUST + Adobe).
- Problem: "Instructed visual assembly" — manipulate a visual timeline (video) through
  natural-language instructions; requires finding relevant content in the timeline and
  in a reference collection, understanding the instruction, and performing edits.
- Contribution: a multimodal LLM (Timeline Assembler) that (i) processes visual content
  and compactly represents timelines, (ii) interprets editing instructions, (iii) emits
  the edited timeline; plus (iv) an automatic dataset-generation method for assembly
  tasks (no human labels); validated on a new benchmark + human evaluation.
- Inputs: input timeline + instruction + (optionally) reference collection. Outputs:
  edited timeline. Key representation idea: *generating timeline data structures
  directly* as the edit surface.
- Code: no public code repo found. License: arXiv.
- Relevance: HIGHEST — the model literally edits a timeline representation, which is the
  academic analog of "AI resolves intents into timeline mutations". If our project
  format is diffable/serializable, an LLM could emit commands against it directly.

### 3.4 JoyAI-Video-Edit: Real-Time Open-Ended Video Editing with Autoregressive Diffusion

- Verified: arXiv https://arxiv.org/abs/2608.03974 (fetched, title+abstract+authors
  match); official repo https://github.com/jd-opensource/JoyAI-Video-Edit (HTTP 200,
  Apache-2.0 SPDX confirmed on repo page); Hugging Face paper page exists.
- Authors: Yicheng Xiao et al. (JD.com / JoyAI team).
- Problem: real-time editing of an OPEN-ENDED video stream (no future frames, no fixed
  duration) with low latency, source fidelity, and long-term temporal consistency.
  This is *pixel-space generative editing* (retiming-free, diffusion-based), not
  timeline assembly — the opposite end of the AI-editing spectrum from EditDuet.
- Architecture: 16B-parameter autoregressive diffusion framework combining:
  chunk-wise autoregressive adaptation; Source-Anchored Distribution Matching
  Distillation (SA-DMD) to preserve source fidelity in 2-step generation; Long-Horizon
  Autoregressive Distillation to curb temporal drift. End-to-end 720p at ~30 FPS on one
  NVIDIA B200.
- Inputs: live/uploaded video stream + editing instructions. Outputs: edited stream.
- Benchmarks: automatic + human evaluations vs streaming editors and offline systems.
- Code: yes, Apache-2.0 (model weights/links per repo; not independently re-verified).
- Relevance: MEDIUM for the core engine (consumer hardware cannot run 16B diffusion;
  B200 required), HIGH as evidence that "open-ended" editing is now a target task class;
  also an ecosystem sign that JD.com open-sources editing models our users may expect
  to plug in.

## 4. Most architecture-relevant papers beyond the four (verified via awesome list links)

1. LAVE: LLM-Powered Agent Assistance for Video Editing (IUI 2024, arXiv 2402.10294) —
   UX study of LLM-agent assistance inside a human editor; supports co-creation design.
2. Learning to Cut by Watching Movies (ICCV 2021, arXiv 2108.04294; code MIT,
   https://github.com/PardoAlejo/LearningToCut) — learns cut placement + shot type from
   films; maps to "AI proposes cuts" service.
3. Match Cutting (WACV 2023, arXiv 2210.05766; code, Netflix) — smooth-transition clip
   matching; maps to a transition-suggestion command.
4. AutoTransition (ECCV 2022, arXiv 2207.13479; code) — transition-effect
   recommendation; maps to a typed transition command.
5. VEU-Bench (CVPR 2025, arXiv 2504.17828) — benchmark for video editing understanding.
6. Edit3K (arXiv 2403.16048; code MIT) — representation learning for editing components
   (shot/type/transition retrieval) — reusable primitives.
7. MovieCuts (ECCV 2022, arXiv 2109.05569; code) — cut-type recognition dataset.
8. GLANCE (MM 2026, arXiv 2604.05076; code) and CutClaw (arXiv 2603.29664; code) —
   music-grounded agentic editing at hours-long scale (2026 wave).
9. Reframe Anything (arXiv 2403.06070) — LLM agent for open-world reframing (auto-
   reframe verticals); maps to subject-tracked crop commands.
10. DIRECT (arXiv 2604.04875; code https://github.com/AK-DREAM/DIRECT) — hierarchical
    multi-agent mashup creation.

## 5. Implications for the engine (first pass)

- The field has converged on agents that manipulate *timeline state through tools*
  (EditDuet) or emit *timeline data structures* (Timeline Assembler). Both validate
  C-003's "same command API" hypothesis; neither paper reports a separate secret editing
  path — the tool layer IS the editor.
- Benchmarks are emerging (CineBench, VEU-Bench, NLE-judge) — an engine that logs
  command history can reproduce these evaluation loops.
- Generative pixel editing (JoyAI-Video-Edit) should be integrated as an *effect/render
  service* downstream of commands, not as the editing substrate.
- Gaps for our engine: no published work resolves agent intents into a *durable,
  frame-accurate, transactional command log* shared with human UI — that is our opening.

## 6. Practical AI features for an editor engine (implementation-shaped)

Canonical tool, license (verified where noted), compute cost class, and command mapping
for each near-term feature. Cost classes: CPU-L (any laptop), CPU-H (multicore, minutes
per hour of media), GPU-L (consumer GPU or Apple ANE), GPU-H (datacenter GPU).

| Feature | Canonical tool / model | License (verified 2026-09-22 unless noted) | Cost | Command mapping |
|---|---|---|---|---|
| Scene/shot detection | PySceneDetect (content-aware + adaptive modes) https://github.com/Breakthrough/PySceneDetect | BSD-3-Clause | CPU-L to CPU-H (OpenCV per-frame) | analysis.detect_shots -> list of cut points; user applies as split commands or reads as markers |
| Scene/shot detection (neural) | TransNetV2 https://github.com/soCzech/TransNetV2 | MIT | GPU-L (TF/PyTorch inference ~fps-realtime) | same as above; higher accuracy on hard cuts/fades; optional backend behind one service interface |
| Silence detection | FFmpeg `silencedetect` filter (already in engine layer, S-005); pydub (MIT) as Python alt | FFmpeg LGPL/GPL variant; pydub MIT (jiaaro/pydub) | CPU-L (decoded audio scan) | analysis.detect_silence -> remove-segment suggestions in transcript/timeline editing (B-Script, ChunkyEdit pattern) |
| Beat detection / music analysis | librosa (onset/beat tracking) https://github.com/librosa/librosa | ISC | CPU-H (heavy per-hour; cache features) | analysis.detect_beats -> markers; music.sync_cuts command aligns cuts to beats (GLANCE/CutClaw pattern) |
| Transcription + word timestamps | faster-whisper (CTranslate2) https://github.com/SYSTRAN/faster-whisper; whisper.cpp https://github.com/ggml-org/whisper.cpp (CPU/ANE-friendly) | both MIT | CPU-H to GPU-L; whisper.cpp runs INT4 on laptops | media.transcribe -> transcript object + word-level timings; enables text-driven editing (edit transcript -> cut commands) |
| Semantic media search | OpenAI CLIP https://github.com/openai/CLIP (image/video frames + text embeddings) | MIT | GPU-L indexing (frames/s), CPU-L query | media.index_embeddings then media.search("warehouse at dusk") -> ranked clip refs for agent retrieval (EditDuet search tool analog) |
| Auto-reframe (vertical/square) | subject tracking: detection+tracking (e.g., YOLO-class detector + tracker); research precedent Reframe Anything (arXiv 2403.06070) | tool-dependent; UNVERIFIED per tool — pick per ADR later | GPU-L realtime-ish per stream | sequence.set_autoframe(subject_track_id, aspect) — produces animated crop keyframes, editable by humans |
| Highlight generation | research: trailer moments (ECCV 2020), sports excitement features, VEU-Bench; production: heuristic multimodal scoring (audio energy + motion + face size) is the robust baseline | n/a (models vary) | CPU-H to GPU-L | analysis.propose_highlights -> keep-range suggestion batch; human approves which ranges become a sequence |
| Transition recommendation | AutoTransition (ECCV 2022, arXiv 2207.13479, code) https://github.com/yaojie-shen/AutoTransition (verified URL redirect from acherstyx) | MIT (verify in-repo LICENSE) | CPU-L once features cached | transition.suggest between adjacent clips -> user applies typed transition command |
| Match-cut suggestions | Netflix Match Cutting (WACV 2023, arXiv 2210.05766) https://github.com/netflix/matchcut | Apache-2.0 (UNVERIFIED this pass) | CPU-H (clip embeddings) | transition.suggest_match_cut -> cross-clip pairing candidates |

Notes:
- All of the above are ANALYSIS or PROPOSAL services: they emit data or command
  batches; they never mutate the timeline directly (26_AI_AGENTS.md sections 5-6).
- Whisper model weights: OpenAI's original Whisper is MIT; whisper.cpp/faster-whisper
  inherit MIT; commercial-use restrictions apply only to some hosted APIs, not these.
- CLIP ViT-B/32 indexing cost: ~1-2 GPU-hours per 10k frames class — budget indexing as
  a background job with progress + cancel (command log supports cancellation events).

## 7. Depth tracking (what v0.2 must add)

- Full read of EditDuet supplementary (tool list granularity) and CineBench metrics.
- LAVE deep-dive (agent-assist UX patterns) — assign to UX track.
- Verify licenses for GLANCE / CutClaw / DIRECT repos (not yet detected on page scrape).
- Map every section-2.2 survey category to a proposed engine service + command names.
