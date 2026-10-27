# 服务观察

只在用户授权启动的长期服务确实运行后登记：register_service({name,pid,project_dir,kind})。kind 为 web/model/proxy/other。提交实际 PID 和存在的绝对目录，不凭端口推测进程，不扫描其他软件。代理为持续应用，使用 kind=proxy，不结束它或改系统代理。ToolHub 只读观察，Agent 不取得停止、重启、代理设置或自动停止权。

list_services 默认10条，只看当前认证来源；按需 limit/offset 或 id 查询。不持续轮询，不等待桌面处理。需要重启/清理时提示用户在服务监控中决定，或继续遵守当前宿主已授权的服务生命周期流程。登记不是程序收录，启动入口仍通过 propose_program 用户选择确认后保存。ToolHub MCP未重连、能力不可用或观察失败时，不伪造成功或绕过宿主权限。
