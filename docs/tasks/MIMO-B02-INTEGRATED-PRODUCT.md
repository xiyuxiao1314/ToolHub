# MIMO-B02 — Integrated ToolHub product delivery, including unaccepted B01 core

## Dispatch and owner decision

**Dispatched by Codex with the owner's explicit approval on 2026-09-30.** This is one very large integrated implementation batch, not a claim that B01 passed. Complete every still-open original B01 target, the consolidated R2 blockers, and **all B02-01 through B02-22** from the two-batch plan. MiMo is the primary developer and decides its internal decomposition, subagent count, dependency order, integration checkpoints and self-review. No per-module Codex approval is required within this boundary.

Formal product reviews used: **R1/R2 = 2 of 4**. The remaining allocation is **R3 = one initial review of the complete integrated delivery; one MiMo consolidated repair; R4 = one verification review with possible bounded Codex corrections**. Internal MiMo reviews, tests, progress and this dispatch do not consume a formal round. Do not represent new B01-only deliveries as R3, reopen R2, or create a fifth formal review implicitly. Four rounds are a ceiling, never a reason to label blocking behavior accepted.

Work on the existing shared task `task_a11c48d272163020b1ee37f1` using `workbenchctl --profile mimo`. Publish concise progress/decision facts when useful, then **one integrated delivery packet** for R3. An extraordinary blocker may be reported without treating incomplete source as a complete delivery. The full original ToolHub design and all its invariants remain the product baseline.

## Authoritative inputs and revision handoff

Read, in this order, `AGENTS.md`, `docs/collaboration/REVIEW_POLICY.md`, `docs/superpowers/plans/2026-09-29-toolhub-two-batch-plan.md`, `docs/tasks/MIMO-B01-CORE.md`, `docs/reviews/B01-R1-FINDINGS.md`, `docs/reviews/B01-R2-RESULT.md`, `docs/reviews/B01-R2-EVIDENCE.json`, and this brief. Also read `README.md`, `ARCHITECTURE.md`, `DOMAIN.md`, `PROTOCOL.md`, `SECURITY.md`, `CONTRIBUTING.md`, `WORKBENCH.md`, the full owner-pasted design, and actual schemas/code. A claim in an earlier self-assessment is evidence to check, not acceptance.

- Latest delivered/reviewed MiMo code: `7a1ba39a8fb27dddffb4eb662a973c8bedf287ed`, from `ToolHub-B01-R1-repair.bundle` v3 (SHA-256 `745b1400125cdc731f2fcf36ea089aba0bfb8c6c1f406c5e04844fd9237e9f60`). The older delivery/map stamp `8715451...` is not this reviewed tip.
- Verified bounded Codex fix: `6df2166e0c55ec64667021e386e75ca93acb066d`, **direct child of `7a1ba39`**, on `codex/toolhub-b01-r2-fixes`. Local and workbench `ToolHub-B01-R2-corrections.bundle` SHA-256 `21c8f94b282c3313b64748760108b89ef98d6bea091f14622237067de0aff325`. Adopt these changes in MiMo's isolated implementation branch, checking ancestry and preserving any newer user work. The changes are atomic policy replacement and persist-before-apply/error propagation, with three regressions. Do not regress them. This correction is not an accepted product baseline.
- MiMo's independent checkout is `D:\ToolHub-mimo\checkout`; it was clean at `7a1ba39` immediately before dispatch. Verify its state anew before changing it. `D:\ToolHub` is Codex's documentation/review main (dispatch-docs tip to be recorded in the dispatch event), **not** a destination for MiMo implementation merges. No hosted remote is configured; `origin` may be local only.
- Existing toolchain: Rust/Cargo 1.95.0, MSVC 14.44/Windows SDK, Git, Node and Python were verified locally. Use absolute paths or session-local environment when needed; do not infer missing tools from sandbox PATH. Ordinary project dependency resolution is in scope; new system toolchain installation, global configuration/PATH changes, new external accounts and credentials require their applicable owner authorization.

## MiMo's internal architecture and delegation

