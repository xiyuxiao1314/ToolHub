# B01 coverage matrix (R1-repair)

| Target | Status | Evidence |
| --- | --- | --- |
| B01-01 identities | Implemented / Partial | core types + validation; path identity still improving |
| B01-02 schemas/resources | Partial | capability aliases + skill parse tests; no full schema package |
| B01-03 protocol/errors | Implemented (core) | jsonrpc 2.0 validation, stable ErrorCode, limits |
| B01-04 authorization | Implemented / Verified (unit+smoke) | F01/F02 repairs; approval binding tests |
| B01-05 persistence | Implemented | SQLite migrations; policy/approval/session reload |
| B01-06 scanner modes | Partial | quick scan smoke; incremental watch not done |
| B01-07 OS discovery | Partial (Windows host) | PATH/known dirs/package smoke; Registry provider still heuristic; macOS unverified |
| B01-08 recognition | Implemented (core) | F07 trust separation |
| B01-09 environments | Partial | kind preserved; full graph weak |
| B01-10 resolver | Implemented | F10 eligibility + version grammar |
| B01-11 execution/audit | Implemented / Verified | F04/F05 + timeout unit + redaction tests |
| B01-12 daemon/IPC | Partial | stdio shared registry; named pipes open |
| B01-13 CLI/reporting | Implemented (core) | F17 structured errors/exit |
| B01-14 MCP | Implemented (core) | F13 lifecycle/list/call envelopes |
| B01-15 sessions/adapters | Partial | sessions durable; launch not real |
| B01-16 skills | Not fully implemented | list empty; registration path missing |
| B01-17 QA | Partial | hardened smoke + 47 unit tests; no CI |
| B01-18 delivery | Implemented | this packet + bundle |

Legend: Implemented = code path exists; Verified = automated/host check ran here; Partial = incomplete; Not fully implemented = open for next pass.
