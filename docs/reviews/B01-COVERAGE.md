# B01 coverage matrix (R2-ready)

| Target | Status | Evidence |
| --- | --- | --- |
| B01-01 identities | Implemented / Verified | stable path IDs, validation tests |
| B01-02 schemas/resources | Implemented | schemas/*.schema.json + skill/protocol tests |
| B01-03 protocol/errors | Implemented / Verified | jsonrpc 2.0 validation, ErrorCode |
| B01-04 authorization | Implemented / Verified | F01/F02 + unit |
| B01-05 persistence | Implemented / Verified | migrations, evidence, scan sessions, approvals reload |
| B01-06 scanner modes | Implemented | quick/full split, scan sessions |
| B01-07 OS discovery | Implemented (Windows) / unverified-host (macOS) | winreg providers + smoke |
| B01-08 recognition | Implemented / Verified | F07 trust separation |
| B01-09 environments | Implemented | project/venv/conda/agent env detection |
| B01-10 resolver | Implemented / Verified | F10 |
| B01-11 execution/audit | Implemented / Verified | F04/F05 |
| B01-12 daemon/IPC | Implemented | user-scoped named pipe source + stdio shared registry |
| B01-13 CLI/reporting | Implemented / Verified | F17 + activity.list |
| B01-14 MCP | Implemented / Verified | F13 |
| B01-15 sessions/adapters | Implemented | F14/F16 detect+config; external launch unverified-host |
| B01-16 skills | Implemented | F15 register/inspect/resolve |
| B01-17 QA | Implemented | hardened smoke + full unit suite |
| B01-18 delivery | Implemented | this R2-ready packet |

Legend: Implemented = source complete for B01 scope; Verified = automated check on this host; unverified-host = no authorized environment.
