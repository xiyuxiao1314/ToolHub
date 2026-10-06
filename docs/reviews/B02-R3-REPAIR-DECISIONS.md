# B02-R3 repair decisions

- **Desktop architecture**: shipped Tauri 2 + React + TypeScript (`apps/desktop` + `ui/`), per dispatched requirement; previous SPA substitution is replaced.
- **Policy authority**: any effective-authority weakening (Deny→Ask/Allow, Ask→Allow) is controller-gated; ordinary authenticated clients cannot mint approvals.
- **Approval digests**: SHA-256 over length-prefixed argv/stdin strings (no U+001F ambiguity); empty digests fail closed (legacy invalidation).
- **Scan reconciliation**: only complete Full scans mark Missing; Quick/partial preserve untouched instances.
- **Recognition trust**: Known requires real PE signature (`PE\0\0`) / ELF; junk MZ or basename-only stays Unknown.
- **Version grammar**: invalid constraints reject the whole resolve; `=` requires matching prerelease status.
- **Discovery**: classify requires existing candidate + finite confidence in [0,1].
- **Skills**: public register goes through `SkillManifest::parse_yaml_like` + path containment.
- **Update**: copy failure restores `.bak` previous artifact.
- **Package**: recursive SHA-256SUMS including schemas, ui-dist, sdk-typescript.

Internal: parent integrated; no external per-module review. Codex R4 is the remaining formal round.
