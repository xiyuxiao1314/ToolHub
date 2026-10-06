# ToolHub B01 R1 — consolidated initial review

## Decision and review accounting

**Verdict: changes_required. B01 is not accepted.** This is formal product review **R1, 1 of 4 total**, including all three internal review domains and independent runtime probes. Resuming after the account-limit interruption did not start another round. MiMo receives one consolidated repair assignment; the next delivered revision is reviewed once as R2. R3/R4 remain allocated to B02. No implementation was merged, modified or published by this review.

Reviewed bundle HEAD: `46a06aa11c370b2ca4432909aa2b8b0a70127090`, branch `codex/toolhub-b01-core`; planning base: `6ffc9dc3c2fb526e740f0e3b29eafb84f4c54fce`. Bundle SHA-256: `ad4aeb965bd5c539d7c2f2337c35fbf840b167ba8142e367ce85f42c8a42375b`. The published delivery document names implementation parent `159bac10c22a18b10a3aa1165b9e521f681cc5b9`; the bundle adds a documentation commit. All findings below refer to the exact bundle HEAD. Source references use repository-relative paths and one-based lines at that revision.

## Independent verification and its limits

On Windows x64 with Rust/Cargo 1.95.0 and existing VS Build Tools/MSVC:

| Check | Independent result |
| --- | --- |
| `cargo fmt --all -- --check` | exit 0 |
| `cargo test --workspace --offline --locked` | exit 0; **42 passed, 0 failed, 0 ignored** |
| `cargo clippy --workspace --all-targets --offline --locked -- -D warnings` | exit 0 |
| `cargo build --workspace --release --offline --locked` | exit 0; daemon and CLI produced |
| Exact-review checkout status | clean before and after review |

The submitted report's count of 41 is corrected by the independent test log. These checks establish that the current Windows workspace builds and its existing tests pass; they do not establish the missing integration/security behavior. Runtime probes used the verified release binaries, separate SQLite databases, harmless owned fixtures and synthetic sensitive values. No installed unknown executable was run during scanning, no global configuration was changed, and no software was installed by Codex.

Existing safeguards did reject explicit Unknown/Blocked trust, excluded a synthetic inherited API key, and avoided persisting captured stdout in the examined execution-record path. No macOS compilation or host run was performed by Codex. Unix compilation defects below are source findings, not a claimed cross-platform test result. This is a risk-focused review rather than a claim to have tested every tool/platform combination.

## Findings and repair acceptance

P1 findings affect security, correctness or an essential integrated B01 path. P2 findings affect additional required B01 functionality/coverage. Priority sets repair order; it does not authorize silently dropping required targets. Related defects are grouped to give MiMo one coherent repair list.

### R1-F01 [P1] Wire bound approval into the real execution decision

**Sources:** `apps/daemon/src/service.rs:271` and `:289`; `crates/executor/src/lib.rs:113` and `:186`; `crates/core/src/execution.rs` approval model. **Targets:** B01-04/11.

The daemon checks only approval usability, never invokes `validate_approval`, and calls the executor without an authorization result. The executor rejects Ask unconditionally. Approving and executing the exact same harmless request in one daemon still returns `denied/approval_required`. Under an independently configured Allow rule, changed arguments and a different claimed caller execute with the unrelated approval and consume it. The latter proves missing binding, **not an Ask bypass**.

Implement one authoritative authorization path binding approval to authenticated caller/session, selected instance, executable identity, canonical argv/cwd/stdin, sanitized environment and relevant policy/trust state. Do not treat hash-read failure as an empty valid identity. Atomically enforce expiry, revocation, replay/consumption and revalidation before launch; Deny must remain authoritative. The current unused helper also needs the missing caller/session/policy checks.

**Acceptance:** real daemon tests: exact approved Ask succeeds once; changed args/cwd/stdin/environment/caller/session/binary or changed policy/trust fail; expired/revoked/replayed approval fails; concurrent reuse cannot launch twice; launch/error consumption semantics are documented and durable across restart.

### R1-F02 [P1] Bind authorization identities to actual peers

**Sources:** `apps/daemon/src/service.rs:54`, `:249`, `:367`; `apps/daemon/src/main.rs:125`. **Targets:** B01-04/12/15.

