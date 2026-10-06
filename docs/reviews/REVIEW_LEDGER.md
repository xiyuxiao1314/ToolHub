# ToolHub formal review ledger

Formal reviews used: **4 of 4**. This ledger is the current review-count authority; onboarding, planning, internal reviewers, build/probe checks, owner-authorized post-R4 residual completion, and the quota-interruption resume do not add rounds.

| Round | Delivery revision | Result | Next step |
| --- | --- | --- | --- |
| R1 — B01 initial review | `46a06aa11c370b2ca4432909aa2b8b0a70127090` | changes_required; 18 consolidated finding groups; independent Windows fmt/test (42)/Clippy/release build pass | MiMo one integrated repair delivery using `MIMO-B01-R1-REPAIR.md` |
| R2 — B01 repair review | `7a1ba39a8fb27dddffb4eb662a973c8bedf287ed`; bounded correction `6df2166e0c55ec64667021e386e75ca93acb066d` | **changes_required**; delivered Windows tests 47/47, corrected 50/50; IPC, identity/scanning, sessions, skills, wire contract and other core blockers remain | Owner decides remaining batch/review allocation; [result](B01-R2-RESULT.md) and [proposal](../tasks/B02-CARRYOVER-PROPOSAL.md) |
| R3 — integrated B01 residual + B02 initial review | `b62604095b248e7c13d44cfdfba8e789c0503c89` | **changes_required**; independent Windows tests 71/71, fmt/Clippy/release/smoke/package pass; 15 consolidated groups remain | One MiMo integrated repair under [repair brief](../tasks/MIMO-B02-R3-REPAIR.md), then R4 verification |
| R4 — integrated product repair review | `codex/toolhub-r4-final` base `6c40a3a`; Codex WIP `442b311`; MiMo repair `604c383`; result `64633aa` | **changes_required** with substantial progress accepted as verified; independence limitation (MiMo self-review after Codex quota exhaustion). See [B02-R4-RESULT](B02-R4-RESULT.md) | Owner-authorized residual completion on this branch (soak + nine-page UI evidence + local toolchain); see [B02-R4-RESIDUAL](B02-R4-RESIDUAL.md) |

R1 planning base: `6ffc9dc3c2fb526e740f0e3b29eafb84f4c54fce`. Bundle SHA-256: `ad4aeb965bd5c539d7c2f2337c35fbf840b167ba8142e367ce85f42c8a42375b`. Implementation parent in the submitted delivery document: `159bac10c22a18b10a3aa1165b9e521f681cc5b9`; review used the exact bundle HEAD above.

R1 artifacts: [findings](B01-R1-FINDINGS.md), [sanitized evidence](B01-R1-EVIDENCE.json), [selected reproduction package](B01-R1-REPRO.zip), [single repair brief](../tasks/MIMO-B01-R1-REPAIR.md).

B01 is not accepted as a standalone batch; residual blockers were folded into the owner-approved integrated B01+B02 product batch. No formal fifth review is opened automatically. Four formal reviews are a ceiling, not permission to accept blockers.

R2 reviewed input bundle SHA-256: `745b1400125cdc731f2fcf36ea089aba0bfb8c6c1f406c5e04844fd9237e9f60`. Codex correction bundle SHA-256: `21c8f94b282c3313b64748760108b89ef98d6bea091f14622237067de0aff325`. [R2 evidence](B01-R2-EVIDENCE.json), [selected reproductions](B01-R2-REPRO.zip), [correction bundle](ToolHub-B01-R2-corrections.bundle). The owner approved the combined product scope and MiMo dispatch on 2026-09-30. R3 is complete with changes_required. Formal reviews used: **4 of 4**.

R3 input bundle SHA-256: `fc90c63f8c525447f0a81eb08c741bf67bd68369e77c2ec55103d0b6e02ebf26`. [R3 findings](B02-R3-FINDINGS.md) (published on docs `main` / restored in residual packet), [evidence](B02-R3-EVIDENCE.json), [selected reproductions](B02-R3-REPRO.zip), [single repair task](../tasks/MIMO-B02-R3-REPAIR.md). Actual browser screenshots: [Settings](B02-R3-SETTINGS.jpg), [Security](B02-R3-SECURITY.jpg). The private Settings fixture path is kept local. R3 made no product correction.

R4 residual completion (owner-authorized 2026-09-30): long soak fully observed (`cargo test -p toolhub-daemon --test soak` **5/5**), nine-page UI operation evidence captured under [evidence/nine-page](evidence/nine-page/), review ledger updated. macOS host verification remains **out of scope by owner direction**. Independent pre-merge code review required before merge/workbench close.
