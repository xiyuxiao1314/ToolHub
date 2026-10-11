# Task artifact packages — local delivery

Date: 2026-10-11. Owner summary: 新增产物导航，按任务登记、扫描与分级回收文件，保留撤销期及人工保留能力。

## Goal and acceptance

Group explicitly selected task files into named packages without moving them during registration. Support Agent registration and read-only scans of user-selected directories. Low priority is eligible only after successful task completion and explicit no-longer-needed confirmation; medium requires an Agent's actual usage/dependency checks and recorded decision; high priority and human retention require desktop decisions. All cleanup uses the operating system Recycle Bin/Trash with no permanent-delete fallback. Both automatic tiers have a 300-second grace period.

Observable acceptance: package/file details and history render; scanning does not mutate files and requires checkbox selection; manual collection selects actual files first; six-package pagination works; human retention cancels scheduled cleanup; changed, locked, referenced and protected files are preserved; ordinary MCP/CLI cannot modify controller-only rules; recycled fixture files appear in the Windows Recycle Bin; existing user records survive migration and installation.

## Base and scope

Base: `1bd8928e5440ad9cc9d0dc556a0de817037fd704`. Branch: `codex/artifact-packages`. The local source commit introducing this brief is the delivery head; its exact ID is reported with delivery. This is an owner-requested feature delivery, not a fifth formal product-review round. State: implemented and internally verified on Windows; macOS native checks remain unverified.

Dependencies: existing authenticated desktop controller, named-object THP validation, bounded MCP bridge, registry migrations, program/Skill/tool references, native file picker and portable integration Skill. One developer owns the shared contract and implementation; no delegated tasks were used.

Allowed changes: artifact core model/schema/migration, daemon lifecycle and native recycle implementation, MCP/protocol/SDK mapping, desktop page/navigation/native picker hooks, relevant tests and documentation. Existing discovery/execution mechanisms are read-only dependencies except for bounded dispatch/timer wiring. No broad refactor, automatic whole-disk deletion, permanent deletion, built-in AI model, high-priority Agent deletion, release publication, or GitHub push is included. Owner decisions establish the three tiers and selected-directory scope; shared contract changes were applied together to server, MCP, SDK, UI, Skill and docs.

## Delivered contract

- Registry migration 6 adds package, directory and history tables.
- THP adds 12 artifact methods. Scope selection, scans, priority/retention updates, manual recycle and registration release require the verified native controller.
- MCP stays bounded at 16 total tools, including four artifact tools. `list_artifact_packages` returns up to 20 summaries per page; `id` alone returns the complete manifest/history.
- A package contains 1–128 explicit regular files, each at most 256 MiB, up to 1 GiB total. Registration is bounded to 1,000 packages and 32 management roots. Files remain at their original paths. Symlinks/reparse points, traversal, Windows alternate data streams and protected directories are rejected.
- Low/medium cleanup validates fingerprints and known references again; active ToolHub jobs/scans defer it. Each native recycle result is persisted; failures or interrupted cleanup preserve pending files and require inspection.
- Desktop adds selection, read-only scan, package filtering/pagination, details/history, human retention, manual confirmation, directory/Recycle Bin opening and copied Agent instructions. Releasing a stale registration preserves the file and permits explicit re-registration.
- Portable Skill, Codex/MiMo local Skills and MCP configuration point to the installed build. Existing unrelated configuration was preserved. Already-running Agent sessions need to reload MCP/Skill metadata.

## Verification

Commands run against this worktree:

```powershell
cargo test -p toolhub-core -p toolhub-registry -p toolhub-protocol -p toolhub-mcp -p toolhub-daemon -p toolhub-cli -p toolhub-desktop
cargo test -p toolhub-daemon -p toolhub-mcp -p toolhub-protocol artifact -- --nocapture
cargo clippy -p toolhub-daemon -p toolhub-mcp -p toolhub-protocol --all-targets -- -D warnings
```

The related Rust suite passed during implementation. After the final path/pagination corrections, all 12 daemon artifact tests, two artifact MCP tests and one protocol artifact test passed; final scoped Clippy passed. Desktop UI TypeScript/Vite builds and SDK declaration checking passed; existing four SDK client tests passed. Release CLI, daemon and desktop builds passed. All 26 packaged file SHA-256 entries matched.

Real Windows Tauri/WebView plus stdio MCP validation passed 16 flows, including actual automatic/manual recycle, medium assertions, high protection, changed-file protection, pagination, clipboard copying, release-preserves-file and no renderer errors. Separately, the installed application passed eight integration checks, including production registry selection, login startup command, embedded Skill, 16 MCP tools and accurate summary/full-manifest reads. Six data-preservation checks confirm the original five program rows and two Skill rows are unchanged, 44 original tool IDs remain, migration 6 applied, no cleanup directories were silently enabled, and all four registered real task helper files remain present.

Evidence:

- [Native Windows checks](../reviews/evidence/artifacts-20261011/native-windows.json)
- [Installed Windows checks](../reviews/evidence/artifacts-20261011/installed-windows.json)
- [Data preservation](../reviews/evidence/artifacts-20261011/data-preservation.json)
- [Behavior and limits](../development/ARTIFACTS.md)

Validation recycled only owned isolated fixtures. Initial harness issues with clipboard permission, asynchronous polling and path separators were corrected; the final reports above use the completed harness. Private configuration/database backups and local screenshots remain outside version control.

## Installation and limitations

Local build: `artifacts-20261011-4`, installed through the existing local installer. The pre-existing enabled login startup now targets this build. This feature is not in the previously published `v0.3.0-beta.1` release assets. The normal desktop is launched for owner testing; test/debug desktop windows are closed.

Windows recycle behavior was tested natively. macOS Finder Trash and platform dialogs were not verified on a Mac in this delivery. Native file picker compilation and shared hooks were checked, but interactive picker selection was not part of the automated WebView flow. External applications' file references/usage cannot all be inferred by ToolHub; an Agent must perform real checks for medium decisions. Filesystem checks do not provide complete isolation from concurrent external replacement. Folder recycling and automatic external restore tracking are excluded; recovery is through the system Recycle Bin/Trash.