The policy/audit identity is request-controlled `agent_id`; the generic local-stdio peer is unused. With tool wildcard Allow and agent `agent.one` Deny, the same client is denied as agent.one but succeeds as agent.two or with no label. Being local does not establish the agent identity used by a rule. Approval/session issuance similarly trusts supplied names.

Derive the principal and session authority from the actual pipe/socket or an explicitly bound stdio launch, and define the trust boundary for identifying agent applications on the same OS account. Caller labels may be descriptive but must not choose their own security principal. Peer rejection must occur before any sensitive method. Avoid promising authentication stronger than the transport can supply.

**Acceptance:** the same untrusted peer cannot change policy outcome/audit authority by altering or omitting agent labels; foreign peers fail according to the documented Windows/macOS boundary; approval/discovery credentials cannot be issued or used for another principal by merely naming it.

### R1-F03 [P1] Apply persisted policy and complete execution context

**Sources:** `apps/daemon/src/service.rs:43`, `:256`, `:557`, `:590`; `crates/executor/src/lib.rs:165`; execution status gate at `service.rs:222`. **Targets:** B01-05/10/11/12.

Startup uses defaults instead of loading stored rules. A rule set through one CLI invocation is persisted but does not govern the next invocation's new daemon. Both policy contexts omit environment and capability. A system-environment Deny therefore loses to wildcard tool Allow even though the library's precedence would deny it. An instance with Blocked status and Known trust also executes; explicit blocked trust is correctly rejected.

Persist/reload effective rules with deterministic replacement and revision semantics; return write errors instead of reporting success. Carry selected capability, environment, command, directory and authenticated principal consistently through approval and launch. Blocked/missing availability must prevent launch independently of trust.

**Acceptance:** set/update Allow/Ask/Deny, restart and confirm the same effective decision; test each scope against less-specific Allow, status transitions, invalid scope/action values and policy changes invalidating approvals. Queries must distinguish defaults, stored and effective rules accurately.

### R1-F04 [P1] Enforce execution bounds while the process runs

**Sources:** `crates/executor/src/lib.rs:228`, `:237`, `:240`, `:255`, `:260`; synchronous service lock in `apps/daemon/src/main.rs:49`. **Targets:** B01-03/11/12.

The deadline is calculated and discarded after blocking `wait_with_output`. A 300 ms sleep requested with `timeout_ms=1` returns Success after 332 ms. Output is buffered in full before truncation: a finite 16 MiB output with a 256 KiB cap increased daemon peak working set by approximately 51 MiB. A 262147-byte valid UTF-8 payload crossing the cap panics at `String::truncate`, exiting the daemon with code 101. Large stdin written before pipe draining can also block. A hung execution can hold the service lock and block other calls.

Use bounded concurrent stdin/stdout/stderr handling, UTF-8-safe response encoding, actual deadlines/cancellation/disconnect semantics and child/process-tree cleanup. Define timeout, cancellation and output-limit results without losing daemon availability. This is missing implementation, not merely an untested best-effort cleanup claim.

**Acceptance:** safe long-running child/grandchild deadline and cancel tests confirm exit/cleanup and subsequent ping; sustained stdout/stderr stays bounded in memory/storage; multibyte cap crossings never panic; large bidirectional I/O does not deadlock; scan/query/cancel remain responsive during execution.

### R1-F05 [P1] Redact sensitive values and normalized private paths

**Sources:** `crates/executor/src/lib.rs:317`; `crates/audit/src/lib.rs:69`; `apps/daemon/src/service.rs:599`. **Targets:** B01-11/13.

Argv redaction masks a `--token` flag but preserves the separate synthetic value in SQLite; hyphenated `--api-key` is also missed. Export matches the home path case-sensitively and skips short usernames. In a synthetic context, `C:/Users/hp/secret-project/python.exe` becomes `C:/home/hp/secret-project/python.exe`, retaining identity/private location while claiming home/username redaction; lowercase backslash variants also leak.

Redact option-value pairs and assignment forms using the same rules across audit/log/error/export. Normalize platform case/separators for path matching, use component boundaries, handle usernames of any length and define default private-path removal and explicit opt-in. Include macOS username/home conventions.

