## 按任务管理产物

用户启用产物管理、要求整理工作文件时，使用以下 MCP 流程；不要把登记当成新的删除授权。仅登记当前任务中自己创建、用途明确的测试脚本、临时输出或备份。正式交付、程序入口、源码、配置、凭据和依赖应保留。包是文件清单，文件保持原目录；每包 1–128 个明确相对文件路径，不能登记通配符或整个目录。

1. `register_artifact_package`：提供真实 `name`、`task`、绝对任务目录 `root`、相对 `files` 数组及 `source`。一次性验证文件可用并记录指纹。未指定 `priority` 时默认 high；低为一次性临时文件，中为需要用途/依赖判断的文件，高为需要人工决定的文件。用户选择的管理目录外可以登记，但不会自动清理；继续任务，不必等待添加目录。
2. 任务成功完成并确认文件不再需要后，调用 `complete_artifact_package(id, no_longer_needed=true, reason)`。低优先级在启用范围内等待 5 分钟后自动进入回收站，Agent 无需停下等待；中优先级返回 needs_review 是正常待判断状态；高优先级保留。
3. 中优先级先 `list_artifact_packages(id=包 ID)` 获取完整文件清单与历史；不用 id 时返回分页摘要（limit 默认 20，offset 分页），再核对清单、状态与真实任务上下文。检查当前使用、项目/程序/Skill 引用；实际确认无用且依赖检查完成，才能 `review_artifact_package(id, decision="recycle", unused=true, dependencies_checked=true, reason=实际理由)`。不确定时 decision="retain"。这个接口记录 Agent 判断，不是 ToolHub 内置模型；不要仅凭名称判断。
4. 普通 Agent 无权改管理范围、降低已登记的优先级、撤销人工保留或删除高优先级包。ToolHub 会再次检查指纹、已知引用和自己的运行任务；它不能穷尽其他应用的文件引用。文件改变、权限/占用或回收站不可用时会暂缓，不能改用直接删除或修改数据库绕过。用户可在撤销期内保留，回收后从系统回收站还原。

示例参数（替换为当前任务真实文件；不登记本文示例路径）：

```json
{"name":"界面回归临时文件","task":"本次界面回归已生成的中间记录","root":"D:\\项目\\任务","files":["test_smoke.log","backup.old"],"priority":"low","source":"当前 Agent"}
```

只报告 ToolHub 返回的真实状态：scheduled 表示等待回收，trashed 才表示已回收，blocked/partial 需查看错误。登记、扫描和任务标记都不是即时永久删除。
