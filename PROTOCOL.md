# Protocol baseline

## Intended protocol

ToolHub Protocol (THP) uses JSON-RPC. The design proposes stdio, Windows named pipes, and macOS Unix domain sockets. Localhost HTTP or WebSocket support is future work, not part of this bootstrap.

All clients use reviewed domain contracts and one registry. Versioned ToolHub Manifests describe tools, interfaces, capabilities, environments, skills, and agents independently of the desktop product.

## Foundation decisions

Before implementing transports or clients, define and review:

- Protocol and manifest version identifiers, negotiation, compatibility policy, and unknown-field behavior.
- Request IDs, method names, parameters, results, stable error codes, cancellation, and deadlines.
- Streaming or event envelopes, ordering, subscriptions, and backpressure.
- Client identity, local transport access controls, permission prompts, and discovery-session scopes.
- Size limits, input validation, redaction, and transport error mapping.
- Daemon health/startup handling with bounded retries and structured failures.
- Execution authorization tokens or equivalent binding, including expiry and revocation.

## Failure semantics

`registry.search` accepts an optional boolean `include_missing` (default `false`). Desktop inventory views use `true` to retrieve unavailable/imported metadata and apply their display filter locally. Ordinary CLI/agent searches keep the default of available instances. Including missing records does not grant execution trust or make their paths runnable.

An unavailable daemon or tool returns a structured error and explicit fallback information. ToolHub does not select or operate an agent's fallback sandbox itself. A result indicating `fallback_allowed` is information for the caller, not permission to bypass host or user controls.

## MCP boundary

Expose a small set of meta-tools, such as tool search, capability resolution, inspection, environment listing, authorized execution, and skill discovery. Do not publish one MCP definition per installed tool.

Wire schemas, method names, error enumerations, and public schema hosting are not implemented or published yet. Changes to shared contracts must be reviewed before dependent modules adopt them.

## Programs extension

All `program.*` mutations other than metadata-only `program.propose` require an OS-verified controller. `program.search` is an ordinary-client read limited to explicitly shared entries. `program.list {}` returns launch entries with live `available` status. `program.scan_start {roots?:string[]}` starts background read-only discovery (default: mounted local fixed/removable disks); `program.scan_status {offset?:number,limit?:number}` returns progress, limitations and at most 200 candidates; `program.scan_cancel {}` requests cancellation and preserves discovered candidates. `program.select {candidate_ids:string[]}` or `{path:string,kind:"file"|"command"}` issues principal-bound selection IDs without writing entries. Desktop manual selection comes only from the native picker. `program.save {items:ProgramDraft[]}` requires existing IDs or selection IDs, validates names/commands/arguments and uses a transaction. Path/cwd changes require newly selected grants; caller-supplied path/cwd fields are not trusted. `program.remove {id}` removes metadata only. `program.launch {id,terminal?:boolean}` creates a local process using the stored entry and sanitized environment. Omitted/false `terminal` defaults to background execution without a console on Windows; true requests an interactive terminal for command/script/console entries. `{pid,submitted:true,terminal:boolean}` indicates submission, not application readiness. Background command shells exit when the command completes; terminal mode keeps output visible. A shortcut retains its own target configuration. Entry commands/arguments are not written to launch activity events. These methods do not authorize ordinary Agent execution or import launch entries from tool reports.

`program.propose {name,kind:"file"|"command",cwd,path?,command?,args?:string[],source?:string}` accepts metadata from an authenticated ordinary client. Paths must be existing and absolute; command entries need single-line CMD syntax. It persists to a separate pending table, never executes, selects or saves a launch entry. Duplicate pending identities per principal update metadata; already-saved identities are not modified; pending intake is capped at 100. MCP exposes this as `propose_program`; the current gateway has twelve fixed meta-tools.

Controller `program.proposals {}` lists pending entries, `program.select {proposal_ids:string[]}` issues principal-bound snapshot grants after the user checks candidates, and `program.proposal_dismiss {id}` deletes only pending metadata. Selection does not collect or execute a program. `program.save` atomically inserts confirmed entries and removes accepted proposals; an invalid batch leaves both tables unchanged. Migration 4 adds the pending table without rewriting existing programs. Ordinary clients cannot list private entries, dismiss, select, save or launch programs; only opt-in metadata is discoverable through program.search.

Full tool discovery additionally searches local disks for an exact allowlist of portable media/archive/document/OCR utilities. The walk skips system/install/dependency/cache directories and links, is depth/entry bounded and cancellable, and does not claim full-drive coverage or execute discovered files. Known installed utility directories and PATH participate in quick discovery. Recognition adds capabilities, not execution trust.

## Agent-ready extensions (2026-10-04)

registry.search accepts bounded limit/offset and detail=summary|full; reviewed task aliases augment ordinary name/path results. The legacy array response remains. Full rows include capabilities, preferred and file-check health metadata. registry.prefer_instance is controller-only and changes selection only; registry.health_instance performs a file/hash check, never an execution probe.

agent.list retains host history and detects per-user installs/configs independently of daemon PATH. agent.observed records self-reported MCP host labels and historical handshake/call timestamps; this does not authenticate the host or grant controller authority.

execute.tool background=true returns execution_id/status immediately on the managed transport; exact policy, pinned image and one-use approval still apply. execute.status and execute.list are owner-bound (controller may inspect), execute.cancel preserves ownership checks. Declared outputs are bounded paths relative to cwd; only existing canonical files confined to cwd are returned. Task summaries persist; unfinished tasks become interrupted on restart; bounded stdout/stderr remain in memory only. Limit: 32 concurrent jobs, 300-second execution deadline, latest 100 task summaries listed. ToolHub does not negotiate experimental MCP Tasks.

