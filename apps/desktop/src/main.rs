//! Native desktop bridge to the authenticated shared user service.
#![cfg_attr(windows, windows_subsystem = "windows")]
mod autostart;
mod file_icon;
mod startup_diagnostics;
#[cfg(windows)]
mod windows_startup_task;
use file_icon::tool_file_icon;
mod integration;
mod installation;
mod market;
use market::{market_add, market_export, market_inspect, market_list, market_pick, market_share};
#[cfg(target_os = "macos")]
mod macos;
mod program_picker;
use autostart::{autostart_set, autostart_status};
use integration::{agent_integration, agent_mcp_check};
use program_picker::{builtin_skill_paths, pick_program_path, pick_skill_folder};
use serde_json::{json, Value};
use std::path::{Path, PathBuf};

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

fn default_export_path() -> PathBuf {
    let docs = dirs::document_dir()
        .filter(|p| p.is_dir())
        .unwrap_or_else(std::env::temp_dir);
    docs.join("toolhub-export.json")
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
    json!({"app":env!("CARGO_PKG_VERSION"),"platform":std::env::consts::OS,"core":env!("CARGO_PKG_VERSION"),"protocol":toolhub_protocol::PROTOCOL_VERSION})
}

#[tauri::command]
fn default_export_path_string() -> String {
    default_export_path().to_string_lossy().to_string()
}

#[tauri::command]
fn save_text_file(path: String, contents: String) -> Result<String, String> {
    if contents.len() > 8 * 1024 * 1024 {
        return Err("file too large".into());
    }
    let path = PathBuf::from(path.trim());
    if path.as_os_str().is_empty() {
        return Err("path required".into());
    }
    if let Some(parent) = path.parent().filter(|p| !p.as_os_str().is_empty()) {
        std::fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    }
    std::fs::write(&path, contents).map_err(|e| e.to_string())?;
    Ok(path.to_string_lossy().to_string())
}

#[tauri::command]
fn read_text_file(path: String) -> Result<String, String> {
    let path = PathBuf::from(path.trim());
    if std::fs::metadata(&path).map_err(|e| e.to_string())?.len() > 8 * 1024 * 1024 {
        return Err("file too large".into());
    }
    std::fs::read_to_string(&path).map_err(|e| e.to_string())
}

/// Scanner roots are directories; resolve an executable input to its containing directory.
#[tauri::command]
fn scan_root_for_path(path: String) -> Result<String, String> {
    let path = PathBuf::from(path.trim());
    if !path.is_absolute() {
        return Err("absolute root required".into());
    }
    let metadata = std::fs::metadata(&path).map_err(|e| e.to_string())?;
    let root = if metadata.is_dir() {
        path
    } else if metadata.is_file() {
        path.parent()
            .ok_or("tool directory unavailable")?
            .to_path_buf()
    } else {
        return Err("tool file or directory required".into());
    };
    Ok(root.to_string_lossy().to_string())
}

/// Open a terminal in the tool's directory (or parent of the executable).
#[tauri::command]
fn open_terminal(path: String) -> Result<String, String> {
    let raw = path.trim();
    if raw.is_empty() {
        return Err("path required".into());
    }
    let p = Path::new(raw);
    if !p.exists() {
        return Err(format!("path not found: {raw}"));
    }
    let cwd = if p.is_dir() {
        p.to_path_buf()
    } else {
        p.parent()
            .map(Path::to_path_buf)
            .unwrap_or_else(|| PathBuf::from("."))
    };
    if !cwd.exists() {
        return Err(format!("path not found: {}", cwd.display()));
    }
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        // Pass the working directory directly; a path must never become shell syntax.
        std::process::Command::new("cmd.exe")
            .arg("/K")
            .current_dir(&cwd)
            .creation_flags(0x00000010) // CREATE_NEW_CONSOLE
            .spawn()
            .map_err(|e| e.to_string())?;
    }
    #[cfg(target_os = "macos")]
    {
        macos::open_terminal(&cwd)?;
    }
    #[cfg(not(any(windows, target_os = "macos")))]
    {
        std::process::Command::new("x-terminal-emulator")
            .current_dir(&cwd)
            .spawn()
            .map_err(|e| e.to_string())?;
    }
    Ok(cwd.to_string_lossy().to_string())
}

/// Reveal a path in Explorer (Windows) or open directory.
#[tauri::command]
fn reveal_path(path: String) -> Result<String, String> {
    let raw = path.trim();
    if raw.is_empty() {
        return Err("path required".into());
    }
    if !Path::new(raw).exists() {
        return Err(format!("path not found: {raw}"));
    }
    #[cfg(windows)]
    {
        // explorer.exe returns exit code 1 even on success when selecting a file.
        std::process::Command::new("explorer.exe")
            .arg(format!("/select,{raw}"))
            .spawn()
            .map_err(|e| e.to_string())?;
    }
    #[cfg(not(windows))]
    {
        std::process::Command::new("open")
            .arg(raw)
            .spawn()
            .map_err(|e| e.to_string())?;
    }
    Ok(raw.to_string())
}

#[tauri::command]
fn open_program_folder(path: String) -> Result<String, String> {
    let p = PathBuf::from(path.trim());
    if !p.is_absolute() {
        return Err("请选择绝对路径".into());
    }
    let folder = if p.is_file() {
        p.parent().ok_or("程序目录不可用")?.to_path_buf()
    } else {
        p
    };
    if !folder.is_dir() {
        return Err("程序目录已失效".into());
    }
    #[cfg(windows)]
    {
        std::process::Command::new("explorer.exe")
            .arg(&folder)
            .spawn()
            .map_err(|e| e.to_string())?;
    }
    #[cfg(not(windows))]
    {
        std::process::Command::new("open")
            .arg(&folder)
            .spawn()
            .map_err(|e| e.to_string())?;
    }
    Ok(folder.to_string_lossy().into())
}

fn main() {
    let login_launch = std::env::args_os().any(|arg| arg == "--autostart");
    if login_launch {
        startup_diagnostics::record("process_started", None);
    }
    let result = tauri::Builder::default()
        .setup(move |_app| {
            if login_launch {
                startup_diagnostics::record("window_created", None);
            }
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            rpc,
            app_versions,
            autostart_status,
            autostart_set,
            agent_integration,
            agent_mcp_check,
            market_list,
            market_pick,
            market_inspect,
            market_add,
            market_share,
            market_export,
            tool_file_icon,
            default_export_path_string,
            save_text_file,
            read_text_file,
            open_terminal,
            reveal_path,
            scan_root_for_path,
            pick_program_path,
            pick_skill_folder,
            builtin_skill_paths,
            open_program_folder
        ])
        .run(tauri::generate_context!());
    if let Err(error) = result {
        if login_launch {
            startup_diagnostics::record("startup_failed", Some(&error.to_string()));
        }
        eprintln!("ToolHub Desktop: {error}");
        std::process::exit(1);
    }
}
