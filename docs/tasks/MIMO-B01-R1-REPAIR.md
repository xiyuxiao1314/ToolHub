# MIMO-B01-R1-REPAIR — one integrated core repair delivery

## Assignment and authority

Continue the existing B01 development assignment in `D:\ToolHub-mimo\checkout`, under `--profile mimo`, on workbench task `task_a11c48d272163020b1ee37f1`. MiMo remains the primary developer and owns internal subagents, integration, independent self-review and repair. This is one large repair delivery, not a sequence of externally reviewed microtasks.

Formal R1 is complete at bundle HEAD `46a06aa11c370b2ca4432909aa2b8b0a70127090`: **changes_required; 1/4 product-review rounds used**. Its implementation parent is `159bac10c22a18b10a3aa1165b9e521f681cc5b9`; original planning base remains `6ffc9dc3c2fb526e740f0e3b29eafb84f4c54fce`. Verify current work before editing and preserve any newer local work. Do not reset or replace another participant's checkout. Codex's `D:\ToolHub` contains only planning/review documentation and is not an implementation merge/push destination.

Read the published `B01-R1-FINDINGS.md`, `B01-R1-EVIDENCE.json`, selected `B01-R1-REPRO.zip`, and original `MIMO-B01-CORE.md`. The R1 findings contain F01-F18 with source anchors, evidence boundaries and acceptance tests. **Repair all F01-F18 and reconcile every original B01-01 through B01-18 subtarget.** The original full runnable-core scope remains authorized; R1 has not authorized moving missing core functionality to B02.

The review bundle/hash is in the findings/evidence. Codex did not modify product source. Existing 42 tests, fmt, Clippy and Windows release build passed independently, so the repair must preserve those while adding meaningful integrated coverage. The existing smoke output is not acceptance evidence because a negative control proved it can succeed when all tool commands fail.

## Large work packages and dependencies

Use these boundaries as an integration plan, adapting worker count to real dependencies. No Codex approval gate is required between packages. The MiMo parent owns shared contracts, workspace/lockfile, migrations, cross-package changes and final evidence.

1. **Contracts and persistent authority — F01/F02/F03/F06/F12.** Freeze one supported domain/wire/schema/resource version and principal/session/approval model. Define stable IDs/path/version semantics, effective policy revisions, validated writes, migration/recovery and truthful error/event/cancel contracts. Review the threat cases internally before downstream implementation. Update schema/types/client/docs together when the contract changes.
2. **Real service and bounded execution — F01-F05/F11/F13/F17.** Implement shared authenticated Windows pipes/macOS sockets, user-scoped ownership/readiness/reconnect, responsive concurrent services and real approval/policy execution. Enforce process/stdin/output/deadline/cancel/disconnect bounds; safe Unicode capture and complete redaction. CLI and interoperable MCP use this same service, typed errors and principal. Activity is real, not a constant.
3. **Native observations and resolution — F06-F10.** Transactional stable scan ingestion, disappearance/promotion and persisted coverage/events. Actual native metadata providers, differentiated bounded Quick/Full and incremental refresh. Corroborated recognition/resources, version/architecture evidence, real environments/ownership/duplicates and eligibility-based resolver. Keep unknown binaries unexecuted and weak hypotheses untrusted.
4. **Declarative skills and agent enrichment — F14-F17.** Register/read/resolve instruction/MCP/package skills safely and query the actual registry. Implement isolated scoped durable discovery enrichment and adapter config/health/launch using existing authorized runtimes. Reporting/import is versioned, validated and redacted; imported foreign paths never become trusted local executables automatically.
5. **Independent integration repair and truthful delivery — F18 plus all groups.** An internal reviewer checks actual daemon/CLI/MCP behavior and hostile/negative fixtures, not only helper tests. Repair internal findings before handoff. Correct the original target matrix and every inconsistent claim in delivery/decisions/runbooks.

Independent packages may run concurrently after contracts are coherent. Use explicit file ownership and parent-controlled integration; avoid parallel edits to shared schema/model/service wiring. Do not add placeholder crates or alternate disconnected implementations to make a coverage row appear complete.

## Required regression clusters

Each cluster must exercise the real exposed service/client path with safe synthetic tools/metadata. Helper unit tests can supplement it. Keep fixtures portable and prevent real secrets, installed unknown commands, arbitrary shell interpretation or configuration changes.

