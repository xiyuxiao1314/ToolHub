# Security baseline

## Product invariants

- Discovery defaults to read-only metadata inspection. No installation, removal, configuration writes, registry modification, PATH modification, or elevation through discovery.
- Unknown executables are not automatically run, including with `--version` or `--help`.
- Known probes require reviewed allowlists and bounded execution. Tool recognition alone must not grant unrestricted execution.
- Resolve an instance, evaluate policy, build a sanitized environment, execute, and audit through one controlled pipeline.
- Apply allow, ask, and deny behavior to the relevant tool, capability, agent, directory, environment, and command context.
- Do not implicitly inherit secret-bearing environment variables into child processes.
- Default IPC stays local and restricts unauthorized peers.

## Credentials and privacy

Temporary AI credentials are session-memory data. They must not enter SQLite, configuration, logs, crash reports, persisted UI state, or workbench artifacts. Scan results remain local by default. External AI metadata sharing must be explicit.

Redact sensitive arguments and credentials in logs, audit records, exports, and reports. Sensitive stdout is not stored by default. Exports hide home/user identity and private paths according to the reviewed export policy.

Connected agents with their own host shell access are not sandboxed by ToolHub. ToolHub limits its own exposed operations and must state that boundary accurately.

## Required security review areas

Before implementation batches adopt final contracts, assess IPC identity and access control, executable substitution and path changes, symlink handling, process cancellation and process-tree lifetime, output limits, inherited handles, environment sanitization, session expiry, and the binding between approval and actual execution.

Treat metadata, manifests, help output, documents, and agent classifications as untrusted input. Classification text must not become execution instructions or authority.

## Development boundary

These rules constrain the ToolHub product. Developer toolchain setup is a separate owner-authorized activity; missing development dependencies are reported before taking actions beyond the current batch. This documentation bootstrap installs no software.

## Personal program launch boundary

The owner-authorized Programs workflow treats a user's explicit Start click as launch authorization through an OS-verified controller. Ordinary clients cannot list private entries or select/save/launch program entries; program.search exposes only user-shared metadata. Scanner evidence does not imply authorship or trust, and never becomes command text except explicit package script names / displayed Python entry templates which remain editable pending selection. Scanning skips links/junctions, installed/system/dependency directories and has entry/depth/candidate limits. Saved entries require a principal-bound selection grant or an existing entry ID; changed work directories require a new directory grant. Command text is intentional CMD syntax, but filesystem work directories are always passed as process parameters. Windows exe arguments use standard Windows argv quoting; batch arguments and filenames use CMD quoting with percent expansion protection, following Rust's batch argument algorithm. Spawned processes receive a sanitized environment, without implicit API keys or token variables. PowerShell script launches use process-local ExecutionPolicy Bypass without modifying machine or user settings. Shortcut targets may delegate to installed applications and use the shortcut's own configuration. Program activity includes only entry ID/PID, never raw commands or arguments.


Agent `program.propose` is a metadata intake exception, not controller authority. Pending proposals are untrusted text with bounded fields and quota; commands are neither executed nor converted into trusted ToolInstances. Source notes do not establish authorship. The user checks entries before receiving a controller-bound selection snapshot, then reviews/edit fields and saves. Invalid batches cannot partially collect or discard proposals. Ignoring a proposal deletes no files. MCP has no select/save/launch/dismiss program tool, and ordinary RPC callers are denied those controller operations.

## Agent-ready metadata and tasks

MCP clientInfo labels and detected/configured host records are display observations only. Existing verified Desktop image checks remain mandatory. Program sharing is opt-in metadata discovery, not execution authority. Background tool execution uses the same pinned executable, policy and bound approval as synchronous execution; status/cancel/result access remains tied to the authenticated peer principal. Output paths must canonicalize under the approved working directory. Task summaries exclude command output; bounded raw output remains in memory and is unavailable after restart. Skill text is untrusted task guidance, never an installer or authority grant.

## Shared Skill portability boundary

