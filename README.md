# ToolHub

ToolHub 是面向 AI Agent 与用户的本机工具基础设施层。

It discovers, identifies, catalogs, resolves, and invokes software already present on a Windows or macOS machine. Native discovery must work without an AI provider. ToolHub is a preferred local runtime; agents may choose other execution environments.

## Current state

This repository contains the documentation and two-batch development plan. Product code, binaries, build configuration, and runtime tests have not been implemented. The full supplied design is mapped to large batch targets; later owner additions are explicit scope changes.

- Local project: `D:\ToolHub`
- Initial branch: `main`
- Primary developer and subagent coordinator: MiMo
- Task planner and independent reviewer: Codex
- Product scope and priorities: project owner
- Collaboration: [ToolHub workbench task](https://agent-workbench-hk.tail7d7b36.ts.net:9443/tasks/task_a11c48d272163020b1ee37f1)

## Read first

1. [Original complete design](docs/design/TOOLHUB_SYSTEM_DESIGN.md)
2. [Agent collaboration rules](AGENTS.md)
3. [Architecture](ARCHITECTURE.md), [domain](DOMAIN.md), [protocol](PROTOCOL.md), and [security](SECURITY.md)
4. [Contribution and review process](CONTRIBUTING.md)
5. [Dependency roadmap](docs/ROADMAP.md)
6. [Workbench access](WORKBENCH.md) and [MiMo onboarding task](docs/tasks/MIMO-000-ONBOARDING.md)
7. [Two-batch implementation plan](docs/superpowers/plans/2026-09-29-toolhub-two-batch-plan.md), [four-round review policy](docs/collaboration/REVIEW_POLICY.md), and [B01 core task](docs/tasks/MIMO-B01-CORE.md)

B01 builds the integrated runnable core and agent-facing interfaces. B02 builds the complete real-data desktop, AI/extension/resource workflows and local release candidates. MiMo handles internal delegation, integration, review and repair; Codex has two formal reviews per batch, four total.

The original design is a product baseline, not an assertion that its schemas, interfaces, or examples have been finalized. Record proposed corrections and unresolved contracts explicitly.

## Product boundary

ToolHub discovers and uses existing tools. It does not become a package manager, system cleaner, sandbox provider, model provider, IDE, or general-purpose agent.

No remote Git repository is configured by this bootstrap. Git history and selected documents can be transferred through the workbench bootstrap bundle.
