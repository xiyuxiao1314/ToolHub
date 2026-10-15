# ToolHub 存储维护 / Storage maintenance

Windows developer cache and old local-package maintenance, PowerShell 7+ wrapper / Python 3.11+ standard library only. Build-cache inspection also requires Git to detect tracked source. This is an independent maintenance command, not an Agent MCP permission or an artifact-cleanup shortcut. It does not change the running installation, user registry, global Cargo/npm caches or project source.

## 预览与执行

```powershell
# Defaults to a preview; running this alone deletes nothing.
& .\scripts\maintenance.ps1

# Only the repo's Cargo target. Permanently removes reproducible build/test outputs.
& .\scripts\maintenance.ps1 -Scope build -Apply

# Old local package copies and installed versions go to Windows Recycle Bin.
& .\scripts\maintenance.ps1 -Scope packages -Apply
& .\scripts\maintenance.ps1 -Scope versions -Apply

# All three scopes, machine-readable audit report.
& .\scripts\maintenance.ps1 -Apply -Report 'D:\maintenance-result.json'
```

Direct Python equivalent: `python -I scripts/maintenance.py --apply --report <path>`. Paths with spaces, Unicode, brackets or apostrophes are supported as ordinary arguments. For custom MCP configuration locations, add `-ReferenceFile 'D:\custom\mcp.json'` (Python: repeat `--reference-file`). The known common host configs, registered tool/program/Skill/artifact paths, running executables/command lines, ToolHub shortcuts and startup actions are automatically inspected. Arbitrary project configs elsewhere on disk are not exhaustively scanned.

## 保留规则

- Keep the newest two valid packages in each local package root, the active build, the immediately preceding active build and all referenced builds. More than two can remain when still referenced. `-Keep` may increase this number; values below two are rejected.
- The only permanent-delete candidate is the literal `target` directory in a verified ToolHub Cargo workspace, with a Cargo cache marker and no tracked files. No arbitrary cleanup paths or broad temp/home scans are accepted. A running Cargo/rustc process blocks build-cache cleanup.
- Package candidates require the expected package schema, build identifier, checksum manifest and three executable files. Unknown folders and `dist/release` public-release archives are kept. Each mutation rechecks references, package metadata, path boundaries and nested links/reparse points. Linked trees are skipped. Failed inspection stops cleanup; failed recycling never falls back to permanent deletion.
- Old packages use Windows IFileOperation with RECYCLEONDELETE and WANTNUKEWARNING; nothing empties the Recycle Bin. Recycled-package bytes are **not** reported as physically freed disk space. Build caches are generated data and are deleted directly to reclaim space.
- Registry databases, user programs, Skills, artifact packages, configuration, native evidence and backup directories are not cleanup targets. A cleanup report contains candidate paths/results, not host credentials or complete configuration text.

## 防止再次膨胀

Everyday dev/test Cargo profiles disable full debug symbols and incremental output. This trades faster repeated compilation/debugger information for lower disk use. To debug deliberately, opt in for that build:

```powershell
$env:CARGO_PROFILE_DEV_DEBUG='2'
$env:CARGO_PROFILE_TEST_DEBUG='2'
$env:CARGO_INCREMENTAL='1'
# Run the intended cargo command; remove these overrides when done.
```

Windows package scripts ship the maintenance command and guide. Successful local packaging also recycles unreferenced older package copies; `package.ps1 -KeepOldPackages` opts out. New local installers attempt old-version recycling after successful activation; the main install is already complete, and missing Python, refused inspection or recycle errors are nonfatal warnings. `install-local.ps1 -KeepOldVersions` keeps every version. Build-cache cleanup remains explicit; no background timer or deletion on ordinary app startup is installed. Existing installed binaries are not overwritten or rebuilt just to clean storage.

Clearing target means the next development build recompiles dependencies; desktop shortcuts and MCP use the separately installed version and keep working. The retained executable set must stay together. Maintenance currently performs native process/reference/recycle checks on Windows only; macOS maintenance is not implemented by this script.

## 验证

`python -m unittest discover -s scripts/tests -p test_maintenance.py -v` checks preview-only behavior, active/current/rollback retention, referenced old versions, registered files, changed live context and manifests, in-progress builds, tracked source, junction escape and failed-recycle preservation. A native disposable-directory smoke check verifies that the Shell operation increases Recycle Bin item count.

Microsoft flag reference: https://learn.microsoft.com/en-us/windows/win32/api/shobjidl_core/nf-shobjidl_core-ifileoperation-setoperationflags

To repack an existing verified executable set without rebuilding the cleared cache, pass `package.ps1 -BinaryDirectory <verified-package-directory> -BuildId <fresh-id>`. This does not rebuild or change native functionality; keep the executable trio from one build.
