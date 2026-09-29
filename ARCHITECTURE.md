# Architecture baseline

## Source and status

The [full system design](docs/design/TOOLHUB_SYSTEM_DESIGN.md) defines the intended product. This document records its architecture baseline; no module is implemented yet.

## Core flow

```text
Discover -> Candidate -> Recognize -> Register -> Describe
         -> Resolve -> Authorize -> Execute -> Audit
```

AI optionally enriches recognition and description. Native discovery and core registry behavior must operate without AI credentials.

## Process boundaries

- `toolhubd`: user-level Rust daemon; owns registry, scanning, resolution, policy, execution, sessions, and events.
- `toolhub`: CLI client of the daemon; returns structured results for agent consumption.
- Desktop: Tauri, React, and TypeScript client; business logic and disk scanning remain behind daemon contracts.
- MCP and agent bridges: limited meta-tool interfaces to the same registry and policy pipeline.
- Storage: SQLite with migrations from the first persistence implementation.
- Default local transport: Windows named pipes and macOS Unix domain sockets. No default public listener.

## Proposed module boundaries

The original design proposes `apps/{daemon,cli,desktop}`, `crates/{protocol,registry,scanner,scanner-windows,scanner-macos,recognizer,capability,environment,ownership,resolver,executor,policy,audit,ipc,mcp,agent-bridge,skills}`, plus schemas, recognition resources, SDK/UI packages, and platform fixtures.

These directories will be created by approved implementation batches, not as empty crates in this bootstrap. Exact crate layouts and APIs must be settled in the foundation contract review.

## Integration contracts to finalize

| Producer | Input | Output | Consumer |
| --- | --- | --- | --- |
| Scanner | scan request and read-only host metadata | `ScanCandidate` | Recognizer |
| Recognizer | candidate and evidence | `RecognitionResult` | Registry |
| Registry | reviewed recognition result | definition, instance, interfaces, evidence | Queries and resolver |
| Resolver | `CapabilityRequest` | candidates and selection rationale | Policy and executor |
| Policy | execution identity, request, trust, permissions | allow, ask, or deny | Executor |
| Executor | authorized `ExecutionRequest` | `ExecutionResult` | Client and audit |

Names above express design intent; field definitions and wire compatibility are not yet frozen.

## Review gates

Freeze shared domain and protocol contracts before parallel implementation consumes them. Require a fixture-driven native discovery path, an explicit authorization path, and bounded daemon failure behavior before treating end-to-end execution as delivered. Desktop may prototype against reviewed mock contracts; displayed production data must come from the shared daemon registry.
