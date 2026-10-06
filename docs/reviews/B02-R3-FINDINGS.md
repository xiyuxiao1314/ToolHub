# ToolHub integrated product — R3 consolidated result

**Decision: changes_required. Formal reviews used: 3 of 4.** B01 and the integrated B02 product remain unaccepted. This is one formal R3, including its internal source lanes and independent checks. MiMo receives one consolidated repair assignment; R4 is the remaining verification round, with bounded Codex corrections permitted. No fifth round is implicit.

Reviewed source: `b62604095b248e7c13d44cfdfba8e789c0503c89`, branch `codex/toolhub-b02-integrated`. Input `ToolHub-B02.bundle` SHA-256: `fc90c63f8c525447f0a81eb08c741bf67bd68369e77c2ec55103d0b6e02ebf26`. Clone/checkout is clean, bundle hash matches, and correction `6df2166e0c55ec64667021e386e75ca93acb066d` is an ancestor. The coverage file and delivery owner-summary still stamp `706fcc5...`; this review uses the actual bundle tip, not that older stamp.

## Independent evidence and positives

Windows: `cargo fmt --all -- --check`, `cargo test --workspace --locked --offline` (**71 passed, 0 failed**), `cargo clippy --workspace --all-targets --locked --offline -- -D warnings`, and `cargo build --workspace --release --locked --offline` all exited 0. The original smoke and package scripts also exited 0 in the isolated checkout. Soak 4, MCP tests 6 and integration 4 are included in the 71, not additional tests. Their assertions establish less than the delivery's conformance/shared-service claims.

Freshly built binaries were independently exercised against private databases, owned endpoints and finite harmless fixtures. Evidence contains 37 primary and 13 supplementary runtime observations, actual-library helper probes, and browser inspection/screenshots of the delivered desktop. Raw private databases, full host inventory, binaries and the inventory-containing smoke log are excluded from the reproduction package. See `B02-R3-EVIDENCE.json` and `B02-R3-REPRO.zip`.

Verified improvements: two real Windows pipe clients now connect and ping; ordinary approval issuance is denied by default; per-connection discovery owner checks deny another connection; a short renamed text fixture stays Unknown; approved harmless execution succeeds; atomic policy replacement/persist-before-apply fixes survive; raw output storage is capped and persistent execution records omit stdout/stderr. These positives do not close the groups below.

Source anchors are repository-relative, one-based, at the reviewed tip. Runtime labels refer to JSON observation keys in the reproduction package. Source deductions are distinguished from executed observations. MacOS host execution, native Tauri packaging, and actual external-agent launch were not verified. Missing production paths are implementation gaps, not unavailable-host checks.

## Consolidated findings and required closure

### R3-F01 [P1] Authenticate controller authority and gate every policy weakening

`apps/daemon/src/service.rs:83–99,535–540,966–974`: controller status comes from inherited `TOOLHUB_ADMIN` or a principal prefix; only setting Allow is restricted. Runtime `ordinary_replace_deny_with_ask` succeeds. A client can remove a stronger prohibition through Ask. There is no independently authenticated controller approving a specific caller. Pipe peers are all barred from controller issuance. Desktop forwards callers into its private daemon identity.

**Close:** derive caller/controller identity from the real transport; keep roles distinct; strictly validate action/scope; compare the old and proposed effective authority for every replacement. Ordinary clients must not mint approval, impersonate a controller, or weaken Deny/Ask. Prove an authorized controller-to-caller Ask flow through the shared service. Environment labels are fixture configuration, not authentication evidence.

### R3-F02 [P1] Deliver one authenticated shared service with stable identity and safe lifecycle

