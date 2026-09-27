# 36_YOUTUBE — YouTube pipeline integration (Data API v3 for automated publishing)

> Status: PARTIAL (v0.1 — wave-3 depth pass 2026-09-24).

Scope note: this doc was originally stubbed as the track-Z "engineering videos
index"; that index remains with 37_TRANSCRIPTS. Per mission end-state (human+AI
engine; owner pipeline includes YouTube automation — owner repos shorts_factory,
ytagent-subtitle-system), THIS doc covers the YouTube Data API v3 upload pipeline
the engine must support. All facts verified from official Google/YouTube docs this
session unless marked UNVERIFIED.

## 1. Data API v3 surface (verified)

- **videos.insert** (upload): `POST https://www.googleapis.com/upload/youtube/v3/videos`
  with `uploadType=resumable`, `part=snippet,status,…` [S-3f7]. Authorized scopes
  (from the method page): `youtube`, `youtube.force-ssl`, `youtube.upload`,
  `youtubepartner`. Writable resource parts include `snippet.{title,description,
  tags,categoryId,defaultLanguage}`, `status.{privacyStatus,publishAt,embeddable,
  license,selfDeclaredMadeForKids,containsSyntheticMedia}`, `notifySubscribers`,
  `recordingDetails.recordingDate` [S-3f7].
  - `status.containsSyntheticMedia` exists on the current resource — relevant to
    AI-generated disclosure workflows (owner's agent pipeline). Semantics page:
    UNVERIFIED this session.
  - `status.publishAt` enables scheduled publish (requires privacyStatus=private;
    classic rule — UNVERIFIED-this-session detail).
- **Resumable upload protocol** (verified, guide [S-3f7]): (1) initiate with POST
  carrying `X-Upload-Content-Length` + `X-Upload-Content-Type` → session URI in
  `Location` header; (2) PUT file data to the session URI, optionally chunked with
  `Content-Range`; (3) on interruption, query status with an empty PUT and resume
  from the offset returned via `Range` in the **308** response; 308 = "Resume
  Incomplete". The guide explicitly notes chunked uploads suit progress
  indicators and unstable networks at the cost of more requests.
- **captions.insert**: `POST …/captions` w/ media body ≤100 MB, accepted MIME
  `text/xml, application/octet-stream, */*`; quota cost **400 units**; properties
  `snippet.{videoId,language,name,isDraft}`; requires `youtube.force-ssl` or
  `youtubepartner` scope [S-3f7]. **`sync` parameter is DEPRECATED** — if true,
  "YouTube will disregard any time codes that are in the uploaded caption file and
  generate time codes automatically" (auto-sync now server-side; send well-timed
  SRT/TTML ourselves — 23/37).
- **videoCategories.list**: cost 1 unit (verified table [S-3f7]); region-dependent
  category ids must be resolved at runtime, not hardcoded.
- **thumbnails.set / playlistItems.insert / videos.update / captions.delete**:
  captions.delete = 50 units (verified example [S-3f7]); thumbnails.set and
  playlistItems.insert = 50 units commonly cited — UNVERIFIED this session (per-
  method table is client-rendered; only highlights present in fetched HTML).

## 2. Quota reality (verified — CHANGED MODEL)

From the official quota-cost page [S-3f7] (fetched 2026-09-24):
> "Projects that enable the YouTube Data API have a default quota allocation of
> **100 search.list calls, 100 videos.insert calls, and 10,000 units per day
> combined for all other endpoints**." … "The search.list and videos.insert
> methods have their own quota buckets."

And the videos.insert reference [S-3f7]: "A call to this method has a quota cost
of **1 unit in the Video Uploads quota bucket**" with "**Quota impact: 100 calls
per day**".

The same quota page still states "videos.insert have the highest cost of 1600
points" — the legacy pricing model. Both statements appear in current official
docs; which model applies to an existing vs newly-created project is
**UNVERIFIED** (docs describe a transition without a date).

