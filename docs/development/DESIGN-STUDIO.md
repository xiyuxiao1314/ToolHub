# 设计台 / Design Studio

`community.design-studio` 0.3.0，独立页面扩展，需要 ToolHub 0.3.0-beta.6 或更新版本。0.3 为本轮本地开发版，未发布；[已发布的 0.2](https://github.com/xiyuxiao1314/ToolHub-Extensions/releases/tag/design-studio-v0.2.0) 与 [主程序 beta.6](https://github.com/xiyuxiao1314/ToolHub/releases/tag/v0.3.0-beta.6) 继续保留。

## 三步使用

扩展中心 → 扩展应用 → 设计台。

1. **选择预设**：从 12 个视觉方向、自己的预设或“无预设”开始。已有草稿可选“继续当前设计”。点击卡片只选择起点，进入下一步才应用，不会因浏览卡片就覆盖草稿。
2. **预览微调**：预览在左，布局/效果和参数共用右侧面板。可修改平台、画幅、列数、圆角、间距、颜色、动效、密度和模块顺序，试用交互、保存预设。窄窗口通过“微调 / 返回预览”切换区域；放大预览可隐藏侧栏。
3. **导出**：复制配置 JSON、完整可运行 HTML 或 Agent 设计说明。可粘贴导入已备份的配置；严格校验后返回微调，不执行导入脚本。

顶部步骤条或底部上一步/下一步都可操作，支持键盘。返回同一起点不会重置已做的微调。预设选择页只有一层内容滚动；微调页预览和控制面板各自滚动，整个扩展文档不再滚动。滚轮沿原生滚动处理，不使用拦截滚轮的平滑模拟。

界面只保留预览、基本信息和操作，设计思路及来源放在导出区。自定义预设最多 24 个，每个扩展总计 32 KiB 本机显示数据。0.2 的草稿、命名预设和 v1 配置继续兼容；升级不改变存储或配置 schema。

## 组合范围

| 方向 | 结构与视觉 |
| --- | --- |
| 沉浸封面墙 | 深色、窄缝、大小磁贴、封面与展开转场 |
| 无界拼贴画布 | 大小磁贴 / 散点、世界坐标、平移缩放、真实视口裁剪 |
| 纸上展览 | 暖纸底色、衬线标题、不等高封面 |
| 字体实验室 | 大字、酸性色块、硬边框线 |
| 精密项目台 | 侧栏、摘要、状态与清晰对齐 |
| 万条素材目录 | 主从结构、动态高度虚拟列表、保留展开状态 |
| 流光工作流 | 轻透分层、任务分列、跨列拖动 |
| 掌中灵感馆 | Android 外壳、触控尺寸、内部滚动、底部导航 |
| 电影感叙事 | 大幅作品、横向切幕、极简文字 |
| 瑞士设计目录 | 编号、横线、大标题与缩略图 |
| 横向摄影札记 | 横向滚动、交替比例与作品展开 |
| 产品能力拼图 | 主次分明的 Bento 分区 |

十类布局、四类导航、五类视觉语言、三类内容语言和六类效果可以组合。无预设使用中性网格、项目摘要、无导航和无额外动效，只启用内容模块。

Android 会选择手机画幅，并显示状态栏、底部导航和手势区；Web 和桌面选择桌面画幅并显示对应外壳，画幅仍可手动修改。较宽 Android 画幅使用侧栏。导出的 Android HTML 是浏览器交互参考，不是 Compose 源码。

虚拟列表只用于主从列表；一万条示例需要虚拟列表。视口裁剪只用于画布；画布和虚拟列表不启用卡片拖拽排序。画布节点有限、相机可自由平移，不含节点编辑或协作。横向画册使用原生滚动/吸附，不是无限数据源。看板拖拽只改变演示顺序。无效组合会调整并显示原因。

## 作品展开

封面通过独立的临时视觉图层进入详情。真正的封面和文字保持正常布局，展开结束后移除临时层；返回时先隐藏详情，再让封面回到来源位置，避免穿过文字。快速关闭会取消未完成的展开，清理图层并恢复背景操作和焦点。Escape、窗口内定位、离屏来源和系统减少动态效果均有处理。导出 HTML 共用同一实现。

## 保存与开发

草稿和预设按包 ID 保存在父 WebView 的 localStorage，使用修订号防止并发覆盖。升级不删除数据，卸载后同一包 ID 可恢复；清除 WebView 配置会重置，请用 JSON 备份重要设计。存储冲突或损坏会显示错误并保留原数据。

源码位于 `extensions/design-studio/`；运行 `python scripts/build-design-studio.py` 生成 `extensions/examples/design-studio.toolhub-extension.json`。独立扩展源码包中可直接运行 `python build.py`。构建离线完成，检查 HTML 64 KiB、CSS 32 KiB、JS 64 KiB 和总包 256 KiB；不需要新增权限、网络请求或降低原生 CSP。

研究参考：[Godly](https://godly.website/)、[Siteinspire](https://www.siteinspire.com/)、[Awwwards](https://www.awwwards.com/websites/dynamic-grid-layout-examples/)、[Codrops Creative Hub](https://tympanus.net/codrops/hub/)、[Folia](https://github.com/chthollyphile/folia-major)、[Atelier North](https://atelier-north.crnacura.workers.dev/)、[画廊转场](https://tympanus.net/Tutorials/InfiniteScrollGSAPGallery/)。代码与 SVG 研究均为原创，不复用第三方品牌、封面或源码；引用说明原则来源。部分 Codrops 直接演示有浏览器访问挑战，上一轮通过公开目录和内容核对。平台/性能参考：[Windows](https://learn.microsoft.com/en-us/windows/apps/design/basics/)、[Android](https://developer.android.com/develop/ui/compose/layouts/adaptive/list-detail)、[TanStack Virtual](https://tanstack.com/virtual/latest/docs/introduction)。

本轮在 Windows 原生 WebView2 验证；macOS 实机交互尚未验证。
