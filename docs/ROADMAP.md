# Dependency roadmap

The complete design remains visible. The following stages organize dependencies; they do not reduce the product to an MVP or claim delivery dates.

| Stage | Work domains | Gate before dependent implementation |
| --- | --- | --- |
| 0 | Documentation baseline and MiMo onboarding | Read source design; identify gaps; receive owner target checklist |
| 1 | Domain, manifests, protocol, registry and migrations | Reviewed shared contracts and persistence behavior |
| 2 | Scanner framework, recognition, Windows and macOS discovery | Read-only candidate pipeline with recognition fixtures |
| 3 | Environments, ownership, origins and duplicate analysis | Evidence-based attribution and preserved multiple instances |
| 4 | Resolver, policy, executor, sanitizer and audit | Authorization and execution security verification |
| 5 | Daemon, IPC, CLI, MCP and agent adapters | Shared registry, bounded failure behavior and integration contracts |
| 6 | Skills and capability requirements | Consistent manifest parsing and availability resolution |
| 7 | Desktop and environment map | Reviewed daemon contracts and real-data client behavior |
| 8 | Integration, platform QA, packaging and update strategy | Verified delivery scope, compatibility and release evidence |

Some independent work can overlap after its input contracts are reviewed. MiMo must plan concurrency from dependencies and file ownership rather than stage numbers alone.

Each dispatched batch will map owner targets to acceptance conditions, tests, allowed directories, and an explicit review gate. Stage 1 is the anticipated foundation area, not a currently authorized feature-development task.

Recognition resources, protocol/manifests, and desktop/core release versions should remain independently versionable as the design requires. CI, fixtures, security review, and integration checks accompany implementation rather than waiting until the final stage.
