# MIMO-B01 — Deliver the complete runnable ToolHub core

## Owner summary

MiMo 按一个大型批次完成可运行核心闭环并内部自审，Codex 只做集中初审和修复后复审，项目总审查上限四次。

## Mission and authorization

Implement an integrated, usable ToolHub core from the full design, including native discovery, registry, environments, resolution, policy-controlled execution, daemon/CLI, MCP, discovery sessions, adapters and skill availability. Do not return only foundation documents or disconnected crate stubs.

This dispatched brief supplies the initial large goal checklist and supersedes MIMO-000's wait-for-coding state for the work defined here. MiMo owns internal scheduling, implementation details, subagents, integration, self-review and repair. Codex performs one consolidated review after delivery and one after the consolidated fix delivery.

Read [two-batch plan](../superpowers/plans/2026-09-29-toolhub-two-batch-plan.md) and [review policy](../collaboration/REVIEW_POLICY.md). Formal review allocation for this batch: R1/R2; no external per-module approval gate.

## Baseline and checkout

- Task: `task_a11c48d272163020b1ee37f1`; use `--profile mimo`.
- Original baseline: `ce8c755f9ecb2e8a6655d6997f21767332502a45`.
- Required task/planning baseline: exact commit in the latest `TWO_BATCH_MANIFEST.json` and dispatch event.
- Existing independent checkout: `D:\ToolHub-mimo\checkout`; preserve `docs/reviews/MIMO-000-ASSESSMENT.md`.
- Local source review repository: `D:\ToolHub`; it is not a hosted push destination.
- Suggested implementation branch: `codex/toolhub-b01-core`.

Update from the local source with read-only fetch of `main`, or clone the published `ToolHub-two-batch-plan.bundle` in a fresh authorized checkout. Verify HEAD against the manifest before development. Do not discard untracked assessment or user changes. Work only in MiMo's independent checkout; keep `D:\ToolHub` available as Codex's review baseline.

## Development tools

