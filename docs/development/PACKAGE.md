# ToolHub B02 local Windows package (unsigned RC)

## Contents

- `toolhubd.exe`, `toolhub.exe`, `toolhub-desktop.exe`
- `schemas/`
- `resources/` (recognition packages, if present)
- This README + `UNINSTALL.md`

## Checksums

After build, generate SHA-256 for each binary and record in `SHA256SUMS.txt`.

## Start

```powershell
$env:TOOLHUB_REGISTRY = "$env:USERPROFILE\.toolhub\registry.sqlite"
.\toolhubd.exe          # optional long-lived; CLI also works standalone
.\toolhub.exe --json status
.\toolhub-desktop.exe   # UI on 127.0.0.1:18765
```

## Uninstall

See `UNINSTALL.md`. Do not uninstall discovered third-party tools via ToolHub.
