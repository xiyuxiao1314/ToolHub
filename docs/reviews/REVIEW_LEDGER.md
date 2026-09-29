# ToolHub formal review ledger

Formal reviews used: **1 of 4**. This ledger is the current review-count authority; onboarding, planning, internal reviewers, build/probe checks and the quota-interruption resume do not add rounds.

| Round | Delivery revision | Result | Next step |
| --- | --- | --- | --- |
| R1 — B01 initial review | `46a06aa11c370b2ca4432909aa2b8b0a70127090` | changes_required; 18 consolidated finding groups; independent Windows fmt/test (42)/Clippy/release build pass | MiMo one integrated repair delivery using `MIMO-B01-R1-REPAIR.md` |
| R2 — B01 repair review | pending | not started | verify one corrected delivery; bounded Codex local corrections permitted |
| R3 — B02 initial review | pending | not started | dispatched after the B01 outcome is recorded |
| R4 — B02 repair review | pending | not started | verify repaired product delivery |

R1 planning base: `6ffc9dc3c2fb526e740f0e3b29eafb84f4c54fce`. Bundle SHA-256: `ad4aeb965bd5c539d7c2f2337c35fbf840b167ba8142e367ce85f42c8a42375b`. Implementation parent in the submitted delivery document: `159bac10c22a18b10a3aa1165b9e521f681cc5b9`; review used the exact bundle HEAD above.

R1 artifacts: [findings](B01-R1-FINDINGS.md), [sanitized evidence](B01-R1-EVIDENCE.json), [selected reproduction package](B01-R1-REPRO.zip), [single repair brief](../tasks/MIMO-B01-R1-REPAIR.md).

B01 is not accepted. No implementation was merged into the local review main, no remote push/release was made, and the workbench project remains open. Unresolved required core functionality is not silently moved to B02. Four formal reviews are a ceiling, not permission to accept blockers; no fifth review starts automatically.
