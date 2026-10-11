## 连接与检查

本流程适用于支持本地 stdio MCP 的客户端，不限定 Codex 或 MiMo。Claude Code、Claude Desktop、Cursor、Gemini CLI、Copilot 使用的配置通常为 mcpServers；VS Code 用户格式为 servers；OpenCode / MiMo 使用 mcp 的本地 command 数组；Codex 使用 TOML 的 mcp_servers。以当前宿主文档和实际配置为准，保留其他服务、凭据和权限。ToolHub 接入说明提供各客户端模板。没有 Skills 功能时将本文用作任务指引，不复制其他宿主的专属 Skills。

使用本机连接信息中的 CLI 绝对路径，不假定 `toolhub.exe` 在 PATH。MCP 使用 stdio，command 是可执行文件，args 是 `["mcp", "serve"]`。按宿主格式合并配置并保留其他服务。CLI 和配对 daemon 放在同一目录（Windows 为 `toolhub.exe` / `toolhubd.exe`，macOS 为 `toolhub` / `toolhubd`）。桌面用户使用桌面包内 CLI；Mac 的标准路径是 `/Applications/ToolHub.app/Contents/MacOS/toolhub`。桌面无需一直打开。

MiMo / mimocode 使用用户目录下 `.config/mimocode/mimocode.jsonc` 的 `mcp.toolhub`：`{"type":"local","command":["<本机 CLI 绝对路径>","mcp","serve"],"enabled":true}`。它的 command 是数组，不使用通用 JSON 的 command 字符串与 args 分离格式；合并时保留 provider 和其他 MCP 服务。ToolHub 列表分别显示本机检测、配置与历史握手/调用；客户端自报名称只是显示记录，不授予权限。

配置接入时，完成 initialize / initialized，再检查 tools/list 和一次 search_tools。网关提供十六个固定元工具：`search_tools`、`resolve_capability`、`inspect_tool`、`list_environments`、`execute_tool`、`search_skills`、`inspect_skill`、`propose_program`、`search_programs`、`request_execution_approval`、`get_task`、`cancel_task`、`register_artifact_package`、`list_artifact_packages`、`complete_artifact_package`、`review_artifact_package`。配置已写入、服务已连接和工具可执行是三个不同状态；报告实测状态。

无 MCP 时，将 `$toolhub` 设为本机 CLI 路径，使用结构化查询：

```powershell
& $toolhub --json status
& $toolhub --json search ffmpeg
& $toolhub --json env list
& $toolhub --json skill list
```

CLI 不提供 propose_program；缺少 MCP 时向用户提供入口信息，不能直接写注册表数据库。
