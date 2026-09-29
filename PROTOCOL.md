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

An unavailable daemon or tool returns a structured error and explicit fallback information. ToolHub does not select or operate an agent's fallback sandbox itself. A result indicating `fallback_allowed` is information for the caller, not permission to bypass host or user controls.

## MCP boundary

Expose a small set of meta-tools, such as tool search, capability resolution, inspection, environment listing, authorized execution, and skill discovery. Do not publish one MCP definition per installed tool.

Wire schemas, method names, error enumerations, and public schema hosting are not implemented or published yet. Changes to shared contracts must be reviewed before dependent modules adopt them.
