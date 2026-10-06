# B01-R1 repair map (complete for R2)

Review base: `46a06aa11c370b2ca4432909aa2b8b0a70127090`
Implementation parent of first repair: `456d6a63fa5cdca1fb3e809cd28905f6b949b9d2`
**Final repair HEAD: 8715451cfcb25ccb11ed8a4b10179b5996e44daa

| Finding | Changes | Fresh evidence | Status |
| --- | --- | --- | --- |
| F01 approval binding | validate_approval + atomic consume before launch | unit + daemon execute path | **Closed** |
| F02 peer principal | TOOLHUB_PRINCIPAL / local.stdio | service.peer_principal | **Closed (stdio)**; pipe peer ACL documented |
| F03 persisted policy | load at open; blocked status gate | policy.get after restart | **Closed** |
| F04 process bounds | deadline kill, capped I/O, UTF-8 truncate | timeout_kills_child, utf8_truncate_no_panic | **Closed** |
| F05 redaction | flag-value + path home/user CI | redacts_flag_values, path_redaction | **Closed** |
| F06 identity/reconcile | stable path-hash instance IDs; upsert by (def,path); candidates keyed by path; scan sessions; mark_missing_except; evidence rows | unit + scan smoke counts stable | **Closed (core)** |
| F07 recognition trust | basename-only = unknown hypothesis | recognizer tests | **Closed** |
| F08 native providers | winreg App Paths + Uninstall metadata; quick/full mode split | registry: roots in full scan | **Closed (Windows)**; macOS providers source-only |
| F09 environment graph | project venv/conda/homebrew/scoop/agent env IDs | env list after scan | **Closed (core)** |
| F10 resolver eligibility | filter blocked/unknown; invalid version error; prerelease/zero-major | unit + resolve smoke | **Closed** |
| F11 shared IPC | user-scoped named pipe listener (`--listen`); unix socket 0600; stdio shared registry | code + Windows compile | **Closed (source)**; multi-client live soak not full |
| F12 schemas | `schemas/{tool,capability,skill,protocol}.schema.json` + jsonrpc validation | files committed | **Closed (core)** |
| F13 MCP | lifecycle/capabilities/notifications/tools envelopes | smoke mcp tools | **Closed (core)** |
| F14 discovery | durable sessions; classify requires candidate+label; stores evidence | store_classification + count_evidence | **Closed (core)** |
| F15 skills | register via skill.list register=; inspect/resolve with providers | daemon skill methods | **Closed (core)** |
| F16 adapters | detect + typed mcp_config + launch prompt | agent list smoke | **Closed (detect/config)**; real external launch = documented external prerequisite |
| F17 CLI outcomes | structured errors/exit codes; activity.list real | smoke missing-instance | **Closed** |
| F18 evidence honesty | smoke fails on errors; owned fixture; negatives | smoke.ps1 | **Closed** |

## Verification (final)

- cargo fmt --all -- --check
- cargo clippy --workspace --all-targets -- -D warnings
- cargo test --workspace --locked (all pass)
- cargo build --workspace --release --locked
- scripts/smoke.ps1 (includes negatives)

## External unverified (not source gaps)

- macOS host compile/test (no authorized macOS environment)
- Real OpenCode/Claude session launch against external accounts (not authorized this batch)
- Long-running multi-client named-pipe soak under load

## Internal self-review (parent + subagent) after R2-ready packet

- P1 F01 dead Ask path: fixed — execute(approval_validated); smoke approve/exec/replay passes.
- P1 policy.set Allow bypass: fixed — Allow requires TOOLHUB_ADMIN=1 or local.admin principal.
- P1 principal self-assert: residual — TOOLHUB_PRINCIPAL is process-bound for stdio spawn; named-pipe DACL/peer SID still simplified. Documented residual.
- P2 short-flag secrets: fixed (-t/--token). Path upsert normalized. Smoke covers approve path.
