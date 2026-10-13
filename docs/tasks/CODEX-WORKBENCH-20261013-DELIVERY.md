# Extension workbench local delivery

Owner-approved implementation, base `b30648f`, branch `codex/extension-workbench`.

Implemented: default optional modules for tools/programs/artifacts/environments/Skills/market/tasks; dynamic navigation and lazy pages; permanent Agent/settings/extensions kernel entries; compatible v1 plus v2 views/themes/layouts; controller-only lifecycle, immutable updates and rollback; bounded dependency and permission validation; data-preserving uninstall and paused artifact cleanup. Frontend pages use a session-only loopback document with an opaque sandbox and a least-authority SDK bridge.

Validation: `cargo test --workspace --offline` (222 passed, 1 existing ignored); SDK `node --test test/client.test.js` (4 passed) and strict NodeNext types; UI `tsc` and Vite production build. Real packaged Windows Tauri/CDP: 11 behavior groups plus 5 app/daemon restart groups and a focused real offline/retry recovery test. Actual MCP retains 16 meta-tools and defaults to five search summaries. No fixture launch script was executed; no cleanup was scheduled.

Installed locally: `workbench-20261013-6` under the user's ToolHub versions directory, launched normally after ending diagnostic instances. Existing 44 tool instances, 6 programs, 2 Skills and 1 artifact package preserved; 7 official modules seeded once. Codex/MiMo command and reference paths updated while preserving other settings and local notes. Existing login task remains enabled and targets the current desktop executable. Backups and detailed evidence: `D:\codex\toolhub-workbench-20261013`.

Security correction: automatic review rejected the early proposal to allow blob scripts in the privileged main document. That policy was removed before implementation. The final document service uses a separate remote origin with no Tauri native capability; every resource is checked against the installed package digest. Parent DOM, network, management bridge and direct native settings-write attempts were blocked in the real WebView.

Limits: no GitHub publication or main merge in this delivery; public release remains beta.2. macOS workbench runtime is unverified. Third-party frontend pages, themes, recognition and Skills are independently editable; native first-party service code still compiles with the host. Arbitrary native plugins, online publishers and package downloads are outside this implementation. Existing Agent MCP sessions need one reconnect after their configured command changes. Downgrading to a v1-only daemon requires the pre-upgrade database backup.