`apps/daemon/src/main.rs:76–99,114–140`; `apps/cli/src/main.rs:317–345`; `apps/desktop/src/main.rs:48–61`; `crates/ipc/src/lib.rs:87–105,120–173`. Windows principals are timestamp connection labels without OS peer/endpoint ownership checks. CLI falls back to private daemons; desktop always private-spawns; Unix CLI has no shared connection. Cached policy/sessions then diverge. Runtime proves real two-client pipe connectivity, but the same OS user reconnecting cannot inspect its prior discovery session (`pipe_same_user_new_connection_session`).

The Unix branch still references Windows-only functions inside `if cfg!(windows)` (both branches typechecked), and unlinks an existing socket without checking live ownership. These are source defects; no macOS compilation is claimed here.

**Close:** one user-scoped managed service, authenticated stable caller identity separate from connection/session, verified endpoint permissions/owner, single-instance/readiness, explicit reconnect and shutdown behavior. Desktop/CLI/MCP must use it consistently; no silent independent policy owners. Compile supported targets and safely distinguish live/stale owned endpoints. Desktop closure must leave service/client state usable.

### R3-F03 [P1] Bind approvals to one complete request and current authorization revision

`apps/daemon/src/service.rs:370–447,562–637,1193–1196`; `crates/executor/src/lib.rs:105–180,200–269,648–652`; `crates/core/src/execution.rs:117–124`. Daemon policy context omits command/capability, executor adds a different context, cwd remains raw/optional, session is hardcoded `cli-session`, and policy digest is action/trust/status/instance rather than a revision. Legacy empty digests bypass checks; hash/save/revoke failures can be ignored. Exact argv hashing joins unescaped U+001F. Actual helper `different_argv_same_digest=true` demonstrates different argument vectors have identical encoded input, not a SHA collision or executed attack.

**Close:** resolve canonical effective cwd/instance/executable and a single full command/capability policy context before authorization; bind authenticated session, exact unambiguous versioned argv/stdin/sanitized-env encoding, policy/trust revisions, expiry and one-time use. Fail closed on missing/hash/persistence errors and invalidate incompatible old approvals. Changes away and back must not revive approval. Prove rejected request variations without launching real tools.

### R3-F04 [P1] Make execution deadlines, cancellation and output bounds real

`crates/executor/src/lib.rs:358–411,460–472`; `apps/daemon/src/service.rs:472–473,517–533`; `apps/daemon/src/main.rs:94–96`. Blocking I/O joins occur before bounded collection. Parent kill precedes unbounded/best-effort tree helper; normally exiting parents can leave inherited pipes. The service mutex blocks other operations during execution. Cancel only logs and reports success for arbitrary IDs.

Runtime: a 150 ms request returned after **942 ms**, waiting for an owned finite 800 ms descendant; `cancel_nonexistent` returns cancelled=true; `utf8_output_cap` returns **262146 encoded bytes** against 262144. No infinite process or attack fixture was used.

**Close:** bounded process-tree/pipe teardown and helper waits, real operation IDs and ownership-checked cancellation, concurrent service responsiveness, final UTF-8 and serialized-frame bounds with accurate truncation. Prove success, child failure, timeout, parent-exit descendant, cancellation while running and concurrent ping using finite owned fixtures.

### R3-F05 [P1] Make scan ingestion transactional and reconciliation coverage-scoped

`apps/daemon/src/service.rs:1076–1128`; `crates/registry/src/repo.rs:44–135,263–270,522`; scanner provider error/truncation handling. Instances are inserted before new environments, foreign-key failure is ignored, interfaces can reference the generated rather than returned persisted ID, and scan completion conceals write failures. Quick globally invalidates unseen available instances whenever roots_failed is empty.

Runtime: one of two new same-name project fixtures is absent after the first scan and appears on the second; an unrelated available fixture becomes Missing after Quick (`unvisited_status_after_quick`).

**Close:** dependency-ordered transactions using the persisted identity across every relation; propagate failed/partial writes. Persist per-provider/root completeness/truncation and reconcile only completely visited scopes. Quick/Custom/provider-error/capped Full must preserve untouched installations. Canonical, case, link/extended-path and legacy-ID tests must maintain the intended distinctness and one identity per canonical file.

