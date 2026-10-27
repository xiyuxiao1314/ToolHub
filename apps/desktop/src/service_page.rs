//! Explicit native action. The browser target is reread from the selected service record.
#[tauri::command]
pub async fn open_service_page(id: String) -> Result<(), String> {
    tauri::async_runtime::spawn_blocking(move || {
        let daemon = super::daemon_bin()?;
        let mut client =
            toolhub_ipc::RpcClient::connect_or_start(&daemon).map_err(|e| e.to_string())?;
        let response = client
            .call("service.page_url", serde_json::json!({"id":id}))
            .map_err(|e| e.to_string())?;
        let url = toolhub_core::service_url::normalize(
            response["url"].as_str().ok_or("请先设置页面地址")?,
        )?;
        #[cfg(windows)]
        {
            use windows::{
                core::{w, PCWSTR},
                Win32::{UI::Shell::ShellExecuteW, UI::WindowsAndMessaging::SW_SHOWNORMAL},
            };
            let wide: Vec<u16> = url.encode_utf16().chain(Some(0)).collect();
            let result = unsafe {
                ShellExecuteW(
                    None,
                    w!("open"),
                    PCWSTR(wide.as_ptr()),
                    PCWSTR::null(),
                    PCWSTR::null(),
                    SW_SHOWNORMAL,
                )
            };
            if result.0 as isize <= 32 {
                return Err(format!("默认浏览器打开失败（{}）", result.0 as isize));
            }
            Ok(())
        }
        #[cfg(not(windows))]
        {
            let _ = url;
            Err("当前服务监控仅支持 Windows".into())
        }
    })
    .await
    .map_err(|e| e.to_string())?
}
