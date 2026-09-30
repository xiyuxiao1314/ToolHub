# B02 integrated coverage (work in progress toward R3)

## Internal review (3 subagents) — 2026-09-30

| Finding | Disposition |
| --- | --- |
| Notifications answered on stdio | **Fixed** — skip no-id |
| Shebang text → Known (Unix) | **Fixed** — ELF/MZ only |
| upsert id vs mark_missing desync | **Fixed** — return stored id |
| Skill Windows drive/UNC paths | **Fixed** |
| execute.revoke unauthenticated | **Fixed** — owner/admin |
| Env-based principal on shared pipe | **Open residual** — needs per-connection SID/token ACL |
| Approval session/policy revision bind | **Partial** — session non-empty check; full revision digest open |
| Timeout I/O join hang | **Partial** — tree kill improved; joins unbounded |
| Resolver blocked-only signal | **Open** — empty vs blocked-only indistinct |



Branch: `codex/toolhub-b02-integrated`
Base: `6df2166` (Codex R2 policy fix adopted)

Status legend: Implemented | Verified (this host) | Partial | Open

## R3 evidence packs (2026-09-30)

| Pack | Tests | Result |
| --- | --- | --- |
| Long-lived daemon soak | `apps/daemon/tests/soak.rs` (4) | 50 mixed requests, scan/query interleave, bad-request recovery, restart durability — all pass |
| MCP conformance | `apps/cli/tests/mcp_conformance.rs` (6) | initialize/capabilities, notification silence, tools/list shape, tools/call + error recovery, error.message string, unsupported method recovery — all pass |
| Shared multi-client | `apps/daemon/tests/integration.rs` (4) | shared registry, non-admin approve denied, invalid jsonrpc, ping/status |

## R2 blockers

| ID | Status | Evidence |
| --- | --- | --- |
| R2-B01 authority | Improved | per-connection pipe principal; admin excludes pipe; approval policy/trust digest |
| R2-B02 shared IPC | Partial | ConnectNamedPipe accept + CLI shared transport; multi-client soak not full |
| R2-B03 process bounds | Improved | timeout + taskkill /T; bounded I/O collect; execute.cancel |
| R2-B04 scan integrity | Improved | preserve blocked trust; path identity upsert |
| R2-B05 trust/env | Improved | MZ/size required before Known; renamed text python stays unknown |
| R2-B06 resolver | Improved | arch/trust prefs wired; invalid version rejects |
| R2-B07 protocol/MCP | Partial | jsonrpc 2.0 validation; MCP error.message string; full conformance suite open |
| R2-B08 discovery | Improved | session bound to principal; cross-principal denied |
| R2-B09 skills | Improved | path/hook rejection; registry-backed inspect |
| R2-B10 cross-client | Partial | CLI semantic exits; activity.list real; full soak open |

## B02 targets (selected)

| Target | Status |
| --- | --- |
| B02-01 Desktop shell | Partial — nine-page local SPA shell (`apps/desktop`), not packaged Tauri |
| B02-02..10 pages | Partial — live RPC views; polish/screenshots pending |
| B02-11 connected agent | Open — detect/config only |
| B02-12 temp credentials | Implemented — memory store + tests |
| B02-13 scanner extensions | Implemented — trait + fixture host |
| B02-14 resource versions | Implemented — package loader + compatibility |
| B02-15 product updates | Open |
| B02-16 cross-client | Partial |
| B02-17 usability | Partial |
| B02-18 Windows RC | Implemented — `scripts/package.ps1` + checksums + uninstall docs |
| B02-19 macOS | Unverified-host |
| B02-20 QA/privacy | Partial |
| B02-21 docs/SDK | Open |
| B02-22 delivery packet | Open (this doc is interim) |