### R3-F06 [P1] Preserve actual recognition evidence and native metadata

`crates/recognizer/src/lib.rs:225–279,312–330`; `crates/scanner/src/providers.rs`; `crates/registry/src/repo.rs:100–130,263–270`. Size plus MZ/ELF magic is counted as independent corroboration for Known; version hints are unpopulated and architecture is host architecture. Candidate metadata is dropped, instance evidence is fabricated, and owner corrections are overwritten. Runtime metadata-only **2048-byte junk MZ python.exe becomes Known** with null version/host architecture. It was never executed. Native Mach-O is excluded by this trust branch.

**Close:** separate format/name hints from independently corroborated product identity and execution trust; parse measured platform metadata and retain provenance. Integrate the originally dispatched discovery sources/modes with explicit coverage and limitations; do not label directory guesses complete native-provider support. Preserve explicit owner/trust corrections and actual evidence through rescan/restart. Unknown discovery remains read-only and never auto-probes executables.

### R3-F07 [P1] Persist a truthful environment graph and resolver/version eligibility

`crates/environment/src/lib.rs:107–123`; `apps/daemon/src/service.rs:73,280–293,1105`; `crates/registry/src/models.rs:5`; `crates/core/src/version.rs:53–128`; `crates/resolver/src/lib.rs:72,144–169`. Project basename identity collapses distinct roots; persisted roots/parents are NULL and startup rebuilds a fresh graph. Resolver substitutes host arch, accepts malformed preference/version values, discards rejected candidates and returns no typed eligibility error.

Runtime: two distinct projects share `env.project.same`; an aarch64 DB instance satisfies x86_64; stable 3.13.0 satisfies invalid `>=banana`; rc1 satisfies `=3.13.0`. Zero-major caret is a separate source defect. CLI/MCP do not forward the full preferences.

**Close:** canonical root-based IDs, persisted/reloaded hierarchy/provenance/ownership/size uncertainty; observed architecture and explicit preferences across clients. One validated version grammar/comparator including prerelease and `^0.0.x`, deterministic ranking, typed rejected alternatives and fallback. UI, capability and skill selection must agree.

### R3-F08 [P1] Enforce durable discovery membership, scope and revocation

`apps/daemon/src/service.rs:740–898`; `crates/registry/src/repo.rs:569–586`. Ownership checks improve isolation but use cached sessions; issuance/revocation write failures are ignored. Classification checks neither candidate existence nor authorized disclosed membership; invalid confidence is defaulted/clamped. candidate.list scope has no actual candidate-list operation.

Runtime: nonexistent candidate with confidence 2 is accepted; another process revokes the session and the original open daemon still accepts classification (`stale_process_classification`). Classification remains untrusted, a positive to preserve.

**Close:** strict typed finite confidence, real candidate existence/disclosure set and scope, explicit submitter/session provenance, stable authenticated caller/session, authoritative current expiry/revocation and persist-before-success. Test reconnect/restart/another caller/another open client and failed persistence. Enrichment must never elevate execution trust.

### R3-F09 [P1] Validate declarative skills and resolve actual requirements

`apps/daemon/src/service.rs:666–724`; `crates/core/src/skill.rs:72–119`; `crates/skills/src/lib.rs:22–57`. Raw RPC registration bypasses model/path/type validation. Availability merely counts status-available providers, ignoring required version/trust and optional requirements. Runtime accepts bogus kind, string requires, leading-backslash path, and reports Available; a >=999 requirement is satisfied by 3.13 rc1.

**Close:** one authoritative instruction/MCP/package manifest model and canonical package containment at the public boundary; strict schemas/types/kinds; same eligible resolver for required/optional capabilities and useful failure reasons. Implement the originally required CLI/UI declarative import/download/register/resolve flows without hooks or tool installation.

