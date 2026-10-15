# Storage maintenance — owner-approved local task

Base 3a9a4ad; branch codex/storage-maintenance. The owner requests safe removal of unused build/test caches and historical packages and a mechanism to prevent recurrence. This is a direct local maintenance delivery, not another historical formal product review. No GitHub publication is requested.

Scope: independent scripts/tests, Cargo dev/test profiles, local packaging/install retention, developer docs. Exclude runtime/protocol/schema changes, user data deletion, global dependency caches and public-release archives. Preview first; preserve current/recent/rollback and live/config/data references; recycle old packages without a permanent fallback. Only verified reproducible Cargo target outputs can be permanently deleted.

Acceptance: inventory before/after, meaningful fixture safety tests and native recycle check; actual cleanup with path/liveness/data protection; installed executables/config hashes and user records unchanged; application and MCP remain healthy. Evidence stays under D:\codex\toolhub-storage-20261015. Windows-only maintenance; macOS unverified.

## Delivery and evidence

Implemented: preview-first Windows maintenance command, Cargo cache validation, current/recent/rollback and process/config/registry reference protection, per-operation context rechecks, reparse/tracked-source refusal, old-package recycling with no permanent fallback, report overwrite protection, conservative package/install retention with opt-outs and nonfatal failures, low-footprint dev/test Cargo profiles.

Verified: 14 Python safety tests; actual installer/packaging retention-hook tests including opt-outs/failure handling; hidden-process UTF-8/reference inspection; Windows recycle smoke test with a Unicode/space/apostrophe path and increased Recycle Bin count; locked offline Cargo manifest metadata; PowerShell/parser and packaging syntax checks; package copy/52-file checksum validation using the verified installed native trio. The disposable packaging verification copy was recycled afterwards. No Rust runtime/protocol change was made or full rebuild performed.

Cleanup: 34.741 GiB reproducible Cargo target removed; 28 obsolete package directories totaling 0.532 GiB recycled. The source/project parent shrank from 35.487 GiB to approximately 0.402 GiB. D: free space increased about 29.34 GiB at the post-cleanup snapshot; logical tree sizes differ from physical free space and other processes may change the latter. Recycled packages are not counted as freed disk space and the user's existing Recycle Bin was not emptied. Global dependency caches, user data, release archives and historical evidence/backups were retained.

Preservation: hashes of three running executables, current.json, two host configs and two integration Skills unchanged. Registry integrity ok; 44 tools, 6 programs, 2 Skills, 1 artifact and 7 installed extensions retained; program/Skill/artifact/extension/policy rows unchanged. Existing GUI process stayed responsive, login task remained enabled with the same path, native CLI status and actual MCP search both succeeded. Current build stays workbench-20261013-6; no live installation or MCP process restart was required.

Limitations: maintenance is Windows-only; Python 3.11+ and PowerShell 7+ wrapper required, Git needed for cache source inspection. The new packaging/install retention applies to future packages; the existing installed executables were deliberately kept unchanged. Build caches are explicitly rebuilt on the next developer build. Arbitrary custom host configs elsewhere require --reference-file. No GitHub publication or main merge in this task.
