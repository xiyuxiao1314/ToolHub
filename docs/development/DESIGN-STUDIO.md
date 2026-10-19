# 设计台 / Design Studio

`community.design-studio` 0.2.0，独立页面扩展，需要 ToolHub 0.3.0-beta.6 或更新版本。独立扩展包、源码与校验值在 [扩展仓库](https://github.com/xiyuxiao1314/ToolHub-Extensions/releases/tag/design-studio-v0.2.0) 发布；主程序安装包在 [ToolHub](https://github.com/xiyuxiao1314/ToolHub/releases/tag/v0.3.0-beta.6) 发布。

## 使用

扩展中心 → 扩展应用 → 设计台。先看上方的可视化方向卡，点击即应用；可按视觉表达、空间探索、产品应用或移动体验筛选。点击“放大预览”隐藏编辑面板。预览只显示示例界面、图像和必要操作，设计解释和来源放在下方输出区。

左侧组合布局、导航、视觉语言、内容语言和效果；右侧调整平台、画幅、列数、圆角、间距、颜色、时长、密度和模块顺序。切换 Android 自动选择手机画幅，Web/桌面选择桌面画幅，仍可手动改变画幅。Android 提供状态栏、内部滚动、底部导航和手势区；较宽画幅改用侧栏。平台外壳随同导出，Android HTML 仍为浏览器交互参考，不是 Compose 源码。

下方可复制 JSON、同源渲染的完整 HTML 或 Agent 提示词。JSON 导入严格校验，不运行导入的脚本。自定义组合可命名保存；最多 24 个预设，每个扩展总计 32 KiB 本机存储。旧版 v1 配置会迁移为 v2，保留全部原字段，并补上项目摘要内容语言和散点画布。

## 十二个起点

| 方向 | 主要结构与视觉 |
| --- | --- |
| 沉浸封面墙 | 深色、窄缝、大小磁贴、封面与展开转场 |
| 无界拼贴画布 | 世界坐标、大小磁贴 / 散点、平移缩放、真实视口裁剪 |
| 纸上展览 | 暖纸底色、衬线标题、不等高封面、编辑式留白 |
| 字体实验室 | 大字、酸性色块、硬边框线 |
| 精密项目台 | 清晰的侧栏、摘要、状态与对齐 |
| 万条素材目录 | 主从结构、动态高度虚拟列表、持久的展开状态 |
| 流光工作流 | 轻透层次、任务分列、跨列拖动 |
| 掌中灵感馆 | Android 手机外壳、触控尺寸、双列与底部导航 |
| 电影感叙事 | 大幅作品、横向切幕、极简文字 |
| 瑞士设计目录 | 编号、横线、大标题与缩略图 |
| 横向摄影札记 | 横向滚动、交替比例与作品展开 |
| 产品能力拼图 | 主次分明的 Bento 分区 |

十类布局、四类导航、五类视觉语言、三类内容语言和六类效果可组合。虚拟列表只适用于主从列表；一万条示例需要虚拟列表。视口裁剪只适用于画布；画布和虚拟列表关闭卡片拖拽排序。无效组合自动调整并显示原因，不以全量 DOM 冒充虚拟渲染。

## 参考研究

本轮查看了 [Godly](https://godly.website/)、[Siteinspire](https://www.siteinspire.com/)、[Awwwards 动态网格](https://www.awwwards.com/websites/dynamic-grid-layout-examples/) 和 [Codrops Creative Hub](https://tympanus.net/codrops/hub/)，结合以下具体参考：

- [Folia](https://github.com/chthollyphile/folia-major)：用户指定的沉浸式内容墙与网格交互参考。
- [Atelier North](https://atelier-north.crnacura.workers.dev/)：Codrops 2026 建筑工作室示例，观察编辑层级、留白与作品索引。
- [Infinite Scroll GSAP Gallery](https://tympanus.net/Tutorials/InfiniteScrollGSAPGallery/)：2026 画廊和作品详情转场参考。
- [Infinite Layers Grid](https://tympanus.net/Tutorials/InfiniteLayersGrid/) 与 [Grid to Fullscreen](https://tympanus.net/Tutorials/GridToFullscreenAnimations/)：空间网格与来源位置转场的技术参考。
- [Android 自适应布局](https://developer.android.com/develop/ui/compose/layouts/adaptive/list-detail)、[Windows 布局](https://learn.microsoft.com/en-us/windows/apps/design/basics/)、[TanStack Virtual](https://tanstack.com/virtual/latest/docs/introduction)：平台与性能原则。

Godly 和 Siteinspire 的页面已实际浏览；Codrops 的部分直接演示在自动化浏览器中出现访问挑战，因此同时使用其公开文章、目录及示例内容核对，未声称逐项体验所有第三方动画。本扩展为原创代码和原创 SVG 视觉研究，不复制站点源码、品牌、专辑封面或捆绑第三方库。引用说明设计原则来源，不表示原站模板已原样移植。

## 交互与输出

封面、字体和摘要可以独立于布局选择。非列表卡片通过来源位置动画进入作品详情；详情只保留作品图像、标题、分类和返回操作，Escape 返回并恢复焦点。长页面的详情使用视口内独立弹层，背景编辑区不可操作。系统减少动态效果或时长为零时直接切换；系统偏好在会话中变化也会生效。

画布示例节点有限，支持无边界相机平移、缩放、复位和裁剪，不包含节点编辑或协作。横向画册使用原生滚动和吸附，不冒充真正无限数据源。看板排序仅作用于示例数据。图像均为离线 SVG，不请求远程素材；无需额外权限。

预览和 HTML 共享验证、绘制及事件代码。Agent 输出包含当前参数、平台技术栈提示、视觉要求、性能要求与验收要求；引用仅在当前组合与所选起点对应时附加。实际产品仍需接入项目业务数据和状态。

## 开发与验证

源码：`extensions/design-studio/`。运行 `python scripts/build-design-studio.py` 生成 `extensions/examples/design-studio.toolhub-extension.json`。构建离线完成，分别检查 HTML 64 KiB、CSS 32 KiB、JS 64 KiB 和总包 256 KiB 限制。共享预览 CSS 在 HTML 的转义惰性 template 中保存，导出时读取，不需要内联脚本或降低原生 CSP。

本机包可通过扩展预览与版本升级流程安装；不需要改数据库或授予执行能力。同一包 ID 保留本机草稿、预设及版本回滚快照。Windows 原生 WebView 验证见本轮交付记录；macOS 实际交互未验证。
