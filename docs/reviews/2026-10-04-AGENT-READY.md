# Agent-ready local Windows delivery — 2026-10-04

## Result and cause

The Codex empty-state bug was caused by transient PATH-only adapter discovery and an empty agents table. Desktop-login PATH omitted the installed Codex binary, and prior detection was not a durable registration. Discovery now checks known per-user install roots and Windows command suffixes, parses only ToolHub host configuration, and persists display inventory/history. UI distinguishes detected software, configured MCP, and historical handshake/successful-call timestamps. Labels never grant authority. The installed production UI now shows Codex and the correct configured CLI path; historical timestamps remain empty until the real host uses the updated server.

The six approved phases in docs/development/AGENT-READY-PLAN.md are implemented. Grouping retains all installation identities. Explicit request preference precedes project then global defaults; all candidates still pass eligibility filters. File/hash checks are distinct from actual authorized execution. Reviewed version probes use the verified desktop and exact approved arguments. Task aliases map Chinese media/document/archive questions to known capabilities, with compact bounded MCP search results. Program proposals preserve purpose/input/output/dependency/example metadata, and require user selection/save. Only opted-in entry descriptions are discoverable; ordinary Agents cannot launch saved program entries. Background tool jobs support bound approval, polling, cancellation and confined output-file metadata. Skill library supports native directory selection/import, instruction copy and dependency resolution, with video and PDF/OCR examples.

## Exact local delivery

Base HEAD: fba33201197c5f892747e06713c750437172932c. Branch: codex/button-audit-20261001. Delivery is the uncommitted working tree, including prior authorized fixes; no new commit or GitHub publication is claimed.

Package: D:/mimoproject/ToolHub/ToolHub/dist/toolhub-agent-ready-20261004-2210
Installed: C:/Users/AA/AppData/Local/ToolHub/versions/agent-ready-20261004-2210
Unsigned Windows release build; portable local installer, not a signed public MSI/NSIS release.

| File | SHA256 |
|---|---|
| toolhub-desktop.exe | a8ef94a93072fd3304b6647ab2fc7d15ce37af7572ad30a3428e4b6433492f63 |
| toolhub.exe | ccf3d4759c8a8aac49745d2ebf8397d0ff24e944f69387bc325f6d5c52fd07a7 |
| toolhubd.exe | 163a9403f520c6dffe5537c50a49e25fe06f99967b8844ba50367e539b25f7dd |

The installer verified all 16 package files; a deliberately modified private copy was rejected before installation. It updated only the existing current-user login task action, desktop shortcut, and ToolHub command/args in Codex TOML. Other parsed Codex configuration is identical to the pre-install snapshot. Installed toolhub-integration SKILL.md was updated to the real stable CLI path and 12 meta-tools; agents/openai.yaml is unchanged. The old project start-ToolHub.cmd now delegates to the installed desktop shortcut, avoiding mixed debug/release GUI versions without changing the saved program entry.

Consistent SQLite backup: D:/codex/toolhub-agent-ready-20261004/registry.before.sqlite. Configuration and Skill backups are local in the same evidence directory, outside the repository; they must not be published.

## Verification

- Release Rust unit tests: 90 passed, 1 ignored finite child fixture (its actual process lifecycle is tested by the parent lifecycle test). Includes discovery/config secrecy, durable history, trust-filtered preference order, task ownership/restart, output confinement, proposal metadata/atomicity and existing Windows startup/controller tests.
- Strict Clippy --all-targets with -D warnings passed for the changed service, desktop, CLI, SDK-facing protocol and domain crates.
- Real release CLI MCP conformance: 6 passed; managed authenticated controller/shared reconnect/frame recovery: 1 passed.
- Frontend TypeScript/Vite release build passed. TypeScript SDK runtime tests: 4 passed; strict NodeNext declaration/type tests passed for synchronous/background execution and new approval/proposal methods.
- skill-creator quick_validate passed for the installed integration Skill and both packaged examples (Python UTF-8 mode).
- Private actual Tauri WebView plus real stdio MCP: initialize 2025-11-25, 12 fixed tools with output schemas, legacy JSON text equals structuredContent.result; Chinese query pagination; installation grouping; global/project/explicit preference; native authorized version check using a real executable path containing spaces, Chinese, brackets and an apostrophe; proposal/check/review/save/share; built-in Skill register/inspect; approval request + desktop confirmation + polling; asynchronous completion with real result.txt; cancellation; no page errors.
- Installed production native UI and real stdio MCP: Codex detected/configured, server self-test passed, 12 tools, grouped 13 available kinds, 38 available / 44 registered / 6 unavailable instances, original 3 program entries accessible through the verified controller, packaged Skills discoverable, startup action matches installed EXE, no page errors.
- Production SQLite comparison preserves all 3 original entry_json values byte-for-byte and all 44 original IDs, definition IDs, paths and statuses. Schema migrated to 5. Existing program processes were not killed.
- Actual login task created the installed GUI window; normal window close produced scheduler result 0, then the task reopened ToolHub for user testing. Enabled choice and login task settings were preserved. A fresh boot/login has not occurred.

Commands used include cargo test --release (agent-bridge/core/MCP/protocol/registry/resolver/daemon/desktop), cargo clippy --all-targets -- -D warnings, cargo test --release --test mcp_conformance --test managed, npm run build, node --test and TypeScript tsc --noEmit --strict --module nodenext. The standard install-local.ps1 -VerifyOnly and -UpdateCodexMcp paths were used.

Evidence directory: D:/codex/toolhub-agent-ready-20261004. Relevant files: native-result.json, production-result.json, production-preserved.json, installed-startup.json, startup-normal-close.json, native-*.png and installed-*.png. Native directory-picker import shares the existing Windows COM picker path; manifest-file registration and packaged imports were exercised. The new folder picker interaction itself was not fully driven through the user-active final window.

## Practical limits

- The current Codex conversation may retain its old MCP child/process list. Direct installed-server verification is complete; reload MCP/start a new host conversation to use all 12 tools and establish real host observation timestamps. Configuration does not prove host loading; see the official Codex MCP documentation at https://developers.openai.com/codex/mcp.
- Managed jobs use compatible ToolHub meta-tools, not experimental MCP Tasks. Maximum deadline is 300 seconds; task summaries persist but raw stdout/stderr are bounded and held only in memory. Restart marks unfinished jobs interrupted. Skill import does not install missing dependencies or confer execution permission.
- File-only checks do not certify arbitrary application readiness. Program launch remains a desktop user action. Scan coverage is bounded and cannot prove AI authorship. PDF/OCR example availability depends on actual eligible local providers.
- Fresh Windows login/reboot and non-Windows UI/runtime checks remain unverified. Formal product review acceptance is separate from this local verification.
