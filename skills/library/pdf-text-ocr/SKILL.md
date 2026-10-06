---
name: toolhub-pdf-text-ocr
description: 经 ToolHub 查找 PDF 文字提取、渲染与 OCR 能力，提取本地 PDF 或扫描页文字；不自动安装 OCR 语言包。
---

# PDF 文字处理

先按用户指定文件查找 document.pdf.text.extract，检查实际工具路径与权限。先提取原生文字并抽查；扫描 PDF 或空文本再检查 document.pdf.render 和 document.ocr。OCR 前检查所需语言数据；缺依赖明确告知，不自动安装。

调用前检查输入范围和输出目录。需要执行批准时用 request_execution_approval，等待用户确认后继续。按页处理扫描件，保留原文件。长任务使用 background=true、工作目录内的 outputs 相对路径以及 get_task；取消用 cancel_task。

完成后检查输出非空并抽查识别质量，保留页码对应关系。说明扫描质量、版式和语言导致的误差，给出实际文件路径。不要宣称 OCR 文字完全准确。
