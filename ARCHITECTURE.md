# Architecture baseline

## Source and status

The [full system design](docs/design/TOOLHUB_SYSTEM_DESIGN.md) defines the intended product. This document records the original architecture baseline and subsequent owner-approved implemented changes.

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

## Owner-approved Programs extension (2026-10-03)

Project launch entries are separate from recognized ToolInstances. The scanner exposes read-only entrypoint candidates; the daemon owns cancellable background discovery, selection grants, atomic SQLite persistence and explicit controller-only launches. Desktop owns the native file/folder picker and user selection/editor flow. Migration 003 adds program_entries without changing tool or agent authorization. A Windows console helper assigns fresh input/output handles and keeps batch output visible. The protocol and SDK add program.* methods atomically; ordinary agents cannot call these methods. This owner-requested extension expands the desktop's personal launch workflow beyond the original tool inventory baseline.

## Owner-approved extension-first workbench (2026-10-13)

The kernel retains identity/MCP, registry storage, authorization/execution/audit, extension lifecycle and recovery. Seven first-party workbench modules contribute navigation and lazily loaded UI; explicit module gates control feature queries without deleting underlying records. Native service implementations remain compiled Rust modules, not unloadable DLLs. First-use package seeding is one-time and transactional.

Versioned v2 packages can contribute interactive frontend documents, bounded theme/layout tokens and dependencies in addition to v1 recognition/portable Skills. Third-party documents are served from an on-demand session-only loopback origin and sandboxed opaque iframe with their own CSP. This origin has no Tauri native ACL. A fixed SDK bridge binds messages to the frame/session/package digest and dispatches only declared methods as an ordinary extension principal.

Official page adapters reside under `apps/desktop/ui/src/official`, kernel shell/loader under `workbench`, manifests under `extensions/official`. Existing Programs/Artifacts/Skills/Market components are loaded through the same official feature registry. Arbitrary native executable plugins, online catalogs and privileged layout code are excluded from this implemented contract. See the authoritative extension guide and schema.

The owner-approved hierarchical shell (2026-10-19 local development) keeps four kernel-owned primary groups. Existing module/page/CLI IDs remain secondary destinations; generic pages are grouped by package and can be pinned by the user. Theme/recognition-only extensions contribute no destinations. Presentation history/pins are local WebView preferences; no protocol, manifest, execution or database contract changes.


The beta.4 workbench adds an optional declarative CLI contribution: extension-owned metadata and argv templates, host-owned native forms and a controller-only preview/start API, and the existing daemon execution pipeline under an ordinary extension identity. Generic iframe scripts retain their previous limited bridge. The FFmpeg package is externalizable independently of core code and includes one portable Agent workflow; no per-plugin MCP tools are generated.
