# 通用 Skill 库本机验证 — 2026-10-04

本轮按用户确认将共享库限定为可跨 Agent 复用的工作流。通用性由真实依赖决定，与作者是谁无关；宿主专属 Skill 留在宿主中，不自动扫描或同步各 Agent 的技能目录。

## 改动

- 增加 portability 声明及解析校验：适用平台、输入输出、必要操作、宿主依赖。
- 新导入要求完整声明和可读说明。明确依赖宿主的包拒绝导入；旧记录保留为待核对。
- 每次读取重查说明；已知宿主工具引用提示核对，不把文字提及当成实际依赖的证明。
- MCP 只查询当前平台可复用的通用流程；实际本机依赖和执行权限另行检查。
- 页面提供完整模板，展示平台、输入输出和权限范围；视频/PDF 示例及接入指引更新。

## 验证

相关 Rust 单元测试 97 项（含独立 Skills 解析 3 项）、严格 Clippy、SDK 4 项运行测试及声明检查、桌面 TypeScript/Vite 构建通过。三件 release 二进制完成同批构建。

使用生产数据库的一致副本，在独立命名空间运行真实 Tauri 窗口和 release CLI 的 stdio MCP。验证旧条目仍在且查询排除；导入两个通用示例后查询可见；宿主专属和缺少声明的导入失败且不改列表；Linux 限定条目在 Windows 显示不适用；修改已登记说明增加 image_gen 引用后，查询排除并停止返回说明。页面无异常，输入输出和权限区域及复制按钮已截图检查。

证据：D:\codex\toolhub-portable-skills-20261004\native-result.json 与 native-portable-skills.png。

## 本机交付

跨过午夜后于 2026-10-05 完成本机交付。版本：portable-skills-20261004-2310；安装目录：C:\Users\AA\AppData\Local\ToolHub\versions\portable-skills-20261004-2310。16 个包文件哈希通过，三件 release EXE 同批安装。

生产真实窗口/MCP 验证通过：两个旧示例保留后更新为带完整声明的通用流程，无 UI 异常，12 个元工具和无查询条件的 Skill 搜索通过。数据库逐字段比对，4 个程序入口、44 个工具安装记录一致。没有导入隔离测试的宿主包。

本机 toolhub-integration Skill 已更新并通过格式验证；其 agents/openai.yaml 未变。Codex 配置解析比对确认仅更新 ToolHub command/args，其他服务及字段未变。当前宿主已有的 MCP 会话仍需重载才会采用新路径，未终止宿主会话。

快捷方式与当前用户登录任务指向本版本；桌面原生状态确认 enabled/command_matches。再通过任务调度器实际打开本版本并确认配对后台与真实窗口，最后留着程序供实测。任务运行时结果 267009 表示仍在运行，不是失败。本轮未重启电脑或实际重新登录。

生产证据：D:/codex/toolhub-portable-skills-20261004/production-result.json、preserved.json、integration-result.json、final-launch.json 和 installed-portable-skills.png。

## 限制

通用性检查依靠声明和有限的已知引用检查，不能证明所有文本完全可移植或安全；未知宿主依赖仍需人工核对。通用流程可能缺少当前电脑可用依赖。权限声明不会代替用户授权和 ToolHub 策略。本轮实际运行验证为 Windows，未运行 macOS/Linux。
