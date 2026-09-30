# ToolHub B01 R2 — repair verification and bounded correction

## Decision

**B01 remains unaccepted: changes_required.** Formal B01 R2 is complete, using **2 of 4 total product-review rounds**. This result includes internal data/client reviews, the root review, runtime probes and verified localized corrections. No additional B01 formal review is scheduled. R3/R4 remain available for the owner's resulting product-batch decision; they are not silently consumed or renamed.

MiMo delivered real improvements, but its latest repair map's Closed labels and coverage's source-complete claims exceed the implementation. Substantial missing authority, IPC, scan reconciliation, environment, skill and protocol behavior remains. A macOS host/account limitation does not explain absent source functionality. This review does not accept core work, move it to B02 automatically, merge implementation into source main, push, release, complete or archive the project.

## Exact revisions and verification

- Reviewed latest repair bundle **v3** HEAD: `7a1ba39a8fb27dddffb4eb662a973c8bedf287ed`; original R1 ancestor `46a06aa11c370b2ca4432909aa2b8b0a70127090` confirmed.
- Input bundle SHA-256: `745b1400125cdc731f2fcf36ea089aba0bfb8c6c1f406c5e04844fd9237e9f60`.
- Published delivery/map still stamp `8715451cfcb25ccb11ed8a4b10179b5996e44daa`; actual bundle governs the review. Future reports must identify the exact delivered tip consistently.
- Isolated Codex correction HEAD: `6df2166e0c55ec64667021e386e75ca93acb066d`, branch `codex/toolhub-b01-r2-fixes`, direct child of the reviewed tip. Only two source/test files changed.
- Correction bundle: `ToolHub-B01-R2-corrections.bundle`; SHA-256 `21c8f94b282c3313b64748760108b89ef98d6bea091f14622237067de0aff325`; complete history verified by Git.

Independent Windows x64 verification used Rust/Cargo 1.95.0 and existing MSVC 14.44; no new toolchain was installed:

| Check | Delivered 7a1ba39 | After bounded corrections 6df2166 |
| --- | --- | --- |
| `cargo fmt --all -- --check` | exit 0 | exit 0 |
| `cargo test --workspace --offline --locked` | **47 passed, 0 failed** | **50 passed, 0 failed** |
| `cargo clippy --workspace --all-targets --offline --locked -- -D warnings` | exit 0 | exit 0 |
| `cargo build --workspace --release --offline --locked` | exit 0 | exit 0 |
| `git diff --check` / final fix checkout | clean before review | diff check passed; clean committed checkout |

Runtime probes use immutable copies of the verified **delivered** release binaries, private SQLite databases and owned harmless fixtures. Corrected policy behavior has fresh real SQLite/daemon regression evidence below. The final report distinguishes source inspection from executed Windows results. No macOS build/host check, unauthorized-user pipe test, external-agent launch or real credential test is claimed. Two security-subagent turns were stopped by tool content filtering; the root completed source checks and ordinary safe approval/process regressions instead. No filtered demonstration result is represented as executed evidence.

## Verified bounded corrections

**C01 — Atomic latest policy replacement.** `crates/registry/src/repo.rs::set_policy` previously retained different actions for one scope/subject, allowing reload order to determine effective policy. It now deletes/replaces that subject's action inside one SQLite transaction. Other subjects are preserved, and insertion failure rolls back to the previous rule.

**C02 — Persist before applying and return write failure.** `apps/daemon/src/service.rs::PolicySet` previously changed its in-memory engine and ignored a database error. It now persists successfully before changing the engine and returns a structured error on failure. Tests confirm previous effective state and restart state survive the failed write.

Three meaningful regressions were added. Before implementation, both the duplicate-action test and actual daemon write-failure test failed for the intended assertions; the rollback control passed. After implementation all three pass. Full workspace verification then passes with 50 tests. Logs `policy-red-all.log` and `policy-green.log` are selected evidence. This fixes only persistence semantics; it does not establish authenticated administration, immediate cross-daemon invalidation or complete policy contexts.

