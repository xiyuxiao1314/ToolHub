# MIMO-000 — Assessment

## Identity

- Workbench task: `task_a11c48d272163020b1ee37f1`
- Profile: `mimo`
- Baseline HEAD: `ce8c755f9ecb2e8a6655d6997f21767332502a45` (matches `BOOTSTRAP_MANIFEST.json`)
- Artifacts read: `MIMO-000-ONBOARDING.md` v1 (`art_2a05c83b69d1d74fc1da4e64`), `BOOTSTRAP_MANIFEST.json` v1 (`art_ad3a6287d6b40c0a1a5a56e3`), original design text (`art_c616d9edf5401fd4e6e9fac1`)
- Bundle: `ToolHub-bootstrap.bundle` v1 downloaded; SHA-256 `634cdda0a5647ba0ac1c70979f491b827f1f9ec3bc3c6b0b04436e3907876bbe` matches manifest
- Review baseline inspected: `D:\ToolHub` (clean worktree at the same HEAD)
- Independent checkout: `D:\ToolHub-mimo\checkout` (HEAD matches; worktree clean; `origin` is the local `D:\ToolHub` path only — not a hosted remote or push destination)
- Local copies of artifacts: `D:\ToolHub-mimo\artifacts\`

## Access and tooling

- Workbench join/read/download succeeded under `--profile mimo`.
- All 13 baseline files present and readable; `MIMO-000-ONBOARDING.md` checkout SHA-256 matches manifest.
- Read-only toolchain check on this machine:
  - Present: Git, Node.js 22.20.0, Python 3.13
  - Missing: `rustc`, `cargo`, `sqlite3` CLI
- No software was installed during onboarding. Rust toolchain setup is a prerequisite for Stage 1 implementation and needs owner authorization later.
- No product code, build, or platform test was executed (none exists in this documentation-only baseline).

## Confirmed read status

Read in full: `README.md`, `AGENTS.md`, `ARCHITECTURE.md`, `DOMAIN.md`, `PROTOCOL.md`, `SECURITY.md`, `CONTRIBUTING.md`, `WORKBENCH.md`, `docs/ROADMAP.md`, `docs/tasks/MIMO-000-ONBOARDING.md`, `BOOTSTRAP_MANIFEST.json`, and the owner-selected original design (`docs/design/TOOLHUB_SYSTEM_DESIGN.md` / workbench attachment). Design examples are treated as illustrative until contracts are reviewed.

## Prioritized design gaps and contradictions

### P0 — must freeze before parallel implementation

1. **Capability identifiers are not canonical.** The design uses `media.frame.extract`, `media.video.frame.extract`, and skill `requires: media.video.frame.extract` interchangeably (`DOMAIN.md` already flags this). Without a reviewed taxonomy and alias policy, registry, skills, and resolver cannot share data safely.
2. **Domain identity formats and cardinality are open.** `DOMAIN.md` lists required foundation decisions (ID formats, relationship cardinality, evidence provenance, confidence meaning, timestamps, trust/availability transitions, path normalization, version constraints). None are frozen in `schemas/` yet.
3. **Protocol wire contract is intent-only.** `PROTOCOL.md` names JSON-RPC and transports but leaves method names, request IDs, error codes, cancellation, event envelopes, size limits, and version negotiation undefined. Downstream daemon/CLI/MCP work cannot proceed without this freeze.
4. **Execution authorization binding is unspecified.** `SECURITY.md` requires approval-to-execution binding (expiry/revocation) but no token or session model is defined. This is the highest security-contract gap.

### P1 — required before Stage 2–4 consumers

5. **Environment sanitizer allow/deny matrix is incomplete.** Design lists PATH/HOME/TEMP/API keys/proxy/locale; no default rules for which secret-bearing variables are stripped vs explicitly passed.
6. **Trust and ownership confidence semantics conflict in form.** Design mixes numeric confidence (`0.72`) with categorical `known/probable/unknown` and trust enums (`Verified/Known/User Trusted/Unknown/Blocked`). Need one meaning and storage representation.
7. **Recognition DB / Tool Definition resource versioning** is required by the design (independent of app/core) but has no format or update channel definition.
8. **Export redaction policy** is referenced (`SECURITY.md`: “reviewed export policy”) but not written.
9. **MCP meta-tool surface** is sketched (`search_tools`, `resolve_capability`, …) but not schema-frozen; risk of accidental per-tool MCP publication is called out but unenforced.
10. **AgentAdapter interface** (detect/launch_discovery_session/generate_mcp_config/…) is listed in the design without a formal trait or discovery-session permission model tied to `PROTOCOL.md`.

### P2 — consistency / packaging notes

11. **14 core systems vs 13 engineering agent roles.** Mapping is implicit; Integration/QA (Agent 13) does not own one of the 14 subsystems. Fine as a staffing map, but batch briefs should name subsystem owners explicitly.
12. **Skill download vs “do not install”.** Skill Center may download skill packages; the invariant forbids installing *tools*. Boundary wording should be tightened so skill payload fetch is not read as tool installation.
13. **Fallback error shape** is an example JSON only; needs a stable enum (`unavailable`, `daemon_error`, …) in the protocol error model.
14. **No hosted Git remote** remains true; delivery path is workbench bundle until the owner provides a repository or authorizes one.

## Suggested first foundation batch (Stage 1)

Not authorized to implement until the owner target checklist and Codex batch brief arrive. Proposed boundaries:

| Item | Proposal |
| --- | --- |
| Goal | Freeze shared domain + protocol contracts and first registry persistence |
| Allowed dirs | `schemas/**`, `crates/protocol/**`, `crates/registry/**`, `docs/architecture/**`, `docs/protocol/**`, `docs/security/**` (or agreed monorepo equivalents) |
| Read-only deps | Existing baseline docs; no scanner/executor/UI |
| Out of scope | Discovery execution, desktop UI, MCP serve, agent adapters, skills engine, any product install/execute path |
| Review gates | (1) capability taxonomy + ID formats signed off; (2) protocol envelope + error model reviewed; (3) SQLite migrations apply cleanly on empty DB; (4) registry unit tests pass; (5) security review of authz-binding design notes |
| Verification | Schema validation, migration up/down tests, registry CRUD tests; document that no platform scanner tests exist yet |
| Directory ownership | MiMo integrates; subagents only after schema freeze and non-overlapping paths are assigned |

## Explicit wait state

**Feature development awaits the owner's target checklist and Codex's next batch brief.** This assessment does not authorize coding, dependency installation, merge to `main`, push, release, workbench report submission, completion, or archival.

## Owner summary (≤160)

MiMo 已独立检出并核对基线，完成设计评估：能力命名与协议/授权绑定为 P0 缺口，待目标清单与批次书后进入开发。
