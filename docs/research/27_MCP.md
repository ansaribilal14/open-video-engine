# 27 — MODEL CONTEXT PROTOCOL (MCP) (Track N)

> Status: PARTIAL (v0.1).

## 1. Protocol status (verified 2026-09-22)

- Canonical repo: https://github.com/modelcontextprotocol/modelcontextprotocol (spec +
  schema + docs). License: MIT (repo README, LICENSE file). Protocol created by David
  Soria Parra and Justin Spahr-Summers (Anthropic); now steered under the MCP community
  with SEPs (specification enhancement proposals).
- Latest revision: **2026-07-28** (schema defined in TypeScript first:
  `schema/2026-07-28/schema.ts`, JSON Schema also published). Docs site:
  https://modelcontextprotocol.io (fetched; nav confirms "Version 2026-07-28 (latest)").
- Base protocol: JSON-RPC 2.0, UTF-8. Client MUST NOT send JSON-RPC responses;
  server MUST NOT initiate JSON-RPC requests (message-direction rules per
  `basic/index.mdx`).
- 2026-07-28 changes observed while reading the spec (v0.1 note — verify each in v0.2):
  - `_meta` request metadata now REQUIRED on every request
    (`io.modelcontextprotocol/protocolVersion`, `clientInfo`, `clientCapabilities`).
  - Responses carry a polymorphic `resultType` (e.g. `"complete"`); multi-round-trip
    requests (MRTR) and subscribe/notify patterns are first-class in `basic/patterns`.
  - **Sampling is DEPRECATED** as of 2026-07-28 (SEP-2577): stays in spec >=12 months,
    new implementations SHOULD NOT adopt; migrate to direct LLM provider integration.
    (Verified warning text in `client/sampling.mdx`.)
  - Elicitation now has two modes: form mode (structured JSON-schema data) and URL mode
    (sensitive interactions MUST go out-of-band through a browser, not the client).

## 2. Primitives

### 2.1 Server features (what an editor engine would expose)

- **Tools** (model-controlled): invoked via `tools/list` / `tools/call`. Tool object =
  `name`, `title`, `description`, `inputSchema` (JSON Schema), optional icons;
  responses carry `content` items + `isError`. Server declares capability
  `{tools: {listChanged: true}}`. Tool sets MUST NOT vary per-connection except by
  authorization scopes; SHOULD be deterministically ordered (cache + prompt-cache
  friendliness). Trust warning in spec: there SHOULD always be a human in the loop able
  to deny tool invocations; applications SHOULD show which tools are exposed, indicate
  invocations in UI, and use confirmation prompts.
- **Resources** (application-controlled context): `resources/list`, `resources/read`,
  subscriptions. For an editor: the project document, timeline snapshots, media
  metadata, transcripts.
- **Prompts** (user-controlled templates): `prompts/list`, `prompts/get`.
- **Utilities**: logging, argument completion, pagination, caching (new in 2026-07-28:
  `ttlMs`, `cacheScope` on listings).

### 2.2 Client features (what a host must implement)

- **Elicitation**: server asks the client to gather user input — form mode (JSON
  schema) or URL mode. Spec hard rules: servers MUST NOT request secrets (passwords,
  API keys, tokens, payment credentials) via form mode; MUST use URL mode for such
  flows; clients MUST show which server is asking and provide decline/cancel.
- **Sampling**: server asks the CLIENT to run an LLM completion (client keeps model
  access + permission control). Deprecated 2026-07-28 — for an editor host, prefer
  calling the LLM provider directly in the host and keeping MCP servers tool-only.
- **Roots**: client-provided directory scope hints (e.g., the open project folder).

## 3. Transports

Verified from `basic/transports/index.mdx` (2026-07-28):

1. **stdio**: newline-delimited messages over the standard streams of a client-launched
   subprocess. Cancellation via `notifications/cancelled`. Best for: local editor
   process, e.g., an engine-side MCP server shipped with the app.
2. **Streamable HTTP**: each message is an HTTP POST to a single MCP endpoint; replies
   arrive as a JSON object or a request-scoped SSE stream. Cancellation by closing the
   response stream. HTTP headers may mirror body metadata for intermediaries; body
   remains source of truth. Best for: remote/shared editing sessions, cloud render.
3. Custom transports allowed if they preserve JSON-RPC format, message patterns, and
   per-request metadata model.

