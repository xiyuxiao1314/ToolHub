# B01 decisions

Provisional contract version: `toolhub.core/v1`, `toolhub.skill/v1`, `toolhub.manifest/v1`, THP `1.0`.

## Identity and model

- Capability canonical spelling: `media.video.frame.extract`. Legacy `media.frame.extract` / `media.video.extract.frame` are aliases only (`toolhub-core` `capability.rs`).
- Definition / Instance / Interface / Capability / Environment / Owner / Origin / Evidence / Trust / Candidate / Agent / Skill / Session / Execution are separate types.
- Confidence (unit interval), OwnerCertainty (known/probable/unknown), and TrustLevel (blocked/unknown/user_trusted/known/verified) are independent. Numeric confidence is never execution authority.
- Multiple instances of one definition are preserved; duplicate analysis never merges or deletes.
- Path normalization: Windows comparison keys lowercase and unify separators; original/canonical paths both stored.

## Protocol

- JSON-RPC 2.0 methods frozen in `crates/protocol/src/methods.rs` (`ping`, `status`, `scan.*`, `registry.*`, `resolve.capability`, `execute.*`, `skill.*`, `discovery.*`, `agent.list`, `policy.*`, `export.report`).
- Stable `ErrorCode` set including `denied`, `expired`, `policy_denied`, `trust_blocked`, `approval_invalid`, `session_invalid`, `unavailable` with `fallback_allowed` as caller information only.
- Limits: max request 1 MiB, max output 256 KiB, default timeout 30s.

## Authorization and safety

- Discovery is read-only; unknown executables are never auto-executed (including `--version`/`--help`).
- Execution approval binds agent/session, instance, executable SHA-256, args digest, cwd digest, env digest, expiry; single-use consume + revoke.
- Environment sanitizer allow-lists PATH/HOME/TEMP/locale-class variables; secret-bearing names (`*TOKEN*`, `*SECRET*`, `*API_KEY*`, AWS/GITHUB/etc.) excluded by default with per-tool opt-in.
- Policy scopes: command > tool > capability > agent > directory > environment; default unknown tool = Ask; `rm`/`del` = Deny.
- Discovery sessions have scoped permissions separate from execution approval; classification is untrusted enrichment.
- Skill registration refuses installer/hook payloads; missing capabilities are reported, never installed.
- MCP exposes 7 meta-tools only (not one per installed instance) and shares daemon policy/audit.

## Persistence

- SQLite (`rusqlite` bundled) with migration `001_init.sql` from empty DB; WAL + foreign keys; schema_migrations table.
- Forward migrations only; replay is idempotent. No destructive down-migration requirement.
- Temporary AI credentials are never written to SQLite/config/logs.

## Crate mapping (consolidated from design)

| Design area | Crate(s) |
| --- | --- |
| Domain | `crates/core` |
| Protocol/manifests | `crates/protocol` |
| Registry/SQLite | `crates/registry` |
| Scanner framework + OS providers | `crates/scanner` |
| Recognition | `crates/recognizer` |
| Environment/ownership/duplicates | `crates/environment` |
| Resolver/policy/executor/audit | `crates/{resolver,policy,executor,audit}` |
| IPC | `crates/ipc` |
| MCP | `crates/mcp` |
| Skills | `crates/skills` |
| Agent bridge/sessions | `crates/agent-bridge` |
| Daemon/CLI | `apps/{daemon,cli}` |

Desktop UI remains B02. Integration/QA lives in `scripts/smoke.ps1` and workspace tests.

## Host toolchain

- `rustc`/`cargo` 1.95.0 at `C:\Users\hp\.cargo\bin` (absolute path).
- VS Build Tools 2022 + MSVC 14.44 + Windows SDK 10.0.26100 installed with owner authorization, without local proxy `:7897`.
- Build/verify via `vcvars64.bat` environment; no machine PATH mutation required.
