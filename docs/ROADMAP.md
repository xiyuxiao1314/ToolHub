# Two-delivery roadmap

The owner requests large consolidated targets and at most four formal Codex product reviews across the project. The full design remains the product scope. The [implementation plan](superpowers/plans/2026-09-29-toolhub-two-batch-plan.md) maps it to two deliverables; the [review policy](collaboration/REVIEW_POLICY.md) governs handoffs.

| Delivery | Scope | Formal Codex review budget |
| --- | --- | --- |
| B01 | Domain/protocol/SQLite, native scanners and recognition, environments/ownership, resolver/security/execution/audit, daemon/IPC/CLI/MCP, sessions/adapters and skills | R1 consolidated review -> MiMo repair -> R2 verification and bounded Codex corrections |
| B02 | Nine-page real-data desktop, connected-agent/temporary-provider flows, scanner extension/resource/update lifecycle, SDK/docs, platform QA and local release candidates | R3 consolidated review -> MiMo repair -> R4 verification and bounded Codex corrections |

Formal rounds used: **0 of 4**. The onboarding assessment and planning work do not consume formal implementation-review rounds.

The original development stages now guide MiMo's internal dependencies rather than eight separate external deliveries:

1. Domain, manifests, protocol and registry.
2. Scanner framework, recognition and OS discovery.
3. Environments, ownership and duplicates.
4. Resolver, policy, executor, sanitizer and audit.
5. Daemon, IPC, CLI, MCP and adapters.
6. Skill availability and manifests.
7. Desktop and full user workflows.
8. Integration, packaging and platform delivery.

MiMo internally locks contracts, delegates bounded independent work, integrates, tests, reviews and repairs before a single handoff. UI can prepare against stabilized contracts when assigned, but all shipped clients share the daemon registry and policy.

B01 is dispatched by `docs/tasks/MIMO-B01-CORE.md`; B02 is planned and awaits dispatch against the reviewed B01 revision. Real access/toolchain blockers and departures from product boundaries are escalated; routine implementation decisions stay with MiMo.

Unavailable host/signing checks and unfinished targets are recorded honestly. Four formal reviews do not authorize accepting blocking defects or claiming unverified platform delivery.
