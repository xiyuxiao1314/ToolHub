# Installed extensions layout

Base: `3a46da7e8767f7490d386bffacc1e0a7507938b7`.

Goal: make Installed a bounded workspace with direct type/platform filters, a paginated compact list and tabbed details. On narrow windows, list and detail replace one another. JSON editing uses a modal instead of growing the page.

Scope: desktop `Extensions.tsx`, workbench CSS, UI verification and this delivery document. Catalog and Appearance retain discovery/preferences flows. No daemon/domain/schema/permission contract changes; existing preview/install/digest/rollback/remove APIs remain authoritative. No extension packages, user library or trust updates. No GitHub publication in this batch.

Acceptance: no outer scrolling in normal desktop windows, no native selects on Installed, pagination and selection remain consistent under resize/filter/remove/import, complete management controls and keyboard dismissal remain usable. Verify isolated UI and native WebView, then build and activate locally while preserving production extension states and library data. Windows verification only; macOS remains unverified.

Status: implemented and verified locally on branch `codex/extensions-installed-layout`; pending owner visual acceptance. No GitHub push or release.

Owner summary: 已安装扩展改为窗口内分页管理，类型与平台使用按钮筛选，详情分标签，并修复安装后选中项丢失与弹窗焦点问题。

## Delivery

- Removed the Installed marketing hero and long introductory paragraph; compact installed/enabled totals remain in the header.
- Type buttons expose result counts; platform buttons replace the second native select. Search/filter changes reset to the first matching page.
- List capacity follows available height (1–8 entries); pages keep the selected package visible across resize. Deletion clamps the page and chooses an available item.
- Details use Overview / Contributions and source / Recognition / Skill tabs. The main page and pager stay bounded; only long detail content scrolls. Narrow content areas switch between list and detail rather than stacking them.
- JSON editing uses a bounded modal with Escape, focus restoration and keyboard trapping. Tabs support arrows and Home/End.
- New imports clear incompatible filters. A newly installed imported package remains selected until the refreshed installed list contains it; the previous implementation dropped the preview before that refresh.
- Native preview, digest checks, enablement, rollback and removal APIs are unchanged. No trust, registry schema, extension package, CLI, daemon or permission changes.

Changed source: `apps/desktop/ui/src/Extensions.tsx`, `apps/desktop/ui/src/workbench/workbench.css`; UI evidence scripts plus this brief. Existing navigation verification now permits a paginated first page while asserting the full result count.

## Verification

Passed TypeScript (`node node_modules/typescript/bin/tsc`), Vite production build and `cargo build --locked --offline -p toolhub-desktop` on Windows.

`scripts/ui-evidence/installed-browser-bridge.mjs` runs the built UI with a relay to a real isolated native controller/daemon. It validates 23 installed packages, all-page coverage, counts/search/empty/filter reset, enable/disable, invalid and valid JSON edits, explicit update, rollback, import under a different filter, export payload, uninstall cancellation/confirmation, last-page clamping, five viewport sizes (1180×780, 1440×900, 960×640, 800×600, 600×720), narrow list/detail switching and Catalog/Appearance regressions. Official package states/digests remain unchanged. File picker/export responses are simulated only at their frontend boundary; native file dialogs themselves are not revalidated. Tauri's immutable native invoke is retained.

`scripts/ui-evidence/verify-installed-native.mjs` separately validates the actual Tauri WebView layout, pagination, filter controls and keyboard tab navigation without registry changes.

Both scripts require an explicit isolated registry, CDP endpoint, output path and `TOOLHUB_INSTALLED_FIXTURE=yes`; they reject the default production registry. The browser suite requires a fresh seven-module fixture. Start the debug desktop with `TOOLHUB_REGISTRY`, an isolated `WEBVIEW2_USER_DATA_FOLDER` and `WEBVIEW2_ADDITIONAL_BROWSER_ARGUMENTS=--remote-debugging-port=19334`. Set:

```powershell
$env:TOOLHUB_INSTALLED_NATIVE_CDP='http://127.0.0.1:19334'
$env:TOOLHUB_INSTALLED_REGISTRY='<isolated registry absolute path>'
$env:TOOLHUB_INSTALLED_FIXTURE='yes'
$env:TOOLHUB_INSTALLED_OUTPUT='<evidence absolute directory>'
node scripts/ui-evidence/installed-browser-bridge.mjs
node scripts/ui-evidence/verify-installed-native.mjs
```

The browser relay binds loopback ports 19335/19336, restricts request origins, blocks unhandled native dialogs/external actions and closes after the run. Existing Playwright dependencies and Microsoft Edge are required. Raw results and a production screenshot are under ignored `output/playwright/installed-manager/`.

## Local activation and limits

Built and checksum-verified `dist/toolhub-extensions-installed-20261021-1` (63 files), then activated with the supported installer and `-KeepOldVersions`. Current build: `extensions-installed-20261021-1`, host `0.3.0-beta.7`. The previous build remains available. The formal app is open on Installed.

Backed up production SQLite/display storage, checked no active jobs, then compared all installed package versions/digests/enabled states, appearance preferences, extension display-state fingerprints and library records after activation. Preserved 47 tool instances, 7 programs, 3 Skills, 1 artifact package and 0 artifact roots; startup may refresh tool timestamps only. No MCP configuration edit.

macOS native verification is unavailable on this Windows machine. Long source/Skill text still uses the bounded detail area; extreme windows shorter than the tested 600 px are outside this acceptance matrix. GitHub publication remains a separate owner action after visual acceptance. Exact base/head and installed-binary SHA-256 are recorded in the local delivery manifest.

