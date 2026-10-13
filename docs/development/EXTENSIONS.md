# ToolHub 扩展工作台 / Extension workbench

ToolHub 的工具、程序、产物、环境、Skill、能力包市场和任务页现在是七个默认启用的官方功能扩展。智能体接入、权限、数据存储、执行与审计、扩展管理和恢复入口保留在内核。导航由已启用的扩展声明生成，官方页面按需加载；扩展管理不会随市场模块一起卸载。

## 使用

- **扩展**：选择功能后启用、停用或卸载。停用后导航消失，对应模块查询接口返回 unavailable；原有数据保留，重新启用/安装后可继续使用。已批准执行与任务结果、权限和审计属于内核，不通过页面停用伪造或放宽授权。
- **导入扩展包**：先选择 `.toolhub-extension.json`，核对内容、版本、平台、权限和依赖后安装。当前为本地目录/示例，不提供在线自动下载。
- **编辑扩展**：修改 JSON、校验预览、确认安装更新。自动填写下一 patch 版本；有 recognition 时必须同步其 version。上一版可以回滚，也可导出 JSON 分享。导入的本地修改只在用户再次确认更新时才会被替换。
- **外观**：安装石墨示例后点击“应用主题”，也可选择主题、左/右/顶部导航、宽松/紧凑密度。内核提供“恢复默认界面”；当前主题停用/卸载后自动回退，保留偏好以便重新启用。
- **工具速览**：安装示例后增加导航，页面脚本通过 SDK 查询本机工具。这是独立可修改的 HTML/CSS/JavaScript 页面，不需要重编译宿主。

首次迁移只安装一次官方功能包。已被用户停用或卸载的官方包不会在重启时偷偷恢复。官方包可从随包目录重新安装。产物扩展停用/卸载会取消已有清理倒计时，保留产物并标为 blocked；恢复模块后需要重新核对安排，不能立即执行过期倒计时。

## v1 与 v2

旧 `toolhub.extension/v1` 识别规则/通用 Skill 包保持兼容，新可选字段不加入旧快照的序列化和摘要。`toolhub.extension/v2` 还支持如下贡献：

| 字段 | 功能 |
| --- | --- |
| views | 最多 8 个导航页面，每项 id/label/icon/order/html/css/script；id 以包 id + 点号开头 |
| theme | 受限颜色变量、layout、density、sidebar_width、radius；不插入任意宿主 CSS |
| permissions | 页面可申请 tools.read、programs.read、programs.propose、skills.read、environments.read、tasks.read |
| dependencies | 最多 16 项 id/min_version；依赖必须已启用、版本满足，禁止环；停用依赖前先停用依赖它的包 |
| recognition / skills | 保留已有资源与可跨宿主复用的指令契约，不安装工具依赖或授予执行信任 |
| host_module / navigation | 保留给 toolhub.builtin.* 官方模块；只引用宿主提供的确定模块，可以定制导航标签/图标/顺序，不允许夹带第三方页面代码 |

权威实现为 `crates/skills/src/extension.rs`，对外格式为 `schemas/extension.schema.json`。示例在 `extensions/examples/`，官方清单在 `extensions/official/`，SDK 在 `extensions/sdk/`。最小宿主版本、语义版本和平台声明必须真实；声明不等于平台验证。

总包最大 256 KiB，安装数量上限 32（包括 7 个官方包）。每页 HTML 64 KiB、CSS 32 KiB、JS 64 KiB；最多 32 条识别规则、64 个能力和 8 个 Skill。禁止未知字段、安装钩子、原生库、目录穿越与宿主专属 Skill。所有资源以完整快照存入 SQLite，来源名称和作者字符串不是签名。

## 页面 SDK

```js
// 先声明 permissions: ["tools.read"]。仅查询选中用途，不列举全部能力正文。
const tools = await toolhub.call('registry.search', {query: 'FFmpeg', limit: 5});
await toolhub.notify(`找到 ${tools.length} 个候选`);
await toolhub.navigate('tools'); // 只能导航到当前已启用页面
```

全局 `toolhub` 提供 Promise 接口：call(method, params)、notify(message)、navigate(page)。查询方法保留原生 THP 参数和返回值；页面需要自行分页/限制输出。该接口不是新 MCP server，Agent 仍使用固定的 16 个元工具与默认 5 条摘要。

| 页面权限 | 可调用方法 |
| --- | --- |
| tools.read | registry.search、registry.inspect_tool、registry.inspect_instance、resolve.capability、capability.list |
| programs.read | program.search：只查询允许共享的元数据 |
| programs.propose | program.propose：只进入用户待确认列表 |
| skills.read | skill.list、skill.inspect、skill.resolve |
| environments.read | environment.list |
| tasks.read | execute.list、execute.status；仍执行任务拥有者检查 |