### R3-F10 [P1] Enforce one JSON-RPC/MCP contract, events and bounded SDK lifecycle

`apps/daemon/src/main.rs:43–64,93–99`; `apps/daemon/src/service.rs:114–155`; `apps/cli/src/main.rs:433–556`; `crates/protocol/src/types.rs`; `schemas/protocol.schema.json`; `packages/sdk-typescript/src/index.js:50–80`. Stdio drops valid notifications before dispatch; pipe replies to them. Invalid object IDs are echoed, scalar params and >1 MiB stdio accepted. Runtime confirms these; notification policy.set is silently discarded. MCP echoes unsupported 2099 version, accepts wrong query type, and malformed JSON terminates it (exit 1). Declared DTOs/schema/results differ; negotiation/events are declarations only. SDK pending calls lack close/error/deadline cleanup and results are any.

**Close:** shared envelope and method-specific typed validation, notification execution without responses, standard bounded/recoverable errors across actual transports, canonical serialized DTOs/schemas. Enforce advertised MCP initialization/version/tool arguments and content/isError failure semantics. Implement ordered bounded events/subscriptions, real operation cancellation, and typed SDK with disconnect/deadline cleanup and working example. Preserve the bounded seven-meta-tool interface unless coherently revised.

### R3-F11 [P1] Complete the nine desktop workflows and protect the local UI boundary

`apps/desktop/src/main.rs:48–100,154–218` is a browser SPA with headings, prompts, a small table and raw JSON; it is not the dispatched complete desktop flow. Actual Settings shows text only; Security shows policy JSON only; Environments has no graph. Tools lacks detail/filter controls. Skills/Agents have no import/launch flows. There is no settings persistence, approval control, connection/reconnect model or event UI. IAB does not support prompt(): that browser-specific probe limitation is recorded separately and is not a claimed native failure.

The loopback /rpc route has no method/origin/session boundary, reads one TCP chunk instead of a bounded complete HTTP body, and uses one blocking private-daemon handle. Unescaped external metadata is inserted via innerHTML. These are source risks, not a demonstrated cross-user exploit.

**Close:** B02-01–10 and B02-17 in full against the authenticated shared service, with real nonempty/empty/error/loading/stale/reconnect states, safe text rendering, correct bounded transport and session boundary. Deliver the dispatched Tauri/React/TypeScript shell or obtain the required owner-approved equivalent contract change; the current substitution was not approved. Supply actual UI operation evidence/screenshots and an unsigned local Windows desktop artifact. Missing implementation cannot be classified as merely missing Tauri screenshots.

### R3-F12 [P1] Integrate connected-agent, temporary-provider and extension lifecycles

`crates/agent-bridge/src/adapters.rs:3–35,87–101,127–135`; `apps/daemon/src/service.rs:901–929`; `crates/agent-bridge/src/temp_credentials.rs:8–25`; `crates/scanner/src/lib.rs:89–116`. Adapter launch remains manual instructions with no runtime caller; endpoint configuration is ignored by CLI. Temporary credentials are an unused memory helper. ExtensionHost has no scan integration and self-declared operations do not constrain execution authority.

**Close:** one actually supported detected adapter with explicit disclosure, scoped session, real launch/status/monitor/cancel and provenance; owned harmless adapter fixture first, unavailable real account/host separately unverified. Integrate temporary-provider session lifecycle and prove synthetic keys absent from persistence/log/export on success/error/cancel. Integrate a working permission-limited scanner fixture with enforced confinement; no arbitrary privileged code loading. No-key core remains usable.

### R3-F13 [P1] Connect validated resource/product updates with atomic recovery

`crates/recognizer/src/resources.rs:30–59`; `crates/agent-bridge/src/update.rs:32–71`. Resource helpers have no recognition/runtime activation caller; malformed kind/tools/version are accepted. Update verification ignores version/min_current, resolves a manifest-relative payload, then apply uses the raw cwd-relative path. Actual helper accepts version banana/minimum999 and malformed resource; apply renames the previous artifact, fails copying, and leaves destination absent. **Backup remains**; this is missing automatic recovery, not irrecoverable deletion.

