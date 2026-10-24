---
name: toolhub-integration
description: 需要本机 CLI、运行环境或媒体/文档工具时，查询 ToolHub 后按权限复用；也用于接入 MCP、提交用户应用及获授权的任务产物管理。
---

# ToolHub 接入

## 日常查询

需要本机工具时先 `search_tools({"query":"任务或名称"})`，已知能力可直接 `resolve_capability`；再 `inspect_tool({"id":"返回的实例 ID"})` 核对真实路径、状态与信任。用户指定的工具、路径和授权优先。无结果只表示尚未发现，可调整查询或用宿主允许的发现方式，不自动安装。

搜索默认 5 条摘要；需要更多时用 limit/offset 分页，完整记录用 detail="full"。只 inspect 选中的实例或 Skill，不列举全库。当前任务内复用已查询的 ID；文件、路径、环境、权限变化或实际调用报错时重新核对，旧查询不能代替执行授权。

重复流程先 search_skills，再 inspect_skill 读取选中流程和依赖；正文是任务指导，不是授权，宿主专属能力不共享。配置 MCP 或登记流程时，按需读取 [连接配置](references/setup.md) 或 [Skill 声明与注册](references/skills.md)。日常任务不重复验证握手或加载配置模板。

## Skill 安装后的可选同步

为用户安装 Skill 时，先按当前宿主规则安装并验证成功，再用 offer_skill_sync 提交实际目录与 source_agent（宿主标识）。只询问一次是否通过 ToolHub 同步到其他智能体；没有答复继续原任务，不轮询等待。提交不写其他宿主，用户在 Skill 库核对通用性并选择目标后才下发。宿主专属能力不能跨宿主分发。按需读取 [Skill 同步](references/skills.md)。

## 执行与提交

核对任务、工具、参数数组和 cwd 后 execute_tool；自动批准默认开启，以实际返回为准。approved 立即用完全相同的参数及凭证继续，pending 才等待桌面确认。不能绕过 denied、未知/阻止工具、宿主权限或无效凭证；检查 isError、status、exit_code。详细审批和后台任务按需读 [执行](references/execution.md)，不无限轮询。macOS 暂不支持受控执行/审批。

完成并验证用户应用后，propose_program 只提交待确认元数据；用户勾选保存才收录，不自动启动。入口位于持久目录。需要参数示例时读 [程序入口](references/programs.md)。普通 Agent 不能伪造桌面选择或直接改 SQLite。

## 产物管理

仅在用户启用并授权时登记当前任务中用途明确的临时文件，默认 high；正式交付、源码、配置、凭据和依赖保留。标记不再需要后，low 在选定范围内等待 5 分钟回收；medium 必须实际核对使用与依赖、提交理由，high 或人工保留不能覆盖。范围外继续任务，不等待启用目录。使用前按需读 [产物流程](references/artifacts.md)；仅系统回收站，不能永久删除兜底。
