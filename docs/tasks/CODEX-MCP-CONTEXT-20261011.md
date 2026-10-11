# MCP context reduction — bounded owner-requested work

Owner summary: 缩小连接定义与默认搜索回复，把接入 Skill 的配置和高级说明改为按需读取，保留权限与现有客户端兼容性。

Base: `da0448e769dfa75f82b2f6e150339043e3a7ed93`; branch: `codex/mcp-context-budget`. This is a bounded implementation requested by the owner, not a fifth formal product-review round.

Acceptance: retain sixteen fixed MCP tools and deterministic definitions; default MCP searches return five summaries with explicit limit/offset/detail=full escape hatches; inspect_tool/inspect_skill and all execution/approval/cleanup checks stay unchanged. Ordinary THP/CLI/desktop queries retain their existing defaults and response shapes. Text and structured results remain available for client compatibility. Split conditional Skill material into packaged references and generate correct portable/macOS resource paths. Measure real stdio output characters/UTF-8 bytes before/after; do not equate bytes with billed tokens. Verify pagination/filtering, strict schemas, error semantics, real daemon/CLI results and Skill resource availability. Local package only; no remote publish or release overwrite.

Allowed: crates/mcp, apps/desktop/src/integration.rs, apps/cli/tests/mcp_conformance.rs, skills/toolhub-integration, protocol/developer docs, README language variants, beta.3 notes and this task evidence. Exclusions: authorization relaxations, lossy execution-output truncation, hidden stale execution caches, new tool-per-plugin definitions, host-specific context/caching guarantees, online changes and unrelated UI work.

## Delivered evidence and limits

- 29 focused Rust unit/process tests passed: six real MCP conformance cases, thirteen desktop tests and ten MCP cases. Build passed; 33 local package files verified. The line-ending regression found during validation was repaired before delivery.
- Real packaged stdio initialize/list/calls verified before and after, against an isolated snapshot and again the activated installation. Fixed sixteen tools; summary/default/explicit full paths succeeded. Explicit complete FFmpeg records remain equal; default FFmpeg text fell 8902→1251 characters (86%). Installed Codex Skill fell 8652→1728 characters (80%). Model tokenizer/billing measurements are not available.
- Activated local build context-20261011-1 (beta.3), normal desktop launched without debug/console helper. Codex and MiMo MCP paths and runtime Skills updated. Existing MiMo current.json guidance retained; other host configuration and implicit-invocation metadata unchanged. Five reference resources exist in the installed package; both host Skills validate.
- Original records retained: 44 tool instances, 6 programs, 2 ordinary Skills, 1 artifact package; no management roots or extensions silently enabled. Backups, captures, metrics and logs: D:\codex\toolhub-token-20261011.
- macOS resource layout is covered by a Windows unit fixture, not native macOS verification. Existing conversations may retain old loaded Skill/tool descriptions until MCP reload or a new conversation. No GitHub release/push or byte-to-billed-token guarantee.