## F01–F18 disposition

All source anchors below refer to original delivered **7a1ba39** unless labeled corrected. Groups are not considered closed when only a helper/example improved.

| R1 group | Verified improvement | Remaining disposition |
| --- | --- | --- |
| F01 bound approval | Real approved Ask succeeds; changed argv and replay are rejected; registry consumption is atomic | **Partial/open:** trusted approval issuer/real caller-session boundary and policy/trust revision binding absent; raw cwd/session semantics incomplete |
| F02 actual principal | Request agent label no longer chooses execution principal | **Open:** daemon-global environment is not authenticated peer identity; no transport peer credential enforcement |
| F03 effective policy | Stored rules load; environment and blocked-status gates added; C01/C02 correct replacement/failure semantics | **Partial/open:** capability context absent; authority and cross-client state incomplete; invalid rule values default silently |
| F04 process bounds | Direct child timeout result; bounded raw capture; valid multibyte output no longer panics | **Partial/open:** descendant/pipe joins remain unbounded, cancellation/concurrency absent, output byte cap can still be exceeded |
| F05 privacy | Existing flag-value/path/short-flag unit cases pass | **Partial:** complete path/user/Unicode and all audit/error behavior is not demonstrated; one-character usernames explicitly skipped |
| F06 stable observations | Ordinary identical-path refresh and exact-text candidate dedupe improve; scan sessions/evidence now written | **Open:** wrong FK order, ignored write errors, legacy/canonical identity faults, global false invalidation and correction overwrite |
| F07 trust/resources | Bare basename is downgraded when no path match exists | **Open:** ordinary path/name confidence still grants Known to harmless renamed text; genuine measured metadata/resources missing |
| F08 native coverage | Actual Windows App Paths/Uninstall reads added; Quick skips one Full provider | **Partial/open:** bounded provider errors/metadata lost; required package/macOS/SDK roots and meaningful incremental/events/cancel absent |
| F09 environments | Some path-pattern environment tags added | **Open:** distinct same-name projects collapse; graph roots/parents/types/corrections not persisted/reloaded/exposed |
| F10 resolver | Blocked-only no longer selected; invalid OR and two earlier version cases improve | **Open:** wire preferences/architecture/trust ignored, rejected outcomes unstructured, version grammar/equality still wrong |
| F11 shared IPC | Named-pipe helper source added and Unix writer reference qualified | **Open:** listener exits, client still private-spawns, no peer/ownership/lifecycle enforcement; Unix cfg/test source fails |
| F12 wire/schema | JSON-RPC version/object-ID rejected; four schema files added | **Partial/open:** sizes/notifications/params/errors/DTOs still wrong; schemas/resources/events/cancel/version not enforced |
| F13 MCP | Successful lifecycle/list/content improved; not-found then successful call stays alive | **Partial/open:** invalid error message type, unsupported version echoed, input/envelope/size validation absent, malformed JSON terminates gateway |
| F14 discovery | Candidate/label fields checked; classification evidence now durable | **Open:** cross-principal access/revocation, stale revocation and nonexistent-candidate classifications accepted |
| F15 skills | Raw manifest registration/list/inspection code exists | **Open:** authoritative parser/path/version/trust checks bypassed; malformed/unsafe/wrong-dependency skills appear available |
| F16 adapters | Agent listing includes typed fixed config | **Open:** adapter trait remains manual/string-interpolated; no real session-launch implementation or runnable classify CLI |
| F17 CLI/reporting | JSON RPC errors improved; activity query added | **Partial/open:** child failure returns CLI 0; exec audit not in activity; report/import contract and required commands absent |
| F18 QA/claims | All-command-failure smoke control now correctly exits 1 | **Partial/open:** unknown fixture still not a scanned runnable marker tool; semantic assertions/integrated regressions and truthful coverage missing |

## Remaining prioritized findings

### R2-B01 [P1] Authority is still self-declared rather than peer/approver bound

**Sources:** daemon `service.rs:79-87`, `:455-527`, `:835-836`; executor `lib.rs:113-156`; core approval `execution.rs:9-24`.