MiMo parent owns shared schemas, dependency manifests/lockfiles, migrations, cross-client contracts, integration, source branch, CI configuration and final packet. Before parallel implementation, reconcile domain IDs, trust/ownership/evidence, JSON-RPC/MCP DTOs, resource/version policy and authority boundary in one provisional contract. Internally review threat cases, then give subagents independent write scopes and read-only dependencies. Propagate parent-approved contract revisions to schemas, daemon, CLI, MCP, TypeScript client, desktop, tests and docs in one coherent change. Do not duplicate domain types or quietly shift safety boundaries. The original `MIMO-B01-CORE.md` directory ownership table is a starting point; MiMo may regroup its own workers while preserving single ownership of shared files.

Suggested lanes, **not** separate Codex review gates: (A) peer identity/approval/policy/executor; (B) registry/scanner/reconciliation/environments/resources; (C) protocol/IPC/CLI/MCP/adapters/skills; (D) desktop/SDK/integration/release. Dependents must wait for the parent-locked contract where necessary. Parent integrates and runs real end-to-end checks before declaring any lane complete. Keep a small decision log for changed interfaces and security choices.

## Required core closure: all B01-01 through B01-18

The original B01 checklist and observable scenarios remain authoritative. Map **every subitem**, not only these summaries, to code and evidence. In particular:

| Original target | Integrated acceptance evidence required |
| --- | --- |
| B01-01 Domain identities | Stable canonical/original path and entity IDs; validated cardinalities, provenance and confidence; separate recognition, ownership and execution trust; meaningful platform path/version normalization. |
| B01-02 Capabilities/manifests | One canonical capability/alias taxonomy, versioned enforced schemas and fixtures, invalid/cyclic alias rejection, independently versioned compatible resources. |
| B01-03 Protocol/errors | Actual typed JSON-RPC methods/DTOs, negotiation, stable errors, cancellation/deadlines, size bounds, events/ordering, compatibility and conformance fixtures across all clients. |
| B01-04 Authorization | Real authenticated transport/caller identity, trusted approval issuance, session/instance/executable/argv/cwd/sanitized-env/policy/trust binding, expiry/revoke/replay and authority-changing policy gate. |
| B01-05 Registry/SQLite | Transactional migrations/writes, stable distinct identities, observed relationships/evidence, disappearing instances, recovery, meaningful concurrent/restart and migration tests. |
| B01-06 Scanner/modes | Read-only candidate discovery, actual Quick/Full/Custom behavior and bounded incremental coverage, normalized dedupe, cancellation/events, unknown executable never auto-probed. |
| B01-07 Windows/macOS discovery | Real supported native sources with explicit provider coverage/errors/metadata, platform-specific fixtures/builds, honest unverified host cases. |
| B01-08 Recognition/resources | Independent corroboration before trust, measured version/vendor/architecture and resource provenance, fallback with uncertainty, malformed/incompatible resource rejection. |
| B01-09 Environments/ownership | Stable project-root/environment identity and graph, distinct same-name projects, correct origin/owner/confidence/size estimates and preserved user corrections. |
| B01-10 Resolver | Actual wire preferences, project/environment/architecture/trust/version eligibility, deterministic ranking and meaningful rejected alternatives/fallback. |
| B01-11 Policy/executor/audit | Default ask/deny behavior, effective cross-client policy, real approval gate, sanitized execution, process-tree bounds/cancellation/output bytes, redacted audit without persistent stdout. Preserve `6df2166` policy fixes. |
| B01-12 Daemon/IPC | One working user-scoped shared service across concurrent clients, authenticated peer identity, single-instance/readiness/connect/reconnect, safe socket/pipe lifecycle and cross-platform compilation. |
| B01-13 CLI/report | Commands operate the shared service; semantic child exit codes; complete redacted useful report and safe import contract; inspect/discovery/skill operations supported. |
| B01-14 MCP | Seven (or documented revised) typed meta-tools, strict request/lifecycle/schema/error/content behavior, actual daemon state, bounded inputs and recoverable malformed requests. |
| B01-15 Discovery/adapters | Candidate/session authorization, per-caller scope and revocation across processes; validated classification provenance; real supported adapter discovery/launch/monitor/cancel path. |
| B01-16 Skills | Validated declarative instruction/MCP/package manifests, package path containment, no hook/tool installation, required/optional capability/version/trust availability via actual resolver. |
| B01-17 QA/integration | Runnable synthetic discovery-to-execution/denial and client-negative fixtures; assertions fail on semantic failure, privacy and concurrency regressions. |
| B01-18 Operational delivery | Usable no-key core, correct docs and versions, packaging, reproducible local artifacts and truthful target/evidence ledger. |