**Quota math (both models):**
- Bucket model (current text): uploads/day = min(100-call bucket) — i.e. **up to
  100 uploads/day** before touching the 10k unit pool (which other calls share;
  10,000 units/day at midnight PT reset).
- Legacy model: `10000 / 1600 = 6.25 → 6 uploads/day` (classic confirmed math).
- Planning rule for the engine: treat the DEFAULT ceiling as **6 uploads/day
  (legacy) to 100/day (bucket)**; design for the smaller number, request audit
  early. "If your API Client reaches the quota limit … you can apply for a quota
  extension by completing an API Compliance Audit where you must specify the use
  case … If you have been audited in the past 12 months and have been marked
  compliant … you can apply for an additional quota extension" (Developer
  Policies [S-3f7]).

## 3. ToS / policy constraints (verified)

- **Downloading prohibition** (Developer Policies [S-3f7]): API clients "must not
  … download, import, backup, cache, or store copies of YouTube audiovisual
  content without YouTube's prior written approval". Direct relevance: engine
  YouTube download adapters are out of API-compliance scope; yt-dlp usage is a
  ToS matter for the USER (§6).
- **Monetization-side content policies** (YouTube Help, not API): July 15, 2025 —
  "repetitious content" policy renamed **"inauthentic content"**, clarified to
  cover "repetitive or mass-produced" content, which is ineligible for
  monetization [S-3f8]. **Reused content**: "channels that repurpose content that's
  already on YouTube or another online source without adding significant original
  commentary, substantive modifications, or educational or entertainment value"
  are not monetizable [S-3f8]. Engine implication: automation templates must ship
  a "transform, don't repost" checklist (original commentary/substantial-mod fields
  in the metadata sidecar).
- **Content ID**: no API for Content ID claims/management without YouTube partner
 -level access (youtubepartner scope exists for CMS-linked users; captions.insert
  shows the content-owner linkage requirement [S-3f7]). Program admission itself:
  UNVERIFIED.
- **Shorts vs long-form**: official Help: Shorts are videos "up to 3 minutes
  long" (creation tools wording; [S-3f8]); "You can upload vertical videos saved
  to your computer or smartphone as Shorts" [S-3f8]. Precise aspect-ratio/quality
  gate (9:16 etc.) for the API-detection path: UNVERIFIED this session. Engine
  export presets therefore: vertical ≤3 min → Shorts-eligible; else long-form.
- **API compliance audits**: covered by Developer Policies (§2) [S-3f7]; the
  standalone "quota-and-compliance-audits" page is client-rendered (no static
  content) — fetch via browser later if needed.
- Automated-upload abuse rules (impersonation, engagement-bait, spam) exist as
  general prohibitions in the policies (spam/deception clauses verified present
  [S-3f7]); a dedicated "automation" clause was not isolated this session —
  UNVERIFIED detail.

## 4. Upload pipeline design for the engine

```
render (doc 30 headless; E-007b segment pipeline)
  → artifact.mp4 + sidecar.toml|json (per-video metadata envelope)
  → upload job (resumable, checkpointed, restartable)
  → post-upload steps (thumbnail, captions, playlist, schedule)
```

- **Metadata sidecar** (one file per publishable video): title/description/tags
  templates with substitution (`{title} {date} {project}`, templating per owner's
  shorts_factory conventions — private), categoryId (resolved via
  videoCategories.list), privacy + `publishAt`, `madeForKids`, `containsSynthetic
  media` flag, caption file refs, thumbnail ref, playlist targets, license/attribution
  block (per 35 §6). Sidecar is engine-agnostic: the same envelope drives CLI,
  app, and agent flows (ADR-010 one-command-API).
- **Resumable upload with checkpoint**: the API's 308/Range protocol maps 1:1 to
  our checkpoint design: persist `{session_uri, next_byte}` after every chunk
  (chunk size configurable; 8–64 MB typical — UNVERIFIED optimum), resume after
  crash/reboot (Android mediaProcessing FGS 6 h budget forces the same shape —
  [S-2e4]; ties to E-007b segment renders). A crash mid-upload must never
  re-upload from byte 0.