**Close:** integrate independently versioned validated resources into actual recognition/capabilities with verified atomic activation/rollback. Product updates require explicit user action, compatible versions, a bound verified payload, staged atomic replacement, cancellation/failure recovery and usable previous artifact. Test only owned fixture artifacts; do not install/update user software or invent a release/signing service.

### R3-F14 [P1] Make reports, activity and privacy useful and accurate

`apps/daemon/src/service.rs:345–356,417–429,475–504,982–1065`; `crates/audit/src/lib.rs:37–46`; `crates/executor/src/lib.rs:542–624`. Export omits the schema required by import, environments/capabilities/provenance; import skips malformed/over-limit entries and write errors. Runtime own-export import fails with unsupported schema. Completed execution is absent from activity; early authorization denials are not recorded. Audit writes can be ignored; paths/opaque short-flag values/Unicode matching remain incompletely redacted (source deductions).

**Close:** schema-valid redacted roundtrip with explicit validation/limit/partial failure, informative imported provenance without trust elevation; complete CLI report/import/activity/discovery/skill commands. Real scan/registry/resolve/execute/denial activity and durable audit state; safe Unicode/path/credential redaction including error/export paths and observable write failure. Persistent stdout remains disabled.

### R3-F15 [P1/P2] Replace weak acceptance evidence and deliver a usable coherent package

`apps/daemon/tests/integration.rs:87–153`, `tests/soak.rs:28–167`; `apps/cli/tests/mcp_conformance.rs:40–185`; `scripts/smoke.ps1:26–105`; `scripts/package.ps1:6–25`; `docs/development/PACKAGE.md`; workspace Cargo version. Shared-client/soak tests spawn private stdio children and use permissive substring/envelope checks; they do not verify actual concurrent pipe behavior or semantic durability. The unknown smoke text fixture is nonrunnable and outside scan roots, deny only queries policy, MCP only lists names.

Package script passes, but produces label0.2.0 with binary0.1.0, omits required resources/SDK, and hashes only top-level files. README's optional daemon command runs stdio rather than --listen; desktop searches bare toolhubd rather than its packaged sibling. Documentation references absent report/resource/update schemas. Packet stamps disagree; source review count is outdated.

**Close:** controlled real shared endpoint and exact-built-binary linkage; semantic bounded concurrent client fixtures and meaningful negatives, including runnable in-scope unknown fixture with a valid marker control that is never automatically executed. Validate package from a clean private directory without global PATH changes, all required resources/migrations/schema artifacts, recursive checksums, correct versions/startup/uninstall docs. Reconcile every original subitem and finding with fresh evidence; distinguish configured/cross-compiled/host-verified and open. Correct packet HEAD/hashes/review ledger. MacOS unavailable-host checks remain honest, separate from known source defects.

## Carryover and disposition

All R2-B01–10 remain partially open, mapped respectively to F01–03; F02; F04/F14; F05–06; F06–07; F07; F10; F02/F08; F09; F02/F10–12/F14–15. Preserved localized fixes are acknowledged above, not treated as whole-group closure.

Every original B01-01–18 and B02-01–22 subitem remains required under `MIMO-B02-INTEGRATED-PRODUCT.md`. This review does not shrink scope to the listed examples. B02 desktop, connected flows, extension/resources/updates, shared-client QA and operational delivery need substantive completion, not coverage wording changes.

One repair task: `MIMO-B02-R3-REPAIR.md`. MiMo should internally integrate and repair all groups before one R4 packet. Codex made no product change in R3. No implementation was merged into documentation main, pushed, publicly released, or marked accepted/complete. A failed final R4 would require an owner scope/workflow decision, not automatic fifth review or acceptance of blockers.
