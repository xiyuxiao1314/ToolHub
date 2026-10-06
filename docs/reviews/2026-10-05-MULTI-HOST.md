# 多客户端兼容与本机交付 — 2026-10-05

原有 MCP stdio 服务不限定 Codex/MiMo，但默认配置读取只有这两种，且未知 clientInfo.name 被忽略。新增统一宿主目录、默认配置读取、接入模板和任意客户端持久历史。执行策略及桌面控制器验证保持原约定。

## 配置与实际验证范围

| 客户端 | 模板格式 | 默认用户配置或操作位置 | 本机证据 |
|---|---|---|---|
| Codex | toml | ~/.codex/config.toml | 格式夹具通过；检测到安装与 ToolHub 配置 |
| MiMo | local-array | ~/.config/mimocode/mimocode.jsonc | 格式夹具通过；检测到安装与 ToolHub 配置 |
| Claude Code | mcpServers | ~/.claude.json（用户级）；项目 .mcp.json | 格式夹具通过；未安装或未检测到，未做真实宿主会话测试 |
| Claude Desktop | mcpServers | %APPDATA%/Claude/claude_desktop_config.json（Windows） | 格式夹具通过；未安装或未检测到，未做真实宿主会话测试 |
| Cursor | mcpServers | ~/.cursor/mcp.json；项目 .cursor/mcp.json | 格式夹具通过；未安装或未检测到，未做真实宿主会话测试 |
| OpenCode | local-array | ~/.config/opencode/opencode.jsonc 或 opencode.json | 格式夹具通过；检测到安装；默认配置未发现 ToolHub |
| Gemini CLI | mcpServers | ~/.gemini/settings.json | 格式夹具通过；未安装或未检测到，未做真实宿主会话测试 |
| VS Code / Copilot | servers | MCP: Open User Configuration；Windows 默认 %APPDATA%/Code/User/mcp.json | 格式夹具通过；检测到安装；默认配置未发现 ToolHub |
| GitHub Copilot CLI | copilot | ~/.copilot/mcp-config.json | 格式夹具通过；未安装或未检测到，未做真实宿主会话测试 |
| Devin Desktop / Windsurf | mcpServers | %APPDATA%/devin/mcp_config.json（当前 Windows 文档） | 格式夹具通过；未安装或未检测到，未做真实宿主会话测试 |

本机检测到 Codex、MiMo、OpenCode、VS Code。安装或配置存在不代表宿主已建立连接。本轮未驱动 Claude/Cursor/Gemini/Copilot/Devin 的真实应用会话；模拟客户端自报这些名称的测试只证明 ToolHub 协议与历史处理。已有 Codex 调用历史保持；MiMo 此前未记录的事件不补造。本轮不把旧记录当作新宿主实测。

其他本地 MCP stdio 客户端可采用通用模板，具体格式由宿主管理；未知名称不再被静默忽略。仅允许远程 MCP 的云端客户端需要单独桥接，本轮没有增加 HTTP 服务。Windsurf 官方文档现跳转到 Devin Desktop；本版给出当前 Devin Windows 配置路径，旧 Windsurf 需在宿主确认实际位置，不宣称旧路径已自动探测。

## 行为与边界

