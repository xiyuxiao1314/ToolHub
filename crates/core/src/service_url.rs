//! User-selected service page links are browser URLs, never shell/protocol commands.
pub fn normalize(input: &str) -> Result<String, String> {
    if input.is_empty()
        || input.len() > 2048
        || input
            .chars()
            .any(|c| c.is_control() || c == '"' || c == '\\')
    {
        return Err("页面地址无效或超过 2048 字节".into());
    }
    let url =
        url::Url::parse(input.trim()).map_err(|_| "请输入完整的 http:// 或 https:// 页面地址")?;
    if !matches!(url.scheme(), "http" | "https")
        || url.host_str().is_none()
        || !url.username().is_empty()
        || url.password().is_some()
    {
        return Err("仅支持不含账号密码的 HTTP/HTTPS 地址".into());
    }
    if url.as_str().len() > 2048 {
        return Err("编码后的页面地址超过 2048 字节".into());
    }
    Ok(url.to_string())
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn validates_browser_targets() {
        for input in [
            "http://127.0.0.1:8000",
            "https://peach.local/",
            "http://[::1]:8900/?view=home",
        ] {
            assert!(normalize(input).is_ok())
        }
        for input in [
            "file:///C:/Windows/cmd.exe",
            "javascript:alert(1)",
            "data:text/html,a",
            "ms-settings:network",
            "https://user:pass@localhost/",
            "https://localhost/\n",
            "http://localhost/\"",
            "http://\\localhost",
            "http://",
        ] {
            assert!(normalize(input).is_err(), "{input}")
        }
        assert!(normalize(&format!("http://localhost/{}", "a".repeat(2048))).is_err());
    }
}
