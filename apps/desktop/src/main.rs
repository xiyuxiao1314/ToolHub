//! Native desktop bridge to the authenticated shared user service.
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]
use serde_json::{json, Value};
fn daemon_bin() -> Result<std::path::PathBuf, String> {
    if let Some(path) = std::env::var_os("TOOLHUBD_BIN") {
        return Ok(path.into());
    }
    let exe = std::env::current_exe().map_err(|error| error.to_string())?;
    Ok(exe
        .parent()
        .ok_or("desktop executable directory unavailable")?
        .join(if cfg!(windows) {
            "toolhubd.exe"
        } else {
            "toolhubd"
        }))
}
#[tauri::command]
async fn rpc(method: String, params: Value) -> Result<Value, String> {
    // Each request has a separate authenticated connection, so cancellation and health queries
    // remain responsive while another request is executing or scanning.
    tauri::async_runtime::spawn_blocking(move || {
        let daemon = daemon_bin()?;
        let mut client =
            toolhub_ipc::RpcClient::connect_or_start(&daemon).map_err(|error| error.to_string())?;
        client
            .call(&method, params)
            .map_err(|error| error.to_string())
    })
    .await
    .map_err(|error| format!("desktop worker failed: {error}"))?
}
#[tauri::command]
fn app_versions() -> Value {
    json!({"app":env!("CARGO_PKG_VERSION"),"core":env!("CARGO_PKG_VERSION"),"protocol":toolhub_protocol::PROTOCOL_VERSION})
}
fn main() {
    tauri::Builder::default()
        .invoke_handler(tauri::generate_handler![rpc, app_versions])
        .run(tauri::generate_context!())
        .expect("error while running ToolHub Desktop");
}