| Cluster | Required observable cases |
| --- | --- |
| Bound authority | Exact approved Ask succeeds once; args/cwd/stdin/env/caller/session/binary/policy/trust changes fail; expiry/revoke/replay/concurrent reuse; Deny wins; spoofed labels cannot alter authority |
| Effective policy | All supported scopes, persistence/update/restart, lower-specificity Allow vs environment/capability Deny, blocked/missing status, invalid rule inputs |
| Process bounds | Deadline and cancellation kill safe child/grandchild; bounded stdout/stderr/storage; multibyte cap crossing; large bidirectional I/O; following query works; disconnect/cleanup semantics |
| Privacy | Neutral random sentinel option values absent from audit/log/error/export; inherited secrets excluded; default path redaction across Windows case/slash/UNC/short names and macOS homes |
| Persistent observations | Two scans refresh stable rows; duplicates do not grow; removed/replaced/promoted tools; partial-root uncertainty; transactional FK order; evidence/interfaces/scans; recovery/concurrency/forward-schema refusal |
| Native recognition | Actual metadata providers and realistic Windows/macOS fixtures; unknown renamed executable never auto-runs/gains trust by basename; genuine measured version/architecture; incompatible resources rejected |
| Environments/resolver | Two project venvs plus Conda/package/agent contexts; ownership uncertainty/corrections; missing/blocked/arch/scope/trust/version eligibility; aliases; invalid/prerelease/zero-major constraints |
| IPC/lifecycle | Two clients share one live service; user-scoped secure endpoint; unauthorized peer; duplicate startup; bounded startup/reconnect/EOF; scan/exec/query/cancel overlap; restart state |
| Protocol/MCP | Every public typed response conforms; invalid version/id/params, notifications, size bounds, parse errors and event ordering; actual MCP initialize/list/call plus error followed by success |
| Discovery/adapters | Principal and candidate isolation, individual scopes, authoritative cross-client revoke/expiry; rejected unsolicited/empty enrichment; durable untrusted valid classification; valid serialized config and real supported launch or exact external blocker |
| Skills/reporting | All declarative skill kinds and safe containment; malformed schema/types/version/path/hook cases; multiple providers and blocked/optional requirements; CLI/MCP shared queries; typed redacted report/import; foreign paths stay unavailable/untrusted |
| Evidence honesty | Failing shim makes smoke fail; actual unknown marker fixture exists; known safe installed tool integration; all original target subitems mapped to code/check/result/remaining limitation |

Codex reproduction scripts target the reviewed schema/stdio baseline. Run them only from a fresh owned fixture directory and adapt their calls/assertions to intentionally changed authoritative contracts. Treat old outputs as defect evidence, not as expected successful repair outputs. Include equivalent regression assertions in the project; do not rely exclusively on uncommitted reviewer probes.

## Verification and platform boundaries

Run `cargo fmt --all -- --check`, `cargo test --workspace --locked`, `cargo clippy --workspace --all-targets --locked -- -D warnings`, `cargo build --workspace --release --locked`, `git diff --check`, and meaningful assertion-based integration/smoke commands. Offline mode is optional if locked dependencies already exist. Record exact commands, toolchain/target, exit status and test totals. Do not modify global PATH; use existing absolute tool paths/session-local toolchain setup.

Repair Unix source/test compilation defects, including the missing writer reference and executable-fixture cfg/permissions. Obtain and record actual macOS compile/test results where an authorized available environment permits. If unavailable, record **unverified host/toolchain**, not compile-only/pass. Commit native fixtures and platform-check configuration without implying a hosted CI run or acquiring a remote account.

No installation/login/remote account, public release/signing, remote push, implementation merge into main, workbench completion or archive is authorized by this brief. Continue all independent repair work if a real external prerequisite blocks a specific host/adapter check. Report the exact prerequisite instead of using it to mark absent source functionality complete. Desktop remains B02.

## One corrected delivery packet

Publish these selected immutable artifacts to the same workbench using the MiMo profile, then mark the participant submitted and post one short Chinese owner summary:

- `B01-R1-REPAIR-DELIVERY.md`: exact original review base, corrected HEAD/branch, clean status, verification outcomes/toolchain and honest limitations.
- `B01-R1-REPAIR-MAP.md`: F01-F18 individually mapped to changed files/commits, real regression commands and fresh results; any unresolved finding identified explicitly. No finding is omitted or closed solely by documentation.
- `B01-COVERAGE.md` new version: every original target and subtarget mapped to implementation, automated integration evidence, host evidence and status. Separate implemented, verified, unverified-host, blocked-access and not-implemented.
- `B01-DECISIONS.md` new version: final domain/protocol/security/resource/adapter changes and compatibility/remaining race assumptions.
- `ToolHub-B01-R1-repair.bundle`: cloneable clean exact revision with SHA-256 manifest and base ancestry. Ensure delivery documents and dispatch event identify the actual bundle HEAD consistently.
- Selected safe fixture/log evidence for critical authority/process/protocol/native/client paths; no full environment or machine scan inventories, real secrets or conversations.

This packet triggers **one R2**. Codex then verifies repairs and may apply/test bounded localized corrections in isolation. R2 is not intended to implement major missing subsystems. If substantial blockers remain, B01 remains unaccepted and the owner decides the next scope/workflow; the four-round ceiling never justifies fabricated acceptance or an automatic fifth review.

## Owner summary

请 MiMo 按 F01-F18 集中修复并补齐 B01 核心闭环，内部集成与自审后一次提交修复包；Codex 下一步只做 R2 复审，当前已使用 1/4 次审查。