### Consolidated R2 repair priorities

**All ten R2-B01 through R2-B10 findings are required, including their detailed closure paragraphs in `B01-R2-RESULT.md`.** The following are the minimum review probes to defeat false closure:

1. **R2-B01 actual authority:** one unprivileged ordinary client cannot mint execution approvals or weaken Deny/Ask policy; a privileged controller is verified from the transport, not global `TOOLHUB_PRINCIPAL`/caller label. Binding covers canonical runtime cwd, session, instance and policy/trust revisions; changed requests, binary, expired/revoked/replayed approval fail.
2. **R2-B02 shared IPC:** start one daemon, connect multiple independent CLI/desktop/MCP clients, authenticate their peers, survive sequential/concurrent requests and desktop shutdown/restart. Fix Windows accept/instance lifecycle and Unix conditional compilation/socket ownership; negative caller and stale endpoint cases.
3. **R2-B03 process bounds/privacy:** finite child/descendant timeouts and cancellation have bounded return even when pipes remain open; byte-limited output and redaction; no indefinite collector joins or daemon-wide lock serialization. Use only owned harmless fixtures.
4. **R2-B04 scan integrity:** canonical path and legacy ID upserts return the persisted identity; related writes are transactional and report failures; Quick/partial/provider-error runs cannot mark untouched instances Missing; preserve blocked trust/owner corrections; canonical/case duplicates have one identity.
5. **R2-B05 trust/environment/native data:** a renamed plain text `python.exe` is not Known/verified solely by path pattern; distinct projects named `same` retain distinct environments; persisted graph/root/parent and actual measured metadata survive restart; record provider coverage/resources.
6. **R2-B06 resolver/version:** project-b/environment-b/required trust/x86_64 cannot select aarch64 project-a; invalid `>=banana` rejects; exact prerelease and `^0.0.x` semantics hold; blocked-only and rejected alternatives return typed explanations.
7. **R2-B07 JSON-RPC/MCP:** notifications get no reply; scalar params, IDs, oversized messages, invalid versions and malformed JSON follow an enforced bounded contract. Declared schema/result DTOs equal runtime; MCP error.message is a string and lifecycle/tool behavior survives bad calls.
8. **R2-B08 discovery/session:** principal B cannot inspect/revoke/classify principal A's session, including after restart; A's already-running daemon observes revocation. Nonexistent/out-of-session candidate and invalid confidence are rejected; untrusted classification never elevates execution trust.
9. **R2-B09 skill availability:** raw registration must pass manifest/path/type validation; unmet/blocked/version-incompatible requirements are unavailable with reasons, not Available; content import/download remains declarative and bounded.
10. **R2-B10 cross-client evidence:** CLI child exit failure propagates; execution activity actually appears with redaction; report/import and adapters are implemented; smoke uses runnable in-scope unknown fixture and negative assertions that truly observe the behavior. A source stub, mock-only service or absent test is not completion.

## Complete B02 target checklist — no target dropped

Implement and prove each target against the real shared daemon/types. The original plan contains additional wording and remains authoritative.

### Desktop user flows (B02-01–10)

