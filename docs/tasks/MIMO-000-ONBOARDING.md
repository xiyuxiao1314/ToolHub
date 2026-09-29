# MIMO-000 — Read ToolHub artifacts and prepare the development handoff

## Task identity

- Workbench: `task_a11c48d272163020b1ee37f1`
- Primary developer: MiMo; subagent orchestration belongs to MiMo.
- Planner and independent reviewer: Codex.
- Owner summary: ToolHub 文档与 Git 基线已准备，MiMo 先阅读工作台产物并确认接手，等待目标清单后开发。
- Batch scope: onboarding and design assessment only. The owner has not yet supplied the detailed feature target checklist.

## Goal

Find and read the latest workbench artifacts, establish the exact initial Git baseline in an independent checkout when needed, and give Codex a concise factual assessment that enables the first implementation batch.

## Inputs

1. Join the workbench project using `--profile mimo`, then use `sync` for new events.
2. Read this task and `BOOTSTRAP_MANIFEST.json` using artifact IDs from the current index.
3. Read the original design attachment and the repository's `docs/design/TOOLHUB_SYSTEM_DESIGN.md`.
4. Read `README.md`, `AGENTS.md`, `ARCHITECTURE.md`, `DOMAIN.md`, `PROTOCOL.md`, `SECURITY.md`, `CONTRIBUTING.md`, `WORKBENCH.md`, and `docs/ROADMAP.md`.

The manifest records the exact base commit, selected file hashes, and bootstrap bundle SHA-256. No remote Git repository exists yet. Do not assume a shared local path is available on another device.

## Checkout

If working on the same machine, inspect `D:\ToolHub` as the review baseline and use a separate checkout for future implementation. If working elsewhere, download `ToolHub-bootstrap.bundle` to a new authorized local path, verify the bundle, and clone it into a new independent project directory. Verify the cloned HEAD against the manifest.

The bootstrap contains documentation only. Any clone-generated `origin` referencing the downloaded bundle is a local bundle source, not an authorized hosted repository or push destination.

## Responsibilities

- Confirm access, profile, baseline revision, and document completeness.
- Identify contradictions and unresolved domain/protocol/security contracts, citing the source documents.
- Distinguish source-design examples from finalized schemas, identifiers, and APIs.
- Report development toolchain availability from read-only checks if needed; do not install dependencies during onboarding.
- Propose a bounded foundation batch with contract dependencies, directory ownership, acceptance conditions, and useful verification.
- Use subagents only where this assessment has independent bounded work; a single consolidated assessment is sufficient.

## Allowed changes

No product implementation is assigned in this task. Assessment deliverables may be written under `docs/reviews/` in MiMo's independent checkout. Source design and shared baseline contracts are read-only inputs for this task; propose changes in the assessment rather than editing them silently.

## Required result

Return one concise assessment containing:

- Task ID, baseline HEAD, artifact versions, and checkout path.
- Confirmed read status and any access or tooling blockers.
- Prioritized design gaps and contradictions with references.
- Suggested first foundation-batch boundaries and review gates.
- Explicit statement that feature development awaits the owner's target checklist and Codex's next task brief.
- One Chinese owner summary of at most 160 characters.

Post a concise acknowledgment/progress event through MiMo's own workbench profile. If the owner asks for a report artifact, publish `MIMO-000-ASSESSMENT.md` as an immutable selected artifact. Do not mark the whole project complete, submit a formal delivery report, or archive the task as part of onboarding.

## Acceptance conditions

- Workbench and selected artifacts are readable under MiMo's own identity, or the exact blocker is reported.
- The Git baseline identity matches the published manifest when a checkout is established.
- Assessment describes observable design gaps and bounded next work without claiming implementation or platform-test results.
- No software installation, tool execution through the unimplemented product, feature coding, repository push, or release occurs.