- **Title/description/tags template system**: versioned templates stored in the
  PROJECT (command-log entries — replayable, auditable), not hard-coded in shells;
  agents may PROPOSE metadata via the same command API with approval gating (26).
- **Scheduled publish**: `status.publishAt` + private privacy in insert; local
  clock vs PT quota reset noted (§2).
- **Caption upload**: SRT/TTML from 23/37 output → captions.insert (400 units);
  do NOT rely on the deprecated `sync` — emit our own exact-tick-derived timecodes
  (ADR-007 guarantees caption timings stay frame-exact with the timeline).
- **Thumbnails**: render frame N (engine command) → thumbnails.set (cost
  UNVERIFIED).

## 5. Quota-aware automation rules (engine defaults)

1. Single default-safe profile: ≤6 uploads/day (legacy math) unless project is
   confirmed bucket-model (≤100/day).
2. Count every auxiliary call against the 10k pool (captions.insert 400! —
   25 caption uploads ≈ one legacy upload slot).
3. Backoff + resume on 5xx; 403 quotaExceeded = stop-and-report, never retry-spam
   (policies: spam clause [S-3f7]).
4. All publish actions appear in the engine command log/receipts (audit parity
   with kinocut's Video Receipts pattern [S-4f8]).

## 6. Alternatives / complements (verified existence + license)

- **youtubeuploader** (porjo/youtubeuploader): exists; LICENSE = Apache-2.0
  (fetched) [S-3f9]. Go CLI for quota-friendly/resumable uploads; useful as a
  reference implementation for chunk/resume edge cases, not a dependency.
- **yt-dlp**: exists; LICENSE = Unlicense (public domain, fetched) [S-3f9].
  Download workflows: technically prohibited by the API Developer Policies
  download clause (§3) and YouTube ToS; keep out of engine distribution; if the
  owner uses it, it is a user-side decision outside the engine binary.
- OAuth note: any upload tool needs a Google Cloud project + OAuth consent
  screen; scope `youtube.upload` is the least-privilege choice for publish-only
  automation (scopes verified on videos.insert page [S-3f7]; full scope table
  page is client-rendered — UNVERIFIED-fetch detail).

## VERIFIED (this pass)

videos.insert endpoint/scopes/status fields; resumable protocol incl. 308 + Range
resume; captions.insert 400 units + deprecated sync; captions.delete 50; 
videoCategories.list 1; bucketed quota model quote; legacy 1600 quote; compliance-
audit quota-extension clause; download prohibition clause; inauthentic-content
rename (2025-07-15) + reused-content definitions; Shorts ≤3 min + vertical upload;
youtubeuploader Apache-2.0; yt-dlp Unlicense.

## UNVERIFIED / NOT-FOUND

- Which quota model (1600-legacy vs 1-unit bucket) applies to a given project;
  transition date. Official pages disagree in wording — recorded verbatim.
- thumbnails.set / playlistItems.insert / videos.update unit costs (client-
  rendered table); OAuth scope reference page content (client-rendered).
- containsSyntheticMedia semantics page; publishAt=private requirement;
  Shorts API-side aspect detection; Content ID program admission terms;
  "automated uploads" clause wording; chunk-size optimum.
- Netflix-style "±22 ms" A/V tolerance (belongs to doc 34; not YouTube).
- YouTube Engineering Talks (track-Z index) — out of scope here; 37_TRANSCRIPTS.

## Depth remaining (v0.2+)

- Render the JS-rendered pages (quota table, scopes, audits) via headless browser
  and pin per-method unit costs.
- E-proposal E-009: sandbox upload of a private test video exercising resumable
  resume-after-kill + captions.insert; record real unit burn against quota.
- Map owner's ytagent-subtitle-system caption output onto 23/37 formats + this
  upload path (owner-session task).
