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

All `program.*` methods require an OS-verified controller. `program.list {}` returns launch entries with live `available` status. `program.scan_start {roots?:string[]}` starts background read-only discovery (default: mounted local fixed/removable disks); `program.scan_status {offset?:number,limit?:number}` returns progress, limitations and at most 200 candidates; `program.scan_cancel {}` requests cancellation and preserves discovered candidates. `program.select {candidate_ids:string[]}` or `{path:string,kind:"file"|"command"}` issues principal-bound selection IDs without writing entries. Desktop manual selection comes only from the native picker. `program.save {items:ProgramDraft[]}` requires existing IDs or selection IDs, validates names/commands/arguments and uses a transaction. Path/cwd changes require newly selected grants; caller-supplied path/cwd fields are not trusted. `program.remove {id}` removes metadata only. `program.launch {id}` creates a local process using the stored entry and sanitized environment; `{pid,submitted:true}` indicates submission, not application readiness. Entry commands/arguments are not written to launch activity events. These methods do not authorize ordinary Agent execution or import launch entries from tool reports.
