# 33 — SECURITY MODEL (Track T)

> Status: PARTIAL (v0.1 — wave-3 depth pass 2026-09-24).
> Owner: agent W3-e. Scope: threat model for an editor (hostile media, malicious
> project files, plugins, AI/MCP), supply chain, IPC/local attack surface, sandboxing
> ladder, v1 rule list. Builds on docs 05/17/26/27/28 and experiments E-003/E-004a.
> Evidence base for ADR-033 (Security, future).

## 1. Threat model

Assets: user media + project files (integrity/confidentiality), render/export output,
user credentials (cloud export tokens), the editor host itself (RCE = total compromise),
AI-agent authority (destructive actions).

Adversaries: (a) hostile media files (parser exploits), (b) malicious project files
received from others, (c) malicious plugins/scripts, (d) malicious/compromised MCP
tool servers, (e) compromised supply chain (crates, FFmpeg builds, update channel),
(f) local malware riding the app's privileges.

Trust boundaries (0 = least trusted):

```
untrusted:  media bytes · project archives · plugin .wasm · script batches · MCP tools · cloud responses
boundary 1: OS process        (engine-core sidecar / browser renderer isolates media parsing)
boundary 2: wasm sandbox      (wasmtime: "execute untrusted code in a safe manner" [S-3e5])
boundary 3: shell ACL         (Tauri capabilities: per-window permissions [S-3e4]; browser CSP/COOP-COEP)
boundary 4: core validator    (schema + referential + range checks before any command applies)
trusted:    Rust engine core  (timeline math, command apply, file I/O) — reviewed code only
```

## 2. Hostile media files (the #1 surface)

Media parsing (demux + decode) of attacker-supplied bytes in C libraries is the
highest-risk surface an editor owns:

- FFmpeg's security page (S-3e0, fetched 2026-09-24) lists **513 CVE identifiers**
  across release branches with per-fix commit hashes; e.g. CVE-2025-59733/59734
  (2025 fixes, tagged `BIGSLEEP-*` — Google's Big Sleep bug-finding agent; our reading
  that AI-agent-found memory-safety bugs in media parsing are now routine is INFERENCE),
  CVE-2026-8461, CVE-2026-30998/30999 fixed in 9.0/master. The page explicitly warns of
  a "spike in AI generated, false positives" — fuzz noise around media parsers is high.
- Continuous fuzzing is the ecosystem baseline: OSS-Fuzz FFmpeg (S-3e1) runs
  libFuzzer/AFL/honggfuzz with ASan/MSan/UBSan, contact ffmpeg-security@. Pattern
  (INFERENCE from the corpus): memory-safety bugs in demuxers/decoders dominate.
- Our wasm decode fallback (doc 12) runs the same class of C-compiled parser code in
  the browser renderer — the sandbox absorbs the exploit impact, not the parsing risk.

Engine posture:
1. Media bytes are parsed only behind boundary 1: desktop = engine-core sidecar
   process (Tauri process model; crash isolation, doc 17 §8); browser = dedicated
   decode workers (renderer process isolation, doc 18 §1); FFI panics contained
   (E-004a `catch_unwind` pattern).
2. The core never links media parsers for untrusted input paths in-process (ties R-01).
3. Ingest treats ALL metadata (titles, EXIF, chapters, subtitles, thumbnails) as
   untrusted data — never rendered as instructions (feeds §5), never trusted for
   decisions without validation.
4. Resource caps at ingest: max probe bytes, max decoded-frame buffer per file
   (decompression-bomb analog), max container nesting — failure = reject file.
5. Version pinning + per-release CVE watch (§6).

## 3. Malicious project files

A project file is a program if the engine replays it — so it must be data (C-007).
E-003 (9/9 PASS) proved the replay core; its schema lessons become VALIDATION RULES:

1. **Explicit engine-issued ids only** in the log; client-chosen ids are rejected
   (E-003 schema lesson).
2. **Exact rationals as (num, den) i64 pairs; floats forbidden.** E-003: float
   payloads diverged 300/300 under two legal policies — float acceptance is not just
   nondeterminism, it is an integrity hole (replay forgeries via rounding).
3. **Referential + range checks before apply:** every clip_id/track_id resolves;
   times inside track bounds; (num,den) in canonical reduced form; den ≠ 0.
4. **Fail-closed:** unknown command types, malformed payloads, or schema-version
   mismatch reject the whole import — never skip-and-continue.
5. **Version-tagged schema + explicit migration pass** (Olive per-version serializers,
   doc 19).
6. **No code execution from project files, ever:** no expression evaluators, no
   embedded scripts, no macro fields. Anything dynamic is a category error.
7. **Command-log injection (extend):** an imported log forges user history; mitigations:
   provenance records (who/what applied, per doc 26 §5), audit receipts (kinocut
   pattern, doc 26 §3), and replay determinism (E-003) so a forged log at least
   reproduces exactly what it claims — visible in the receipt diff.

