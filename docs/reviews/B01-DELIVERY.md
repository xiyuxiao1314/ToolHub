# B01 delivery — runnable ToolHub core

- Branch: `codex/toolhub-b01-core`
- Base: `6ffc9dc3c2fb526e740f0e3b29eafb84f4c54fce`
- Head: *(this delivery commit)*
- Workbench task: `task_a11c48d272163020b1ee37f1`
- Checkout: `D:\ToolHub-mimo\checkout`
- Platform: Windows x64 (host smoke). macOS compile-only.

## Outcome

Integrated Rust workspace implementing native discovery → candidate → recognition → registry → capability resolve → policy → execution → audit, plus daemon/CLI/MCP/discovery sessions/skills against one SQLite registry. Release binaries:

- `target/release/toolhubd.exe`
- `target/release/toolhub.exe`

## Toolchain and verification commands

Environment: `vcvars64.bat` (VS Build Tools 2022, MSVC 14.44.35207, Windows SDK 10.0.26100); `cargo`/`rustc` 1.95.0 at `C:\Users\hp\.cargo\bin`. No local proxy used for install (owner constraint: avoid `:7897`).

| Command | Result |
| --- | --- |
| `cargo fmt --all -- --check` | pass (formatted via `cargo fmt --all`) |
| `cargo clippy --workspace --all-targets -- -D warnings` | pass |
| `cargo test --workspace --locked` | **41 passed**, 0 failed |
| `cargo build --workspace --release --locked` | pass |
| `git diff --check` | pass |
| `powershell -File scripts/smoke.ps1` | pass (see coverage) |

## Internal review findings and repairs

1. Daemon logs polluted stdout JSON-RPC → tracing to stderr (repair).
2. Discovery sessions lost across CLI process spawns → load/persist from SQLite (repair).
3. Environment kind overwritten to `unknown` by scan ingest → insert-if-missing only (repair).
4. Clippy `too_many_arguments` on registry/executor/audit constructors → explicit allow (API shape intentional).
5. MSVC linker initially missing → owner-authorized VS Build Tools install without proxy (host blocker closed).

## Host smoke highlights (2026-09-29)

- Quick scan: 1123 candidates, **14 recognized** (Python×5, Node, Java×3, Rust×2, Git, FFmpeg, Clang), 1109 unknown preserved.
- `resolve language.python.execute` / `media.video.frame.extract` (alias) return deterministic selection + alternatives.
- Export redacts home/user (`~` paths); secrets/args redaction unit-tested.
- Agents detected: `codex` on PATH; discovery session usable across CLI calls.

## Limitations / unverified

- macOS host behavior not run (providers present).
- Windows named-pipe multi-client long-lived daemon not hardened; B01 supported transport is stdio JSON-RPC with durable registry file.
- Process-tree cancellation and approval TOCTOU races need a dedicated hardening pass.
- Incremental scan is rescan-based, not a FS watcher.
- No CI (no hosted remote). No desktop UI (B02). No signed release.

## Reproduce

```powershell
# after vcvars64 + cargo on PATH
cd D:\ToolHub-mimo\checkout
cargo test --workspace --locked
cargo build --workspace --release --locked
powershell -ExecutionPolicy Bypass -File scripts\smoke.ps1
```

## Chinese owner summary (≤160)

B01 可运行核心已交付：扫描识别、注册表、解析、策略执行与 CLI/MCP/会话已打通并通过 41 项测试与本机冒烟，待 Codex 初审。
