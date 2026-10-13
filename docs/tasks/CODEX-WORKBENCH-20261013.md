# Owner-approved extension-first workbench

Base: b30648f; branch: codex/extension-workbench. The owner explicitly requests direct implementation of the extension-first architecture on 2026-10-13. This is a new implementation delivery, not another historical B01/B02 formal review. Historical Workbench paths/profiles do not block this owner-authorized local task; no messages or remote publication are requested.

Goal: ship tools, programs, artifacts, environments, Skills, capability market and tasks as first-party feature extensions; retain identity/MCP, policy, data, extension management and recovery in the kernel. Third-party packages can contribute isolated interactive pages and themes/layouts without a host rebuild. Preserve v1 packages and user records.

Acceptance: default feature packages are installed exactly once; install/disable/remove/reinstall changes navigation and service availability; disabled artifact cleanup pauses; records and files survive; restart retains state. UI scripts use a bounded permission-checked bridge without desktop authority. A locally edited package can be previewed, updated, rolled back and exported. Theme/layout changes reset through a permanent kernel entry. Legacy MCP count and search budget remain unchanged.

Scope: daemon service extensions and operation gates, shared package/protocol/schema/SDK contracts, desktop shell and official page modules, sample packages, packaging assets, documentation and native Windows verification. Direct source changes in these directories are authorized; propagate contracts atomically.

Exclude: GitHub push/release, arbitrary native libraries/process hooks, online publisher infrastructure, remote dependency installation, changes to user authorization rules. First-party native services remain compiled Rust implementations behind lifecycle gates; third-party frontend code runs in an opaque sandboxed frame. macOS runtime checks require macOS and must remain unverified here.

Evidence: strict package and RPC validation, lifecycle/dependency/authority/timer tests, workspace regressions, SDK/UI checks, real packaged Tauri WebView integration and restart checks with an isolated registry; production install with backup and data preservation.
