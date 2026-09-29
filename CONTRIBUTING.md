# Development and review

## Start a batch

1. Read the current workbench artifacts and batch task.
2. Verify the base revision and establish an isolated checkout or branch.
3. Review contracts and assign non-overlapping directory ownership before delegating work.
4. Record unresolved decisions and blockers explicitly.

The initial repository contains documentation only. Build commands become required once the relevant implementation and configuration exist; do not claim Rust, desktop, or platform tests were run for this bootstrap.

## Changes

Keep domain and protocol definitions centralized. Apply SQLite migrations from the first registry implementation. Use fixture-driven scanner tests rather than depending on the developer's installed software.

Document behavior-changing contract decisions and update affected documentation. Avoid unrelated refactors and unrequested dependency upgrades. Never weaken existing tests to disguise a regression.

## Verification

Use tests appropriate to the changed behavior: domain/registry/resolver/policy tests; scanner fixtures; scanner-to-registry-to-executor integration; IPC lifecycle checks; and relevant platform CI.

For documentation-only changes, check links, document consistency, encoding, and Git diffs. Report unsupported platform verification separately.

## Consolidated delivery to Codex

Include:

- Batch ID, base revision, head revision, branch, and changed-file scope.
- Implemented acceptance conditions and their evidence.
- Exact verification commands, exit results, and platform/environment.
- Internal review findings and repairs.
- Remaining failures, unverified items, and proposed follow-up work.
- Reviewable commits or a Git bundle when no remote is available.
- One short Chinese owner summary.

MiMo performs internal integration and review. Codex independently examines the delivered revision and critical behavior before recording acceptance. Successful self-review alone does not close a batch.

## Git policy

The bootstrap is a local repository on `main`. Development branches use `codex/` by default. Do not invent remotes or push, merge feature work into `main`, or publish releases without applicable owner authorization. Commit messages must describe actual changes.
