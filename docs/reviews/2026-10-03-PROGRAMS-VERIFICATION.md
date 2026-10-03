# ToolHub 程序页实现与 Windows 验证（2026-10-03）

本轮按用户确认新增“程序”导航：所有本地磁盘扫描并过滤系统/已安装软件/依赖；扫描勾选和手动原生选择后才可填写保存；支持文件入口和命令入口。保留此前按钮修复及已有工作，继续在 `codex/button-audit-20261001` 分支修改；验证完成时尚未提交、推送或合并。

## 实现结果

- 程序卡片支持启动、打开所在目录、编辑、收藏、搜索及移除；移除只删除条目。
- 支持 EXE、BAT、CMD、PS1、LNK 和自定义 CMD 命令。工作目录来自选择器，文件参数逐行填写。快捷方式保留自身目标/参数。
- 扫描所有可用本地固定/可移动磁盘，也可选择单独目录。过滤系统、AppData、已登记安装路径、依赖、缓存、链接/联接和编译中间文件，保留 dist/build/target 主入口。
- 基于 Agent/项目标记、启动文件及 package.json 启动脚本提供候选及依据，不声称能证明 AI 作者身份；扫描不会执行或自动添加。
- 候选分页、取消、未覆盖根目录和限制提示已实现。上限 200 万目录项、5000 候选、48 层；每次最多选择 100 项。
- 程序条目由 daemon 管理，SQLite migration 003 独立保存；选择凭据绑定经验证的控制器，原有工具/Agent 授权规则保留。批量保存具有事务性。
- Windows 控制台使用独立输入/输出句柄；批处理和命令输出保留可供检查。启动成功提示表示进程创建请求已提交，并不代表应用内部初始化一定成功。

## 修复及验证

| 检查 | 结果 |
| --- | --- |
| 原生文件选择，取消后无表单；选中后路径只读 | 通过 |
| 原生命令目录选择，命令不能为空，按工作目录实际执行 | 通过 |
| 原生 LNK 选择保留快捷方式路径和自身参数 | 通过 |
| EXE/BAT/CMD/PS1/LNK/命令入口实际创建测试输出 | 通过 |
| 中文、空格、&、百分号、引号、尾部反斜杠 | 通过；首轮发现 BAT 百分号文件名展开，已修复并复测 |
| Windows 基础目录变量大小写及环境块排序 | 通过；子进程保留基础变量，合成测试凭据不继承 |
| 未选择保存、其他控制器使用选择凭据、重复添加 | 拒绝，符合预期 |
| 批量保存有无效项 | 整体取消，不发生部分写入 |
| 扫描不添加，依赖过滤、分页、取消、未扫描磁盘提示 | 通过 |
| 121 候选：加载更多、筛选、选中后聚焦编辑、取消不添加 | 通过 |
| 收藏、编辑、搜索、Ctrl K、刷新、目录按钮 | 通过 |
| 文件失效禁用启动，仍可打开父目录；移除不删除文件 | 通过 |
| 应用重启后条目保留 | 通过 |
| 390 / 640 / 1180 宽度及七个导航页面 | 通过，无横向溢出或未处理页面错误 |

React/TypeScript 构建与 SDK 声明类型检查通过；工作区 Clippy（all targets，warnings as errors）通过，后续环境修复的 daemon Clippy 也通过。受影响单元测试共 40 项在相关验证轮次通过；服务集成、managed 连接和 soak 共 10 项通过。没有声称重新运行整个工作区的所有测试。

Windows 批处理的百分号保护参考 [Rust 的批处理参数实现](https://github.com/rust-lang/rust/blob/main/library/std/src/sys/args/windows.rs)，并扩展到脚本路径。Windows 子进程环境块遵循 [Microsoft 的环境变量说明](https://learn.microsoft.com/en-us/windows/win32/procthread/changing-environment-variables)。这些规则只处理 ToolHub 创建的子进程，不修改用户或机器环境。

## 本地证据与运行

- 测试数据库：`D:\codex\toolhub-programs\test.sqlite`，与正常数据隔离；测试入口未加入正常列表。
- 正常数据库升级前备份：`D:\codex\toolhub-programs\production-before-programs.sqlite`。
- 修改前源文件副本：`D:\codex\toolhub-programs\before-programs`。
- 测试输出：`D:\codex\toolhub-programs\output`，含 final-build.txt、env-build.txt、final-native-results.json、extra-results.json、pagination-results.json、lnk-picker-result.json、env-native-result.json 及截图。首轮失败证据保留在 native-results.json，最终对应复测通过。
- 程序：`D:\mimoproject\ToolHub\ToolHub\target\debug\toolhub-desktop.exe`；后台 toolhubd.exe 必须在同一目录。
- 快捷启动：`D:\mimoproject\ToolHub\start-ToolHub.cmd`。

当前为 Windows 开发版 exe，未生成新安装包。macOS 原生选择器与该程序启动流程未验证。
