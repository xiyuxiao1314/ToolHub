# B02-R3 repair delivery — R4 packet

- Task: `task_a11c48d272163020b1ee37f1`
- Branch: `codex/toolhub-b02-r3-repair`
- R3 reviewed base: `b62604095b248e7c13d44cfdfba8e789c0503c89`
- **Repair HEAD: tip of branch published in `ToolHub-B02-R3-repair.bundle`** (see `B02-R3-REPAIR-MANIFEST.json`)
- Adopted ancestor: `6df2166` policy atomic-replace preserved

## What changed (R3-F01–F15)

| ID | Outcome |
| --- | --- |
| F01 | Policy Deny→Ask / Ask→Allow / Deny→Allow all require controller; ordinary clients cannot weaken |
| F02 | Tauri/React/TS desktop (`apps/desktop`) talks to shared daemon RPC; per-connection pipe principal |
| F03 | Length-prefixed argv digests; fail-closed empty digests; policy/trust digest bind |
| F04 | Ownership-checked `execute.cancel`; nonexistent cancel is truthful |
| F05 | Missing reconciliation only on complete **Full** scans |
| F06 | PE `PE\0\0` signature required for Known (junk MZ stays unknown) |
| F07 | Invalid version constraints reject; Eq requires matching prerelease status |
| F08 | Strict finite confidence + candidate existence required for classify |
| F09 | Skill registration through authoritative `SkillManifest` validation |
| F10 | (existing) MCP conformance suite retained |
| F11 | **Tauri 2 + React/TS nine-page UI** (`ui/`), typecheck+build pass, release desktop binary |
| F12–F13 | Temp creds + update rollback retained/extended |
| F14 | Export `schema: toolhub.report/v1` |
| F15 | Recursive package checksums + UI dist + SDK + schemas |

## Verification

| Check | Result |
| --- | --- |
| cargo fmt / clippy -D warnings | pass |
| cargo test --workspace --locked | pass |
| cargo build --workspace --release | pass (incl. Tauri desktop) |
| npm typecheck + vite build (ui) | pass |
| scripts/package.ps1 | pass recursive SHA256SUMS |
| scripts/smoke.ps1 | pass |

## Limitations

- macOS host still unverified-host.
- Real third-party agent account launch still unverified (owned fixture path only).
- Full UI screenshot evidence will be attached as available on this host.

## Chinese owner summary

R3 修复已提交：策略削弱拦截、审批绑定、扫描/识别/技能加固，并落地 Tauri 九页桌面与递归打包，待 R4。