页面不直接获得桌面选择、程序启动、文件访问、扩展安装、策略修改、执行或产物删除接口。需要这些功能时跳转官方页面，由用户和原有核心策略处理。查询结果和包正文不构成额外授权。

## 页面隔离

仅在打开扩展页面时，桌面端启动绑定 127.0.0.1 随机端口的文档服务。它只为内核已发放的随机会话提供 index.html/view.css/view.js，不接受磁盘路径、目录浏览或 RPC；每次资源读取核对当前包和摘要，最多 32 会话、8 个连接。

该来源属于 Tauri 远程页面，未分配原生命令权限。iframe 与响应 CSP 都设置 sandbox allow-scripts，未设置 allow-same-origin、弹窗或顶层导航。页面有自己的 CSP，脚本和样式仅可读取当前随机会话的固定资源。fetch/XHR、子框架、表单提交和外部图片不开放；主应用原有脚本 CSP 保持不变。桥核对窗口来源、会话 token、当前包摘要及声明权限，限制并发和请求大小；daemon 强制按 extension:id 普通身份执行，不能继承桌面 controller。

这是一种受限的前端扩展运行方式，不声称提供完整操作系统沙箱。任意本机后台插件、独立进程钩子和 Rust 动态库不在本版接口中；官方原生服务实现仍随宿主编译、由扩展生命周期控制。第三方可以独立编写页面逻辑、主题、规则与 Skill，不能通过 JSON 替换核心权限或原生业务实现。

## THP 与生命周期

| 方法 | 参数 | 权限 |
| --- | --- | --- |
| extension.list | {} | 只读安装快照和启用状态 |
| extension.preview | {package} | 只读校验，摘要、兼容性、冲突和 Skill 依赖 |
| extension.install | {package,digest,expected_digest?,source?} | 已验证桌面 controller；更新必须更高版本 |
| extension.set_enabled | {id,enabled,expected_digest} | controller；停用前检查依赖 |
| extension.rollback | {id,expected_digest} | controller；重新校验依赖和前一快照 |
| extension.remove | {id,expected_digest} | controller；移除包，保留用户记录和原文件 |
| extension.preferences | {} 或 {settings:{theme_id?,layout?,density?}} | 查询只读，写入 controller；{} settings 恢复默认 |
| extension.invoke | {id,expected_digest,method,params?} | 有限页面桥；从不取得 controller |

更新、启停、回滚和卸载提升授权 revision，旧执行凭证失效。扫描/执行期间拒绝修改扩展，先完成或取消当前操作。移除识别规则会使原实例不可用，保留 provenance；重新安装需重新扫描。卸载不删除 OS 软件、普通 Skill、程序和产物记录。包快照不做静默自动更新。

本机测试使用独立 Windows 注册表和真实 Tauri WebView；macOS 新页面服务器和导航生命周期尚未进行原生验收。降级到不支持 v2 的旧本地开发版本时，应先关闭新程序并恢复升级前数据库备份，不能让旧 v1 解析器读取新模块快照。

## Windows acceptance — 2026-10-13

- Rust workspace: 222 passed, 1 existing ignored; TypeScript SDK: 4 passed and strict types checked; desktop UI: type check and production build passed.
- Packaged Tauri/CDP: 11 native behavior groups passed, including default modules, install/disable/remove/reinstall, local edits/rollback, independent JavaScript and query bridge, appearance/reset, all modules disabled together, 16-tool MCP with five-summary search, and all ten first-party/kernel pages.
- Desktop and paired daemon restart: 5 persistence groups passed. Disabled environment stayed disabled, removed market was not reseeded, program rows and digests stayed unchanged, high-priority artifact files survived, and Agent/settings/recovery remained reachable.
- Browser isolation: parent DOM and network blocked; SDK management denied; attempted direct native settings write did not change kernel preferences (WebView2 did not deliver a response in the sandbox). Top navigation uses one horizontal row and leaves at least 75% of the viewport for content; right layout leaves over 60% width for content.
- Evidence kept locally under `D:\codex\toolhub-workbench-20261013`; sample launch files were never executed and cleanup was never scheduled.

![Native workbench](../images/extension-workbench.png)

![Native theme and top navigation](../images/extension-theme.png)

Connection-recovery repair: a real isolated daemon was stopped and temporarily renamed, the Agent refresh produced a connection banner, then the original helper was restored and “Retry connection” cleared the banner and reloaded tools. The final local package is `workbench-20261013-6`; its CLI/daemon hashes match the full Rust/native acceptance, with a focused desktop recovery check after the last UI-only repair.
