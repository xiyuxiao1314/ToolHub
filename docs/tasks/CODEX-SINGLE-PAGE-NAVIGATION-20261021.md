# Single-page extension navigation repair

Owner request: the AI Prompts sidebar repeats the same name as a package group and page. Base: 500508ae2c19c654f53cd587b4cb4dbb0572519e (current production beta.7). Independent branch codex/navigation-single-page; pending prompt-editor beta.8 work remains on codex/prompt-library-editor and is excluded from this fix.

Goal: single-page packages render one direct secondary link; multiple-page packages retain expandable groups. Search must use the full installed page count to choose grouping and preserve selected/pinned destinations, keyboard activation and package-name matching. No schema, SDK, extension content or database changes.

Allowed files: WorkbenchNavigation.tsx, existing navigation evidence script, this task/delivery documentation. Read-only dependencies: navigation model, host and styles. Direct Codex implementation; no delegation.

Checks: UI typecheck/build; existing native navigation acceptance including single-page direct links and multi-page search; verify screenshot in current-size Windows QA. Build production-matched beta.7, back up production libraries and display storage, preserve installed extension digests/settings, install and reopen only if no active jobs. No GitHub publication; macOS unverified.
