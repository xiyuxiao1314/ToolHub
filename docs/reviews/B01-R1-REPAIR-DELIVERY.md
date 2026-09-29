# B01-R1 repair delivery (R2-ready)

- Formal review base: `46a06aa11c370b2ca4432909aa2b8b0a70127090`
- Branch: `codex/toolhub-b01-core`
- **Delivery HEAD: bbaa4868c0d2088919ccb9055f74383bee0db5f8
- Task: `task_a11c48d272163020b1ee37f1`

## Scope completed

All F01–F18 acceptance items addressed in source with automated evidence; macOS host and external agent launch recorded as unverified-host (not missing source). See `B01-R1-REPAIR-MAP.md` and `B01-COVERAGE.md`.

## Verification

| Command | Result |
| --- | --- |
| cargo fmt --all -- --check | pass |
| cargo clippy --workspace --all-targets -- -D warnings | pass |
| cargo test --workspace --locked | all pass |
| cargo build --workspace --release --locked | pass |
| scripts/smoke.ps1 | pass with negatives |

Toolchain: rustc/cargo 1.95.0 + MSVC 14.44 (owner-authorized, no local proxy).

## Chinese owner summary

开放项已补齐并统一提交：扫描身份/注册表 Provider/环境图/管道/Schema/会话/技能/适配器均落地，验证通过，待 R2。
