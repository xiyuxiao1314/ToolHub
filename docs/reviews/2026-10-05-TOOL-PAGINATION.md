# Tool list pagination — local delivery 2026-10-05

Status: implemented, locally verified and installed. Owner requested pagination for the desktop Tools page. This localized correction is outside the historical formal batch review rounds.

## Revision and scope

- Base HEAD: `fba33201197c5f892747e06713c750437172932c`.
- Branch: `codex/button-audit-20261001`; working tree contains prior owner-authorized changes. No new commit, push, release publication or merge was performed.
- New `apps/desktop/ui/src/ToolsList.tsx`; bounded changes in `App.tsx`, `GroupedTools.tsx` and `styles.css`.
- No backend, domain, database schema, RPC or MCP contract changes.

## Behavior

- Default six items per page; six, twelve and twenty-four are supported, with the preference retained after restart.
- Grouped mode paginates whole definitions. All installation instances of one tool stay together, with a bounded inner scroll when expanded.
- Flat mode paginates installation instances. Counts and category totals remain based on the entire filtered result.
- Top and bottom controls support previous/next and page selection; boundaries are disabled. Search, category, availability preference and view changes reset to page one. A shrinking result clamps the page into range. Empty results have no pagination controls.
- Page changes retain the right-side selected detail; existing filter behavior may choose a new detail if the selected instance is excluded.

## Verification and evidence

- UI TypeScript/Vite build passed: `npm run build --prefix apps/desktop/ui`.
- MSVC release desktop build passed: `cargo build --release -p toolhub-desktop --locked --offline -j 4`.
- Scoped `git diff --check` passed.
- Native WebView tests passed with an isolated database, after restart, and in the installed app. Thirteen groups produce 6/6/1 pages; thirty-eight instances produce 6/6/6/6/6/6/2 pages. Twelve and twenty-four item sizes, last-page boundaries, both controls, searching, category reset, empty results, retained detail and saved page size were verified.
- Search for FFmpeg correctly matches fifteen instances across FFmpeg and FFprobe under the existing search semantics. The installed test initially exceeded its twenty-second wait; later inspection showed the correct results and a full repeat passed. The delay did not reproduce on the repeat; its cause is unconfirmed.
- Production data compared exactly against the pre-install snapshot: four programs, forty-four tool instances and two Skills preserved. MiMo and other host configs unchanged. Codex MCP command updated only to the new versioned CLI; remaining config and Skill agent YAML preserved.
- Evidence folder: `D:\codex\toolhub-pagination-20261005` (`native-result.json`, `restart-result.json`, `production-result.json`, screenshots, preservation and launch reports).

## Installed delivery

- Build: `tool-pagination-20261005-185330`; twenty-five package files verified.
- Package: `D:\mimoproject\ToolHub\ToolHub\dist\toolhub-tool-pagination-20261005-185330`.
- Installed desktop: `C:\Users\AA\AppData\Local\ToolHub\versions\tool-pagination-20261005-185330\toolhub-desktop.exe`.
- Desktop SHA256: `96EBE5BF7CD65620A8F0061947938587DD58A3E070DB84188195D7E695446A3E`.
- Source SHA256: ToolsList `F8FA03D90F8C79B3A373012000784042A46E8A0413B710B4BD67D69127624F2C`; GroupedTools `472392F1572254D26D33A8F96FE8AFA27739F851CC348F93D4C7F88C59766229`; App `AAB77BBB95C1E119659BB26199DA9B8E2B25968A61821A63075949138D4FBF7C`; styles `5CC58D7D43202C169C839466355F7E4AFDE07D4B37C2ACA34FB0BC7618E15640`.
- Verification CDP window was closed. Normal launch through the existing enabled login task succeeded, with a ToolHub window and its paired daemon (desktop PID 118676, daemon PID 109816 at verification time). Previous installation retained; unrelated Agent/MCP processes untouched.
- Checks were performed on this Windows machine. Actual reboot, macOS and Linux are unverified and are not pagination acceptance requirements.
