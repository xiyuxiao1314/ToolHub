# B02 integrated decisions (interim)

## Contracts

- Capability IDs: dotted lowercase; video frame extraction = `media.video.frame.extract` (aliases only for legacy).
- JSON-RPC 2.0; notifications (no id) get no response.
- Resource/Update/Skill/Report schemas: `toolhub.resource/v1`, `toolhub.update/v1`, `toolhub.skill/v1`, `toolhub.report/v1`.

## Security

- Execution approval binds instance, exe hash, args/cwd/stdin/env digests, agent, session (non-empty), and policy/trust digest at mint.
- Admin (`execute.approve`, `policy.set` Allow) is NOT granted to named-pipe connection principals; only `local.admin*` or `TOOLHUB_ADMIN=1` on local stdio spawn.
- Per-connection principal `pipe.conn.*` isolates discovery sessions on shared pipes.
- Known trust requires executable metadata (MZ/ELF + size), not basename/path alone.
- Temporary AI keys: process memory only.

## Persistence

- SQLite migrations from empty; forward-only.
- Scan identity: path-normalized; upsert returns stored id; blocked/user_trusted trust preserved across rescans.
- Foreign report import paths become `missing`/`unknown` only.

## Desktop

- B02 desktop is a local loopback SPA shell (`toolhub-desktop`) talking to the same daemon RPC — documented equivalent to Tauri packaging for this RC; native Tauri packaging remains B02-18 residual if webview toolchain is unavailable.

## Internal review

Three subagent reviews (security / scan-trust / protocol) informed P1 fixes at 380b34f..e46dccc. Residual: OS-level peer SID ACL, full cancel method, macOS host.