**Path traversal in project archives (zip-slip):** Snyk's research (S-3e6, VERIFIED
page): archive entries whose names contain `../` or absolute paths escape the
extraction directory during unpacking — a cross-ecosystem vulnerability class.
Rules for `.ove` project bundles: reject absolute paths and `..` segments (after
canonicalization the result must be inside the target root); reject symlink/hardlink
entries; check extracted size against declared size (zip-bomb cap); extract to a
fresh temp dir, never over user paths; total entry count + depth caps.

## 4. Plugin / script sandboxing (ties to doc 28)

Wasmtime's stated goal (S-3e5, VERIFIED): to "execute untrusted code in a safe manner
inside of a sandbox. WebAssembly is inherently sandboxed"; docs cover Spectre
mitigations and fault-based bounds checks. wasmtime 49.0.0 on crates.io (S-3e9).
Specific interruption/fuel mechanics UNVERIFIED this pass — doc 28 owner.

Sandboxing ladder (the decision table for "where does this input run"):

| Input | Tier | Runs where | Rule |
|---|---|---|---|
| media bytes | untrusted | sidecar/worker/wasm-decode | boundary 1 + caps (§2) |
| project files | structured data | core, behind validator (§3) | data only |
| plugins (.wasm) | semi-trusted | wasmtime, capability-based host imports (doc 28) | deny-by-default imports; host fns validate every call |
| scripts | semi-trusted | compiled to command batches via the SAME public API (docs 26/29) | no host access beyond commands; no eval anywhere in core |
| native dylib plugins | trusted code | NOT in v1 (would need signing + review) | excluded from ladder |
| MCP tool servers | untrusted services | adapter + policy engine (doc 27) | tool poisoning assumed (§5) |

## 5. AI features + MCP (references docs 26/27 — no duplication)

Doc 26 owns: media-derived text (captions, transcripts, EXIF, model summaries) is
UNTRUSTED DATA — prompt injection via poisoned subtitles is a documented editor
threat; deterministic command resolution; eight safety gates; per-transaction approval.
Doc 27 owns: MCP confused deputy, token passthrough, SSRF, local-server compromise,
tool poisoning / rug pulls (`listChanged` = re-approval event), sampling deprecated.

Engine-level additions here:
1. Media-derived text enters model context only escaped + delimited as data; never
   concatenated into instruction prompts.
2. Tool results derived from media inherit media's untrusted tier (doc 27 corollary).
3. Auto-apply allowlist = read-only analysis commands only (doc 26 §8 default).
4. Every AI-applied command carries provenance + is undoable as an inverse batch (E-003).

## 6. Supply chain

- **Rust crates:** RustSec Advisory Database + `cargo-audit` scanning Cargo.lock
  (S-3e8, VERIFIED: rustsec.org with worked example); `cargo-deny` 0.20.2
  (advisories + licenses + sources + bans) and `cargo-auditable` 0.7.6 (embeds the
  dependency list into release binaries — auditable artifacts) VERIFIED versions
  (S-3e9). CI: `cargo deny check` + `cargo audit` on every PR; failures block merge.
- **FFmpeg/GStreamer:** yearly-major churn (doc 05 §7, R-04) → pin exact versions,
  test CI against LTS + current majors; **CVE workflow:** watch ffmpeg.org/security.html
  per pinned release (S-3e0 — per-branch fix lists with commit hashes); triage within
  N days; upgrade = patch release, not feature release. GStreamer security policy page:
  NOT-FOUND this pass (gitlab.freedesktop.org raw fetch blocked by anti-bot;
  raw.githubusercontent 404) — re-verify (doc 06 owner).
- **WASM plugins:** hash-pinned registry + signature verification before load;
  wasmtime sandbox is the runtime backstop (§4). Plugin signing primitive = minisign
  family (minisign-verify 0.2.5 VERIFIED on crates.io) — exact tauri/plugin signing
  chain UNVERIFIED (see §7).
- **Signing/updates (VERIFIED, S-3e4):** Tauri's updater "needs a signature to verify
  that the update is from a trusted source. This cannot be disabled"; public key
  embedded in `tauri.conf.json`, private key signs installers, per-artifact `.sig`
  files. Consequence: update-server compromise ≠ RCE as long as the private key stays
  offline. OVE: same model for engine-core sidecar binaries and plugin bundles.

## 7. IPC / local attack surface

- **Tauri ACL (VERIFIED, S-3e4):** permissions grouped into capabilities assigned per
  window/webview; command scopes; remote-URL gating; asset-protocol scope. Pattern:
  default-deny; the webview gets a capability set, not the user's whole disk.
  Asset-protocol scope = project dir only (path-exposure minimization).
- **IPC parsing surface:** E-006 numbers double as a security rule — large JSON
  payloads crossing IPC are both a perf hazard (42% of a frame) and parser surface;
  bulk data moves via custom-protocol binary paths with explicit scopes (doc 17 §2).
