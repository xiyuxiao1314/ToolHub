# Development and review

## Start a batch

1. Read the current workbench artifacts and batch task.
2. Verify the base revision and establish an isolated checkout or branch.
3. Review contracts and assign non-overlapping directory ownership before delegating work.
4. Record unresolved decisions and blockers explicitly.

The initial repository contains documentation only. Build commands become required once the relevant implementation and configuration exist; do not claim Rust, desktop, or platform tests were run for this bootstrap.

The owner has selected two large batches: runnable core and complete desktop/delivery lifecycle. Follow `docs/superpowers/plans/2026-09-29-toolhub-two-batch-plan.md`, the dispatched batch brief, and `docs/collaboration/REVIEW_POLICY.md`. Internal work packages remain focused and independently testable; they do not trigger per-subtask Codex reviews.

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

Each batch has two formal Codex rounds: consolidated initial review, one MiMo repair delivery, then verification and bounded direct Codex corrections. The total project ceiling is four formal reviews; unresolved blocking behavior is not accepted to satisfy the ceiling.

## Git policy

The bootstrap is a local repository on `main`. Development branches use `codex/` by default. Do not invent remotes or push, merge feature work into `main`, or publish releases without applicable owner authorization. Commit messages must describe actual changes.
