# Skill 收录与分发

从 ToolHub 0.3.0-beta.10 开始，Skill 库可以记录一个 Skill 的来源、版本和完整目录，并分发给用户选择的本机智能体。MCP 连接、宿主是否支持 Skills、文件已写入、宿主实际加载是四种不同状态。

## 使用

1. 先让当前 Agent 按它自己的规范安装 Skill，并确认安装成功。
2. Agent 通过 `offer_skill_sync` 提交目录，只提示一次是否希望同步；没有答复继续原任务。提交不会写入其他宿主。
3. 在 **工作空间 → Skill 库 → 已安装 / 待同步** 检查候选，或用 **收录 Skill** 选择本地目录。
4. 核对来源宿主、版本、链接和适用范围。收录保存完整目录的不可变快照。通用声明由用户确认，已知宿主引用只是辅助检查。
5. 勾选目标，点击 **同步所选智能体**。结果和版本进入同步记录；需要在目标宿主重载或新开会话核对实际可用性。

目录最少包含符合 Agent Skills 格式的 `SKILL.md`（YAML `name`、`description` 与正文）。`scripts/`、`references/`、`assets/` 等一同保留，不运行脚本、不安装依赖、不修改 MCP 配置或工具信任。旧版 ToolHub `skill.json` + 无 frontmatter 的 `SKILL.md` 在声明有效且通用时可通过“能力流程 → 收录到同步库”生成标准元信息，源文件不改。

## 新安装的智能体

已存在的默认 Skill 根目录可被只读识别：Codex 本机既有 `~/.codex/skills`、MiMo 本机既有 `~/.config/mimocode/skills`、Claude Code `~/.claude/skills`、Cursor `~/.cursor/skills`、OpenCode `~/.config/opencode/skills`、Gemini CLI `~/.gemini/skills`。前两项也经本机实际目录核对；不声称各版本/自定义配置都采用同一路径。

以后安装 dsh 或其他软件，在 **智能体目录 → 添加智能体目录** 选择该宿主实际读取的专用 Skill 根目录，填写显示名称和稳定宿主标识。点击该目标的 **选择下发**，从已有 Skill 中选择分发。不会猜测未知软件的目录，也不会因一次 MCP 握手就自动写文件。未创建根目录的宿主先按其规则完成安装/初始化，再选择目录。手动移除目标登记保留其现有 Skill 文件。

默认位置依照当前公开文档：[Agent Skills 标准](https://agentskills.io/specification)、[Claude Code](https://code.claude.com/docs/en/skills)、[Cursor](https://cursor.com/docs/skills)、[OpenCode](https://opencode.ai/docs/skills/)、[Gemini CLI](https://geminicli.com/docs/cli/skills/)。例如 Claude Desktop/Cowork 云端不等同于本机 Claude Code 目录；没有已确认的本机导入规则时使用手动目标，不承诺通用热加载。

## 状态与更新

| 状态 | 含义 |
| --- | --- |
| 待下发 | 尚无该目标的分发记录 |
| 已有分发记录 | 摘要中的历史状态；打开条目后重新读取实际目录 |
| 已写入 · 待宿主重载 | 文件完整校验成功；未验证宿主运行时加载 |
| 已有相同内容 | 目标存在相同目录；未覆盖用户安装 |
| 可更新 | 来源更新，而目标仍与 ToolHub 上次下发的快照一致 |
| 本地有差异 | 同名但不是相同内容，或用户已修改；禁止强制覆盖 |
| 不兼容 | 宿主专属 Skill 与目标宿主不同，或声明平台不适用 |
| 目录不可用 / 文件已移除 | 目标目录或副本已变化；不会据旧记录直接覆盖 |

通过 **更新来源** 重新读取同一源目录，校验预览 digest 与旧版本，再显式分发。只有未被手工修改的、ToolHub 管理的旧副本可替换；更新前备份和恢复记录保留在 Skill 根目录父目录的 `.toolhub-skill-staging`。同名不同来源先人工合并，不能用强制选项覆盖。来源消失后已收录快照仍可下发。

历史在同一 SQLite 注册表 `skill_distribution_v1` 设置中持久化，记录来源、技能、版本、摘要、目标、时间与结果，不记录正文。保留最近 500 条。文件快照在注册表旁 `skill-distribution/snapshots/<sha256>`；不包含在普通 JSON 设置导出中。备份、快照和注册表需一起迁移才能保留离线分发能力。

## 边界与接口

- 最多 200 个已收录 Skill，100 个待确认提交，32 个手动目标。每次发现最多 200 个候选，仅遍历已登记 Skill 根目录直接子目录。
- 包最多 256 文件、1024 目录/文件条目、12 层，单文件 4 MiB，总包 16 MiB；SKILL.md 64 KiB，frontmatter 8 KiB。支持常见标量/块 description，复杂不支持的 name 声明拒绝而不猜测改写。
- 符号链接、Windows 目录联接、设备文件、越界路径、常见凭据文件拒绝；`.git`、`node_modules`、`__pycache__` 等不分发。并不声称发现所有正文秘密或宿主依赖，收录前仍需检查。
- 快照总量 256 MiB，单个父目录的暂存/备份空间上限 64 MiB（写入预留保守余量）。达到限额保留数据并报错，不自动永久删除；人工整理完成的历史后重试。
- 宿主专属 Skill 可保留来源与记录，但只可下发到相同来源宿主标识。已知宿主 API、allowed-tools、hooks 等声明会要求宿主专属；同名宿主标识是显示/兼容元数据，不是执行身份。
- 查询先读摘要；仅选中条目检查完整副本。MCP `search_skills` 仍默认 5 条，不返回全库正文；`inspect_skill` 按需读取选中的通用包和支持文件名。没有 `skill.json` 的原生包标注 `workflow_only` / `dependencies_declared:false`，不能宣称本机依赖已经满足。

普通 MCP/CLI 客户端只能 `skill.offer`。桌面控制器使用 `skill.distribution.list/discover/preview/collect/sync/target/target_remove/dismiss`；修改需最新 revision，收录与同步需 expected_digest。没有“force”、执行、下载或其他宿主配置写入接口。第三方 iframe 的 `skills.read` 不包含分发控制器权限。

Windows 真实 Tauri 验证覆盖完整包、多目标、dsh 补发、重启持久化、版本更新、冲突、分页及窗口尺寸。macOS 未做本机验证。原生目录选择器沿用既有实现；自动化验收的目标登记使用隔离 fixture 路径，未自动点击操作系统目录选择对话框。
