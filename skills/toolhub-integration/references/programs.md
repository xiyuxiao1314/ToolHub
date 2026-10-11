## 提交自己创建的应用入口

完成用户要求的应用、验证启动入口后，可通过 MCP 提交元数据。提交不会运行命令，也不会正式收录。入口放在持久项目目录，避免临时构建、依赖缓存或系统安装目录；不要提交所有中间产物。来源备注说明实际 Agent，不声称扫描证明作者身份。

Windows 文件入口支持 exe/bat/cmd/ps1/lnk；macOS 支持可执行文件、.sh/.command，暂不支持 .app 整体入口。提供所在平台真实存在的绝对文件路径和工作目录，参数为数组。以下为 Windows 示例：

```json
{"name":"用户应用","kind":"file","path":"D:\\项目\\应用\\start.cmd","cwd":"D:\\项目\\应用","args":[],"source":"创建该应用的 Agent"}
```

命令入口提供存在的绝对工作目录和所在平台的单行命令（Windows 使用 CMD 语法，macOS 使用 sh 语法），命令参数写在 command 内。以下为 Windows 路径示例；Mac 应改为 /Users 等真实路径：

```json
{"name":"用户应用","kind":"command","cwd":"D:\\项目\\应用","command":"npm run start","source":"创建该应用的 Agent"}
```

将真实信息作为 `propose_program` 参数。结果 `status: pending` / `requires_user_confirmation: true` 表示待确认，向用户报告“请到 ToolHub 程序页刷新、勾选、核对后保存”。`already_saved` 表示入口已收录，本次没有修改原条目。不能把待确认 ID 当成可执行实例 ID。

普通 Agent 无权自动选择、保存或启动程序条目。不得伪造选择凭证、直接改 SQLite、使用控制器接口绕过勾选。用户可编辑或忽略提交；只有选中并保存后才进入程序架。桌面启动授权与 MCP execute_tool 的策略授权分别检查。