Execution uses one daemon-global `TOOLHUB_PRINCIPAL` (default local.stdio), including for all named-pipe callers; the constructed stdio peer remains unused. The Allow admin gate trusts daemon environment or a principal prefix, and changing Deny to Ask is not subject to that gate. execute.approve has no trusted-approver authorization. The ordinary private test client obtains an approval with neither admin environment nor principal credential, then executes its exact request successfully. This verifies the API behavior, not an unauthorized external-user compromise.

The approval model contains a session string but the validator does not check it; policy/trust revision fields do not exist. cwd digest hashes raw text, so relative/omitted cwd is not bound to its resolved runtime meaning. Hash errors at issuance still become an empty string, while execution correctly fails hash read.

**Required closure:** explicitly trusted approval/control channel, documented actual OS/agent identity boundary, authenticated per-peer contexts and sessions, complete policy/trust/canonical runtime binding and all privilege-changing policy mutations authorized. A renamed environment variable or shared local label cannot close F01/F02.

### R2-B02 [P1] Named-pipe service exits and CLI never connects to it

**Sources:** daemon `main.rs:67-83`; IPC `lib.rs:103-128`; CLI `main.rs:292-300`.

The Windows helper creates max-one pipe instance with no accept step or peer enforcement, while the listener immediately loops to another creation. **Executed:** under a random owned endpoint/fixture database, `toolhubd --listen` exits 1 with `create_named_pipe failed`; no framed ping connection succeeds. The endpoint was isolated from any user/MiMo service. CLI still always creates a separate stdio daemon and never calls connect_named_pipe. Library helpers and a durable database do not implement shared concurrent service/readiness/reconnect.

On Unix, cfg!(windows) still typechecks calls to functions defined only under #[cfg(windows)], introducing a compile defect. `/tmp/toolhubd.sock` is still unconditionally removed even when live. Scanner tests still reference Windows-only evil on Unix and omit POSIX executable bits. These are static source defects, distinct from an unavailable macOS host.

**Required closure:** working authenticated user-scoped server/client accept/connect path, single-instance/ownership/readiness/reconnect/cancel/concurrency and target-correct native tests. This requires integrated work, not merely more soak evidence. The accept lifecycle is described by Microsoft's [ConnectNamedPipe API](https://learn.microsoft.com/en-us/windows/win32/api/namedpipeapi/nf-namedpipeapi-connectnamedpipe).

### R2-B03 [P1] Process deadlines do not bound descendants and output is not byte-bounded

**Sources:** executor `lib.rs:333-350`, `:355-377`, `:422-441`; daemon `main.rs:50,77` global lock.

Direct parent kill now occurs and returns TimedOut, a real repair. However, unconditional writer/reader joins wait for inherited descendant pipe handles. **Executed:** a safe finite descendant confirms its startup in captured output and sleeps 800 ms; a 150 ms request returns only after **1,390 ms**. The source can wait indefinitely for a nonterminating descendant, although no such process was left running in review. The collector also always waits approximately 500 ms because its completion helper always returns false and parent senders remain held. A hung operation retains the global service mutex; no cancellation/process-tree mechanism exists.

UTF-8 no longer crashes the daemon, but the 262144-byte raw capture becomes **262146 output bytes** after replacement decoding; the returned cap is violated. A following ping succeeds.

**Required closure:** actual process-tree and I/O teardown with bounded join/cancellation/disconnect and responsive service, enforce the final encoded response bounds, and negative descendant/bidirectional/Unicode tests through the service.

### R2-B04 [P1] Scan writes and reconciliation corrupt available state and user corrections

**Sources:** daemon `service.rs:912-940`; registry `repo.rs:53-75`, `:235-244`, `:484-503`; recognizer `lib.rs:261,290`.