## 4. Authorization and capability scoping

- Authorization is OPTIONAL, transport-level, defined only for HTTP-based transports:
  OAuth 2.1 draft, RFC 6750 bearer tokens, RFC 8414 server metadata, RFC 7591 dynamic
  client registration, Protected Resource Metadata. stdio servers SHOULD NOT use this
  flow (credentials from environment instead).
- Scope minimization is a named security practice in the spec's Security Best Practices:
  request the narrowest scopes; tool listings MAY vary by authorization scope (the
  `tools/list` spec explicitly allows returning only tools the caller's scopes permit).

## 5. Security essentials (verified sources: spec Security Best Practices page; S-4f9 papers)

- **Confused deputy** (proxy + static client ID + consent cookies -> stolen
  authorization codes). Mitigations: per-client consent, no silent consent reuse.
- **Token passthrough**: server must not forward client tokens to third parties
  (breaks attribution, revocation, audience validation).
- **SSRF** (against the server and against authorization servers): validate URLs,
  egress rules.
- **Local MCP server compromise**: a local server runs with user privileges — an
  editor shipping a local MCP server must treat its tool calls as untrusted input to
  the engine (see 26_AI_AGENTS.md threat model).
- **Tool poisoning attacks (TPA)**: hidden malicious instructions inside tool
  descriptions or tool outputs exploit LLM sycophancy (S-4f9: MCPXKIT/"Systematic
  Analysis of MCP Security", arXiv 2508.12538; also Invariant Labs' analysis and Simon
  Willison's write-up). Related: tool "rug pulls" — redefining tools after the user
  approved them; mitigated by treating `listChanged` notifications as re-approval
  events.
- **Editor-specific corollary**: media files are untrusted data. Any MCP tool that
  returns metadata/transcripts derived from media must be treated like tool output from
  a third party — never as instructions (prompt-injection surface).

## 6. Example MCP servers relevant to media/creative tools

- **kinocut** (https://github.com/KyaniteLabs/kinocut, Apache-2.0, verified): a
  "guardrailed video editing MCP server for AI agents" — local-first FFmpeg tools,
  201 typed MCP tools + 173 CLI commands, Video Receipts, quality gates, Shorts/Reels
  repurposing; Python client; formerly `mcp-video`. Strongest public precedent for
  exposing an editor as MCP tools with guardrails.
- Official reference servers (https://github.com/modelcontextprotocol/servers): Filesystem
  (configurable access controls), Git, Fetch, Memory, Sequential Thinking, Time,
  Everything (test). Filesystem/Git patterns are the closest analogs for project-file
  access control.
- MCP Registry (https://registry.modelcontextprotocol.io/) — published-server discovery.
- SDKs: TypeScript, Python, Rust, Kotlin, Go, Java, C#, PHP, Ruby, Swift (official).
  NOTE for C-004/ADR-001: an official Rust MCP SDK exists; Kotlin SDK exists for
  Android surfaces.

## 7. Design position for the Open Video Engine (proposal, not decision)

- The engine exposes the SAME command API through three surfaces: (a) in-app UI,
  (b) scripting/headless (doc 29/30), (c) an MCP server wrapping those same commands
  (doc 28 PLUGINS adjacency). This makes "AI agent" just another client with a
  permission policy — the literal implementation of C-003.
- Tool-per-command family rather than tool-per-operation: e.g. `timeline.insert_clip`,
  `timeline.trim`, `project.undo`, `analysis.detect_shots`. inputSchema gives the model
  typed parameters; `isError` + structured content give it feedback.
- MCP is the FUTURE-PROOFED transport, not the internal model: internal command bus
  stays engine-native; MCP adapter is a thin, versioned shell. Do not let MCP semantics
  leak into the project format (C-007).
- Sampling: do not adopt (deprecated 2026-07-28). Host-side LLM calls + MCP tools only.

## 8. Depth tracking (v0.2)

- Read schema/2026-07-28/schema.ts end-to-end; catalog every method relevant to editors.
- Track SEP pipeline for features affecting editors (MRTR semantics, tool-result
  subscriptions, elicitation modes).
- Prototype experiment E-00x: wrap 10 engine commands in a stdio MCP server and drive
  them from Claude Code to validate the C-003 "one API, two clients" claim.
