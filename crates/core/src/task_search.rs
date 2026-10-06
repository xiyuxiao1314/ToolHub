//! Reviewed task aliases shared by UI, CLI and MCP through registry.search.
#[derive(Clone, Copy, serde::Serialize)]
pub struct TaskHint {
    pub capability: &'static str,
    pub label: &'static str,
    pub terms: &'static [&'static str],
    pub example: &'static str,
    pub permission: &'static str,
}
pub const TASK_HINTS: &[TaskHint] = &[
    TaskHint {
        capability: "media.video.transcode",
        label: "视频压缩 / 转码",
        terms: &["视频", "压缩视频", "转码", "video", "ffmpeg"],
        example: "ffmpeg -i input.mp4 -c:v libx264 -crf 24 output.mp4",
        permission: "读取输入文件，写入输出目录",
    },
    TaskHint {
        capability: "media.audio.transcode",
        label: "提取音频 / 音频转换",
        terms: &["音频", "提取音频", "audio"],
        example: "ffmpeg -i input.mp4 -vn output.mp3",
        permission: "读取媒体文件，写入输出目录",
    },
    TaskHint {
        capability: "media.video.frame.extract",
        label: "视频截图 / 提取帧",
        terms: &["视频截图", "提取帧", "截帧"],
        example: "ffmpeg -i input.mp4 -frames:v 1 frame.png",
        permission: "读取视频，写入图片",
    },
    TaskHint {
        capability: "document.pdf.text.extract",
        label: "提取 PDF 文字",
        terms: &["pdf", "提取文字", "文档文字"],
        example: "pdftotext input.pdf output.txt",
        permission: "读取 PDF，写入文本",
    },
    TaskHint {
        capability: "document.pdf.render",
        label: "PDF 转图片",
        terms: &["pdf", "扫描", "pdf转图片"],
        example: "pdftoppm -png input.pdf page",
        permission: "读取 PDF，写入图片",
    },
    TaskHint {
        capability: "document.ocr",
        label: "扫描件 / 图片文字识别",
        terms: &["ocr", "识别", "扫描", "图片文字"],
        example: "tesseract page.png output -l chi_sim+eng",
        permission: "读取图片，写入文本；检查语言数据",
    },
    TaskHint {
        capability: "archive.extract",
        label: "解压文件",
        terms: &["解压", "压缩包", "archive", "7zip", "7-zip"],
        example: "7z x archive.zip -ooutput",
        permission: "读取压缩包，写入解压目录",
    },
    TaskHint {
        capability: "archive.create",
        label: "创建压缩包",
        terms: &["压缩文件", "打包文件", "压缩包"],
        example: "7z a output.zip input",
        permission: "读取源文件，写入压缩包",
    },
    TaskHint {
        capability: "document.convert",
        label: "文档格式转换",
        terms: &["文档转换", "markdown", "pandoc", "word"],
        example: "pandoc input.md -o output.docx",
        permission: "读取源文档，写入目标文档",
    },
    TaskHint {
        capability: "media.image.convert",
        label: "图片格式转换",
        terms: &["图片转换", "图片缩放", "imagemagick"],
        example: "magick input.png -resize 50% output.jpg",
        permission: "读取图片，写入图片",
    },
];
pub fn matching(query: &str) -> Vec<TaskHint> {
    let query = query.trim().to_lowercase();
    if query.is_empty() {
        return vec![];
    }
    TASK_HINTS
        .iter()
        .filter(|hint| {
            query == hint.capability || hint.terms.iter().any(|term| query.contains(term))
        })
        .copied()
        .collect()
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn chinese_tasks_resolve_to_reviewed_capabilities() {
        assert!(matching("帮我识别扫描 PDF")
            .iter()
            .any(|h| h.capability == "document.ocr"));
        assert!(matching("提取音频")
            .iter()
            .any(|h| h.capability == "media.audio.transcode"));
        assert!(matching("unrelated-tool").is_empty());
    }
}
