# B01-R1 repair map (F01–F18)

Base reviewed: `46a06aa11c370b2ca4432909aa2b8b0a70127090`
Repair HEAD: *(this commit)*
Branch: `codex/toolhub-b01-core`

| Finding | Change | Fresh evidence | Status |
| --- | --- | --- | --- |
| F01 approval binding | `executor::validate_approval` (args/cwd/stdin/env/binary/principal); daemon consumes approval atomically before launch | unit: approval consume-once; smoke exec path uses validated approval for Ask | **Repaired** |
| F02 peer principal | `TOOLHUB_PRINCIPAL` / `local.stdio`; policy+approval use transport principal, not spoofable labels | code path in `service.rs` `peer_principal()` | **Repaired (stdio scope)**; named-pipe peer ACL still open (F11) |
| F03 persisted policy | load rules at `DaemonService::open`; blocked/missing status gate; env in PolicyContext | policy.get after restart uses stored rules | **Repaired** |
| F04 process bounds | threaded stdin/stdout/stderr with caps; deadline + kill; UTF-8-safe truncate | unit: `timeout_kills_child`, `utf8_truncate_no_panic` | **Repaired** |
| F05 redaction | flag-value + assignment redaction (`--api-key=`, `--token v`); case-insensitive home/user path replace | unit: `redacts_flag_values`, `path_redaction_home_and_user` | **Repaired** |
| F06 registry identity | *(partial)* path-based upsert conflicts; candidates still regenerate IDs | smoke rescan | **Partial** — stable ID by normalized path + transactional reconcile remains |
| F07 recognition trust | basename-only → `TrustRecord::unknown` (hypothesis); corroborated path+name → known | unit recognizer tests | **Repaired (core)** |
| F08 native providers | still directory/package heuristics; no live Registry/App Paths read | smoke coverage roots | **Partial / not-fully-implemented** |
| F09 environment graph | env kind no longer clobbered by scan | smoke env list | **Partial** — full venv/conda graph still weak |
| F10 resolver eligibility | filter blocked/unknown before ranking; invalid version constraint is error; prerelease/zero-major caret | unit: version_constraints; resolve smoke | **Repaired (core)** |
| F11 shared IPC | stdio JSON-RPC + durable SQLite shared by CLI calls; Unix compile path cfg-gated | Windows smoke multi-command | **Partial** — Windows named-pipe multi-client listener not hardened |
| F12 wire validation | jsonrpc must be 2.0; id type check | daemon reject invalid envelope | **Partial** — full JSON schemas/fixtures not committed |
| F13 MCP | initialize capabilities; notifications no response; tools/list `{tools:[…]}`; tools/call content envelopes; errors do not kill server | smoke `mcp tools` + code path | **Repaired (core)** |
| F14 discovery sessions | sessions persist+reload; revoke/expiry checks | discovery list after start | **Partial** — per-candidate scope isolation not complete |
| F15 skills | still no real skill package registration path | skill.list empty | **Partial / not-fully-implemented** |
| F16 adapters | detect list only; no real launch | agent list | **Not fully implemented** |
| F17 CLI outcomes | structured `--json` error object + documented exit codes without `process::exit` inside MCP client | smoke missing-instance fails | **Repaired (core)** |
| F18 evidence honesty | smoke fails on command errors; owned unknown fixture; negative missing-instance check | `scripts/smoke.ps1` exit non-zero on failure | **Repaired** |

## Verification (this repair)

- `cargo fmt --all -- --check` — pass
- `cargo clippy --workspace --all-targets -- -D warnings` — pass
- `cargo test --workspace --locked` — **47 passed**, 0 failed
- `cargo build --workspace --release --locked` — pass
- `scripts/smoke.ps1` — pass including failing-shim style negative (missing instance must fail)

## Explicitly unresolved (not closed by docs)

- F06 full transactional identity/reconcile
- F08 live Windows Registry / package metadata providers and full mode matrix
- F09 complete environment graph for project venvs
- F11 authenticated multi-client named pipe / unix socket production listener
- F12 full versioned JSON schema suite
- F14 candidate-level discovery isolation
- F15 declarative skill registration
- F16 real discovery launch through OpenCode/Codex/Claude (external runtime)
- macOS host verification still unverified
