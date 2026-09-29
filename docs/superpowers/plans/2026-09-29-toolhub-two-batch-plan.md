# ToolHub two-batch implementation plan

> For agentic workers: MiMo owns execution and internal subagent coordination. Break the goals below into bounded internal work packages, integrate and internally review them, then deliver one packet per milestone. Use available execution/development skills where applicable; external formal review follows the four-round policy rather than a review per subtask.

**Goal:** Build the complete ToolHub product from the supplied design in two large integrated deliveries with at most four formal Codex review rounds.

**Architecture:** A Rust user daemon owns one SQLite registry, native scanners, recognition, capability resolution, policy, execution and audit. CLI, MCP, agent/skill bridges and the Tauri/React desktop consume reviewed shared contracts. AI-assisted recognition is optional; native behavior and safety controls stand on their own.

**Tech stack:** Rust, Tokio, Serde, Clap, tracing, SQLite; Tauri, React, TypeScript for desktop. MiMo selects compatible dependencies and commits lockfiles, recording version/toolchain decisions.

## Baseline and authority

- Project: `task_a11c48d272163020b1ee37f1`; source/review baseline `D:\ToolHub`.
- Original documentation commit: `ce8c755f9ecb2e8a6655d6997f21767332502a45`.
- The current planning commit is recorded in the published planning manifest and dispatch event. Start B01 from that exact commit, not from a stale bootstrap-only checkout.
- MiMo checkout: `D:\ToolHub-mimo\checkout`; preserve the existing untracked onboarding assessment.
- Source design: `docs/design/TOOLHUB_SYSTEM_DESIGN.md`.
- Governance: [review policy](../../collaboration/REVIEW_POLICY.md) and [B01 brief](../../tasks/MIMO-B01-CORE.md).

The owner has requested larger goals and fewer reviews. This plan uses the supplied complete design as the current target checklist. Later owner additions are explicit scope changes; they do not require waiting before starting the dispatched B01 work.

## Dependency strategy inside B01

- [ ] Establish project/toolchain checks, workspace, shared domain, manifests, protocol and execution-approval contracts.
- [ ] Internally review and provisionally lock the contracts; create ownership and dependency maps.
- [ ] Implement registry, scanner framework and recognition against the same contract.
- [ ] Delegate Windows/macOS metadata backends and environment/ownership work when their inputs are stable.
- [ ] Implement resolver, policy, sanitized execution and audit; integrate native discovery-to-execution.
- [ ] Build daemon/IPC/CLI, bounded MCP meta-tools, discovery sessions, adapters and skill resolution on the working core.
- [ ] Integrate fixture, security, lifecycle and host smoke checks; repair internally and deliver one B01 packet.

Subagents can overlap independent work. Shared schemas, workspace manifests, lockfiles and integration wiring have a single designated parent/integration owner. Contract changes are logged and propagated internally before dependent code uses them; there is no external review after every contract edit.

## B01 — Runnable core and agent-facing product

The detailed dispatched target list is [MIMO-B01-CORE.md](../../tasks/MIMO-B01-CORE.md). B01 includes the core parts of all eight original development stages necessary for a usable CLI/MCP product, not only the originally proposed Stage 1.

Observable milestone: from a fresh local registry, a no-key native scan discovers known tools and preserves unknown candidates; CLI/MCP inspect the same registry, capability resolution explains choices, policy-controlled execution of a known safe tool records a redacted audit entry, and discovery sessions/skill requirements operate through the same daemon contracts.

B01 formal review allocation: **R1 + R2**. MiMo returns a single internally reviewed delivery, performs one consolidated repair pass, then Codex verifies and applies suitable localized corrections.

## B02 — Complete desktop product and delivery lifecycle

**Planning status:** reserved second milestone. Its goals are visible now so B01 contracts support them. Start B02 implementation after its dispatch against the resulting B01 review revision; this plan does not instruct MiMo to start a second branch concurrently.

### Product and UI targets

- [ ] **B02-01 Desktop shell:** Tauri/React/TypeScript application connects to the existing daemon, handles startup/reconnect/error states and consumes shared generated/validated types. No UI-local scanner or second registry.
- [ ] **B02-02 Overview:** real counts, latest scan, new/unknown tools, recent activity and environment changes, with honest empty/loading/stale states.
- [ ] **B02-03 Tools and detail:** search/filter by category, capability, vendor, environment, owner, trust and architecture; show all instances/interfaces, evidence, paths, dependencies and availability.
- [ ] **B02-04 Environment map:** usable hierarchy/graph of origins, environments, tools, likely owners and duplicates; show attribution uncertainty and distinguish executable/environment size estimates. No cleaning or deletion actions.
- [ ] **B02-05 Capabilities:** canonical taxonomy, aliases, provider instances and resolver explanations consistent with CLI and skill results.
- [ ] **B02-06 Skills:** instruction/MCP/package skill views, manifests, required/optional capability status and useful missing-dependency messages. Import/download of declarative skill content must not install its tool dependencies or run hooks.
- [ ] **B02-07 Agents:** detected integrations, health, connection state, recent ToolHub activity, environments and supported discovery launch paths; represent unsupported adapters honestly.
- [ ] **B02-08 Activity:** scanner, registry, resolver and execution events with real timestamps, filters and redaction. Default sensitive stdout is not persisted.
- [ ] **B02-09 Security:** effective policy, approval UI, unknown-executable state, discovery scopes and temporary credential lifecycle; edits affect the daemon policy and are reflected by other clients.
- [ ] **B02-10 Settings:** scan roots/modes, preferences, privacy/export controls, daemon behavior and independent app/core/recognition versions; settings survive restart except sensitive session data.

