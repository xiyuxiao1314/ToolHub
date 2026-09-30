# ToolHub developer guide (B02)

## Components

| Binary | Role |
| --- | --- |
| `toolhubd` | Shared daemon: registry, scan, resolve, policy, execute, audit |
| `toolhub` | CLI client (`--json` for agents) |
| `toolhub-desktop` | Local nine-page UI shell on 127.0.0.1 |

## Build

```powershell
# MSVC env (vcvars64) + cargo 1.95
cargo build --workspace --release --locked
```

Outputs under `target/release/`.

## Runtime

- Registry: `%USERPROFILE%\.toolhub\registry.sqlite` or `TOOLHUB_REGISTRY`
- Daemon: `TOOLHUBD_BIN` or sibling of `toolhub.exe`
- Shared service: `toolhubd --listen` (user-scoped named pipe)
- Admin for approve / policy Allow: `TOOLHUB_ADMIN=1` (stdio spawn only; pipe peers are never admin)

## SDK

Minimal TypeScript client: `packages/sdk-typescript` (`ToolhubClient` + `nodeStdioTransport`).

## Schemas

`schemas/*.schema.json` are local authoritative wire/manifest contracts (`toolhub.skill/v1`, `toolhub.resource/v1`, `toolhub.update/v1`, `toolhub.report/v1`).

## Safety

- Native discovery needs no AI key; unknown executables are never auto-run.
- ToolHub does not install/uninstall discovered tools or edit PATH.
- Temporary AI keys stay in process memory (`temp_credentials`).
- Exports redact home/user/secrets by default.
