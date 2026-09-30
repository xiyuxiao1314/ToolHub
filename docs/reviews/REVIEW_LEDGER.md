# ToolHub formal review ledger

Formal reviews used: **2 of 4**. This ledger is the current review-count authority; onboarding, planning, internal reviewers, build/probe checks and the quota-interruption resume do not add rounds.

| Round | Delivery revision | Result | Next step |
| --- | --- | --- | --- |
| R1 — B01 initial review | `46a06aa11c370b2ca4432909aa2b8b0a70127090` | changes_required; 18 consolidated finding groups; independent Windows fmt/test (42)/Clippy/release build pass | MiMo one integrated repair delivery using `MIMO-B01-R1-REPAIR.md` |
| R2 — B01 repair review | `7a1ba39a8fb27dddffb4eb662a973c8bedf287ed`; bounded correction `6df2166e0c55ec64667021e386e75ca93acb066d` | **changes_required**; delivered Windows tests 47/47, corrected 50/50; IPC, identity/scanning, sessions, skills, wire contract and other core blockers remain | Owner decides remaining batch/review allocation; [result](B01-R2-RESULT.md) and [proposal](../tasks/B02-CARRYOVER-PROPOSAL.md) |
| R3 — B02 initial review | pending | not started | dispatched after the B01 outcome is recorded |
| R4 — B02 repair review | pending | not started | verify repaired product delivery |

R1 planning base: `6ffc9dc3c2fb526e740f0e3b29eafb84f4c54fce`. Bundle SHA-256: `ad4aeb965bd5c539d7c2f2337c35fbf840b167ba8142e367ce85f42c8a42375b`. Implementation parent in the submitted delivery document: `159bac10c22a18b10a3aa1165b9e521f681cc5b9`; review used the exact bundle HEAD above.

R1 artifacts: [findings](B01-R1-FINDINGS.md), [sanitized evidence](B01-R1-EVIDENCE.json), [selected reproduction package](B01-R1-REPRO.zip), [single repair brief](../tasks/MIMO-B01-R1-REPAIR.md).

B01 is not accepted. No implementation was merged into the local review main, no remote push/release was made, and the workbench project remains open. Unresolved required core functionality is not silently moved to B02. Four formal reviews are a ceiling, not permission to accept blockers; no fifth review starts automatically.

R2 reviewed input bundle SHA-256: `745b1400125cdc731f2fcf36ea089aba0bfb8c6c1f406c5e04844fd9237e9f60`. Codex correction bundle SHA-256: `21c8f94b282c3313b64748760108b89ef98d6bea091f14622237067de0aff325`. [R2 evidence](B01-R2-EVIDENCE.json), [selected reproductions](B01-R2-REPRO.zip), [correction bundle](ToolHub-B01-R2-corrections.bundle). R3 and R4 have **not** started; the combined product batch is a proposal only.