**Executed:** first scan loses one new-venv instance because the dependent environment is inserted after the instance. The next scan finds it. A reused legacy instance ID is updated but then marked Missing because seen/interface bookkeeping uses the newly generated ID. Normal and equivalent Windows extended paths produce two IDs with the same canonical path. Quick globally marks an existing unvisited tool Missing. An injected evidence-write failure still returns success with completed scan sessions/errors=[] and marks currently scanned instances Missing. Rescan overwrites a user Blocked trust/ownership correction. Case-only candidate paths create another row; preserved metadata remains empty.

**Required closure:** transactional dependency order and truthful errors, returned persisted identity, canonical normalized identities, root/provider-scoped presence reconciliation based on complete coverage, preserve corrections and real metadata/provenance. Disabling an individual mark-missing call would avoid one symptom but would not implement the original lifecycle.

### R2-B05 [P1] Path heuristics still grant trust and environments merge distinct projects

**Sources:** recognizer `lib.rs:222-232`; environment `lib.rs:59-72,119-125`; daemon `service.rs:915-919`; native provider source.

**Executed:** plain harmless text named python.exe under an ordinary bin path receives Known trust from PathPattern confidence .9. Separate tenant-a/same/.venv and tenant-b/same/.venv share env.project.same; stored kind is detected with NULL root/parent. Native observed versions remain absent. Resource loading is absent, schema files are not consumed, and package/macOS/SDK metadata gaps remain source omissions. Actual Windows Registry reads are a positive improvement, but their metadata is discarded downstream.

**Required closure:** independent corroborating evidence and trust transitions, actual binary/package/version/architecture provenance, canonical environment-root identities with coherent persisted graph/query outputs, required native resources/providers and honest bounded coverage/incremental behavior.

### R2-B06 [P1/P2] Resolver ignores real constraints and version grammar remains permissive

**Sources:** daemon `service.rs:242-264`; resolver `lib.rs:29-35,59-109`; core version `version.rs:54-114`.

**Executed:** project-b cwd/environment preference plus minimum verified trust and x86_64 request selects project-a's Known/aarch64 provider and reports x86_64. The service still supplies fabricated architecture/default preferences. Blocked-only now has no selection but error=null and no rejected alternative explanation. `>=banana` selects 3.13; `=3.13.0` and `=3.13.0-rc2` both select 3.13.0-rc1; ^0.0.3 selects 0.0.9. The repaired OR, stable-minimum prerelease and ^0.2 examples pass.

**Required closure:** observed metadata and validated wire preferences reach eligibility filtering/ranking; typed Missing/Blocked/Unavailable reasons and alternatives; fully defined tested numeric/tool-version grammar, exact prerelease comparison and zero-major semantics shared with skills.

### R2-B07 [P1] Protocol and MCP conformance remains incomplete

**Sources:** daemon `main.rs:43-55`, service `:89-105`; protocol `types.rs:16,27,42,84`; CLI `main.rs:385-498`.

**Executed JSON-RPC:** scalar params and a 1,048,646-byte request succeed, notifications receive id:null replies, invalid object IDs are echoed in error envelopes, and `{}` is misclassified ParseError. Actual resolve/search/export still differ from declared DTOs. Schema files lack authoritative method/result/event compatibility and are unenforced; cancellation/events/version contracts remain absent.

