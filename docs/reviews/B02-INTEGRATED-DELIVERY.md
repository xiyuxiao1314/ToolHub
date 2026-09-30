# B02 integrated delivery — R3 packet

- Workbench task: `task_a11c48d272163020b1ee37f1`
- Branch: `codex/toolhub-b02-integrated`
- Base adopted: `6df2166e0c55ec64667021e386e75ca93acb066d` (Codex R2 bounded policy fix)
- Planning ancestor: `7a1ba39` / `46a06aa`
- **Delivery HEAD: the tip of `codex/toolhub-b02-integrated` published in `ToolHub-B02.bundle`** (exact SHA recorded in `B02-PACKET-MANIFEST.json`)
- Clean status: yes (this packet commit included)

## Outcome

One integrated implementation of unaccepted B01 core closure + B02 product surfaces on a single branch, with internal subagent security/scan/protocol reviews and repairs before handoff. This packet is for **R3** only (reviews used: R1/R2 = 2/4).

## Verification (Windows x64, rustc/cargo 1.95.0, MSVC 14.44)

| Command | Result |
| --- | --- |
| `cargo fmt --all -- --check` | pass (formatted via fmt) |
| `cargo clippy --workspace --all-targets -- -D warnings` | pass |
| `cargo test --workspace --locked` | **all pass** (unit + integration + soak + MCP) |
| `cargo build --workspace --release --locked` | pass (`toolhubd`/`toolhub`/`toolhub-desktop`) |
| `scripts/smoke.ps1` | pass (scan/resolve/export/MCP/approve/exec/replay/negatives) |
| `scripts/package.ps1` | pass (`dist/toolhub-0.2.0` + `SHA256SUMS.txt`) |

### Test evidence highlights

- **Daemon soak** (`apps/daemon/tests/soak.rs`): 50 mixed requests, scan/query interleave, invalid-input recovery, restart durability — 4/4.
- **MCP conformance** (`apps/cli/tests/mcp_conformance.rs`): initialize+capabilities, notification silence, tools/list object shape, tools/call + error recovery, string `error.message`, unsupported-method recovery — 6/6.
- **Integration** (`apps/daemon/tests/integration.rs`): shared registry multi-client, non-admin approve denial, invalid JSON-RPC — 4/4.
- **Security unit**: approval consume/replay, timeout kill, redaction, MZ/ELF trust gate, policy atomic replace (Codex C01/C02 preserved).

## Architecture / security decisions

See `B02-INTEGRATED-DECISIONS.md`. Summary: per-connection pipe principals; admin not inherited by pipe peers; approval binds instance/exe/args/cwd/stdin/env + policy/trust digest; Known trust requires MZ/ELF+size; temp credentials memory-only; report import paths stay untrusted.

## User-visible flows

- Native no-key scan → recognized tools vs unknown candidates
- CLI `--json` scan/search/resolve/exec/approve/policy/skill/discovery/export
- Desktop nine-page shell (`toolhub-desktop`) on 127.0.0.1 against same daemon RPC
- MCP meta-tools for agent clients

## Limitations (honest)

- Native Tauri packaging/screenshots not produced (local SPA desktop shell is current UI surface).
- macOS host compile/run **unverified-host** (no authorized macOS environment).
- Real external agent session launch (OpenCode/Claude) not executed (no account/runtime authorization).
- Named-pipe peer is per-connection principal, not OS SID ACL.

## Chinese owner summary

R3 交付包已定稿：核心加固、soak/MCP 证据、打包与文档齐备，HEAD 706fcc5，待 Codex 认证。