program.propose accepts purpose/inputs/outputs/dependencies/examples. User selection/save remains mandatory. program.search only returns entries explicitly shared by the user (metadata.agent_visible). Sharing metadata is not permission to execute; program.launch remains controller-only.

skill.inspect returns bounded instructions from validated package assets and dependency status. Updating a Skill also updates its source path/kind/schema. Registration never runs installers or hooks.

MCP provides 12 fixed meta-tools: the existing 8 plus search_programs, request_execution_approval, get_task, cancel_task. tools/call retains JSON text content and adds structuredContent.result validated by a wrapper outputSchema. No per-installation tools are generated.

request_execution_approval supports submission with instance_id or polling with request_id alone (THP execute.approval_status). Only the original authenticated caller or the verified Desktop may retrieve an issued one-use approval. resolve_capability accepts cwd and preferred_instance for project-aware selection.

## Portable Skill library (2026-10-04)

The shared library collects reusable instruction workflows, independently of their authoring Agent. It does not scan or synchronize host Skill directories. New directory imports and the legacy skill.list registration alias use the same validation before any upsert: readable, confined instruction content and a complete portability declaration are required. Explicit host-specific scope or nonempty host_dependencies are rejected without replacing an existing entry. Script/service dependencies belong in requires, rather than host-specific MCP configuration in the shared library.

toolhub.skill/v1 remains parse-compatible with old records; portability is optional for historical parsing, but required for new shared-library imports. Its strict fields are scope (portable|host_specific), platforms (windows|macos|linux), nonempty inputs and outputs, permissions (read_files|write_files|execute_tools|network_access), and host_dependencies. The schema defines the same nested fields and bounds. A portable workflow for another OS may be retained but is unsupported_platform on the current OS.

skill.list retains every historical entry and adds compatibility {status,reusable,issues,assessment}. skill.inspect recomputes compatibility from the stored declaration and current package instructions on each read. Missing declarations, unreadable instructions or known host API references yield needs_review; explicit host dependencies yield host_specific. Incompatible instructions are withheld and instruction_authority remains false. Updating only the package text cannot bypass this check. Author mentions alone do not imply host dependency.

MCP search_skills filters compatibility.reusable=true even with no query. inspect_skill reports incompatible states as isError with the structured explanation. Dependency availability is checked separately: a portable workflow can still have missing_capabilities locally. Declared permissions do not grant execution authority. Compatibility is declaration validation plus a bounded check for known references, not proof of arbitrary instructions' portability or authenticity.

## MiMo host display diagnostics (2026-10-05)

agent.list includes MiMo installation/configuration metadata. Windows discovery reads bounded current-user/machine uninstall entries and known install locations; it checks paths without execution. The user config reader reads at most 1 MiB from ~/.config/mimocode/mimocode.jsonc (preferred) or mimocode.json, normalizing JSONC comments and trailing commas without evaluating code. It extracts only mcp.toolhub local command/enablement metadata, omitting provider credentials and other MCP entries; malformed metadata is reported separately.

MiMo/mimocode self-reported MCP names map to a single persistent mimo history entry, with the same non-authoritative observation semantics as existing hosts. No execution/trust/controller changes occur. Config presence and a detected executable do not establish a live MCP session. Historical events discarded by prior versions are not fabricated; future events are recorded normally. The copyable bootstrap includes the MiMo command-array format separately from generic JSON/Codex TOML.

## Multi-host display and version negotiation (2026-10-05)

The MCP stdio endpoint remains host-neutral with twelve fixed meta-tools. Known host profiles supply config templates and bounded read-only default user configuration discovery. Config metadata never establishes an active session or execution authority. Default readers do not resolve every project override, host enablement file or custom config location; this scope is disclosed in the UI.

agent.observed records known aliases and arbitrary self-reported client names as display history, never as authorization identities. Unknown normalized names receive stable SHA256-derived IDs, bounded to 64 unknown entries and 128 display characters; repeated entries remain updatable at capacity. History survives restart. Internal ToolHub self-test names are excluded. No controller/trust/approval changes result.

initialize keeps the requested version when supported (2024-11-05, 2025-03-26, 2025-06-18, 2025-11-25). Unknown requested versions receive the latest supported version, 2025-11-25, rather than the oldest; the client must assess compatibility. This does not claim support for experimental Tasks or newer protocol versions. See the multi-host review for actual Windows evidence and real-host limitations.


## Portable capability bundle catalog (2026-10-05)

The Desktop market reads bounded local `toolhub.bundle/v1` instruction bundles, validates portability and previews dependencies through THP `skill.preview` ({manifest, instructions, cwd?}). Preview uses the same resolver as `skill.resolve`, including trust, version and preferred-instance eligibility, and never registers or executes content. Ineligible provider instances are distinguished from missing instances in requirement notes; availability does not confer permission.

`skill.register` accepts optional boolean `if_absent`. When true it requires a package path and atomically inserts only absent IDs; a conflicting ID returns InvalidParams without replacing the old entry. Existing registration behavior remains compatible when omitted. The Desktop market additionally compares manifest and instructions to treat identical existing content as already collected.

System file selection reads a bounded preview; explicit collection saves normalized manifest, SKILL.md and the bundle into a content-addressed directory adjacent to the registry before atomic registration. Share/export serializes only the portable bundle, not registry paths, host config or credentials. Source/author declarations are not authentication; arbitrary instruction text still needs review. Schema: schemas/skill-bundle.schema.json. Limits and native Desktop commands: docs/development/MARKET.md. No remote index, dependency installation or new MCP meta-tools; collected Skills remain discoverable through search_skills and inspect_skill.
