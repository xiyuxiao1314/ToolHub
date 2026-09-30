# B02 integrated coverage (work in progress toward R3)

Branch: `codex/toolhub-b02-integrated`
Base: `6df2166` (Codex R2 policy fix adopted)

Status legend: Implemented | Verified (this host) | Partial | Open

## R2 blockers

| ID | Status | Evidence |
| --- | --- | --- |
| R2-B01 authority | Partial | approve requires admin; policy Allow gated; principal still process-env bound |
| R2-B02 shared IPC | Partial | ConnectNamedPipe accept + CLI shared transport; multi-client soak not full |
| R2-B03 process bounds | Improved | timeout + taskkill /T tree kill; join bounds still best-effort |
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
| B02-18 Windows RC | Partial — release binaries exist |
| B02-19 macOS | Unverified-host |
| B02-20 QA/privacy | Partial |
| B02-21 docs/SDK | Open |
| B02-22 delivery packet | Open (this doc is interim) |
