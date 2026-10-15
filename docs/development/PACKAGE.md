# ToolHub local Windows package

This local distribution contains optimized native toolhub-desktop.exe, toolhubd.exe and toolhub.exe from one build, schemas and the Skill library. It is unsigned; no public release or installer signing is implied.

Build the frontend, then cargo build --release -p toolhub-cli -p toolhub-daemon -p toolhub-desktop --locked. Run scripts/package.ps1 with a fresh BuildId. Packages are never overwritten. The separate maintenance tool can recycle obsolete packages while preserving active, referenced and rollback builds.

Inside the resulting directory, run install-local.ps1 -VerifyOnly to check all files. install-local.ps1 installs under the current user's LOCALAPPDATA/ToolHub/versions/build-id, updates only the existing ToolHub login task, creates a ToolHub desktop shortcut and activates current.json. Existing user data in USERPROFILE/.toolhub remains. Startup stays at its existing enabled/disabled choice. The optional -UpdateCodexMcp switch changes only the ToolHub command/args section and preserves all other host configuration. Prior configuration/action metadata are retained; failed activation restores task/config.

Launch the desktop shortcut normally. After changing an MCP executable path, reload the host's MCP connection or start a new conversation; existing sessions may retain the previous server process and tool list. The desktop's Agent page can test the packaged MCP server independently, but this does not prove the host has reloaded.

Keep all three executable files together. Desktop identity pinning must be renewed with its paired daemon when upgrading; do not mix builds or disable controller verification.

New Windows packages include maintenance.ps1/maintenance.py (Python 3.11+ standard library). After successful activation the installer performs best-effort old-version recycling; lack of Python or an inspection/recycle failure does not fail the install. Use -KeepOldVersions to skip that step. See [storage maintenance](STORAGE.md).

Successful local packaging prunes unreferenced older package copies using the same conservative recycle policy; -KeepOldPackages opts out. Package creation remains successful if maintenance is unavailable.
