---
name: toolhub-media-compress
description: 使用 ToolHub 查找本机 FFmpeg，按用户要求压缩视频或提取音频并验证输出；不用于下载来源不明的媒体。
---

# 视频与音频处理

先明确输入文件、目标格式、质量或大小要求。通过 search_tools 按任务查询，resolve_capability 解析 media.video.transcode 或 media.audio.transcode，再 inspect_tool 检查路径与信任状态。选择不表示获准执行。需要审批时 request_execution_approval 并等待用户在桌面批准，使用返回的会话和一次性批准继续。

使用参数数组，不拼接未引用的路径。输出存到用户指定目录；避免覆盖原文件。长任务可 execute_tool background=true，outputs 填工作目录内的相对文件名，用 get_task 查询；用户取消时 cancel_task。工具运行在现有策略约束下，最长 300 秒，超过时拆分任务或使用用户同意的终端流程。

根据源媒体选择编码器与质量；不要默认保证压缩比例。用 media.video.probe 验证输出时长、大小与媒体流。报告实际生成路径和验证结果。示例参数是说明，需要替换真实路径：-i input.mp4 -c:v libx264 -crf 24 output.mp4。
