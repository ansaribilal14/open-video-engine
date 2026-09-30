# Security Policy

The open-video-engine is an offline-capable video editing engine. It has
no network service surface: nothing listens, nothing phones home. Its
attack surface is **untrusted file and process input** — exactly the
things a video editor opens on a user's behalf:

| Surface | Untrusted input | Boundary crate |
|---|---|---|
| media import / export | media files (via libav/FFmpeg, strictly adapter-confined) | `ove-decode`, `ove-encode` |
| project folders | `manifest.json`, `commands.jsonl`, snapshot state | `ove-project` |
| plugins (ADR-021) | plugin process stdout (line-JSON protocol v1) | `ove-plugin` |
| scripting | Rhai script sources | `ove-script` |
| AI clients | JSON-RPC 2.0 lines over stdio | `ove-mcp` |

## Supported versions

Only the latest `main` receives security fixes. There are no release
branches yet.

## Reporting a vulnerability

Report privately via GitHub **Security Advisories → "Report a
vulnerability"** on this repository (maintainer-only intake). Please do
not open a public issue for anything you believe is exploitable.

Include: the affected surface (table above), a minimal reproducer (file
or procedure), and the observed vs expected behavior. You will get an
acknowledgment and an assessment; accepted reports are credited in the
fix commit unless you prefer otherwise.

## Hardening posture (ADR-022, wave 18)

- **Resource budgets at every boundary, enforced at read time, failing
  typed.** Project files, plugin protocol lines, script values/output,
  and MCP client lines all have declared size caps (see the table in
  [ADR-022](research/adr/ADR-022-security-hardening.md)); hostile input
  costs the host at most cap+2 bytes before a typed error.
- **Project registry paths are data, not instructions.** Asset paths in a
  shared manifest must be relative and cannot escape the project folder.
- **Plugins are untrusted peers**: default-DENY capabilities, typed
  protocol aborts, a 10 000-proposal session budget, and a 1 MiB
  per-line read cap; plugin processes are exec'd directly (no shell).
- **libav confinement**: FFmpeg linkage exists only inside `ove-decode`/
  `ove-encode` (CI link-scan gate); the rest of the workspace never links
  it. Supply-chain posture: `cargo audit` + `cargo deny` run in CI.
- **Determinism is a security property here**: exports are byte-exact
  re-producible, so a compromised render path cannot hide behind "run
  to run variation".

## Known limits (honesty over assurance theater)

- The bundled-FFmpeg decode path trusts libav's own limits for hostile
  *media* shapes (e.g. extreme pixel dimensions); a decoder-level
  pixel-bomb cap is a named residual in ADR-022 and rides the difficult
  real-media corpus leg.
- A hostile *plugin* can still flood the host's stderr with log lines
  (bounded per line, unbounded in count) — the session-wall-clock budget
  is deferred to the Tier-B sandboxing conversation (ADR-022 residuals).

Media files are never committed to the repository (`*.mp4` is
repo-ignored); credentials must never enter the repo (standing rule,
`docs/DEV_ENV.md`).
