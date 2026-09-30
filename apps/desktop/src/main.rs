//! ToolHub Desktop (Tauri 2 + React/TS). Talks to the shared daemon via JSON-RPC.

#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use serde_json::{json, Value};
use std::io::{BufRead, BufReader, Write};
use std::process::{Command, Stdio};
use std::sync::Mutex;

struct DaemonLink {
    #[allow(dead_code)]
    child: std::process::Child,
    stdin: std::process::ChildStdin,
    reader: BufReader<std::process::ChildStdout>,
}

impl DaemonLink {
    fn spawn() -> Result<Self, String> {
        // Prefer shared named pipe if present; else spawn toolhubd sibling.
        let bin = std::env::var("TOOLHUBD_BIN").unwrap_or_else(|_| {
            let mut p = std::env::current_exe().unwrap_or_default();
            p.pop();
            p.push(if cfg!(windows) {
                "toolhubd.exe"
            } else {
                "toolhubd"
            });
            p.to_string_lossy().to_string()
        });
        let mut child = Command::new(bin)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .spawn()
            .map_err(|e| e.to_string())?;
        let stdin = child.stdin.take().ok_or("stdin")?;
        let stdout = child.stdout.take().ok_or("stdout")?;
        Ok(Self {
            child,
            stdin,
            reader: BufReader::new(stdout),
        })
    }

    fn call(&mut self, method: &str, params: Value) -> Result<Value, String> {
        let id = 1;
        let req = json!({"jsonrpc":"2.0","id":id,"method":method,"params":params});
        writeln!(self.stdin, "{}", req).map_err(|e| e.to_string())?;
        self.stdin.flush().map_err(|e| e.to_string())?;
        let mut line = String::new();
        let n = self.reader.read_line(&mut line).map_err(|e| e.to_string())?;
        if n == 0 {
            return Err("daemon closed".into());
        }
        let resp: Value = serde_json::from_str(&line).map_err(|e| e.to_string())?;
        if let Some(err) = resp.get("error") {
            return Err(err.to_string());
        }
        Ok(resp.get("result").cloned().unwrap_or(Value::Null))
    }
}

#[tauri::command]
fn rpc(state: tauri::State<'_, Mutex<DaemonLink>>, method: String, params: Value) -> Result<Value, String> {
    let mut guard = state.lock().map_err(|e| e.to_string())?;
    guard.call(&method, params)
}

#[tauri::command]
fn app_versions() -> Value {
    json!({
        "app": "0.2.0",
        "core": "0.2.0",
        "protocol": toolhub_protocol::PROTOCOL_VERSION,
    })
}

fn main() {
    let link = DaemonLink::spawn().expect("start toolhubd");
    tauri::Builder::default()
        .plugin(tauri_plugin_shell::init())
        .manage(Mutex::new(link))
        .invoke_handler(tauri::generate_handler![rpc, app_versions])
        .run(tauri::generate_context!())
        .expect("error while running ToolHub Desktop");
}