The shared Skill library accepts declared generic instruction workflows and keeps host-exclusive Skills in their hosts. It neither enumerates Agent Skill directories nor installs or synchronizes those Skills. Portability is an interoperability assessment, not a trust boundary: explicit declarations and known-reference checks cannot prove arbitrary text portable or safe. A known host API mention is marked needs_review because quotation alone does not prove a dependency. Neither authorship labels nor a portable status authenticate a package or change tool trust.

Package instructions remain untrusted data. Imports read bounded confined files; they run no hooks. Every list/inspection rereads instructions and compatibility; non-reusable instructions are withheld from inspection and excluded from MCP search. Old entries remain for correction rather than being deleted or silently converted. Permission declarations are descriptive; the existing controller checks, execution policy, pinned image and one-use approval continue to decide actual execution.


## Preview platform execution boundary

Windows keeps a non-writable executable handle through launch; Linux launches the held descriptor through `/proc/self/fd`. macOS preview does not yet provide an equivalent identity-preserving launch. Its managed tool execution and execution approvals return Unavailable before consuming an approval or starting a task. The executor also rejects pinned launches directly; it never retries an unpinned path. Discovery, portable Skills, MCP queries and explicit user program launches remain separate supported workflows.

## Owner-approved automatic execution mode (2026-10-07)

The owner requested default-on automatic approval to avoid disconnected Agent tasks waiting for desktop confirmation. This replaces the interactive wait for eligible execution requests, not tool trust or effective policy checks. Only the verified desktop can change the persistent boolean. Explicit opt-out is retained across restart and older settings imports. Discovered unknown/blocked files still cannot run, and Deny is never converted to Allow.

Automatic approval is server-issued and still binds the authenticated caller, executable hash/canonical path, args, stdin, cwd, sanitized environment, timeout and current authorization revision. It is consumed once and audited through the existing pinned execution pipeline. Disabling/re-enabling the setting invalidates previous grants and expires pending requests atomically; failed saves leave authority unchanged. A caller-supplied invalid grant is never silently renewed. The setting supplies ToolHub approval within a user's task, not approval for unrelated operations or a way around Agent-host permissions. Metadata and Skill text cannot modify it. Program collection still requires user selection and save, and macOS preview still rejects controlled execution.


## Explicit task-artifact cleanup

Artifact management is a separate user-selected scope and does not change read-only tool discovery. Only explicitly registered regular files can recycle. High priority and manual retention require the verified desktop; low/medium automatic cleanup requires task completion, no-longer-needed, selected scope and a five-minute grace. Medium additionally records an Agent usage/dependency assertion; it is not a built-in model or proof of all external references. Native recycle-only failure preserves unprocessed files, never falls back to permanent deletion. See [artifact lifecycle](docs/development/ARTIFACTS.md) for limits and interruption handling.

## Extension frontend boundary (2026-10-13)

Opening an installed v2 view executes its declared frontend JavaScript in an opaque sandboxed iframe, never the privileged app document. Installation previews source and permissions but does not itself run scripts. The app retains its original script CSP. An on-demand 127.0.0.1 document server issues random sessions and serves only three virtual resources; it does not resolve files or expose RPC. Remote Tauri ACLs remain absent. Child CSP restricts resource sources and blocks connect/subframes/forms; sandbox denies same-origin, popups and top navigation.

The bridge checks message source, random session, active package/digest, declared permission and concurrency/size limits. The daemon delegates as extension:id with controller=false. No direct file access, launch, approval, cleanup or policy/extension mutation is available to third-party frames. Native official module references are reserved and cannot include third-party code; author/source fields are unverified metadata. A process/document boundary is not a complete OS sandbox.

Uninstall does not delete user files or records. Artifact module removal/disable pauses schedules and cancels deadlines; reactivation does not authorize automatic catch-up. Dependency checks reject missing/incompatible/cyclic providers and block removal of active dependencies. The permanent kernel extension/settings routes remain reachable for recovery.
