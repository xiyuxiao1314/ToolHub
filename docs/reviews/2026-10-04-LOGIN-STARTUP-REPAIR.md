# Windows login startup repair

Implemented and verified on Windows, 2026-10-04. Branch `codex/button-audit-20261001`; base `fba33201197c5f892747e06713c750437172932c`. Delivered in the existing working tree, preserving earlier desktop, Programs, Agent/MCP and utility-discovery changes. No commit, push or release was made by this repair.

## Findings and limits

The user's HKCU Run value existed, contained the correctly quoted current desktop EXE, and referenced an existing local file. No ToolHub-specific StartupApproved disable value, application crash event, WER report or compatibility elevation flag was found in the inspected locations. Launching with System32 as the working directory and Machine + User PATH opened the application successfully.

These observations do not establish the cause of the earlier missed login launch. The old switch checked only Run registration and recorded neither Windows launch attempts nor desktop startup milestones. Its enabled state could not prove that an application window had opened. The historical failure remains unverified; Fast Startup being enabled is not evidence that it caused the failure.

## Change

- Replaced new login-startup registrations with one native Windows Task Scheduler task per actual current-user SID. It uses InteractiveToken and LeastPrivilege, without a password or elevated run level. The task action separates the absolute EXE, `--autostart` argument and explicit EXE-parent working directory. A current-user logon trigger waits 15 seconds; failure retries are one minute apart, up to three times. Battery operation is allowed; desktop runtime is unlimited; duplicate task instances are ignored.
- Enabling creates and validates the task before removing only ToolHub's old HKCU Run value. A scheduler failure leaves that old registration intact. Disabling removes only the owned task and legacy ToolHub value. Task ownership is checked before replacement/deletion. Reads do not migrate or enable settings. Existing legacy registrations have an explicit repair button; this user's already-enabled choice was migrated through that button.
- Settings reads actual task Enabled state, validates action/principal/logon-trigger configuration, and displays the last task result. A disabled task shows off. Read/write failures cannot produce a successful toggle notification. Windows normalizes principal and trigger identities differently, so native account lookup compares their SIDs. LastRunTime is rendered as Windows local wall time; the positive never-run sentinel is suppressed.
- Login launches retain a bounded local `startup-last.json` under `%LOCALAPPDATA%\ToolHub`: process started, application window created, or startup failed. It stores time/PID/stage and a bounded application error, without arguments or environment values. Logging is best effort and cannot block startup. This records window creation, not full renderer/service readiness.
- The desktop uses Windows GUI subsystem in debug builds as well as release builds, removing the desktop console window. No daemon, MCP, program-launch permission, registry schema or tool execution policy was changed.

The Windows mechanism follows [Microsoft's logon-trigger task registration example](https://learn.microsoft.com/en-us/windows/win32/taskschd/logon-trigger-example--xml-). The former mechanism was the [current-user Run key](https://learn.microsoft.com/en-us/windows/win32/setupapi/run-and-runonce-registry-keys).

## Verification

- TypeScript and production frontend build passed.
- `cargo test -p toolhub-desktop --locked --offline -j 4`: nine tests passed. Native tests create only unique disabled fixture tasks; verify Unicode/spaces/ampersand/apostrophe path roundtrips, current-user principal and trigger, retry settings, disabled state, cleanup, preservation of another task, and refusal to replace/delete a foreign description. Registry tests use an isolated HKCU test key. Task date conversion and never-run handling are covered. Fixture tasks left by initial failed identity assertions were removed after the correction; none of those tasks was executed.
- `cargo build -p toolhub-desktop --locked --offline -j 4` and desktop all-target Clippy with `-D warnings` passed. PE header confirmed GUI subsystem 2.
- Actual Tauri WebView/Playwright: read the enabled legacy registration, clicked repair, verified enabled task backend and matching configuration, refreshed status, and observed no JavaScript errors. The final native settings check matched Windows' actual local LastRunTime and the recorded window-created diagnostic. Test browser profiles and registry were separate from user data.
- Actual Task Scheduler Run: the final desktop EXE opened a ToolHub window, remained running, and wrote window-created diagnostics. The host token was not elevated, task run level was least privilege, and no cmd/PowerShell/conhost child was created. Task state 4 / `0x00041301` means it was still running, not a failure. The real current-user task is enabled and the old ToolHub Run value is absent. This checks immediate task execution; it does not exercise a new Windows logon or its 15-second delay.
- CLI status/doctor from System32 succeeded against the original user registry, reporting 44 tools and passing its registry/public-listener/native-discovery checks. The final production desktop was left open. No program entries were edited by the repair.
- `git diff --check` passed. Changes for this request are limited to desktop startup modules, the startup setting, desktop Windows feature flags, main entrypoint and documentation. Earlier working-tree changes remain intact.

Evidence: `D:\codex\toolhub-autostart-20261004\build-results.txt`, `login-reproduction.json`, `native-ui.json`, `native-final-ui.json`, `settings-repaired.png`, `registered-task.xml`, `task-launch-result.json`, and the backed-up original startup registration. The next real Windows login remains for the user to verify; if it fails, Settings and the startup diagnostic now provide additional evidence.


## Follow-up: production desktop controller after rebuild

The owner subsequently reported `denied: verified desktop controller required` on the production Programs page. The still-running production daemon had started during the pre-build login reproduction and pinned the old desktop EXE digest. Rebuilding the desktop did not restart that daemon. Read-only CLI status/doctor continued to pass, but they do not prove desktop controller authorization. The private-registry native UI tests used freshly started test daemons and did not expose this production mismatch. This was a missed post-update check, separate from Windows login task configuration.

Stopped only that identified old managed daemon after checking its executable, `--listen` mode, known parent and absence of application child processes. Started a fresh daemon through the normal managed-service path and refreshed the actual production Programs page. It then displayed the user's three saved entries and no controller-denial banner. No program metadata was edited, and no identity/digest checks were relaxed. The desktop and enabled login task remain running/configured.

Developer update verification must stop/restart the managed daemon after replacing the desktop image, once active work is idle, and verify a controller-only operation such as the actual desktop `program.list`. Public status/doctor checks alone are insufficient. Keep the canonical-path, same-user and pinned-digest checks; do not accept a changed image automatically to hide an update lifecycle error. This recovery was verified on the real production UI using the Computer Use skill.
