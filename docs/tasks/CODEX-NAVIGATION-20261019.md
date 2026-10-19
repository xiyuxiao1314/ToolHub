# Hierarchical navigation — owner-approved local implementation

Base: 2c06b80786141b98e11378d5e3f40ffc05a1303e. Branch: codex/hierarchical-navigation. Codex owns this bounded implementation; no delegation or remote release is requested.

Goal: keep four kernel-owned primary destinations (workspace, agents, extensions, settings), move optional modules and extension contributions into contextual secondary navigation, and improve density without weakening extension isolation.

Acceptance: all enabled existing routes remain reachable; third-party CLI/views never add primary destinations; theme/recognition-only packages create no navigation; searchable extension application directory and user-controlled workspace pins persist locally; navigation remembers destinations and can collapse; removed/disabled pages recover to extension management. Installed packages, catalog and appearance have distinct views. Left/right/top layouts, narrow windows, keyboard focus, accessible names and empty/all-disabled states remain usable.

Allowed changes: desktop UI shell/navigation/extension management, focused UI verification scripts and navigation documentation. Native services, SQLite, package schemas, SDK privileges, execution policy and MCP contracts are read-only. Navigation preferences are presentation-only localStorage, matching existing desktop UI preferences. No new manifest fields or permissions.

Documentation packaging follows the new screenshot link by copying that one image in the existing local/native package asset lists; no installer or archive-format change.

Verification: TypeScript production build; behavioral browser checks with explicit fixture/shim labels, many extension contributions, persistence and removal; real Windows desktop smoke if local build is available. Do not claim macOS interactive verification. Build output must not overwrite a running installation. Deliver reviewable source, screenshots and an exact validation record; no push/release/main merge in this batch.
