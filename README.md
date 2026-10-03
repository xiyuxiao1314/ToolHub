# ToolHub

ToolHub 是面向 AI Agent 与用户的本机工具基础设施层。

It discovers, identifies, catalogs, resolves, and invokes software already present on a Windows or macOS machine. Native discovery must work without an AI provider. ToolHub is a preferred local runtime; agents may choose other execution environments.

## Current state

The repository now contains the Rust daemon and CLI, a Tauri/React desktop, and automated tests. The current desktop has Tools, Programs, Agents, Environments, Market, Tasks, and Settings pages. Windows development executables are built under `target/debug/`; the daemon executable must remain beside the desktop executable.

The Market currently offers local path discovery and report import. Online template installation, automatic log retention, and agent sandbox selection are not implemented. See [the Windows button audit](docs/reviews/2026-10-01-BUTTON-AUDIT.md) for the current local verification and fixes. The original two-batch plan and design remain historical product targets rather than completed-feature claims.

- Local project: `D:\ToolHub`
- Initial branch: `main`
- Primary developer and subagent coordinator: MiMo
- Task planner and independent reviewer: Codex
- Product scope and priorities: project owner
- Collaboration: [ToolHub workbench task](https://agent-workbench-hk.tail7d7b36.ts.net:9443/tasks/task_a11c48d272163020b1ee37f1)

## Read first

1. [Original complete design](docs/design/TOOLHUB_SYSTEM_DESIGN.md)
2. [Agent collaboration rules](AGENTS.md)
3. [Architecture](ARCHITECTURE.md), [domain](DOMAIN.md), [protocol](PROTOCOL.md), and [security](SECURITY.md)
4. [Contribution and review process](CONTRIBUTING.md)
5. [Dependency roadmap](docs/ROADMAP.md)
6. [Workbench access](WORKBENCH.md) and [MiMo onboarding task](docs/tasks/MIMO-000-ONBOARDING.md)
7. [Two-batch implementation plan](docs/superpowers/plans/2026-09-29-toolhub-two-batch-plan.md), [four-round review policy](docs/collaboration/REVIEW_POLICY.md), and [B01 core task](docs/tasks/MIMO-B01-CORE.md)

B01 builds the integrated runnable core and agent-facing interfaces. B02 builds the complete real-data desktop, AI/extension/resource workflows and local release candidates. MiMo handles internal delegation, integration, review and repair; Codex has two formal reviews per batch, four total.

The original design is a product baseline, not an assertion that its schemas, interfaces, or examples have been finalized. Record proposed corrections and unresolved contracts explicitly.

## Product boundary

ToolHub discovers and uses existing tools. It does not become a package manager, system cleaner, sandbox provider, model provider, IDE, or general-purpose agent.

No remote Git repository is configured by this bootstrap. Git history and selected documents can be transferred through the workbench bootstrap bundle.

## 程序启动入口

桌面“程序”页管理用户选择的 `.exe`、`.bat`、`.cmd`、`.lnk`、`.ps1` 和命令入口。手动添加先通过 Windows 文件/目录选择器选中，再填写名称和参数；扫描候选必须勾选并确认保存后才加入列表。命令使用 CMD，工作目录来自目录选择器；参数逐行填写，快捷方式使用自己的目标和参数。

默认扫描所有可用的本地固定/可移动磁盘，过滤系统、AppData、已登记安装目录、依赖和编译中间文件。保留项目的 dist/build/target 主入口，读取 package.json 的 start/dev/serve/preview 脚本与常见 Python 入口。标记和文件名只能提供候选依据，不能证明由 AI 创建。指定目录可缩小扫描范围；扫描有取消、分页和范围限制提示，候选不会自动运行或添加。

程序条目保存在共享 SQLite 中，与工具安装记录分开。启动是经验证的本机控制器发起的用户操作，不授予普通 Agent 新的执行权限。启动成功提示表示已提交进程创建请求；应用内部的启动报错需查看其窗口。移除只删除列表条目。Windows 已实现；其他平台的原生选择器尚未实现。
