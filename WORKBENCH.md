# Workbench collaboration

- Task: `task_a11c48d272163020b1ee37f1`
- [Open ToolHub project](https://agent-workbench-hk.tail7d7b36.ts.net:9443/tasks/task_a11c48d272163020b1ee37f1)
- Local Codex review baseline: `D:\ToolHub`
- Owner-facing summary: 项目文档基线已建立，MiMo 从工作台读取启动任务与产物，目标清单到齐后进入开发。

## Profiles and binary

On this Windows machine, invoke the absolute executable path:

```text
C:\Users\hp\AppData\Local\AgentWorkbench\bin\workbenchctl.exe
```

Codex uses `--profile codex`; MiMo uses `--profile mimo`. Credentials are not included in this repository. A missing or unenrolled profile is a blocker to report, not a reason to reuse another agent's credentials.

## MiMo read sequence

The commands below use MiMo's own enrolled profile. Flags appear before positional task IDs for CLI compatibility.

```powershell
& 'C:\Users\hp\AppData\Local\AgentWorkbench\bin\workbenchctl.exe' --profile mimo join --include-text=true task_a11c48d272163020b1ee37f1
& 'C:\Users\hp\AppData\Local\AgentWorkbench\bin\workbenchctl.exe' --profile mimo sync task_a11c48d272163020b1ee37f1
```

Read the latest `MIMO-000-ONBOARDING.md` and `BOOTSTRAP_MANIFEST.json` text artifacts through the API using their returned artifact IDs. Locate the `ToolHub-bootstrap.bundle` attachment. Download it only when preparing a local checkout; use a new destination path.

```powershell
& 'C:\Users\hp\AppData\Local\AgentWorkbench\bin\workbenchctl.exe' --profile mimo read <artifact-id>
& 'C:\Users\hp\AppData\Local\AgentWorkbench\bin\workbenchctl.exe' --profile mimo download --output <new-bundle-path> <bundle-artifact-id>
git bundle verify <new-bundle-path>
git clone <new-bundle-path> <new-independent-checkout>
```

On another device, use its installed workbench client and authorized local paths. A path in a shared document does not itself distribute files.

## Bootstrap artifacts

- Existing full-design text attachment: the original owner-selected design.
- `MIMO-000-ONBOARDING.md`: the first readable task brief.
- `ToolHub-bootstrap.bundle`: one documentation baseline with Git ancestry; no application code or credentials.
- `BOOTSTRAP_MANIFEST.json`: exact baseline revision, selected file hashes, bundle hash, and verification results.

Artifacts are immutable versions. Use the artifact index and manifest rather than guessing attachment IDs or treating a stale registration snapshot as the latest project state.

## Coordination

Technical events use concise English. Publish one Chinese `summary` after a meaningful handoff, blocker, or delivery. Codex posts batch tasks and review findings; MiMo posts concise progress and selected results within the authorized collaboration workflow. Report submission, project completion, archival, and release publication remain distinct actions with their applicable authorization.
