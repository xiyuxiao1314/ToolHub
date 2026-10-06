# Multi-host Agent compatibility — 2026-10-05

Owner requested compatibility beyond Codex and MiMo. Base fba33201197c5f892747e06713c750437172932c; branch codex/button-audit-20261001. Preserve existing uncommitted work, programs, installations, reusable Skills and other hosts' configurations.

| Phase | Work | Acceptance | State |
|---|---|---|---|
| 1 | Audit stdio protocol, config readers and observed client names | Identify host-specific limits without treating config presence as a connection | Completed |
| 2 | Shared host catalog and copyable templates | Ten known host profiles plus generic stdio; UTF-8/JSONC and exact command/args; default configuration readers redact unrelated data | Implemented; fixture/native checks passed |
| 3 | Persistent observation for arbitrary MCP clients | Known aliases share history; unknown clients get bounded stable IDs; restart preserves observations; no authority granted | Implemented; unit and real stdio checks passed |
| 4 | Native integration UI and portable task guidance | Selector/copy shows selected format; unsupported Skills use task text; configuration, detection and historical observations distinct | Implemented; eleven templates verified in native Windows UI |
| 5 | Version negotiation and local delivery | Existing supported versions preserved; unknown request offers latest supported version; user data/configs preserved; installed window starts | Verified on Windows |

No Agent installation, global permission changes, automatic third-party MCP writes, exclusive Skill import, HTTP gateway, GitHub publication or formal B01/B02 review in this scope. Local implementation and fixture verification do not establish every real host/version/platform combination. See [evidence](../reviews/2026-10-05-MULTI-HOST.md).
