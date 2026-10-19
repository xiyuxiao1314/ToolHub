# Design Studio 0.1 — local delivery

Base: 9aa64cf, branch codex/design-studio. Implemented and Windows-verified locally. No formal product-review round, main merge, GitHub push or release publication was requested. Core development version is beta.6 to identify the optional storage SDK; public beta.5 remains unchanged.

## Delivered

Standalone community.design-studio v2 package, 0.1.0, 75,532 bytes; 6 structures, 4 navigation choices, 4 visual languages, 6 effects, 3 performance modes, 5 reorderable/optional modules, 8 builtin presets and up to 24 saved user presets. Config/HTML/Agent instructions share one strict configuration model and renderer; pasted JSON is never evaluated. HTML is a browser interaction reference, not generated native Android/desktop business code. Unsupported combinations are constrained with an explanation.

The host adds only optional, package-scoped local presentation storage: 32 KiB UTF-8 per package, current enabled digest checks and revision-based writes. Parent WebView localStorage is separate from SQLite/config exports; no arbitrary namespace, disk, network, backend settings or execution API. CSP/native ACL unchanged. Editable extension sources and reproducible generated JSON are retained; the independent extension repo has matching source/package/catalog/checksums on its local codex/design-studio branch.

## Verification

- Strict UI typecheck/production build and paired Windows release desktop/CLI/daemon build: passed.
- Cargo targeted native tests: 16 desktop + 10 Skills passed, including bounded package/min-host validation, legacy snapshots, CSP resources and native asset contracts.
- Model checks: preset validation, bounds/unknown fields, imported-content escaping, all structure/navigation/appearance combinations and conditional performance/drag constraints passed.
- Presentation storage unit and native checks passed: two-package isolation, CAS conflicts, UTF-8 quota, corrupt-data preservation, disabled/removed package revocation and re-enable recovery.
- 10 native design workflow groups passed: actual sandbox/CSP loading, all eight presets, configuration/parameters/modules, real 10,000-item virtualization and expansion retention, canvas camera/culling, cross-column kanban drag, search/navigation/instructions, strict import and executable standalone HTML with escaped user text, saved preset reload/deletion, responsive geometry and no runtime/CSP errors. The drag harness targets the destination column heading to avoid native automatic scrolling changing its hit target.
- A focused native check verified that the search bar remains pinned during actual preview scrolling; overflow:clip prevents the renderer root becoming an unintended sticky scroll container.
- Real desktop + daemon restart recovered a saved user preset. Native clipboard copy returned success; copy fallback remains explicit if refused.
- Local package verified all 63 files. Fresh design-studio-20261019-1 installation preserves backup/current predecessor, exact pre-start content fingerprints and unrelated Codex/MiMo config; ToolHub paths/startup shortcut point at the new paired version. Production has the original 47 instances, 7 programs, 3 standalone Skills, 1 artifact and original 9 extension snapshots, plus the selected Design Studio package. Electron 37.10.3 remains queryable. Compiled catalog/source package digests match; production presets start clean, without QA entries.
- Independent repository: 5 manifests/catalog entries and hashes validated; raw-source rebuild produces identical package bytes.

Evidence: D:\codex\toolhub-design-studio-20261019 (installation backups, storage/restart/sticky checks, production smoke and final exact Git delivery manifest); output/playwright/design-studio (native workflow results, screenshots, escaped standalone HTML). No source tests execute discovered local tools.

## Limits

macOS interactive verification unavailable. Saved presets are local WebView data: clearing its profile resets them, so copy important JSON to back it up. Canvas is a finite sample with camera/culling, not a document editor/collaboration engine; drag changes only demonstration item order. Virtualization is not network pagination. No online asset download, framework installation or Agent execution capability is added. Both repository branches are local, not published.
