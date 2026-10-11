# Declarative extensions — owner-approved implementation

Owner summary: 为市场增加可安装、停用、更新、回滚和卸载的声明式扩展，以 Blender 识别规则与通用 Skill 验证无需改动应用源码即可扩展能力。

Base: `0183648fce3236152207b7c8fecb1c62d730aa04`; branch: `codex/declarative-extensions`.
Codex owns this bounded local implementation following the owner's acceptance of the first-phase proposal. This is not a new formal product-review round or permission to overwrite the published beta.2 release.

## Acceptance

- Strict, bounded `toolhub.extension/v1` combines existing recognition resources and portable Skill bundles; unknown fields, executable payloads, namespace conflicts and incompatible installations fail before mutation.
- The authenticated daemon stores packages transactionally in the shared SQLite registry. Restart retains enablement and version history. Installation, update, rollback, disable and uninstall require the verified desktop controller.
- Enabled rules join the native read-only scanner; extension Skills are projected into existing CLI/MCP/desktop Skill queries. No new per-tool MCP definitions or automatic dependency installation.
- Disable/update/uninstall invalidate extension-owned instances and stale execution approvals. Uninstall retains real files, ordinary Skills, tool records, programs and artifact packages.
- Market has an extension-management view with explicit preview, dependencies, source, digest, compatibility, enablement, uninstall confirmation, import/export and a Blender example.
- Schema, TypeScript SDK, developer docs and package asset lists change with the implementation. Unit/integration and real Windows WebView flows provide evidence. macOS native verification remains explicitly unverified until run on that platform.

## Ownership and exclusions

Allowed: crates/{skills,recognizer,registry,protocol}, apps/{daemon,desktop}, schemas, packages/sdk-typescript, extensions, scripts/package*, README*, PROTOCOL.md and docs. Existing scanner/resolver/policy/IPC contracts are read-only dependencies except narrow integration fixes justified by tests.
Use existing ResourcePackage, ResourceTool, SkillBundle and SkillManifest models. Any contract adjustment propagates to schema, SDK, docs and tests in the same branch. No executable plugins, runtime downloads, host-exclusive Skills, online trust claims, arbitrary hook loading, broad refactors or GitHub release mutation.

## Evidence

Run relevant Rust suites, UI typecheck/build, SDK tests and exact native Windows installation/disable/restart/update/rollback/uninstall flows against an isolated registry. Preserve production user records when delivering a local build. Capture market screenshots and record final evidence/limitations here.

Decision: the integrated local development version is 0.3.0-beta.3 (unreleased). Workspace/UI/SDK version fields, lockfiles and the canonical ExtensionDisabled instance status are allowed narrow shared-contract changes. The example requires this feature version; no beta.2 release asset is overwritten.

## Implemented and locally verified

- Windows local development package: `extensions-20261011-1`, version `0.3.0-beta.3`; UI build and native CLI/daemon/desktop release build passed. Installer checksum validation passed (28 package files).
- Workspace regression run excluding desktop: 195 passed, one pre-existing ignored case, zero failures. Desktop unit tests: 12 passed, zero failures (207 Rust tests passed in total). Unix scan fixtures explicitly set executable metadata without executing their contents. SDK transport tests: 4 passed; SDK strict TypeScript contract and UI typecheck/build passed.
- Twelve real Windows WebView/MCP check groups passed with zero renderer errors: market installation/enablement; unchanged 16 meta-tools; independent JSON created after packaging; native scan; unknown-trust retention; ordinary CLI install denial; policy-controlled execution of a copied Node fixture; stale approval invalidation; changed-rule rescan; update/rollback; uninstall preserving files and ordinary tools; original catalog and nine navigation pages.
- A separate real daemon/desktop restart retained the installed snapshot, enabled state and extension Skill queries. The production registry has no extension auto-installed. User records retained: 44 tool instances, 6 program entries, 2 ordinary Skills, 1 artifact package; no cleanup roots enabled.
- Existing Codex/MiMo MCP paths and the local connection Skill paths now point to the installed beta.3. Both Skills validate; unrelated host configuration is preserved. Normal desktop started without a debug port or console helper.
- Evidence packet: `D:\codex\toolhub-extensions-20261011` (workspace-tests.log, desktop-tests.log, ui-build.log, build-release.log, native-check.json/log, restart-check.json, install-check.json, backups and screenshots). Screenshot in developer documentation uses isolated fixture data.

## Verification limits

macOS native build/UI verification for this new feature has not run. File-picker code is shared with the existing native JSON picker; package reading/validation and selected-file backend flows were checked, but Windows native chooser accept/save interaction remains a user smoke check. Source is locally committed on the feature branch; beta.3 is not published to GitHub and the existing beta.2 release is unchanged. Executable plugins and online catalogs remain excluded.
