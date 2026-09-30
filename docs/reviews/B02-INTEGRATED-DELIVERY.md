# B02 integrated delivery (in progress — not R3)

- Branch: `codex/toolhub-b02-integrated`
- Adopted Codex base: `6df2166` (R2 policy atomic replace)
- Current HEAD: *(stamped at final packet)*
- Review budget: R1/R2 used (2/4); R3 reserved for complete integrated product

## Scope completed (summary)

R2 core hardening (authority, trust MZ gate, scan identity, session principal, skill safety, protocol notifications, CLI exits), B02-12 temp credentials, B02-13 extensions, B02-14 resources, B02-15 update verify/apply, B02-18 local RC packaging script, B02-21 TS SDK + developer docs, desktop nine-page local shell, integration tests.

## Verification (this host)

- cargo fmt/clippy -D warnings/test --workspace --locked/build --release --locked
- scripts/smoke.ps1 (approve/exec/replay + negatives)
- scripts/package.ps1 produces dist/toolhub-0.2.0 with SHA256SUMS.txt

## Open toward R3

- Native Tauri packaging/screenshots (local SPA shell is current desktop)
- Long-lived multi-client named-pipe soak under load
- macOS host evidence
- Full MCP conformance fixture suite
- B02-11 real external agent launch

## Chinese owner summary

B02 核心加固与打包脚本已落地，测试与冒烟通过；Tauri 打包/macOS/长驻 soak 仍开放，未达 R3。
