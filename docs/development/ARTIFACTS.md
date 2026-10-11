# 产物包生命周期

这是用户显式授权的任务文件管理，独立于工具/环境发现：已有工具扫描仍然只读，不删除软件、源码或依赖。文件保留在原位置；一个包记录任务、来源、拥有者、优先级、明确文件清单、大小、SHA-256、修改时间、去留理由和历史。

## 使用方式

桌面产物页先选择具体项目/任务目录。扫描仅返回名称/目录符合临时、测试、备份模式的候选，必须勾选才登记。人工也可以先选择文件再填写包信息。默认高优先级。支持打开原目录、筛选、分页、调整优先级、人工保留、任务完成标记、复制 Agent 判断任务、确认回收和查看历史。

低：任务完成且 no_longer_needed=true 后，启用范围内排队 300 秒。中：完成后等待接入的 Agent 核对用途、占用和引用并记录 recycle/retain 判断；recycle 同样等待 300 秒。高或人工保留：禁止 Agent 自动清理。目录外登记不阻断任务，只暂缓清理；选择范围后需要重新确认完成/人工规则才排队。自动清理依赖本机 daemon 运行，关机时不清理；已排队记录重启后继续，人工保留和暂停目录依旧生效。

## MCP / THP

| MCP | THP | 作用 |
|---|---|---|
| register_artifact_package | artifact.register | name/task/root/files 必填，priority 默认 high，source 选填；登记不删除 |
| list_artifact_packages | artifact.list | query/offset/limit 可选，每页最多 20 个摘要；id 单独提供时返回完整文件清单和历史 |
| complete_artifact_package | artifact.complete | id/no_longer_needed=true/reason；仅创建者或桌面 |
| review_artifact_package | artifact.review | id/decision/reason；recycle 必须 unused=true/dependencies_checked=true |

用户原生桌面专用：artifact.root_add(path)、root_disable(id)、scan(root_id)、update(id,priority?/retained?)、recycle(id,confirmed=true)、forget(id)。普通 MCP/CLI 不能冒充桌面。读取 artifact.roots、inspect(id) 可用，但非共享范围内包仍检查拥有者。当前 MCP 固定 16 工具，不按文件/包数量增长。SDK 类型同步维护。

## 边界与失败

每包 128 文件，每个最多 256 MiB，总计 1 GiB；最多 1000 个包和 32 目录。只处理明确列出的普通文件，不回收目录，不接受通配符或越界路径。拒绝符号链接/Windows 重解析点、系统/配置/凭据/依赖路径。扫描限制深度 6、20000 项、500 候选并报告未读取/截断；整个扫描只读。

清理重新核对指纹、已知工具/Skill/程序引用；ToolHub 执行或扫描期间暂缓。Windows 原生 IFileOperation 使用 RECYCLEONDELETE，进度回调拒绝非回收操作；失败不使用 remove_file。macOS 使用固定 Finder delete AppleScript，路径通过 argv 传入，失败不兜底。不能检测任意外部应用的一切引用：Agent 必须实际核对依赖与使用情况。文件替换与并发仍需保守人工处理，不能把检查当作完整文件系统隔离。

回收逐文件持久化结果，某文件失败会停下并保留其余文件；partial 可人工核对后重试。进程在回收中断后将包标为 blocked，要求核对原目录和系统回收站，禁止猜测成功或自动重试。清理历史留在包中。恢复从系统回收站进行；ToolHub 记录保留回收时的历史，不声称自动跟踪外部还原。

## 验证

隔离数据库与自己创建的测试文件用于生命周期、权限、修改/占用阻断和原生回收测试；不对用户历史残留执行自动清理。macOS 未在本轮原机运行，应保持 beta 限制说明。


文件改变导致旧登记阻断时，桌面可“解除登记并保留文件”：只更新管理记录，文件不删除，历史保留；扫描/选择当前版本后重新收录。普通 Agent 无权解除登记。released 与已回收不相同，不能把解除登记报告成文件被清理。

列表分页返回摘要和准确文件数量/大小，避免大量产物清单撑满 IPC 响应；完整文件仅按包 inspect 获取。桌面分批加载摘要后以 6 个包展示分页。