- **B02-01 Desktop shell:** Tauri/React/TypeScript (or recorded equivalent owner-approved contract change), shared service connection, startup/reconnect/error states, generated/validated types; no UI-local registry or scanner.
- **B02-02 Overview:** actual counts, latest scan, new/unknown discoveries, activity and environment changes; honest empty/loading/stale states.
- **B02-03 Tools/detail:** real multi-field search/filter, all instances/interfaces, evidence, paths, dependencies and availability.
- **B02-04 Environment map:** usable hierarchy/graph of origins/projects/environments/tools/likely owners/duplicates, uncertainty and distinct size estimates; no cleanup actions.
- **B02-05 Capabilities:** canonical taxonomy/aliases, providers and resolver explanations equal CLI/skill outcomes.
- **B02-06 Skills:** instruction/MCP/package views, manifests and required/optional status; declarative import/download never installs tool dependencies or runs hooks.
- **B02-07 Agents:** actual detected integrations, health, connection state, supported discovery launch and recent activity; unsupported adapters are labeled honestly.
- **B02-08 Activity:** real scanner/registry/resolver/execution events, timestamps, filters and redaction; stdout not persisted by default.
- **B02-09 Security:** effective policy, controlled approvals, unknown executable state, discovery scopes, temporary credential lifecycle; edits reach daemon and other clients.
- **B02-10 Settings:** scan roots/modes, preferences, privacy/export, daemon and independent app/core/resource versions; nonsecret settings persist restart.

### Connected flows, extension and update lifecycle (B02-11–15)

- **B02-11 Connected agent:** choose supported detected agent, limited discovery session, explicit metadata disclosure, real launch/monitor/cancel and provenance-backed classification; do not claim control over a third-party host shell.
- **B02-12 Temporary provider:** configured approved metadata only; synthetic credential tests prove keys stay nonpersistent and redacted on success, error and cancellation; native/no-key mode remains fully usable.
- **B02-13 Scanner extensions:** permission-limited enforceable contract and working fixture plugin; do not silently load arbitrary code with broad host authority.
- **B02-14 Resource versions:** independently load/version/validate recognition/capability data; atomic verified local update, rollback and invalid-package/compatibility cases.
- **B02-15 ToolHub product updates:** verified package/update plumbing with explicit user action, version compatibility, cancellation and recovery. Keep discovered-tool installation prohibition separate. Do not invent a server, signing identity or release channel.

### Integrated quality and delivery (B02-16–22)

- **B02-16 Cross-client:** desktop, CLI and MCP see the same scan/policy/skill/activity state; daemon survives desktop closure; restart/reconnect/order cases pass.
- **B02-17 Usability:** nine primary pages navigable, keyboard focus, readable content, reduced motion, useful empty/error states, responsive narrow desktop; no dead controls/fabricated metrics.
- **B02-18 Windows release candidate:** reproducible local daemon/CLI/desktop bundle plus migrations/resources, checksums, startup and uninstall instructions. Produce an artifact; do not run installers against user software merely for testing.
- **B02-19 macOS Intel/ARM:** configure and exercise actual available host/CI builds/tests; distinguish configured, cross-compiled and host-verified. If unavailable, mark unverified and supply exact required environment; do not mark full macOS support complete.
- **B02-20 QA/privacy:** meaningful unit/fixture/integration/actual UI/IPC/native Windows flows, denial/credential/log/export regressions with synthetic values; inspect binaries/artifacts for accidental secrets.
- **B02-21 Documentation/SDK:** developer and public manifest/protocol docs, local authoritative schemas, minimal working TypeScript client. No remote schema-domain deployment implied.
- **B02-22 Delivery packet:** one exact clean source revision/cloneable bundle, checksums, runnable local artifacts, complete target/finding-to-evidence matrix, screenshots, internal review/repair notes and known host/signing/account limitations.

## Integrated verification and acceptance demonstrations

Do not optimize for a test count. A passing unit suite or manual-only screenshot does not establish these interactions. Use private synthetic databases, controlled scan roots and harmless executable fixtures. At minimum, demonstrate:

1. Fresh no-key install/start -> actual native scan -> distinct recognized instances and unknown candidates -> stable rescans and root-scoped Missing behavior -> persisted evidence/environment/owner corrections -> desktop/CLI/MCP consistent query after restart.
2. Multiple clients use the same long-lived daemon. An authenticated policy edit is immediately effective in a second client; an unauthorized edit/approval/session crossover is denied. Real allow/ask/deny and exact/replay/expired/changed/hash-replacement approval flows produce a redacted, persistent audit record.
3. A safe child failure maps to the CLI failure exit; timeout/cancel of owned finite process trees return within documented bounds; output/redaction/UTF-8 limits hold; no real user process or unknown discovered executable is run.
4. Resolver honors actual project, environment, architecture, trust, policy and version preferences, explains selected/rejected providers and remains consistent with skill availability and desktop capability views.
5. Malformed/boundary JSON-RPC and MCP requests, notifications, unsupported versions, event/cancel behavior and valid calls after invalid calls match schemas. No memory-only fake client substitutes for protocol bytes.
6. A supported connected-agent discovery path genuinely launches/monitors/cancels; disconnected/no-account path is usable and honestly labeled. Temporary provider synthetic-key test proves no persistence/log/export leak.
7. Exercise all nine desktop pages with real nonempty and empty/error data; navigate controls, filters, settings, approval/discovery and reconnect. Supply screenshots and commands for reproducible local review.
8. Local resource update/rollback, extension confinement, app update verification/cancel/recovery, migrations and packaging have invalid and interrupted cases. Keep user software and global machine state untouched.

Run workspace formatting, relevant static analysis and tests, release builds, appropriate frontend test/typecheck/lint/build, actual Windows IPC/UI smoke and artifact integrity checks. Negative controls must prove the assertion harness fails when expected. Record exact exit codes, platform/compiler/runtime versions and logs. Use a real available macOS host only when authorized/available; otherwise document the gap. Distinguish executed evidence, source inspection, mocked checks and unavailable host checks.

## Reporting contract for one R3 handoff

MiMo internally reviews and repairs before submitting. Provide:

1. `B02-INTEGRATED-DELIVERY.md`: exact base, HEAD, branch/clean status, changed-file scope, architecture/security decisions, commands and **actual** outputs/exits, user-visible flows and limitations.
2. `B02-INTEGRATED-COVERAGE.md`: a row for **each B01-01–18 and B02-01–22 subitem**, plus F01–F18 and R2-B01–B10. State implemented/verified/unverified/open, code references, automated/actual-host evidence and blocker ownership. Do not use Closed for source stubs or host-untested behavior.
3. `B02-INTEGRATED-DECISIONS.md`: contract/protocol/domain/security changes, version/migration implications, internal subagent ownership, independent review findings and repairs.
4. A complete cloneable source Git bundle and SHA-256 manifest; Windows local artifacts/screenshots/test reports and chosen small sanitized reproductions. The delivery/map/manifest **must stamp the same actual bundle HEAD**. Do not upload credentials, complete chat history, private machine inventories, databases or full environment dumps.
5. A one-sentence Chinese owner summary (<=160 characters) plus concise English workbench technical status; mark participant `submitted` only for the integrated packet. Remain available for one consolidated R3 repair assignment and R4 final verification.

## Scope and authorization boundaries

Native discovery works without AI credentials. Discovery is read-only by default. Unknown executables are not automatically probed. ToolHub does not install/uninstall/upgrade discovered tools, clean environments or mutate global PATH. ToolHub's own verified update feature requires explicit user action and must not silently publish or execute a release. Credentials are temporary and private; host observations are private by default. The product must not claim it sandboxes external agents.

This dispatch authorizes local implementation, project dependencies, isolated branches, local builds/tests and an unsigned local release-candidate artifact. It does **not** authorize new system toolchains, remote accounts/keys, real provider access, unapproved host/CI access, hosted Git push, public release, actual install/uninstall operations, merging implementation into `D:\ToolHub` main, workbench `report`/`finish`/`archive`, or an extra formal review. Escalate only a genuinely required permission/access blocker or a departure from these product boundaries; resolve routine engineering choices internally.