**Executed MCP:** successful initialization/list/content and error-then-success improve. Unsupported version 2099-01-01 is echoed; wrong input types/version/object ID and oversize requests succeed. Tool failure puts an object in error.message. Malformed JSON emits a non-RPC error and exits 1. These are supported-contract failures, not merely missing client-account evidence; see [JSON-RPC 2.0](https://www.jsonrpc.org/specification) and the advertised MCP version's [lifecycle](https://modelcontextprotocol.io/specification/2024-11-05/basic/lifecycle) and [tools](https://modelcontextprotocol.io/specification/2024-11-05/server/tools).

**Required closure:** one enforced typed bounded contract, error/notification/version/input/lifecycle recovery, complete schemas/fixtures, event/cancel integration and real client conformance tests.

### R2-B08 [P1] Discovery operations still cross caller/candidate boundaries

**Sources:** daemon `service.rs:631,667,692-745`; registry `repo.rs:543-562`.

**Executed:** a second fixture principal enumerates, inspects, classifies and revokes the first principal's session. The original open daemon still classifies after cross-process revoke. A nonexistent candidate accepts a classification; out-of-range confidence is silently clamped and evidence is persisted. candidate.list-only still cannot list candidates. Durable untrusted evidence is an improvement, but it is not scoped authorized enrichment.

**Required closure:** authenticated caller/session/candidate membership, exact scopes, current authoritative revocation, strict validated classification DTO/provenance and protected credential listing. No execution trust elevation from classification.

### R2-B09 [P1] Skill registration bypasses its validator and availability resolver

**Sources:** daemon `service.rs:542-609`; unchanged core `skill.rs:72-139` and skills `lib.rs:22-78`.

**Executed:** bogus kind, ../outside path and string-valued requirements register and appear Available. A >=999 requirement with provider 1.0 is Available, as is a blocked-only requirement. The service does not invoke authoritative SkillManifest/path/availability checks, instead scanning any provider. The unchanged parser itself still accepts unsupported versions/unsafe Windows forms and drops package semantics. Required CLI registration/resolve operations remain absent.

**Required closure:** validated declarative instruction/MCP/package registration, canonical package containment and no hooks/installation, authoritative eligible-provider/version/trust evaluation for required and optional requirements, actual shared client queries and malformed/blocked outcomes.

### R2-B10 [P1/P2] Product/reporting/adapters and evidence are still incomplete

**Sources:** CLI `main.rs` command enum/execution return; daemon `service.rs:767-792,853-876`; unchanged agent-bridge `adapters.rs:17-35`; `scripts/smoke.ps1:31-76,95`.

**Executed:** a harmless registered fixture child exits 2; CLI still exits 0 with status failed. A recorded execution does not appear in activity; that endpoint reads only the generic activity table. Report still omits environments/capabilities and import has no method/command/implementation. Adapter launch remains manual text in source, so external account access is not its only blocker. Core resources/compatibility/migration recovery/platform fixture work is also absent.

The failing-command smoke control now fails correctly. Its purported marker executable is text that does not create a marker, is not a runnable Windows executable, and is not placed in a scanned root; no-marker remains vacuous. Policy only queries rules, MCP only lists tool names, and approved-exec success checks an outer exit code that still masks child failures. No full integrated negative/conformance suite is supplied. One-character username redaction is skipped at executor:571, and Unicode case-mapping slicing at :579 needs an actual regression before claiming arbitrary-path safety.

**Required closure:** semantic CLI exits, real redacted scan/execution/denial activity, useful compatible report and safe import, actual adapters, runnable discovered negative fixtures and truthful full target-to-evidence mapping. Broad source gaps cannot be marked host-unverified or Closed(core) to obtain acceptance.

## Evidence package and next decision

`B01-R2-EVIDENCE.json` records independent commands, totals, hashes, source-vs-runtime boundaries and selected observations. `B01-R2-REPRO.zip` includes selected harmless scripts/results and the data/protocol domain reports, not binaries/databases/user machine inventory/environment dumps or real credentials. Full local build logs are retained and integrity hashes recorded.

The verified correction bundle is safe to inspect/transfer into MiMo's isolated branch; no automatic cherry-pick, merge or release occurred. It is not a complete accepted product bundle.

The existing four-round policy explicitly requires the owner to decide the resulting workflow when a batch remains blocked after its second round. `B02-CARRYOVER-PROPOSAL.md` is a concrete proposal, **not a dispatch**: combine all unresolved B01 acceptance work with the original 22 B02 targets, then use only R3/R4 for the integrated product. Core contract/authority repair must precede dependent desktop integration inside MiMo's batch. Alternative: retain a core-only repair milestone and explicitly revise the later allocation with the owner. No new formal review or B02 development is started by this result.

## Owner summary

R2 复审仍未通过：交付 47 项测试通过，Codex 局部修正后 50 项通过，但共享 IPC、扫描、会话、技能和协议仍阻塞；已发布结果与修正包，审查已用 2/4，后续批次待确认。
