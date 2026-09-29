# Four-round review policy

## Owner decision

The owner requests large consolidated goals, MiMo-managed implementation and subagents, and at most four formal Codex product-review rounds across this project. This policy supersedes earlier wording that could require a separate Codex approval for each intermediate domain or schema decision.

## Delivery and review budget

| Milestone | Delivery | Formal Codex rounds |
| --- | --- | --- |
| B01 | Runnable core: native discovery through registry, resolution, authorized execution and audit, with CLI/MCP/skills/agent-session integration | R1 consolidated review; MiMo repair; R2 verification and bounded Codex corrections |
| B02 | Complete desktop product, AI-assisted flows, extension/data lifecycle, platform QA and local release candidates | R3 consolidated review; MiMo repair; R4 verification and bounded Codex corrections |

The budget starts with the first implemented B01 delivery. MIMO-000 onboarding assessment, task planning, workbench synchronization, and routine status reads are not formal product reviews. Formal rounds used: **0 of 4**. Record subsequent review results once, with exact revision IDs and evidence.

## Inside a large batch

MiMo determines internal work packages, dependency order, subagent count, file ownership, tests, integration checkpoints, and internal reviews. Those checkpoints do not require repeated owner/Codex handoffs. Context capacity is useful for continuity; it does not remove interface dependencies or the need for focused artifacts.

MiMo first records a coherent domain/protocol/security contract, uses an independent internal review where useful, and locks a provisional version for its subagents. Within the batch, parent-approved contract revisions update schemas, clients, tests, documentation, and a decision log atomically. Codex reviews these decisions with the consolidated delivery.

Escalate only decisions outside the dispatched product boundary, required new installation authorization, access blockers, or changes that weaken the established safety/privacy invariants. Routine naming, crate layout, query design, UI composition, and internal scheduling are MiMo decisions within the brief.

## Each formal round

1. Deliver an integrated clean revision with a complete target-to-evidence matrix and internally repaired findings.
2. Codex examines the exact code and relevant running behavior, then produces one prioritized consolidated finding list. Distinguish blockers from improvements.
3. MiMo repairs the first-round list together, reruns affected verification, and delivers one corrected revision plus finding-to-evidence mapping.
4. Codex's second round verifies repairs and critical regressions. Codex may directly apply and test small localized corrections in a dedicated review-fix branch/check-out, preserving MiMo's work and recording the resulting revision.

Direct Codex corrections are authorized by the owner's requested workflow. They do not authorize broad rewrites, remote push, release publication, or silent merging into `main`. Follow local code-safety and applicable testing rules when correcting code.

One round may include multiple necessary evidence checks and verification of localized corrections before its result is issued; do not relabel a new developer delivery after a finished round as the same review to evade the budget.

## Completion honesty

Four reviews are a workflow ceiling, not a guarantee of correctness. Do not reduce verification, invent passing results, or accept unresolved blocking defects to fit it. If a batch remains blocked after its second round, consolidate the unresolved state and stop acceptance of the affected behavior; the owner decides the resulting scope or workflow change. Do not automatically open a fifth formal round.

## Delivery packet

- Exact base/head commit, branch and clean status; cloneable bundle when no hosted remote exists.
- Stable target IDs mapped to implementation, automated checks, host/manual checks and limitations.
- Full decision log and changes to domain/protocol/security contracts.
- Internal review findings and repairs; an independent security/integration pass where feasible.
- Relevant commands, exit results, toolchain/platform, and usable logs with secrets redacted.
- Reproduction instructions and fixture inputs for critical flows.
- For repairs: every Codex finding mapped to its change and fresh verification.
- One Chinese owner summary; selected workbench artifacts under MiMo's own profile.

Product review is risk-focused and evidence-based; it is not a promise of reading every line or exhaustively checking every host/tool combination.
