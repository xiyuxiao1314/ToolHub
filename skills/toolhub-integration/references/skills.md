## 登记依赖本机能力的 Skill

宿主 Agent 的本机 SKILL.md 用来指导 Agent；ToolHub Skill 注册表用来描述工作流及依赖。二者不等同。宿主支持 Skills 时将本文保存到它实际识别的 Skill 目录，保持隐式调用可用；不支持时作为任务指引。

用户要求登记自己的 Skill 时，在持久目录提供 SKILL.md 和 skill.json：

```json
{
  "schema": "toolhub.skill/v1",
  "id": "agent.my-skill",
  "name": "My Skill",
  "description": "实际工作流说明",
  "kind": "instruction",
  "instruction_file": "SKILL.md",
  "requires": [{"capability": "media.video.transcode"}],
  "portability": {"scope":"portable","platforms":["windows","macos","linux"],"inputs":["用户指定的视频"],"outputs":["压缩视频"],"permissions":["read_files","write_files","execute_tools"],"host_dependencies":[]}
}
```

新导入仅收录通用流程说明。skill.json 必须加入 portability：scope=portable；platforms 为 windows/macos/linux 的适用列表；inputs/outputs 写明实际输入输出；permissions 为 read_files/write_files/execute_tools/network_access 的必要操作；host_dependencies 必须为空。独立脚本或服务通过 requires 声明本机能力。宿主专属 Skill 留在对应宿主中；旧条目没有声明时标为待核对，不能当成通用技能复用。声明和已知引用检查不能证明任意正文完全可移植，执行授权仍单独检查。

替换 ID、名称和依赖；没有依赖时 requires 为空。包内路径必须存在、相对且不越出目录，不含安装脚本或 hooks。注册只读取声明：

```powershell
& $toolhub --json skill register '<已授权的 Skill 目录>\skill.json'
& $toolhub --json skill inspect '<真实 Skill ID>'
& $toolhub --json skill resolve '<真实 Skill ID>'
```

MCP search_skills / inspect_skill 用于查询，不能代替注册。注册成功不表示依赖满足，以实际解析结果为准。


## 已安装 Skill 的可选分发（beta.10）

先遵循当前宿主的安装规则，将整个目录安装到该宿主实际支持的位置并完成验证。随后调用 `offer_skill_sync({"path":"实际绝对目录","source_agent":"codex / mimo / dsh 等真实宿主标识","version":"可选版本","source_url":"可选原始链接"})`。或 `toolhub --json skill offer <目录> --source-agent <宿主标识>`。

提交仅保留待确认元数据，不自动收录、不写入其他智能体，不代表宿主已加载。安装后提醒用户到“已安装 / 待同步”自行选择；不自动操作 ToolHub 界面、不替用户选择目标，继续当前任务，不等待确认。桌面“Skill 库 → 已安装 / 待同步”检查完整包，选择通用或宿主专属、收录后勾选目标。未知宿主先由用户选择实际 Skill 根目录登记；不会猜测 dsh 等客户端的目录。MCP 连接与文件分发分别管理。

宿主专属工具、hooks、allowed-tools 等声明限制跨宿主同步。完整支持资料一起复制，最多 256 文件 / 16 MiB，单文件 4 MiB，SKILL.md 64 KiB；含凭据、目录联接或符号链接的包需先整理，不会自动执行或安装依赖。原始 SKILL.md 通用声明需用户核对；已知引用检查不是完整可移植性证明。

“已同步”表示文件校验成功；ToolHub 不检测宿主加载状态。目标宿主已经显示技能时无需操作，找不到时再刷新技能列表或新开会话。同名异内容、已有手工修改、来源与版本冲突会保留，不强制覆盖。已管理且未修改的旧副本可显式同步新版本；替换前的备份和恢复记录在目标 Skill 根目录的父目录 `.toolhub-skill-staging` 中保留。历史保留最近 500 条。新接入智能体可选择补发既有 Skill，不自动写入。
