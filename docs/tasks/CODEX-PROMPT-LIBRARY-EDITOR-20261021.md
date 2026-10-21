# Prompt library editing support — 2026-10-21

Owner request: add/edit/delete prompts, Markdown input, single/all export and portable preset import. Base: 500508ae2c19c654f53cd587b4cb4dbb0572519e. Direct Codex implementation authorized by the owner; no delegated work.

Scope: desktop extension bridge, isolated package storage, SDK runtime/docs and relevant tests; beta.8 version metadata. Independent extension implementation lives in ToolHub-Extensions. No protocol, database, execution, Agent config or inventory contract changes. Keep iframe CSP/sandbox unchanged.

Contract: package state quota grows from 32 KiB to 1 MiB UTF-8 with existing revisions/namespaces unchanged. New files.exportText(name,text) asks the privileged parent to open a native Save dialog; <=2 MiB UTF-8 and .md/.json/.txt plain filenames only, no destination supplied by iframe, current installation/digest checked before and after selection, cancellation returns false. No page disk path is exposed. Host source/token/current digest and single-export guards remain required. Native command absent from remote ACL. User source files are never deleted.

Acceptance: CRUD persists; .md and bundle JSON imports are validated/previewed and applied atomically with explicit conflict policy; single Markdown and whole-library JSON round trip; errors/cancel preserve data/drafts; bounded pagination/responsive dialogs. Build native Windows, test storage/export bounds and real native file transfer, preserve existing inventory/extensions/preferences, update local installation after backup. macOS code path remains unverified. No GitHub push/release authorized in this task.

Deliver: exact revisions, package/checksum catalog updates, bilingual instructions, meaningful parser/merge/UI/native evidence and known platform limits.