- **Browser posture (web app):** strict CSP (MDN CSP page VERIFIED present, 2026-09-24):
  no inline/eval, restricted `connect-src`; cross-origin isolation via COOP/COEP
  (required for threads, doc 12 §1) doubles as side-channel hardening. No privileged
  browser extension in v1; CORS is irrelevant to the core because the core performs no
  network I/O (rule 8 below) — all egress goes through the shell with user-visible actions.
- **Secrets handling:** user tokens for cloud export are NEVER stored in project
  files or the command log — a token in the log would replay into network calls on
  every open (E-003 replay semantics). Store in OS keychain (desktop) / origin storage
  outside the project document (web); redact secrets from logs, receipts, and crash
  dumps. MCP form-mode elicitation must not request secrets (doc 27 §2.2).

## 8. Crash-only design = security-relevant reliability

Checkpoint + replay (E-003) gives: kill-9 safety, no partial writes, and a corruption
recovery path — rebuild from last checkpoint + longest valid hash-chained log prefix;
invalid tail = quarantined for forensics, not executed. Failure is contained the same
way for accidental corruption and tampering.

## 9. Concrete rules (v1 minimum)

1. Parse media only behind boundary 1 (sidecar/worker/wasm); core never demuxes in-process.
2. Validate every command payload: schema (E-003) + referential + range checks; fail-closed.
3. Cap resource usage per command batch (op count, payload bytes, memory, time budget)
   — resource exhaustion is a security failure.
4. No network I/O from the engine core; all egress via shell with user action.
5. Project file = structured data only; no code execution, no eval, no macros (§3).
6. Archive import: zip-slip rules (§3) + decompression-bomb caps.
7. Ingest caps per file: probe bytes, decode buffer, container nesting (§2.4).
8. Plugins = wasmtime with deny-by-default capability imports; native dylib plugins
   excluded from v1 (§4).
9. Media-derived text = escaped data into any model context (§5.1).
10. Secrets never in project files/log; keychain/origin storage; redact outputs (§7).
11. Signed updates, non-disableable verification (S-3e4); plugin bundles hash-pinned.
12. CI supply-chain gates: cargo-audit + cargo-deny on every PR (S-3e8/S-3e9).

## 10. ADR-033 connect + risk register

- This doc is the evidence base for **ADR-033 (Security)**; the sandboxing ladder (§4)
  is its proposed core decision; §9 is its acceptance checklist.
- Extends existing risks: R-09 (AI destructive actions — docs 26/27) and R-08 (IPC
  JSON — E-006) now have rule-level mitigations (§7, §9.3).
- PROPOSED new rows (register not edited by this agent): **R-13** hostile-media parser
  exploit reaching the core process (mitigation: §2 boundary-1 rule, sidecar split);
  **R-14** malicious project-file import from untrusted sources (mitigation: §3
  validator + zip-slip rules).

## Sources (fetched 2026-09-24)

- S-3e0 ffmpeg.org/security.html — 513 CVE ids, reporting policy, per-branch fixes.
- S-3e1 google/oss-fuzz projects/ffmpeg/project.yaml — fuzz engines + sanitizers.
- S-3e4 v2.tauri.app/security/capabilities/ + /plugin/updater/ — ACL + mandatory signatures.
- S-3e5 docs.wasmtime.dev/security.html — sandbox guarantees + Spectre notes.
- S-3e6 security.snyk.io/research/zip-slip-vulnerability — path traversal class.
- S-3e8 rustsec.org — cargo-audit/cargo-deny tooling.
- S-3e9 crates.io API — cargo-deny 0.20.2, cargo-auditable 0.7.6, wasmtime 49.0.0,
  minisign-verify 0.2.5, blake3 1.8.7.
- MDN Content-Security-Policy page (status-verified; details = topic-level).
- Internal: E-003, E-004a records; docs 05/06/12/17/19/26/27/28.

## UNVERIFIED / NOT-FOUND (this pass)

- Exact Tauri updater signing primitive (minisign-family assumed; CLI `tauri signer`
  docs not fetched) and plugin-signing chain.
- wasmtime fuel/epoch interruption details (doc 28 owner).
- BIGSLEEP tag interpretation (inference, marked in §2).
- GStreamer security/CVE policy page (fetch blocked; raw 404).
- Apple ProRes SDK licensing, Avid DNxHR terms (affects proxy formats; doc 35).
- Per-CVE component-level detail (FFmpeg page lists ids + hashes, not components).
- WebKitGTK isolation-pattern specifics on Linux; browser-renderer crash-containment
  behavior for wasm decode fallbacks.

## Depth remaining (v0.2)

- Replay OSS-Fuzz/known reproducer corpus against our ABI boundary as an engine
  experiment (E-0xx proposal) — measure crash containment, not parser fixes.
- Tauri Isolation-Pattern PoC for untrusted-frontend scenarios (doc 17 §1 follow-up).
- Validator as a separate crate with property tests: fuzz command logs (cargo-fuzz)
  for panics on the import path; zip-slip test suite for the archiver.
- Threat-model workshop per platform (browser/Android/desktop) before ADR-033 ACCEPT.
- Secrets e2e flow design (keychain + receipts redaction) with doc 30 (headless auth).
