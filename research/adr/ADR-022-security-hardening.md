# ADR-022: Security hardening v1 — untrusted-input resource budgets at every client boundary

- Status: ACCEPTED
- Date: 2026-09-30 (WAVE 18)
- Consumers: ove-project, ove-plugin, ove-mcp, ove-script, ove-cli (indirect), SECURITY.md

## Context

Waves 1–17 built the engine surface out (decode → render → encode/mux →
persistence → engine → CLI → audio → keyframes → GPU parity → platform
evidence → conformance → headless → MCP → scripting → plugins). Every one
of those surfaces consumes UNTRUSTED input: media files (libav), project
folders (manifest.json + commands.jsonl + snapshot state), plugin stdout
(line-JSON protocol v1), script sources (Rhai), and MCP client lines
(JSON-RPC 2.0 over stdio). The W17 hardening review (certification-loop
wave gate) found the v1 posture treated only *correctness* of hostile
input (typed rejections, corruption hard-stops, default-DENY capabilities)
but not its *resource shape*: every reader buffered unboundedly.

The wave-18 conformance probes made the gaps concrete — and one of them
was not theoretical:

- **ove-script**: a 12-line string-doubling script drove the host into
  allocator OOM. The pre-fix engine was **SIGKILLed by the kernel OOM
  killer** in the conformance test run (Rhai's default engine leaves
  string/array/map sizes effectively unbounded).
- **ove-plugin**: a hostile plugin process (self-claimed identity) could
  push one multi-gigabyte stdout line (buffered whole by the host before
  any check) and accumulate unbounded receipts.
- **ove-mcp**: the stdio server buffered each client line without bound
  before dispatch.
- **ove-project**: `Project::open` read manifest.json, snapshot state, and
  each log line without a size bound; the asset registry trusted manifest
  paths as instructions (a shared project folder could point assets
  outside the project directory).

Directive §36 (SECURITY ENGINEER) + §37 (SANDBOXING ENGINEER) require the
capability/sandboxing posture to be real, and the anti-overbuild rule
forbids speculative machinery: the fix must be *budgets + typed errors*,
not a security framework.

## Decision

**1. Every untrusted reader gets a DECLARED resource budget, enforced at
READ time, failing TYPED.** The caps are named public constants — a
documented contract, not magic numbers:

| Surface | Budget | Constant | Typed failure |
|---|---|---|---|
| project manifest.json | 16 MiB | `ove_project::manifest::MANIFEST_MAX_BYTES` | `ManifestInvalid("manifest too large: …")` |
| project commands.jsonl line | 8 MiB | `ove_project::log::LOG_LINE_MAX_BYTES` | `LogCorruption { reason: "line too large: …" }` |
| project snapshot state | 64 MiB | `ove_project::STATE_FILE_MAX_BYTES` | `SnapshotInvalid("state file too large: …")` |
| plugin stdout line | 1 MiB | `ove_plugin::PLUGIN_LINE_MAX_BYTES` | `PluginError::Protocol("line too large: …")` (abort) |
| plugin proposals/session | 10 000 receipts | `ove_plugin::PLUGIN_MAX_PROPOSALS` | `PluginError::Protocol("proposal budget exhausted: …")` (abort) |
| MCP client line | 1 MiB | `ove_mcp::LINE_MAX_BYTES` | JSON-RPC error −32000 ("line too large: …"), server keeps serving |
| Rhai string | 16 MiB | `ove_script::SCRIPT_STRING_MAX_BYTES` | typed script error |
| Rhai array / map / call depth / operations | 1e6 / 65 536 / 128 / 1e6 | `SCRIPT_ARRAY_MAX` / `SCRIPT_MAP_MAX` / `SCRIPT_CALL_LEVELS` / `SCRIPT_MAX_OPERATIONS` | typed script error |
| Rhai `emit` output per run | 1 MiB | `ove_script::SCRIPT_EMIT_MAX_BYTES` | `ScriptError("emit budget exceeded: …")` |

Legitimate scale is far below every cap (log entries are KB-scale, plugin
protocol lines are sub-KB, legit manifests ~KB; the caps carry ≥1000×
headroom), so the budgets are invisible to honest use and hard against
hostile use.

**2. Read-time enforcement, not parse-time.** Line-oriented readers use a
`take(cap + 2)` + `read_until(b'\n')` window: a hostile line costs the
host at most cap+2 bytes of buffering before the typed failure, regardless
of how many gigabytes follow it. `Project::open` stats the manifest and
snapshot state BEFORE the wholesale read. Nothing unbounded ever reaches
serde_json.

