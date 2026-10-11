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