- 配置文件最多读取 1 MiB；支持 UTF-8 BOM、JSONC 注释和尾逗号，只提取 ToolHub 服务 command/args/enablement/path。Provider、env、其他服务等信息不输出。Claude Code 的 projects 范围有界读取并明确显示；不遍历磁盘上的项目配置。
- 默认用户路径检测不代替宿主的有效配置解析。自定义路径、项目覆盖、多 VS Code 配置档、Gemini 独立启用文件及宿主信任，均需在实际客户端核对。VS Code 可读取默认用户 servers 格式和用户 portable mcpServers 格式。
- 未知客户端名称按规范化名称生成稳定 ID，展示名称最多 128 字符，未知历史最多 64 个。达到上限后仍允许 MCP 操作，只不增加新展示记录；现有记录仍更新。已知宿主记录不占此限。ToolHub 自检不产生假 Agent。
- 自报名称只是观察元数据，不能作为认证身份；不会改变 controller_verified、执行规则、批准或程序勾选条件。旧软件检测和历史记录继续保留，不显示为当前在线。
- Windows 安装登记读取有界，按已知软件名和 exe 文件名匹配。检测 GUI 不会让它误走 OpenCode CLI 自动运行。
- 支持 MCP 的宿主可复用工具查询、能力解析、通用 Skill 查询和程序待确认提交流程。无 Skills 功能时使用同一正文作为指引。宿主专属 Skill 不自动收录或跨 Agent 共享。
- MCP 保持四个已支持版本的响应；请求未知版本时改为返回最新已支持 2025-11-25，由客户端决定是否兼容。不伪称支持请求的新版本。依据 [MCP 版本协商规范](https://modelcontextprotocol.io/specification/2025-11-25/basic/lifecycle)。

## 验证

相关 Rust 60 项通过：Agent bridge 17、daemon 24、Desktop 9、MCP 4、CLI 真实进程协议 6；一个有限子进程夹具按设计忽略。相关严格 Clippy、桌面 TypeScript/Vite 构建通过。三件 release EXE 同批构建，16 个包文件哈希验证通过。

一致数据库副本上的真实 Tauri 窗口检查了 11 个客户端模板及复制 payload；通过 7 组本轮 stdio 测试客户端检查 initialize、initialized、12 元工具和 search_tools，涵盖四个受支持版本及未知版本回退。重启隔离桌面和后台后观察时间与条目仍保留，页面无异常。此 QA 历史只在副本中，未写到用户数据库。

正式安装的原生窗口再次验证 11 模板、当前 CLI 路径、12 元工具自检、自启动任务与列表；页面无异常。逐字段比对确认 4 个程序、44 个工具安装记录、2 个通用 Skill 完整保留；MiMo、OpenCode、Copilot 和 VS Code 已有配置哈希不变。本轮未配置其他宿主或关闭其 MCP 进程。

## 交付

版本 multi-host-20261005-1610，安装目录 C:\Users\AA\AppData\Local\ToolHub\versions\multi-host-20261005-1610，包位于 dist/toolhub-multi-host-20261005-1610。现有登录任务和快捷方式指向本版，实际任务启动显示 ToolHub 正式窗口，最后桌面 PID 115096 / 配套后台 PID 109888。未重启电脑或进行新 Windows 登录。

安装器只更新 Codex 中 ToolHub 的 command/args，解析比对确认其他字段和服务不变；本机通用接入 Skill 更新并通过 skill-creator 验证，agents/openai.yaml 保持原样。旧 MCP 进程可能需要在下一次失败后重试或宿主重载，不自动重放执行请求。

证据：D:/codex/toolhub-hosts-20261005。native-result.json、restart-result.json、production-result.json、preserved.json、integration-result.json、final-launch.json、installed-host-help.png、installed-hosts.png。实际运行测试为 Windows，其他平台和每个真实宿主完整任务执行仍未验证。

## 官方格式参考

- [Codex](https://developers.openai.com/codex/mcp/)
- [Claude Code](https://code.claude.com/docs/en/mcp)
- [Claude Desktop](https://modelcontextprotocol.io/docs/develop/connect-local-servers)
- [Cursor](https://prod.cursor.com/docs/mcp)
- [OpenCode](https://opencode.ai/docs/mcp-servers/)
- [Gemini CLI](https://geminicli.com/docs/tools/mcp-server/)
- [VS Code / Copilot](https://code.visualstudio.com/docs/agent-customization/mcp-servers)
- [GitHub Copilot CLI](https://docs.github.com/en/copilot/reference/copilot-cli-reference/cli-config-dir-reference)
- [Devin Desktop / Windsurf](https://docs.devin.ai/desktop/cascade/mcp)

MiMo 使用本机实际 JSONC 配置和已验证的 command 数组格式。其他格式参考为本轮查阅的官方文档；客户端版本变化可能需要更新目录，本地配置文件不提供活跃连接保证。