**Acceptance:** neutral random sentinels supplied through token/password/api-key/Authorization forms are absent from all persisted/error/export artifacts; Windows mixed case/slash/UNC and macOS home fixtures are redacted without masking unrelated paths. Record denied attempts safely too.

### R1-F06 [P1] Preserve registry identities and reconcile scan state transactionally

**Sources:** `crates/recognizer/src/lib.rs:248`; `crates/registry/src/repo.rs:55`, `:214`; `crates/registry/src/migrations/001_init.sql:40`; `apps/daemon/src/service.rs:648` and `:650`. **Targets:** B01-01/05/06.

Rediscovery generates a new instance ID but upsert only handles ID conflicts; the independent definition/path uniqueness rejects the second discovery and ingestion ignores the error. Two real scans leave fixture IDs/last_seen unchanged. Same-path unknown candidates get new rows on each pass (fixture count 1 to 2; overall parent run 73 to 146 to 219). Removed fixture executables remain Available and selected after a successful rescan. Nonbuiltin environment writes also occur after dependent instance insertion. Evidence/interfaces/scan_sessions tables remained empty after the native probes, and candidate canonical metadata is discarded.

Resolve stable normalized identity before write, transact dependent entities in correct order, preserve observation/provenance/user corrections, propagate failures and reconcile seen/absent/promoted candidates by actual root coverage. A failed/partial scan must not infer disappearance. Persist scan sessions and relevant relationships. Validate registry writes against the core domain. Add forward-version refusal/recovery/backup and interrupted/concurrent-write evidence required by the original brief.

**Acceptance:** repeated real scan updates metadata/last_seen without duplicate IDs; link/case variants and distinct installations follow the documented identity rule; removal/replacement/promotion works across restart; partial roots preserve uncertainty; injected write errors fail visibly; evidence/interface/environment foreign keys and migration/recovery tests are exercised.

### R1-F07 [P1] Separate recognition confidence from executable trust

**Sources:** `crates/recognizer/src/lib.rs:180`, `:221`, `:250`, `:258`; native rules at `:128`; no versioned resource loader or `schemas` directory. **Targets:** B01-02/08.

A harmless text file named python.exe becomes Known/Available based on basename alone. Confidence is raised without corroborating binary/package/publisher evidence, and architecture is assigned from the host. All four recognized native rows in the parent fixture run lacked versions. GCC/MSVC patterns map to Clang; representative .NET/agent recognition and real metadata fixtures are absent. Rules/resources are compiled constants rather than independently versioned, validated local packages.

Weak filename/path matches must remain untrusted hypotheses until supported by recorded evidence. Keep recognition certainty separate from execution trust. Implement static metadata/package/signature/bundle evidence and accurately observed version/architecture; bounded known probes, if used, require their own reviewed allowlist and execution controls. Add compatible versioned recognition/capability resource loading and representative fixtures from the brief.

**Acceptance:** renamed marker-producing unknown fixtures never execute during scan or gain Known execution trust solely from basename; genuine tools have provenance and measured metadata; representative language/tool/compiler/SDK/agent fixtures resolve correctly; malformed/incompatible resources fail without partially replacing live data. Default policy is Ask, so the current filename finding alone is not evidence of default unauthorized execution.

### R1-F08 [P2] Implement actual native providers and honest coverage/modes

**Sources:** `crates/scanner/src/providers.rs:28`, `:85`, `:151`, `:205`, `:248`; `crates/scanner/src/lib.rs:47`. **Targets:** B01-06/07/17.

The named Windows Registry provider only walks directories; no Registry/App Paths/installed-app metadata is read. Package scans omit actual package metadata/depth; the Python glob is filtered by literal existence before expansion. macOS source lacks bundle/LaunchServices/package/SDK metadata required by the brief. Missing/denied/capped subtrees can be flattened into success. Quick and Full have no meaningful provider difference; there is no persisted incremental/watch/debounce/cancellation/event path.

Complete the scoped Windows/macOS metadata providers and realistic fixtures from B01-07; no full-disk scan is required. Return provider/root/subtree coverage, failures and truncation/continuation honestly. Implement meaningful Quick/Full and bounded incremental invalidation/events with cancellation. Separate absent features from unavailable host checks.

