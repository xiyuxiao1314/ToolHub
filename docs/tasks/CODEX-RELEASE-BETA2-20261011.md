# ToolHub beta.2 publication

Date: 2026-10-11. Owner summary: 上传已验收的新功能源码，更新 Windows 与两种 Mac 架构安装包、中英文说明和校验文件。

Base: `5da1cc90be579239d093b706deaf64aa04bba738`; remote main before this delivery: `21eda5a6bf163d13bb310a0c638836ad63c6b7d0`. Branch: `codex/release-v0.3.0-beta.2`. Owner authorization covers GitHub source synchronization and updated release packages. Repository/account must remain `xiyuxiao1314/ToolHub`; preserve public visibility and historical releases.

Goal: publish the automatic-approval and task-artifact feature commits as v0.3.0-beta.2, alongside verified Windows x64, macOS ARM64 and macOS x64 desktop/CLI assets. Acceptance requires native CI checks on all three platforms, package architecture/version/resource/license checks, matched GitHub asset digests, nine total assets including SHA256SUMS, bilingual README/release notes, and an accessible public release.

Existing interfaces: local package/install scripts, pinned GitHub native release workflow, version fields in Rust/Tauri/npm, bounded 16-tool MCP and user-registry migration 6. Allowed changes: release/version metadata, packaging resources, release cache fallback, bilingual documentation/screenshots and a portable test path correction. Implementation and shared contracts remain from the feature commits. No unrelated dependency upgrade, history rewrite, visibility change, user-data deletion, npm publication or workbench archival is included.

One developer handles preparation, validation and publication; no subagents were used. This is routine release verification and does not add a formal product-review round. Native file cleanup checks use owned fixtures. Private credentials, database backups and installer configuration backups remain outside source control and package payloads.

The portable traversal test now uses a native absolute path rather than asserting that a Windows spelling is absolute on Unix. The assertion still rejects absolute paths; no production path validation was relaxed. Packages include both README languages and the artifact lifecycle guide. The Mac Apple Events usage description now covers explicitly approved Finder Trash operations as well as Terminal opening. Cache fallback reuses prior platform dependency caches; Cargo continues validating the lockfile and fingerprints.

Verification: frontend TypeScript/Vite build, four SDK client tests, source and unpublished-history disclosure checks, Skill format validation, Windows release workspace checks/build, and GitHub native release matrix. Release package verification checks all binary architectures, version manifests, Mach-O signing markers/executable bits, Apple DMG trailers, Windows inner checksums, resources and dependency licenses. Final native CI URLs/source revision and asset digests are recorded in the published release.

Limitations: Mac CI proves native compilation and automated checks, not physical Mac GUI/Finder Trash acceptance. Mac controlled tool execution/approval and login startup remain unavailable. Windows packages are unsigned; Mac signatures are ad-hoc and not notarized. Restore remains a system Recycle Bin/Trash operation. No automatic updater is added. Existing historical review ledgers are not rewritten.

The first ARM64 native run exposed a test fixture race: concurrent RPC clients could auto-start a daemon before the explicitly pinned fixture daemon bound its listener, selecting the default desktop controller image instead. The fixture now waits for its owned listener before starting concurrent requests and reports controller errors directly. Authentication, concurrency, reconnect, malformed-frame recovery and ordinary-CLI rejection assertions remain in place; production IPC and authorization are unchanged.
