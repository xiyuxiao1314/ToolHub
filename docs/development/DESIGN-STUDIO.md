# 设计台 / Design Studio

独立 v2 页面扩展，包 ID 为 community.design-studio，版本 0.1.0。需要 ToolHub 0.3.0-beta.6 的扩展本机存储 SDK；beta.5 不包含此接口。当前为本机可试用开发版，未发布 GitHub 安装包。

## 从真实流程试用

1. 扩展中心 → 扩展应用 → 设计台，也可用图钉固定到工作空间。
2. 从 8 个预设选择一个起点，点击“应用预设”。
3. 左侧组合页面结构、导航、视觉和交互效果；右侧设置名称、用途、平台、画幅、列数、密度、圆角、间距、强调色、动效、数据量和主题。
4. 页面模块可以显示/隐藏，并用 ↑/↓ 调整顺序；主要内容必须保留。
5. 在中间直接点击、展开、筛选、拖拽、滚动或平移缩放，核对真实效果。
6. 下方切换配置 JSON、可运行 HTML、Agent 设计说明和参考边界，点击复制。浏览器拒绝剪贴板时会全选内容并明确提示手动复制。
7. 为组合命名并保存为预设；自定义预设可重新应用、两次点击删除。JSON 可复制备份，再粘贴导入。导入只解析严格配置，不运行 HTML/JS。

## 当前组合范围

- 6 种页面结构：卡片网格、Metro 磁贴、瀑布流、主从列表、无限画布、任务看板。
- 4 种导航：侧边、紧凑栏、顶部、无导航。
- 4 种视觉语言：简洁工作台、轻透 Fluent、杂志排版、硬边设计。
- 6 种可叠加效果：卡片展开、悬停抬升、入场错峰、拖拽排序、搜索栏吸顶、焦点强调。
- 3 种性能策略：标准渲染、动态高度虚拟列表、画布视口裁剪。
- 5 种可排序模块与 8 个内置预设；最多 24 个自定义预设。

这些选项并非全部兼容。第一版虚拟化只用于主从列表；一万条数据需要虚拟列表。视口裁剪只用于画布。虚拟列表与画布不启用卡片拖拽排序；画布本身支持平移。切换结构时会显示调整原因，导出的配置始终反映实际预览。列数只用于网格、磁贴和瀑布流。

## 输出与边界

预览与独立 HTML 使用同一份验证配置和渲染/交互函数。HTML 可复制保存后用浏览器打开，不需要外部 CDN。平台选项改变 Agent 说明；Android 和桌面示例仍是浏览器交互参考，不冒充 Compose、Qt 或其他原生项目源码。

画布展示有限的示例节点，支持相机平移、缩放、复位和裁剪，不提供节点编辑、持久画布文档或协作。看板拖拽只改变演示项目顺序。搜索和筛选操作示例数据，不接入真实业务后端；虚拟化不代表网络分页。动画关闭及系统减少动态效果时不影响核心操作。

代码输出是可运行设计参考，仍需 Agent 接入真实内容、业务状态、后端、加载与错误处理。不会安装 UI 库、执行本机程序、读取其他扩展数据或新增 MCP 工具。

## 本机预设

草稿和预设保存在父 WebView 的 localStorage，每个包最多 32 KiB。卸载保留这份显示数据，重新安装同一包 ID 可恢复；清除该 WebView 配置文件会重置。预设不进入数据库或普通配置导出，请用 JSON 备份重要组合。并发窗口使用修订号检查，冲突时提示重新打开，防止覆盖另一窗口的保存。

## 扩展开发

可编辑源在 extensions/design-studio/{manifest.json,view.html,view.css,preview.css,preview.js,view.js}。运行 `python scripts/build-design-studio.py` 生成 extensions/examples/design-studio.toolhub-extension.json；脚本检查 HTML/CSS/JS 与总包的原生限制，不联网。

参考：[Windows 布局](https://learn.microsoft.com/en-us/windows/apps/design/basics/)、[tldraw 画布](https://tldraw.dev/examples/basic)、[Motion 布局动画](https://motion.dev/docs/react-layout-animations)、[TanStack 虚拟列表](https://tanstack.com/virtual/latest/docs/introduction)、[Android 自适应主从](https://developer.android.com/develop/ui/compose/layouts/adaptive/list-detail)。演示自行实现，不捆绑这些库；包与源代码采用 MIT。