Codex independently verified working `rustc 1.95.0` and `cargo 1.95.0` at `C:\Users\hp\.cargo\bin\` under the desktop user. A PATH lookup alone is insufficient to conclude they are missing. Use absolute paths or session-local PATH without modifying machine/user PATH.

Node/Python/Git are reported available; verify needed commands. Windows MSVC C++ component availability was not confirmed by the component query. Run a small compile/link probe using existing tools before creating an install request. The Rust toolchain is already installed; do not reinstall it.

Existing Cargo/npm dependency resolution in the project is part of normal development. Installing missing system toolchains/components, changing global PATH/configuration, or acquiring new remote accounts needs applicable owner authorization. If a host component is genuinely missing, provide its exact name, proposed bounded installation and affected checks; continue independent design/implementation work while dependent verification is blocked. SQLite CLI is not required if the chosen Rust database library and fixtures provide the necessary tests.

## Directory ownership

MiMo parent/integration owner controls workspace manifests, lockfiles, shared contracts, integration wiring and reports. Assign non-overlapping implementation domains to subagents using these proposed boundaries:

| Domain | Allowed paths | Read-only dependencies after contract lock |
| --- | --- | --- |
| Foundation/contracts | `crates/protocol/**`, `crates/core/**`, `schemas/**`, `docs/architecture/**`, `docs/protocol/**`, `docs/security/**` | Baseline documents |
| Persistence | `crates/registry/**` | Protocol/core and schemas |
| Scanner framework | `crates/scanner/**`, `tests/fixtures/scanner/**` | Protocol/core |
| Windows/macOS discovery | `crates/scanner-windows/**`, `crates/scanner-macos/**`, `tests/windows/**`, `tests/macos/**` | Scanner and protocol |
| Recognition/capabilities | `crates/recognizer/**`, `crates/capability/**`, `resources/**` | Protocol, scanner, schema |
| Environments/ownership | `crates/environment/**`, `crates/ownership/**` | Protocol, registry, candidate inputs |
| Execution/security | `crates/resolver/**`, `crates/executor/**`, `crates/policy/**`, `crates/audit/**` | Protocol, registry, capabilities |
| Clients/lifecycle | `apps/daemon/**`, `apps/cli/**`, `crates/ipc/**` | Core service interfaces |
| Agent/MCP/skills | `crates/mcp/**`, `crates/agent-bridge/**`, `crates/skills/**` | Reviewed daemon/protocol/policy interfaces |
| Integration/QA | `tests/integration/**`, `scripts/**`, `.github/workflows/**`, `docs/development/**`, `docs/reviews/**` | All public contracts; broad feature refactors remain parent-controlled |

These are ownership boundaries, not a requirement to use ten workers or create unused crates. MiMo may consolidate coherent crates if it records the mapping and preserves separation of concerns. Desktop and its UI belong to B02; B01 defines usable contracts for them.

## Internal sequencing

- [ ] Verify baseline/toolchain, create workspace and an executable host build probe.
- [ ] Resolve B01-01 through B01-04 contracts together; parent plus an internal independent reviewer checks consistency and threat cases.
- [ ] Record a provisional contract version/decision log before downstream parallel implementation.
- [ ] Implement registry/scanner/recognizer in parallel where inputs allow; unlock OS and environment work when contracts stabilize.
- [ ] Implement execution/policy/audit, then integrate daemon/CLI/MCP/skills/sessions against the actual service.
- [ ] Run full integration and security/lifecycle regressions, internally review and repair, then deliver once.

Each internal unit should have a focused behavior check before implementation and a passing check after, a bounded code change, and an identifiable commit. MiMo selects exact code/test steps rather than receiving Codex microtasks.

## Target checklist

### B01-01 — Domain identities and relationships

- [ ] Define canonical ToolDefinition, ToolInstance, Interface, Capability, Environment, Owner, Origin, Evidence, Trust, Candidate, Agent, Skill, session and execution types.
- [ ] Specify IDs, relationship cardinality, timestamps, evidence provenance, numeric confidence ranges, ownership labels and trust transitions with validation tests.
- [ ] Keep confidence, ownership certainty and execution trust as separate concepts. A numeric confidence is not permission to execute.
- [ ] Define platform path/link normalization and version constraint behavior; preserve multiple instances and original/canonical paths.

### B01-02 — Capability taxonomy and manifests

- [ ] Select canonical capability identifiers, including one video-frame extraction spelling; preserve legacy example spellings only through an explicit alias strategy.
- [ ] Produce versioned tool/capability/skill/protocol schemas and valid/invalid fixtures; enforce IDs and required fields consistently with Rust types.
- [ ] Resolve aliases deterministically; detect invalid/cyclic aliases and unknown capability requirements without inventing providers.
- [ ] Version recognition/tool-definition resources separately from core/app; reject incompatible or malformed local resource packages.

### B01-03 — Protocol and error model

- [ ] Freeze JSON-RPC methods, request/result/error types, protocol negotiation and manifest compatibility policy in schemas/docs and executable tests.
- [ ] Define stable error codes, cancellation/deadlines, request/output size limits and event envelopes with ordering/backpressure behavior.
- [ ] Implement structured unavailable/denied/expired/invalid-request states and fallback metadata without granting host authorization through fallback flags.
- [ ] Provide conformance fixtures and typed service/client boundaries; use one authoritative domain model across clients.

### B01-04 — Authorization contract and safety boundary

- [ ] Design approval binding to caller/session, selected instance and executable identity, canonical arguments/cwd, sanitized environment, relevant trust/policy state and expiry.
- [ ] Enforce revocation, replay protection and approval consumption semantics; revalidate identity and relevant policy before execution, denying changed requests or binaries.
- [ ] Authenticate/authorize local peers using the actual transport boundary, not caller-supplied agent-name strings. Record tested handling and residual executable-replacement races.
- [ ] Model discovery-session scopes separately from execution approval. Unknown metadata/classification/help text is untrusted data, not execution authority.

### B01-05 — Registry and SQLite lifecycle

- [ ] Implement migrations from an empty database, validated writes/queries, transactions, stable identities and relationships for the core entities.
- [ ] Persist recognition evidence, environment/ownership attribution, trust, skill requirements, scan sessions and redacted execution metadata.
- [ ] Test duplicate discoveries/upserts, changed or disappeared instances, concurrent access, migration replay and interrupted-write recovery.
- [ ] Define supported forward migration/recovery and backups. Do not invent a requirement for destructive down migrations; document actual compatibility and recovery behavior.

### B01-06 — Candidate/scanner framework and modes

- [ ] Implement read-only discovery -> candidate -> recognition -> registry orchestration, bounded cancellation/timeouts and scan events.
- [ ] Support quick/full modes with reviewed metadata roots; no default recursive full-disk scan and no AI/key prerequisite.
- [ ] Preserve unknown/ambiguous candidates for inspection without fabricating known tool identities.
- [ ] Implement meaningful incremental refresh/invalidation, bounded watcher/debounce behavior, persisted scan state and fixture-driven tests; do not claim full incremental support if merely rerunning every scan.

### B01-07 — Windows and macOS discovery

- [ ] Windows: working providers for PATH, Registry/App Paths/installed applications, Program Files/x86, AppData/WindowsApps, safe command metadata, winget/Scoop/Chocolatey metadata, Visual Studio/Windows SDK, WSL/Docker and agent/environment roots where present.
- [ ] macOS: working providers for PATH/system/local/Homebrew roots, Applications/user bundles/LaunchServices, Homebrew/MacPorts, Xcode/CommandLineTools/frameworks and agent/environment roots where present.
- [ ] Preserve access-denied, missing root, link, architecture and environment cases; partial scans report coverage and errors rather than success for unvisited roots.
- [ ] Add realistic scanner fixtures for both platforms and perform actual Windows read-only smoke discovery. Mac compile/host evidence is distinct and must not be inferred from fixtures.

### B01-08 — Recognition and capability resources

- [ ] Recognize a representative fixture set spanning Python/Node/Java/.NET/Rust/Go, Git, FFmpeg, Docker, C/C++ compilers/SDKs, SQLite, Android/JADX/adb and configured agent executables; absent host tools still have reproducible metadata fixtures.
- [ ] Rank evidence from signatures/publisher/bundle/package/path/static metadata and reviewed known probes; store source/confidence and ambiguity.
- [ ] Unknown binaries never run automatically, even for version/help. Tests include an unknown executable fixture whose execution would leave an observable marker.
- [ ] Known probe allowlists include argument rules, time/output limits and failure semantics; scanning is not a back door to arbitrary command execution.

### B01-09 — Environments, origins, owners and duplicates

- [ ] Model system/user/package-manager/venv/conda/project/application/WSL-container/agent-associated contexts with a coherent environment graph.
- [ ] Determine known/probable/unknown ownership using explicit evidence and retain user correction without unsupported blame from directory names.
- [ ] Report duplicate runtime installations, versions/architectures, detection/use times and available size metadata without deleting or merging them.
- [ ] Expose stable environment/owner/duplicate query results ready for the B02 environment map, including missing and uncertain attribution cases.

### B01-10 — Capability resolver

- [ ] Resolve capability and version requests against available instances using cwd/project environment, architecture, scope, trust and user preference.
- [ ] Return alternatives and a deterministic explanation; missing/blocked providers produce explicit structured results.
- [ ] Keep selection separate from execution authorization; a resolved Python instance does not authorize arbitrary code.
- [ ] Test alias/version/architecture/project-preference conflicts and unavailable candidate changes.

### B01-11 — Policy, sanitizer, executor and audit

- [ ] Implement allow/ask/deny, effective-rule precedence and changes across tool/capability/agent/directory/environment/command context.
- [ ] Build child environments from an explicit approved set; secret-bearing variables are excluded by default, with controlled per-tool opt-in and synthetic-secret regression checks.
- [ ] Execute without implicit shell interpretation; handle cwd/argument validation, stdin/stdout/stderr bounds, deadlines, cancellation, exit codes and child/process-tree cleanup.
- [ ] Record redacted identity/tool/capability/args/cwd/duration/result. Sensitive stdout is not persisted by default; logs, audit and error paths obey the same redaction rules.
- [ ] Test changed executable/approval replay/expired or revoked session, blocked and unknown tools, malicious metadata, secret arguments, oversized output and timeout/cancellation.

### B01-12 — Daemon and local IPC

- [ ] Implement user-level `toolhubd`, health/readiness, single-instance behavior, startup/shutdown and shared services over Windows named pipes/macOS Unix sockets.
- [ ] Restrict local peers, enforce protocol limits and handle disconnect/reconnect/concurrent clients without a default public listener.
- [ ] CLI detects unavailable daemon and performs bounded startup attempts or a structured failure; no indefinite hangs.
- [ ] Prove persistence and lifecycle behavior: desktop-independent operation, restart, overlapping scan/query, crash/recovery and cancellation of outstanding work.

### B01-13 — CLI and redacted reporting

- [ ] Provide usable scan/search/list/inspect/resolve/environment/duplicate/skill/discovery/activity/export commands, execution and permission interaction where supported.
- [ ] Provide structured `--json` output, stable errors/exit codes and documented examples using actual implemented command names.
- [ ] Produce a useful redacted machine report; hide usernames/home paths/secrets/sensitive args by default, with explicit tested policy for opt-in fields.
- [ ] Implement validated report/manifest import with provenance and compatibility checks; paths from another machine do not become available/trusted executable instances without local validation.
- [ ] CLI reads the same registry and service as all other clients; do not silently create a separate direct-write database path.

### B01-14 — MCP gateway

- [ ] Implement a bounded metatool set for search/resolve/inspect/environments/authorized execution/skill queries using supported MCP contracts.
- [ ] Commit input/output schemas and test invalid input, denied execution, expiry and empty registry behavior; tool count must not grow per installed instance.
- [ ] Share daemon auth/policy/audit rather than bypassing it inside MCP handlers.
- [ ] Provide a working stdio integration/reproduction using a test client; document actual supported client configuration.

### B01-15 — Discovery sessions and agent adapters

- [ ] Implement expiring/revocable sessions limited to candidate listing/inspection, metadata reading, allowlisted probes and classification submission.
- [ ] Define AgentAdapter capabilities/health/config-generation/session-launch contracts. Implement generic CLI/MCP access plus detection/configuration adapters for OpenCode, Codex and Claude Code, and at least one real configured/detected discovery launch when the host provides a supported runtime.
- [ ] Session-launch instructions enforce no installation/removal/config changes through ToolHub; classifications preserve provenance and validation.
- [ ] Test session isolation, expiry, revoked credentials, unsolicited classification and metadata scope. Missing external agent/account becomes an honest unsupported/blocker state, not a fake successful launch.

### B01-16 — Skills

- [ ] Parse instruction SKILL.md, MCP and package-skill manifests into a shared skill model.
- [ ] Resolve required/optional capabilities and versions against the registry; show available/missing/blocked states without installing tool dependencies.
- [ ] Define a declarative package/import boundary; skill content registration must not run package hooks, executable payloads or installers.
- [ ] Test valid/malformed manifests, unsafe paths, missing capabilities and version conflicts; provide real CLI/MCP queries over the resulting records.

### B01-17 — Fixture, security and integration coverage

- [ ] Commit portable metadata/registry/protocol fixtures and explicit expected outcomes; avoid coupling scanner tests to this developer machine.
- [ ] Exercise scan -> recognition -> registry -> resolver -> policy -> execution -> audit through the actual daemon/CLI, with synthetic safe tools and a known installed tool.
- [ ] Add negative/compatibility/lifecycle tests for the above targets, including no-key mode, multiple instances, unknown execution prevention, peer access and redaction.
- [ ] Configure Windows/macOS platform checks where available; distinguish committed workflow configuration, successful compilation and verified host behavior. No CI or hosted repository account is assumed.

### B01-18 — Operational completeness and delivery

- [ ] Produce a runnable core build and reproducible local startup/smoke instructions. A set of crates that individually compiles without an integrated daemon/CLI path is insufficient.
- [ ] Document configuration, migrations, native scan coverage, policy, protocol, resource versions, known limitations and B02 input contracts.
- [ ] Run the full internal review/repair loop, especially an independent security/integration review, and deliver one clean revision.
- [ ] Include every target in a coverage matrix, with evidenced completed items and explicit blocked/unverified/not-implemented items. Do not silently narrow the checklist or mark incomplete required targets as accepted.

## Required verification and observable scenarios

Commit a workspace lockfile and make these commands meaningful for the B01 workspace:

```powershell
& 'C:\Users\hp\.cargo\bin\cargo.exe' fmt --all -- --check
& 'C:\Users\hp\.cargo\bin\cargo.exe' test --workspace --locked
& 'C:\Users\hp\.cargo\bin\cargo.exe' clippy --workspace --all-targets --locked -- -D warnings
& 'C:\Users\hp\.cargo\bin\cargo.exe' build --workspace --release --locked
git diff --check
```

Record actual results and explain target/platform-specific limitations. Integration commands and implemented CLI names go into a reproducible smoke script and the report, not invented output in this brief.

The final smoke evidence must cover:

| Scenario | Observable expected result |
| --- | --- |
| Empty registry, no AI key, native scan | Known metadata becomes real tool instances; unknown items remain candidates |
| Two runtime copies | Both instances and environment/evidence remain separately queryable |
| Capability canonical/alias and version request | Deterministic candidates and explanation; conflicts are explicit |
| Known safe tool allowed | Successful bounded process, structured result and redacted audit |
| Unknown executable with side-effect marker | No execution or marker during discovery/default-denied execution |
| Ask/deny and tampered/expired/replayed approval | No unauthorized process; stable error and audit outcome |
| Synthetic secrets in inherited env and args | Secrets absent from unapproved child environment and persisted logs/export |
| Daemon absent/restarted, concurrent clients | Bounded startup/failure and recovery; one shared durable registry |
| MCP and CLI inspect same item | Consistent identity/data and shared authorization behavior |
| Skill missing dependency | Useful missing-capability result; no installer execution |
| Expired/revoked discovery session | Inspection/probe/classification rejected according to the contract |
| Timeout/cancel and oversized output | Work is bounded; process cleanup and limits are observable |

Use synthetic values and safe fixture commands. Tests must not modify the user's actual installed tools, registry/PATH or unrelated application configuration.

## Consolidated delivery

Write `docs/reviews/B01-DELIVERY.md` plus `docs/reviews/B01-COVERAGE.md` and `docs/architecture/B01-DECISIONS.md` in the implementation branch. Include exact revision identities, target evidence, actual toolchain/commands, internal findings/repairs, native host coverage, protocol/resource compatibility, reproduction steps and limitations.

Publish the selected readable reports and a cloneable `ToolHub-B01.bundle` under MiMo's own workbench profile; include SHA-256 and base/head references. Keep temporary logs/artifacts outside tracked source unless deliberately selected and sanitized. This brief authorizes the normal task-specific artifact/progress handoff, not whole-project completion or archive.

After R1, address the whole consolidated finding list in one repair pass and return `B01-REPAIR.md` with finding -> commit -> verification mappings and a new immutable bundle version. R2 verifies the repair and allows bounded Codex corrections. B02 starts from the recorded reviewed revision after its separate dispatch.

## Scope boundaries

Desktop UI, complete temporary-provider UX, third-party runtime plugin loading, signed/public release and final platform packaging are B02 work. B01 supplies coherent working APIs and versioned contracts for them. Do not start B02 implicitly, install missing system components, create/push a hosted repository, merge feature work into the Codex baseline, or weaken product safety/privacy boundaries without applicable authorization.
