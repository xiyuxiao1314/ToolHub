# ToolHub B01 — local startup and smoke

## Build (Windows)

```powershell
# VS Build Tools C++ required (MSVC + Windows SDK)
# Use an MSVC environment (vcvars64.bat) or a developer prompt
cd D:\ToolHub-mimo\checkout
C:\Users\hp\.cargo\bin\cargo.exe build --workspace --release --locked
```

Binaries: `target\release\toolhub.exe`, `target\release\toolhubd.exe`.

## Runtime

- Registry DB: `%USERPROFILE%\.toolhub\registry.sqlite` (override with `TOOLHUB_REGISTRY`).
- Daemon binary: `TOOLHUBD_BIN` or sibling of `toolhub.exe`.
- CLI always talks JSON-RPC to `toolhubd` over stdio; state is durable in SQLite.
- Native discovery works with **no AI API key**. Unknown executables are never auto-run.

## Smoke

```powershell
powershell -ExecutionPolicy Bypass -File scripts\smoke.ps1
```

Expected: status JSON; quick scan counts; search hits for known tools; resolve with explanation; discovery session listed; export with `~` redacted paths; MCP meta-tool names.

## Safety invariants

- ToolHub does not install/uninstall/upgrade tools or modify PATH.
- Policy default for unknown tools is `ask`; `rm`/`del` denied.
- Secret env vars are not passed to child processes by default.
- Temporary AI credentials are never persisted.
