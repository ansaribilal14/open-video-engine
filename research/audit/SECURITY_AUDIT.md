# SECURITY AUDIT (forensic, 2026-09-26)

> Question answered: are there secrets, unsafe patterns, or unaddressed threat-model
> items in the current repo state? Method: full-history secret scan; tracked-file
> inventory review; doc 33 rules vs current practice; experiment harness safety review.

## 1. Secret scan (critical, given the PAT incident history)

| Check | Method | Result |
|---|---|---|
| PAT/token patterns in **full git history** | `git log --all -p` grep for `ghp_*`/`gho_*`/`github_pat_*` (20+ char forms) | **0 matches** |
| PAT in working tree | grep across tracked files | 0 matches |
| `.env` handling | .gitignore excludes `.env`/`.env*`; no .env committed | clean |
| worklog (where tokens were historically discussed) | lives OUTSIDE the repo (`/home/z/my-project/worklog.md`) — verified not tracked | clean by design |
| doc 39 environment note | records that a 401-invalid token was used for probes and "never written to any file" | consistent with scan |

**Standing rule (binding):** rotated PATs are used only for git push operations in the
session environment; never pasted into repo files, logs, or experiment records. Users
should continue rotating tokens that have appeared in chat contexts.

## 2. Experiment harness safety

- E-001/E-006 CDP harnesses launch local Chrome headless; no external endpoints beyond
  fetching the project's own test media; no eval of remote content.
- E-007/E-007b invoke ffmpeg CLI via python subprocess (documented sandbox quirk:
  standalone ffmpeg killed by sandbox, subprocess workaround recorded) — inputs are
  locally generated testsrc media; no untrusted media parsing in-repo yet.
- No network-listening services committed; E-009's MCP server is stdio-based.

## 3. Threat-model posture for the slice (from doc 33 — verified present in corpus)

| Rule (doc 33) | Current status | Slice obligation |
|---|---|---|
| Untrusted media parsed in sidecar/isolated boundary (FFmpeg 513-CVE history) | rule only | ove-decode: libav* linkage confined to adapter crate; crash containment via process/worker boundary documented as ADR-004 risk |
| Command-payload validation (7 rules, extends E-003: explicit ids, no floats, ranges) | rule + E-009 edge rejection | ove-project/ove-timeline: schema validation on every command load, reject-on-invalid policy |
| Project files as attack surface (zip-slip, XXE-class) | rule | ove-project: no archive extraction v1; JSON schema validation; no code execution in project load |
| Supply chain: cargo audit/deny | not yet in CI | D-3 CI work includes audit+deny |
| Sandboxing ladder (desktop shell) | design only | deferred to shell phase (not slice) |
| MCP tool-description poisoning (P-017) | cited in corpus | ADR-027 work; receipts/tiers already in ADR-010 design |

## 4. Rust-code safety review (the only engine code)

- ove-time: no unsafe blocks; panics are documented invariants (den>0, overflow context)
  rather than UB paths; no external input parsing surface yet. No security findings.
- Future crates inherit the panic-containment pattern proven by E-004a at FFI edges.

## 5. Verdict + actions

**Clean.** No secrets in history or tree; harnesses are local-only; the corpus's threat
model is unusually mature for this stage. Slice obligations are recorded above and in
ENGINE_BUILD_PLAN (CI with audit/deny; adapter-isolated libav* linkage; schema-validated
command ingestion).