### AI, extension and resource lifecycle targets

- [ ] **B02-11 Connected agent flow:** select a detected supported agent, create a limited discovery session, inspect the metadata disclosure, launch/monitor/cancel the session and review classifications with provenance. ToolHub cannot claim to sandbox a third-party host shell.
- [ ] **B02-12 Temporary provider flow:** explicitly configured temporary API provider processes only approved metadata; keys stay in nonpersistent session memory, are redacted on every error path and are destroyed on completion/cancellation. Disabled mode remains fully usable.
- [ ] **B02-13 Scanner extensions:** permission-limited extension contract and a working example/fixture plugin. Third-party code cannot silently acquire broad host write/execution authority; document actual enforcement and disable unsupported unsafe loading paths.
- [ ] **B02-14 Resource versions:** load/version/validate recognition and capability data separately from desktop/core. Apply local verified resource updates atomically with compatible rollback and invalid-package tests.
- [ ] **B02-15 Product updates:** separate ToolHub's own update lifecycle from its prohibition on installing discovered tools. Implement configured, verified package/update plumbing with explicit user action, version compatibility, cancellation and recovery tests. Do not invent a live update server, signing identity or release channel.

### Integration and delivery targets

- [ ] **B02-16 Cross-client behavior:** desktop edits/scan results/policy decisions appear in CLI/MCP, daemon remains usable when the desktop closes, and restart/reconnect/event ordering preserve correct state.
- [ ] **B02-17 Usability:** nine primary pages reachable; keyboard focus, readable layout, reduced-motion behavior, useful empty/error states and responsive narrow desktop layout verified. No dead controls or fabricated metrics.
- [ ] **B02-18 Windows release candidate:** reproducible local daemon/CLI/desktop artifacts, migrations and recognition resources packaged together with startup/uninstall documentation. Do not install/uninstall user software while testing unrelated workflows.
- [ ] **B02-19 macOS Intel/ARM evidence:** configure platform builds/tests and exercise an authorized macOS host or available CI if one exists. Distinguish configuration, cross-build and real host verification. Missing host/signing access is a recorded blocker, not a passing platform claim.
- [ ] **B02-20 QA and privacy:** run fixture/unit/integration suites, actual Windows UI/IPC/native discovery flows, execution-denial/security regressions and credential/log/export inspection using synthetic values.
- [ ] **B02-21 Documentation and SDK:** complete developer docs, public manifest/protocol documentation and minimal TypeScript client usage. Refer to local schemas; owning or deploying `schemas.toolhub.dev` is outside the current task.
- [ ] **B02-22 Local delivery packet:** source bundle and exact revisions, build artifacts, checksums, target/evidence matrix, UI screenshots, known limitations, signing/host gaps and repairable issues. Hosted Git push, public release and production publication remain separate owner-authorized actions.

B02 formal review allocation: **R3 + R4**. A local unsigned release candidate may be delivered honestly; it is not a signed/public release. Unsupported/unverified items remain in the coverage matrix and block claims of complete platform delivery.

## Source-design coverage

| Original scope | B01 | B02 |
| --- | --- | --- |
| Core daemon, tool models, SQLite and shared registry | Working core and contracts | Desktop integration and release migration verification |
| Scanner/candidate/recognition, Windows/macOS, scan modes/incremental behavior | Native pipelines, fixtures and first host smoke | Broader host QA, UI scan controls and tuning |
| Capability/environment/ownership/duplicate analysis | Canonical data and analysis APIs | Capability views and environment map |
| Resolver/executor/policy/sanitizer/audit/fallback | Working controlled execution and negative cases | Approval UI and full client regression |
| CLI/IPC/MCP/protocol/manifests | Agent-facing working contracts and metatools | SDK/docs and desktop contract compliance |
| Discovery sessions/agent adapters/skills | Working session scopes, supported adapters and skill availability | Complete user workflows and temporary-provider UX |
| Plugins/recognition resources/update separation | Extension contracts and versioned local resources | Example extension and verified package/update lifecycle |
| Nine desktop pages/privacy/export | Core endpoints and redacted exports | Complete real-data UI and usability evidence |
| QA/packaging/release versions | Workspace checks, fixtures and core smoke | Platform build/package/host evidence and local release candidates |

Every unresolved feature is listed in the delivery matrix; silently dropping low-visibility systems to fit two batches is not allowed.

## Evidence and review cadence

MiMo runs the internal implementation -> tests -> integration -> independent internal review -> repair loop as needed. External handoffs occur only at integrated delivery and the consolidated repair delivery. Useful progress events may state a real contract freeze, completed integration, or blocker; they do not trigger extra Codex review rounds.

The batch report records actual command results, not assumed successes. No global minimum test count is imposed; negative/security/compatibility cases and observable behavior matter more than quantity. Formal review policy defines the remaining-defect handling and exact round count.
