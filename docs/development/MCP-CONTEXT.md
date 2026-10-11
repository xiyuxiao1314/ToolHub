# Agent context footprint

The development beta.3 gateway retains sixteen fixed meta-tools; installed tools, Skills and declarative extensions do not create per-item MCP definitions. Definitions are deterministic. Shorter initialization instructions and descriptors reduce connection metadata while preserving policy guidance.

| MCP search | Default | More detail |
| --- | --- | --- |
| search_tools | Five summaries | limit/offset, detail=full or inspect_tool(id) |
| search_skills | Five reusable Skill summaries | limit/offset, detail=full or inspect_skill(id) |
| search_programs | Five user-shared app summaries | limit/offset, detail=full for existing declarations |

Use narrow task/name queries. A full page may have more matches; increase offset by limit to continue. A short/empty page ends traversal. Results remain arrays. Limit is 1–1000 and offset 0–100000. Summary Skill descriptions and program purposes are capped at 256 Unicode characters plus an ellipsis; full details remain available. Query filtering and host-portability checks happen before paging. Native THP/CLI/desktop searches retain their previous defaults.

Inspect only the selected instance or workflow. Reuse inspected IDs within the current task when appropriate, and re-check if files/paths/environment/permission state change or an actual invocation fails. There is no server-side stale discovery/approval cache: normal live inspection, executable pinning, current policy and one-use approval checks remain. Execution output and error/status fields are not silently shortened. Backward-compatible text and structured results are both retained; client display/context behavior is outside the server's control.

The packaged toolhub-integration Skill has a concise daily entrypoint. Setup, exact approvals/background execution, program proposals, Skill registration and artifact management live in separate references read only for the relevant task. Desktop-generated instructions point to the installed resources (macOS Contents/Resources, portable package directory otherwise); copy/import of the main Skill keeps those local links. Configuration templates remain in the one-time bootstrap prompt, not the daily Skill. Unrelated host config and implicit-invocation preferences must be preserved.

Measure initialized instructions, tools/list, model-facing content text and full wire JSON separately. Use the same registry/query before and after. Count Unicode characters and UTF-8 bytes when a verified local tokenizer is unavailable. These numbers describe payload footprint, not exact tokens, token price, model accuracy or a host's prompt-cache savings. Clients may include text, structured content, both or neither in model context. Actual billed tokens require host/model-side measurement.

Boundary: no compressed/encoded text that forces agents to decode, no removal of authorization checks, no global change to desktop search, no new task polling loop and no arbitrary all-powerful dispatcher. The next useful measurement is real task usage: number of searches/inspections/polls and long tool outputs. Host-supported lazy tool discovery/cache may help but needs per-client validation.

## Measured local Windows result

Same registry/query on the packaged beta.3 CLI, 2026-10-11. The FFmpeg full-record result is byte-for-data equivalent when explicitly requested; the default page changes from 15 full rows to 5 summaries. Codex daily Skill measurement includes the actual local path/reference links.

| Measured text | Before characters | After characters | Reduction |
| --- | ---: | ---: | ---: |
| FFmpeg default search | 8902 | 1251 | 85.9% |
| Python default search | 1190 | 185 | 84.5% |
| Skill default search | 1429 | 395 | 72.4% |
| MCP initialization instructions | 521 | 206 | 60.5% |
| MCP tools/list | 8935 | 8694 | 2.7% |
| Installed Codex daily Skill | 8652 | 1728 | 80.0% |

These reductions describe these payloads; do not report them as an overall token/cost reduction. Tool list reductions are modest because full strict schemas and sixteen tools remain. Client-side inclusion and tokenizer behavior need separate measurement.
