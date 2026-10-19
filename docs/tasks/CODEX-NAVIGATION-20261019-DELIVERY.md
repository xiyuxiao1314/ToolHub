# Hierarchical navigation — local delivery

Base: 2c06b80786141b98e11378d5e3f40ffc05a1303e. Branch: codex/hierarchical-navigation. Status: implemented and verified locally on Windows; no formal product-review round, main integration or GitHub publication in this batch.

## Implemented

- Four kernel-owned primary destinations, contextual secondary links, package-collapsed application pages and route recovery. Existing module/CLI/view IDs and native SDK authorization remain intact.
- Searchable application directory, per-page pins (maximum eight), per-group history, persistent collapse and invalid-preference recovery. Pins survive disable/re-enable; uninstall removes stale pins. Presentation-only localStorage, with no schema/MCP/database/permission changes.
- Distinct installed, catalog, appearance and Agent-integration views. Catalog remains builtin/local with an explicit community repository link.
- Unified line icons, compact inventory status, breadcrumb, keyboard skip link and responsive flyout. Existing left/right/top layouts, theme variables, density and Ctrl+K search preserved.

## Verification

- Desktop UI: npm run build (strict TypeScript and Vite production build), exit 0.
- Native desktop: cargo build --release --locked --offline -p toolhub-desktop, exit 0. CLI/daemon reused from the verified beta.5 Windows release; backend source unchanged.
- scripts/ui-evidence/verify-navigation.mjs: 15 native Tauri/WebView2 groups passed; 32 installed packages / 178 application pages, only four primary entries, real sandbox SDK navigation, pin cap/persistence, invalid routes, six appearance combinations, 640px flyouts, 1180px default window, all optional packages disabled, malformed stored JSON, no JavaScript exceptions. Two harness issues were corrected before the final pass: initial WebView readiness and waiting for the asynchronous extension reload before testing the all-disabled state. No product check was bypassed.
- Production install: copied to a fresh navigation-20261019-1 version, checksummed 54 package files, preserved the old version and registry/config backups. Exact fingerprints of all six persistent user-content tables matched before first start. Counts: 47 tool instances, 7 programs, 3 standalone Skills, 1 artifact, 0 artifact roots, 9 extensions. MCP Skill queries additionally include the FFmpeg extension workflow.
- Production native smoke: verified controller-only program.list, existing counts, keyboard tool/program search, four primary groups and no error banner; startup command points at the new desktop. Updated only ToolHub paths in existing Codex/MiMo integration configuration and Skill references, preserving unrelated settings.
- Fixture guard: a negative run against the actual production CDP endpoint rejected its registry identity before any mutation. The native acceptance runner requires an explicit isolated registry path. Reclaimed 1.364 GiB of reproducible Cargo outputs through the existing maintenance verifier after packaging; installed binaries and evidence were retained.

Native acceptance artifacts (results, geometry, screenshots), installation backup/plan and exact local Git head manifest are under D:\codex\toolhub-navigation-20261019 and output/playwright/navigation. The documentation screenshot uses isolated sample packages.

## Limits

macOS interactive layout checks were not run. Navigation preferences are local WebView state and are not included in configuration export; clearing that profile resets them. No online marketplace installer, arbitrary extension-owned primary navigation or new execution rights were added. Public beta.5 assets are unchanged.
