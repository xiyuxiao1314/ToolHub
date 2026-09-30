# B02 integrated coverage — R3

HEAD: `706fcc5e15b9a1130085796f2f81d43df5593c09`

Legend: **V** = verified this host | **I** = implemented (code+tests) | **P** = partial | **U** = unverified-host | **O** = open

## R2 findings

| ID | Status | Evidence |
| --- | --- | --- |
| R2-B01 authority | I/V | per-connection `pipe.conn.*` principal; admin excludes pipe; approval policy/trust digest; tests approve denial + replay |
| R2-B02 shared IPC | I/P | ConnectNamedPipe accept + CLI shared transport; soak 50 reqs; multi-client pipe load not full |
| R2-B03 process bounds | I/V | timeout+taskkill /T; bounded I/O collect; execute.cancel; unit timeout test |
| R2-B04 scan integrity | I/V | path identity upsert returns stored id; blocked/user_trusted preserved |
| R2-B05 trust | I/V | MZ/ELF+size before Known; renamed text stays unknown (unit+smoke) |
| R2-B06 resolver | I/V | arch/trust prefs; invalid version rejects; blocked-only explanation |
| R2-B07 protocol/MCP | I/V | jsonrpc 2.0 validate; notifications silent; MCP conformance 6/6 |
| R2-B08 discovery | I/V | session principal isolation; classification untrusted evidence |
| R2-B09 skills | I/V | path/hook rejection; registry-backed inspect/resolve |
| R2-B10 CLI/report | I/V | semantic exits; report import untrusted paths; activity.list |

## B01-01..18 (mapped)

| Target | Status | Notes |
| --- | --- | --- |
| B01-01 identities | I/V | stable path-hash IDs, confidence/trust split |
| B01-02 capabilities/schemas | I/V | canonical taxonomy, schemas/*.schema.json |
| B01-03 protocol | I/V | methods, errors, limits, conformance tests |
| B01-04 authorization | I/V | approval binding + admin gates |
| B01-05 registry | I/V | SQLite migrations, evidence, sessions |
| B01-06 scanner modes | I | quick/full, scan sessions |
| B01-07 OS discovery | I/U | winreg providers; macOS unverified |
| B01-08 recognition | I/V | MZ corroboration, fixtures |
| B01-09 environments | I | project/venv/conda/agent tags |
| B01-10 resolver | I/V | eligibility + explanation |
| B01-11 execution/audit | I/V | sanitizer, redaction, timeout |
| B01-12 daemon/IPC | I/V | shared service + soak |
| B01-13 CLI/report | I/V | commands, exits, export |
| B01-14 MCP | I/V | 7 metatools + conformance |
| B01-15 sessions/adapters | I/P | principal sessions; real launch U |
| B01-16 skills | I/V | register/inspect/resolve |
| B01-17 QA | I/V | soak+MCP+smoke+package |
| B01-18 operational | I/V | package.ps1, runbook, checksums |

## B02-01..22

| Target | Status | Notes |
| --- | --- | --- |
| B02-01 shell | I/P | local SPA desktop (Tauri packaging P) |
| B02-02..10 pages | I | nine pages over live RPC |
| B02-11 connected agent | P | detect/config; launch U |
| B02-12 temp credentials | I/V | memory store tests |
| B02-13 extensions | I | trait + fixture host |
| B02-14 resources | I/V | package loader + tests |
| B02-15 updates | I/V | verify/apply/cancel tests |
| B02-16 cross-client | I/P | shared registry tests; UI soak P |
| B02-17 usability | P | reduced-motion CSS; full a11y pass P |
| B02-18 Windows RC | I/V | package.ps1 + SHA256SUMS |
| B02-19 macOS | U | no host |
| B02-20 QA/privacy | I/V | secret/redaction tests |
| B02-21 docs/SDK | I | DEVELOPER.md + TS SDK |
| B02-22 packet | I | this delivery set + bundle |

## Internal reviews

Three subagent reviews (security / scan-trust / protocol) at `1da0b64`; P1 fixes through `e46dccc`; residual listed in decisions.