**3. The MCP server answers exactly ONE typed error per oversized client
line and discards its tail** (`discarding` state until the next newline),
then keeps serving — a hostile line cannot wedge the session, and the
one-line-one-response protocol discipline is preserved.

**4. The repair path stays bounded and honest.** `quarantine_and_split`
records an oversized log line in the quarantine by REASON only ("LINE n:
line too large …") — its bytes are never stored, because the verbatim
quarantine of a multi-gigabyte line would itself be the memory attack.

**5. Project registry paths are DATA, not instructions.** On manifest
load, every `assets[].path` and `assets[].probe` must be relative and free
of `..` components; violations fail `ManifestInvalid("asset path escapes
project folder …")`. A shared/hostile project folder cannot point the
engine at files outside the project directory. (Relink-by-path was already
forbidden by R-13/hash-identity; this closes the path itself.)

**6. Plugin process launch is direct exec.** `PluginHost::spawn_with_args`
passes arguments verbatim to the executable. There is NO shell anywhere in
the host — no interpolation surface exists. (This is also the launch
mechanism the hostile-flood conformance instrument uses.)

**7. Scripting-edge budgets are engine-configuration, not language
changes.** Rhai runs with the declared caps; a script that exceeds one
gets a typed error message. `emit` (the host-side output collector) is
budgeted identically — host memory is part of the attack surface.

**8. The plugin hostile peer is modeled in conformance.**
`ove-plugin-flood` (a committed instrument binary, mode-argumented)
emits one 2 MiB line before any manifest, or a valid manifest followed by
11 000 default-DENY proposals with balanced reply draining (the host can
never deadlock on a full stdin pipe mid-attack). The W18 tests pin typed
aborts at BOTH the pure state machine and the real wire, plus engine-state
invariance of the denied flood.

**9. The certification loop carries the security leg to real media.**
`ove-plugin/tests/realworld_security.rs` (env-gated, same discipline as
the W17 gate): hostile proposal-flood against a session holding the REAL
NASA reference media → typed budget abort, state hash byte-unchanged, spot
renders byte-identical → the LEGIT plugin still applies its split through
the same command bus → render invariance → exact 80-frame export →
independent ffprobe verification → deterministic record
(`realworld_record_security.json`, wave + commit + hash).

## Consequences

- The host process can no longer be killed or OOMed by any of the five
  untrusted inputs through a size/vector path; every exhaustion now fails
  typed, at read time, with the budget named in the message.
- Determinism is preserved: caps are inputs to the same pure functions;
  the emit budget keeps scripted output reproducible; the W18 real-media
  export sha256 (`ff5e67f8…`) matched the pre-docs build bit-for-bit on
  re-run.
- Workspace 185/185 GREEN (171 pre-existing + 14 security conformance);
  fmt/clippy GREEN. CI runs the full suite including the bundled-FFmpeg
  job; the hostile-flood instruments run in CI (no real media involved).
- Honest-use behavior is unchanged: every cap is ≥1000× legitimate scale.

## Named residuals (not done in v1, deliberate)

- **Log-flood CPU**: a hostile plugin may send up to `PLUGIN_MAX_PROPOSALS`
  receipts but unbounded *log* lines; the host relays them to stderr. A
  session wall-clock/line budget is the Tier-B sandboxing conversation
  (28_PLUGINS §9), not a v1 line cap.
- **Protocol-abort child reaping**: a plugin session aborted on a protocol
  violation drops the `Child` without `wait()` (zombie until the parent
  exits). Real hosts are long-lived; reaping belongs with the user-approval
  ladder (ADR-021 reopen list).
- **Decoder pixel-bomb**: libav `max_pixels`/dimension caps are NOT set in
  this wave; a hostile media file with extreme dimensions still relies on
  libav's own limits. This rides the decode-hardening conversation with the
  VFR/long-GOP corpus leg (REALWORLD §10) so the cap can be tested against
  real difficult media, not synthetic assertions.
- **Windows path semantics**: the asset-path validator treats `\` as an
  ordinary character (POSIX). A Windows-hostile-path review is deferred to
  the platform-leg wave that first targets Windows.

## Confidence

0.88. The caps are enforced at read time with committed hostile-peer
conformance (pure + wire + real media), and the one measured attack
(Rhai memory bomb) is reproducibly converted from SIGKILL to a typed
error. What would reopen this ADR: an untrusted-input path discovered
outside the five surfaces; a legitimate workload exceeding a cap; a
platform where `take`+`read_until` semantics differ; a requirement for
per-connection (not per-line) MCP budgets before the MCP transport
diversifies beyond stdio.
