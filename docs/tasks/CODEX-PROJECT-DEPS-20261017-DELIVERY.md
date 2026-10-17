# Electron and selected-project dependency discovery — verified local delivery

Base 504abf9; branch `codex/project-dependency-discovery`. Development app/CLI/daemon/SDK version `0.3.0-beta.5`. Final local build ID `project-deps-20261017-2`; not a GitHub release.

## Result

- Formally registered the selected complete Electron using extension preview/install and a custom scan, then replaced the one-time adapter with the built-in Electron rule and a project scan. Stable instance ID `inst-2bbc9027c0cc0dd4`; version **37.10.3**, project **vam-organizer**, `project_local` origin, project environment, `available`, trust **unknown**. Real Electron was never executed.
- The old 96 MiB prefix reader missed version resources in this roughly 195 MiB executable. Windows now reads a small native header and at most 1 MiB of version resources using version.dll data APIs. Explicit real-file metadata regression passed.
- Bounded Electron adapter reads exact selected manifests/package/path metadata and the resolved executable. Default all-disk scans still skip node_modules. Relative path traversal, external package links, oversized metadata and unsupported binary headers are rejected. An explicit distribution directory can be selected without executing npm/package scripts. Shared paths deduplicate; all project references survive.
- Schema migration 8 stores project observations atomically with inventory ingestion. Missing package/binary records are distinct from executable instances. Incomplete/cancelled scopes retain prior observations. Missing/invalid binary replacements retire stale available records; user trust and blocked decisions survive rescans.
- Added desktop project selection/custom distribution selection/status panel and project attribution; CLI `scan --mode project --roots ... [--electron-dist ...]`; typed SDK methods and `registry.project_dependencies`. The MCP surface remains **16 generic tools**.
- At the owner's additional request, the verified incomplete Atria Electron **33.4.11** npm package was moved to **Windows Recycle Bin**, retaining manifest and lock. It had neither path.txt nor dist binary and no ambient distribution override. The real Atria project now correctly reports `missing_package`.

## Verification

- Relevant Rust units: daemon 60 at initial full pass plus the additional shared-distribution regression; environment 4, protocol 10, recognizer 11, registry 15, scanner 9. Targeted final lifecycle tests (2) passed after the invalid-replacement fix.
- Scanner integration tests (4): complete/npm-only/absent dependencies, multiple versions, repeated roots, shared explicit distributions, traversal/absolute/non-native rejection, metadata byte limits, cancellation. Unix confinement test is included but was not run on Windows.
- Core (23), MCP (10), protocol (10) contract tests passed. Managed daemon authentication/reconnect integration passed after CLI build (its first run failed because the CLI executable had not yet been built).
- SDK 5 tests and strict TypeScript consumer check passed. Desktop `tsc && vite build` passed. Paired native binaries built offline and all 54 package files passed checksum verification.
- Isolated native desktop scanned both actual selected projects; Electron version/project/unknown trust and Atria missing-package state passed; repeated scans kept one Electron instance. Native UI inspected and screenshots captured. File picker was not automatically exercised; actual scans used the supported native RPC with the owner's explicit paths.
- Final installed beta.5 native desktop and fresh **actual MCP** `search_tools` / `inspect_tool` calls passed: one Electron 37.10.3, correct project and unknown trust. `tools/list` still returns 16 tools.
- Production integrity/data checks passed: all prior tool IDs preserved, manual trusted/blocked decisions preserved, 6 programs / 3 standalone Skills / 1 artifact / 9 original extensions unchanged; 47 total instances and 2 new project observations. No new execution tasks. Both host MCP/Skill paths, shortcut and enabled logon task point to the final install.

## Upgrade handling and limitations

The one-time recognition extension conflicted with the new reserved Electron filename during the first upgrade. Removed it through the **old supported desktop extension API**, then restarted beta.5 and rescanned; no production SQL mutation was used. Backups retained before registration and both activation steps. Existing chat processes launched before activation still hold the old MCP executable/server identity and require reconnecting ToolHub or starting a new Agent session; a fresh final-install MCP process was independently verified successfully.

Real runtime verification was Windows only. This first adapter does not ascend to hoisted/workspace packages, execute JS resolution, install dependencies or cover every npm tool. The [project discovery guide](../development/PROJECT_DEPENDENCIES.md) states these limits and the extension points. Core main/GitHub release were not published by this task.

Local evidence is retained under `D:/codex/toolhub-electron-20261017/`: unit/integration/build logs, fixture-check.json, production-final.json, mcp-final.json, final-check.json, package checks and screenshots. Compilation maintenance removed 2.139 GiB of regenerable target data after live-reference verification, keeping runtime/rollback/user storage.