**Acceptance:** metadata-only fixtures cover Registry/package/bundle/SDK/environment roots; missing/denied/link/cap cases are explicit; mode differences and unchanged/change/delete incremental behavior are observable; actual Windows readonly smoke records accurate coverage. macOS host evidence remains unverified unless actually run.

### R1-F09 [P1] Construct and expose real environment/ownership graphs

**Sources:** `crates/environment/src/lib.rs:25`, `:52`, `:92`; `crates/registry/src/repo.rs:494`; `apps/daemon/src/service.rs:148`, `:650`. **Targets:** B01-05/09/10.

The graph starts with System/User and detection searches for nodes never inserted before falling back to those same two kinds. Separate project-a and project-b venv fixtures both become env.user; persisted environments contain only two rootless defaults. Queries return abbreviated tuples and duplicate name/count pairs, losing placements, versions/architecture and attribution needed by clients. Environment graph state is not restored at daemon open.

Detect stable environment roots/parents from explicit markers/metadata across project/venv/Conda/package/application/WSL-container/agent contexts; retain uncertainty and user corrections. Persist before instance references and reload the graph. Expose typed roots, edges, placements, origin/owner evidence and full duplicate instances with available detection/use/size metadata.

**Acceptance:** two project venvs remain distinct and drive cwd resolution; Conda/package/agent fixtures and uncertain ownership survive restart; inspection/duplicate/environment outputs supply coherent IDs and relationships without inferring known ownership from directory names alone.

### R1-F10 [P1] Filter resolver eligibility before ranking and honor request constraints

**Sources:** `crates/resolver/src/lib.rs:64`, `:103`, `:121`; `apps/daemon/src/service.rs:180`, `:192`; `crates/core/src/version.rs:76`, `:85`. **Targets:** B01-01/10/16.

Blocked trust only reduces a score and can still be selected with `error:null`; `require_trust` is unused. The daemon rewrites every architecture as x86_64 and uses default preferences, ignoring cwd/project/environment/trust inputs. A Blocked/aarch64 fixture satisfies a request for verified/x86_64. Deleted paths also win resolution (F06). Invalid constraint `>=3.13 || true` is silently discarded; `3.13.0-rc1` satisfies `>=3.13.0`, and `0.9.0` satisfies `^0.2.0`.

Apply availability, architecture, requested scope/trust/version eligibility before deterministic ranking; carry all documented preferences and return structured missing/blocked outcomes plus excluded alternatives/reasons. Invalid grammar must be an error. Define consistent SemVer/tool-version semantics, including prerelease and zero-major caret rules if those forms are supported, and use them in skills too.

**Acceptance:** blocked-only/missing/wrong-architecture/version-unknown scenarios never return an eligible selected provider; project preferences choose the correct venv; aliases and explanations remain deterministic; malformed/prerelease/zero-major fixtures behave as documented through the real service.

### R1-F11 [P1] Deliver shared local daemon transport and repair Unix targets

**Sources:** `apps/daemon/src/main.rs:68`, `:91`, `:106`; `apps/cli/src/main.rs:264`; `crates/scanner/src/lib.rs:108`; `crates/ipc/src/lib.rs` endpoint/peer logic. **Targets:** B01-07/12/17.

Windows `--listen` is a stdio fallback, not a named-pipe listener. Every CLI invocation spawns a separate daemon, with no existing-service connect/readiness/reconnect path; memory approvals and mutable state are not shared. The Unix listener uses a global temporary socket, removes it unconditionally and lacks enforced peer credentials/permissions. Unix code calls unimported `write_frame`; scanner tests reference a variable defined only under cfg(windows), and POSIX executable fixtures lack execute bits. These source defects contradict the submitted macOS compile-only claim; no successful macOS command/log accompanies it.

Implement user-scoped Windows pipes/macOS sockets, authenticated peers, bounded startup/readiness, single-instance ownership, shared concurrent services, disconnect/shutdown/reconnect and stale-endpoint recovery. Do not delete another live owner's endpoint. Repair all target-specific source/test defects and provide exact available cross-platform compilation evidence.

**Acceptance:** separate CLI/MCP clients see the same live approvals/policy/session state; unavailable/slow startup returns bounded structured errors; concurrent scan/exec/query/cancel and crash/restart work; endpoint ownership and unauthorized-peer tests pass. Actual macOS compile/tests/host results must be distinguished; Windows passing checks do not establish them.

