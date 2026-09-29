# B01-R1 repair delivery

- Review base: `46a06aa11c370b2ca4432909aa2b8b0a70127090`
- Repair branch: `codex/toolhub-b01-core`
- Repair HEAD: *(see bundle)*
- Task: `task_a11c48d272163020b1ee37f1`

## What changed

Security-critical execution path, process bounds, redaction, policy persistence, recognition trust separation, resolver eligibility, MCP/CLI error contracts, and honest smoke negatives. See `B01-R1-REPAIR-MAP.md`.

## Verification

| Command | Result |
| --- | --- |
| cargo fmt --all -- --check | pass |
| cargo clippy --workspace --all-targets -- -D warnings | pass |
| cargo test --workspace --locked | 47 passed, 0 failed |
| cargo build --workspace --release --locked | pass |
| scripts/smoke.ps1 | pass (includes failing negative) |

Toolchain: rustc/cargo 1.95.0 + VS Build Tools MSVC 14.44, Windows x64. No proxy used for install.

## Limitations (honest)

Named-pipe multi-client daemon, full native Registry providers, complete skill registration, discovery candidate isolation, and macOS host runs remain open (see map). These are **not** claimed as Done.

## Chinese owner summary

R1 修复已提交：审批绑定、超时/输出边界、脱敏与 MCP/CLI 错误契约已加固，47 测试与加固冒烟通过；部分扫描/技能/管道项仍开放，待 R2。
