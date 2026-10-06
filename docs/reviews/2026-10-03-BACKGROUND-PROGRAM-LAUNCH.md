# Background program launch — 2026-10-03

The Programs page previously opened a new console for commands/scripts and deliberately kept CMD `/K` or PowerShell `-NoExit` alive. This left a persistent terminal beside GUI applications launched through npm or a script.

The ordinary **启动** action now defaults to background execution on Windows. Console executables and script wrappers use `CREATE_NO_WINDOW`; CMD uses `/C`, PowerShell does not use `-NoExit`, and completed background scripts do not open a keep-alive shell. GUI executables still show their application windows. **终端启动** explicitly requests the previous interactive behavior for input and output. Shortcuts retain their target configuration. Existing entries need no migration or reselection.

`program.launch {id,terminal?:boolean}` remains restricted to an OS-verified controller. Omitted/false `terminal` is background mode; wrong types are rejected. The result includes `{pid,submitted:true,terminal}`; submission does not imply application readiness. Stored arguments, selected paths, working directories and the sanitized environment remain in use. Background standard streams are disconnected; use terminal mode when output or interaction is needed.

Native testing also found an existing CMD quoting problem for user commands beginning with a quoted executable path. The command is now enclosed in an outer pair with `/S`, preserving the user's inner quotes and shell syntax.

Validation:

- TypeScript/Vite production build, desktop/daemon builds, and affected all-target Clippy checks with `-D warnings` passed using locked offline Rust dependencies.
- Initial daemon/protocol checks passed 22 unit tests and 10 integration/managed/soak tests. After the CMD correction, all 14 daemon unit tests, daemon build and Clippy passed again.
- Final real Tauri/WebView verification passed all 10 checks: console EXE, BAT, CMD and PS1 have no console; command shells and helpers exit; GUI EXE remains visible; an npm/Node/Electron chain opens a visible window without a console; explicit terminal mode retains a console; invalid mode values are rejected; both UI actions and 390/640/1180 layouts pass. No page errors were recorded.
- The native fixtures and database are isolated under `D:\codex\toolhub-background`; final evidence is `output/native-results.json`. The first failed run is preserved separately. No fixtures were added to the normal registry.
- The user's running Pika process was preserved. Its occupied old helper executable was renamed within the existing debug directory so the new binary could be built. Already-running launches keep their previous mode; the change applies to new launch requests.

Native behavior was verified on Windows. macOS terminal behavior was not tested.

Windows flag semantics follow [Microsoft process creation flags](https://learn.microsoft.com/en-us/windows/win32/procthread/process-creation-flags); command lifetime and outer quoting follow [Microsoft CMD documentation](https://learn.microsoft.com/en-us/windows-server/administration/windows-commands/cmd).