### R1-F12 [P1] Use authoritative typed wire contracts with validation and bounds

**Sources:** `crates/protocol/src/lib.rs:16`; `crates/protocol/src/types.rs:16`, `:25`, `:81`; `apps/daemon/src/main.rs:43`; `apps/daemon/src/service.rs:118`, `:193`, `:599`. **Targets:** B01-01/02/03.

The daemon accepts jsonrpc=1.0, an object-valued id and numeric params, then returns a successful ping. A stdio request larger than the declared 1 MiB cap is accepted. Services construct ad hoc JSON instead of declared result DTOs: resolution uses canonical instead of canonical_capability and omits the selected score; export omits required environments; search/inspect contracts similarly drift. No authoritative JSON schemas/valid-invalid conformance fixtures, negotiation, cancellation or event ordering/backpressure implementation exists.

Validate JSON-RPC envelopes/method inputs and serialize authoritative result/error types across daemon/CLI/MCP. Enforce limits before full allocation on every supported transport and define parse/invalid-request/invalid-params/method-not-found behavior. Implement version compatibility, typed events/cancel semantics and versioned tool/capability/skill/protocol schemas with fixtures. JSON-RPC notifications require no response; version/id/params constraints follow the [JSON-RPC 2.0 specification](https://www.jsonrpc.org/specification).

**Acceptance:** real transport conformance covers invalid envelope, IDs, scalar params, unknown methods, notifications, oversize/truncated input, mismatched versions and every public result shape; schema/Rust/client models stay consistent. Error cases must not crash or hang the service.

### R1-F13 [P1] Make MCP interoperable and keep serving after errors

**Sources:** `apps/cli/src/main.rs:290`, `:365`, `:378`, `:429`; `crates/mcp/src/lib.rs`. **Targets:** B01-14/17.

The live initialize result omits server capabilities, initialized notifications receive an unsupported-method response, tools/list returns a bare array, and tools/call returns raw daemon JSON instead of MCP content. Calling inspect on a missing instance exits the entire gateway with code 3 and no JSON-RPC reply because shared DaemonClient calls process::exit. Tool-schema names alone are not an interoperable gateway.

Implement lifecycle/version negotiation, capabilities, notification handling, bounded schemas and standard tools/list and tools/call result/error envelopes. Ordinary request/tool errors must become protocol responses, not terminate the server. Route authorized calls through the shared principal/policy/audit path. These expectations follow the supported version's official [lifecycle](https://modelcontextprotocol.io/specification/2024-11-05/basic/lifecycle) and [tools](https://modelcontextprotocol.io/specification/2024-11-05/server/tools) contracts.

**Acceptance:** a real MCP client initializes, sends notifications, lists and invokes all seven bounded metatools; malformed/not-found/denied/expired calls receive the correct response and a following valid call still works. No stdout logging pollution, unbounded requests or per-installed-tool schema explosion.

### R1-F14 [P1] Implement scoped discovery operations and durable classifications

**Sources:** `apps/daemon/src/service.rs:433`, `:470`, `:488`, `:511`, `:533`, `:675`; `crates/agent-bridge/src/lib.rs:97`. **Targets:** B01-04/05/15.

Session listing exposes all session IDs without caller binding; inspection uses a session ID as an id and returns all candidates rather than a scoped candidate operation. A candidate.list-only session has no usable candidate-list path. classify accepts only session_id, claims accepted/stored enrichment but validates no candidate/classification and writes no evidence (runtime evidence count stays zero). Sessions are loaded into private process memory only at startup, so another process's revocation is not promptly reflected. Advertised classify CLI instructions target a nonexistent command.

Define caller-bound session credentials and separate session management from candidate listing/inspection, metadata/probes and classification. Validate scope, candidate membership, payload/provenance and expiry/revocation at each operation against authoritative state. Persist untrusted classification without elevating execution trust; return write failures. Do not expose usable credentials through public listings.

**Acceptance:** two principals cannot enumerate/use/revoke each other's protected sessions/candidates; candidate.list/inspect/metadata/classify each obey their own scope; revocation from another client is immediate; empty/unsolicited/bad classifications fail; valid classification is durable and queryable with provenance and still grants no execution authority.

### R1-F15 [P1] Wire actual skills and validate declarative manifests/paths

**Sources:** `apps/daemon/src/service.rs:423`; `crates/core/src/skill.rs:78`, `:108`, `:133`; `crates/skills/src/lib.rs:25`, `:66`, `:92`. **Targets:** B01-02/13/14/16.

Skill endpoints are constants: a persisted synthetic skill is absent from skill.list and inspect reports no packages registered. There is no integrated registration/import path or SKILL.md parsing. The parser accepts unsupported schema versions by prefix, silently defaults wrong field types, forces Instruction and drops package_path. Windows drive/UNC/rooted paths escape the current check. Availability uses suffix matching and the first provider; optional versions are ignored and blocked providers cannot be represented.

Implement declarative instruction/MCP/package registration and real registry-backed CLI/MCP queries without hooks/installers. Validate exact supported schemas/types/kinds, contained paths including symlink/platform cases, and required/optional capability aliases/versions through the authoritative resolver. Return Available/Missing/Blocked/Malformed with provider evidence.

**Acceptance:** register/read/restart/query each kind; malformed schemas/types/unsafe paths and executable hooks fail without side effects; multiple providers/version constraints choose an eligible provider; suffix collisions, blocked requirements and optional constraints are reported accurately; missing dependencies never trigger installation.

### R1-F16 [P2] Complete adapter configuration and discovery launch

**Sources:** `crates/agent-bridge/src/adapters.rs`; `crates/agent-bridge/src/lib.rs:97`; agent CLI paths in `apps/cli/src/main.rs`. **Targets:** B01-15.

The current bridge primarily searches command names and returns static instructions/generic configuration. It does not implement a real discovery-session launch against a supported detected runtime. Generic configuration assembled by string interpolation does not safely encode arbitrary endpoint/path content. The submitted host report detects Codex, yet real launch remains manual and the generated instructions include the missing classify command (F14).

Implement the declared adapter capabilities/health/config-generation/session-launch contract for generic CLI/MCP and OpenCode/Codex/Claude Code. Generate configuration through typed serialization. Connect session credentials and only permitted discovery operations. An unavailable external runtime/account is an explicit blocker, not success; installation/login is not authorized by this repair assignment.

**Acceptance:** valid config round-trips for spaces/quotes/backslashes; supported-runtime fixtures exercise detection/health/launch; perform one bounded real configured/detected discovery launch where existing authorization/runtime permits, or report the exact access blocker honestly. Launch must produce validated durable enrichment through F14.

### R1-F17 [P2] Implement structured CLI outcomes, reporting/import and activity

**Sources:** `apps/cli/src/main.rs:248`, `:295`; `apps/daemon/src/service.rs:593`, `:599`; `crates/protocol/src/types.rs:81`. **Targets:** B01-05/11/13/18.

CLI --json errors print only human stderr (missing-instance probe: exit 3, empty stdout), while execution-result failures can still exit 0 after printing. activity.list always returns an empty array despite execution rows, and report/manifest import has no method/command/implementation. Export omits environments and most provenance/relationship fields. Definition writes also discard description/vendor/category data required for useful reports.

Provide stable structured result/error envelopes and exit semantics without process termination inside reusable clients. Expose redacted activity, a typed compatible machine report, and validated report/manifest import with provenance. Imported foreign paths must not become Available/trusted local executables without local validation; transactional validation must precede writes.

**Acceptance:** JSON success/not-found/invalid/denied/timeout/child-failure cases are parseable and have documented stable exits; actual execution/denial appears in activity; report round-trip preserves allowed relationships; malformed/incompatible/foreign-path imports fail or remain untrusted unavailable data; default export passes F05 privacy checks.

### R1-F18 [P1] Replace false-positive smoke evidence with integrated negative checks

**Sources:** `scripts/smoke.ps1`; `docs/reviews/B01-COVERAGE.md:3`; `docs/reviews/B01-DELIVERY.md`; test/integration layout. **Targets:** B01-17/18 and all target evidence.

The smoke script ignores native command failure. A controlled toolhub shim failing all 15 invocations with exit 9 still yields script exit 0 and SMOKE DONE. Its unknown-marker assertion does not create a marker-producing fixture, so it can pass vacuously. Current unit tests pass while all runtime failures above persist. Coverage marks disconnected or missing implementations Done, including skills/classification/IPC, and calls timeout handling best-effort although no deadline is enforced.

Make smoke fail on command/semantic errors, create owned non-destructive negative fixtures and assert actual outcomes. Commit realistic portable integrated regressions for the service/client paths, not just helper mirrors. Commit platform-check configuration or local equivalents independently of whether a hosted remote exists. Correct coverage and decisions by subtarget: implemented, verified, unverified-host, blocked-access or not-implemented. Mac compile claims require command/log/target evidence.

**Acceptance:** the failing shim fails smoke; unknown executable fixture exists and leaves no marker during scan; at least one safe installed known-tool execution plus Ask/Deny/tamper paths run through daemon; all F01-F17 regressions are traceable; no missing required feature is marked Done merely because a crate/helper builds.

## Required-target reconciliation

The original `MIMO-B01-CORE.md` remains the acceptance scope. This review does not move core work to B02. MiMo should complete the gaps in the same repair delivery, recording an exact external blocker where a host/account check truly cannot run.

| Original target | Repair groups | Specific evidence required beyond current unit tests |
| --- | --- | --- |
| B01-01 identities/model | F06/F09/F10/F12 | stable paths/IDs, validated relationships/version semantics, typed outputs |
| B01-02 schemas/resources | F07/F12/F15 | committed versioned schemas, valid/invalid fixtures and compatible resource loading |
| B01-03 protocol/errors | F04/F12 | limits, negotiation, cancellation/events and transport conformance |
| B01-04 authorization | F01/F02/F14 | caller-bound approval/session integration and negative cases |
| B01-05 persistence | F03/F06/F09/F14/F17 | durable policy/evidence/interfaces/scans/skills/activity, migrations/recovery |
| B01-06 scanner modes | F06/F08 | refresh/invalidation, differentiated bounded modes/events/cancel |
| B01-07 OS discovery | F08/F11 | actual native metadata providers, representative fixtures, honest Windows/macOS evidence |
| B01-08 recognition | F07 | corroborated evidence, versions/architecture, representative tool/resource fixtures |
| B01-09 environments | F09 | real graph, uncertain ownership and complete duplicate placements |
| B01-10 resolver | F06/F09/F10 | available eligible providers, constraints/preferences and rejection reasons |
| B01-11 execution/audit | F01-F05/F17 | policy-bound execution, bounded processes, redaction and activity |
| B01-12 daemon/IPC | F02/F04/F11/F12 | actual shared authenticated service, lifecycle and concurrent clients |
| B01-13 CLI/reporting | F05/F12/F17 | JSON error/exit contract, report/import, real skill/discovery/activity commands |
| B01-14 MCP | F02/F12/F13/F15 | real compatible client lifecycle/metatool/error persistence tests |
| B01-15 sessions/adapters | F02/F14/F16 | isolated scoped durable enrichment and real supported launch/evidenced blocker |
| B01-16 skills | F10/F15 | safe typed declarative registration and real availability queries |
| B01-17 QA | F08/F11/F18 | committed integrated negative/compatibility/platform checks |
| B01-18 delivery | F17/F18 | clean exact revision, usable reproducible core and truthful target coverage |

## Evidence and next handoff

`B01-R1-EVIDENCE.json` is a sanitized summary of independent command results and observations. `B01-R1-REPRO.zip` contains selected reproduction scripts and raw local result files only; rerun with new isolated fixture databases and the repaired release binaries. No user database, machine scan inventory, credentials, full conversations or environment dump is included. Local full command logs are retained by Codex; their hashes are recorded for integrity.

The single repair brief is `MIMO-B01-R1-REPAIR.md`. MiMo owns internal decomposition, subagents, contracts, integration and internal review. Deliver one corrected clean HEAD, bundle plus hash, updated original-target coverage and a finding-to-change-to-fresh-evidence map for F01-F18. Do not request per-module Codex reviews. R2 verifies that delivery and permits bounded local Codex corrections; large unresolved blockers are not suitable for a last-minute reviewer rewrite. No fifth formal review is opened automatically.

## Owner summary

R1 初审未通过：现有 42 项测试和构建通过，但审批、执行边界、扫描状态与 MCP 等核心闭环存在阻塞；已集中派发一轮修复，审查额度使用 1/4。
