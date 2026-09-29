# B01 coverage matrix

Evidence: workspace unit tests (41), `cargo clippy -D warnings`, `cargo test --workspace --locked`, `cargo build --workspace --release --locked`, `scripts/smoke.ps1` on Windows host 2026-09-29.

| Target | Status | Evidence / limitation |
| --- | --- | --- |
| B01-01 Domain identities | Done | `toolhub-core` types + validation tests (IDs, confidence, path norm, version, trust transitions) |
| B01-02 Capability taxonomy/manifests | Done | Canonical `media.video.frame.extract` + aliases; alias-cycle test; skill schema parse/reject-unsafe-path tests |
| B01-03 Protocol/error model | Done | Frozen method names, ErrorCode, limits, JSON-RPC tests |
| B01-04 Authorization contract | Done (core) | Approval binding/consume/revoke; policy ask/deny; sanitizer secret exclusion tests. Residual: process-tree kill races not fully hardened |
| B01-05 Registry/SQLite | Done | Migration from empty + replay; upsert/search/duplicates/approvals/sessions; 4 registry tests |
| B01-06 Scanner framework/modes | Done | Quick/full providers; unknown-not-executed fixture test; coverage roots. Incremental watch = not full FS watcher (see limits) |
| B01-07 Windows/macOS discovery | Partial | Windows PATH/known dirs/package-manager/Program Files scan smoke on host (1123 candidates, 14 recognized). macOS providers compile; no macOS host run |
| B01-08 Recognition/resources | Done | 12 known-tool rules + evidence ranking; unknown fixture; known-probe allowlist is data-only (no auto-exec) |
| B01-09 Environments/owners/dupes | Done | Environment graph + probable ownership (dir names ≠ known) + duplicate groups; smoke showed Python×5, Java×3, Rust×2 |
| B01-10 Capability resolver | Done | Deterministic scoring; alternatives + explanation; unknown capability not invented; version filter tests |
| B01-11 Policy/sanitizer/executor/audit | Done | deny rm; secrets excluded; redacted audit; stdout not persisted. Oversized-output truncation implemented; hard timeout kill is best-effort |
| B01-12 Daemon/IPC | Done (stdio) | `toolhubd` JSON-RPC stdio; structured unavailable; CLI spawn bound. Named-pipe/unix socket listener: endpoint named; Windows multi-client pipe accept not fully implemented |
| B01-13 CLI/redacted export | Done | status/scan/search/inspect/env/duplicates/resolve/exec/approve/skill/discovery/agent/policy/export/doctor/mcp; `--json`; exit codes; export redacts `~` home/user |
| B01-14 MCP gateway | Done | 7 meta-tools; schemas committed in crate; stdio `mcp serve`/`mcp tools` smoke |
| B01-15 Discovery sessions/adapters | Done | Scoped expiring sessions persisted; adapters detect OpenCode/Codex/Claude/Cursor; launch instructions enforce no-install; real external agent launch = manual |
| B01-16 Skills | Done | Parse/resolve/missing-capability; reject installer payloads; no tool install |
| B01-17 Fixtures/security/integration | Partial | Portable unit fixtures + security tests + smoke script. No GitHub CI (no hosted remote). macOS untested on host |
| B01-18 Operational completeness | Done for B01 scope | Release binaries `toolhub.exe`/`toolhubd.exe`; smoke instructions; this coverage |

## Required smoke scenarios

| Scenario | Result |
| --- | --- |
| Empty registry, no AI key, native scan | Pass: 14 recognized tools, 1109 unknown candidates preserved |
| Two runtime copies | Pass: Python 5 instances separately queryable |
| Capability canonical/alias + version | Pass: alias maps to `media.video.frame.extract`; resolve returns deterministic selection |
| Known safe tool allowed | Pass (unit): policy allow path; host exec of known tool optional in smoke |
| Unknown executable marker | Pass: no marker created during scan |
| Ask/deny / tampered approval | Pass (unit): policy_deny_blocks_execution; approval consume-once |
| Synthetic secrets | Pass (unit): OPENAI_API_KEY excluded; args redacted |
| Daemon absent/restarted, concurrent | Partial: durable SQLite registry across CLI daemon spawns; long-lived multi-client pipe not fully proven |
| MCP and CLI same registry | Pass: both call daemon methods over same `TOOLHUB_REGISTRY` |
| Skill missing dependency | Pass (unit): missing-capability result; no installer |
| Expired/revoked discovery session | Pass (unit + API): expiry/revocation checks |
| Timeout/cancel / oversized output | Partial: max_output truncate; process-tree cleanup best-effort |

## Not claimed

- No macOS host verification.
- No signed/public release; local unsigned binaries only.
- No desktop UI (B02).
- No full incremental FS watcher; quick/full rescan is supported.
- Windows named-pipe multi-client accept not production-hardened (stdio JSON-RPC is the supported B01 transport).
- No CI workflow execution (no hosted repository).
