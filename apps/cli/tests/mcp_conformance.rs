//! Process-level MCP conformance over the real shared daemon client.
use serde_json::{json, Value};
use std::io::{BufRead, BufReader, Write};
use std::process::{Child, ChildStdin, Command, Stdio};
use std::sync::mpsc::{self, Receiver};
use std::time::Duration;
fn binary(name: &str) -> std::path::PathBuf {
    let variable = if name == "toolhub" {
        "TOOLHUB_BIN"
    } else {
        "TOOLHUBD_BIN"
    };
    if let Some(path) = std::env::var_os(variable) {
        return path.into();
    }
    let exe = std::env::current_exe().unwrap();
    let mut dir = exe.parent().unwrap();
    if dir.ends_with("deps") {
        dir = dir.parent().unwrap();
    }
    dir.join(if cfg!(windows) {
        format!("{name}.exe")
    } else {
        name.into()
    })
}
struct McpClient {
    child: Child,
    daemon: Child,
    stdin: ChildStdin,
    lines: Receiver<String>,
    _dir: tempfile::TempDir,
}
impl McpClient {
    fn spawn(ready: bool) -> Self {
        let dir = tempfile::tempdir().unwrap();
        let db = dir.path().join("mcp.sqlite");
        let daemon = Command::new(binary("toolhubd"))
            .arg("--listen")
            .env("TOOLHUB_REGISTRY", &db)
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()
            .unwrap();
        std::thread::sleep(Duration::from_millis(150));
        let mut child = Command::new(binary("toolhub"))
            .args(["mcp", "serve"])
            .env("TOOLHUB_REGISTRY", &db)
            .env("TOOLHUBD_BIN", binary("toolhubd"))
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .spawn()
            .unwrap();
        let stdin = child.stdin.take().unwrap();
        let stdout = child.stdout.take().unwrap();
        let (sender, lines) = mpsc::channel();
        std::thread::spawn(move || {
            for line in BufReader::new(stdout).lines() {
                match line {
                    Ok(line) => {
                        if sender.send(line).is_err() {
                            break;
                        }
                    }
                    Err(_) => break,
                }
            }
        });
        let mut client = Self {
            child,
            daemon,
            stdin,
            lines,
            _dir: dir,
        };
        if ready {
            client.send(json!({"jsonrpc":"2.0","id":100,"method":"initialize","params":{"protocolVersion":"2024-11-05","capabilities":{},"clientInfo":{"name":"test","version":"1"}}}));
            assert!(client.read()["result"].is_object());
            client.send(json!({"jsonrpc":"2.0","method":"notifications/initialized"}));
        }
        client
    }
    fn send(&mut self, value: Value) {
        self.raw(&value.to_string());
    }
    fn raw(&mut self, line: &str) {
        writeln!(self.stdin, "{line}").unwrap();
        self.stdin.flush().unwrap();
    }
    fn read(&mut self) -> Value {
        serde_json::from_str(
            &self
                .lines
                .recv_timeout(Duration::from_secs(5))
                .expect("bounded MCP response"),
        )
        .unwrap()
    }
}
impl Drop for McpClient {
    fn drop(&mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
        let _ = self.daemon.kill();
        let _ = self.daemon.wait();
    }
}
#[test]
fn initialize_negotiates_only_supported_version() {
    let mut c = McpClient::spawn(false);
    c.send(json!({"jsonrpc":"2.0","id":1,"method":"initialize","params":{"protocolVersion":"2099-01-01","capabilities":{},"clientInfo":{"name":"test","version":"0"}}}));
    assert_eq!(c.read()["result"]["protocolVersion"], "2024-11-05");
}
#[test]
fn rejects_tool_requests_before_initialized() {
    let mut c = McpClient::spawn(false);
    c.send(json!({"jsonrpc":"2.0","id":1,"method":"tools/list","params":{}}));
    assert_eq!(c.read()["error"]["code"], -32600);
}
#[test]
fn notifications_get_no_reply_and_null_id_is_request() {
    let mut c = McpClient::spawn(true);
    c.send(json!({"jsonrpc":"2.0","method":"ping"}));
    c.send(json!({"jsonrpc":"2.0","id":null,"method":"tools/list","params":{}}));
    let value = c.read();
    assert!(value["id"].is_null());
    assert_eq!(value["result"]["tools"].as_array().unwrap().len(), 7);
}
#[test]
fn schema_types_enforced_and_recovery_after_malformed_json() {
    let mut c = McpClient::spawn(true);
    c.raw("{bad");
    assert_eq!(c.read()["error"]["code"], -32700);
    c.send(json!({"jsonrpc":"2.0","id":2,"method":"tools/call","params":{"name":"search_tools","arguments":{"query":false}}}));
    assert_eq!(c.read()["error"]["code"], -32602);
    c.send(json!({"jsonrpc":"2.0","id":3,"method":"tools/list","params":{}}));
    assert_eq!(c.read()["result"]["tools"].as_array().unwrap().len(), 7);
}
#[test]
fn tool_semantic_failure_is_content_and_is_error() {
    let mut c = McpClient::spawn(true);
    c.send(json!({"jsonrpc":"2.0","id":1,"method":"tools/call","params":{"name":"inspect_tool","arguments":{"id":"missing"}}}));
    let value = c.read();
    assert!(value.get("error").is_none(), "{value}");
    assert_eq!(value["result"]["isError"], true);
    assert!(value["result"]["content"][0]["text"].is_string());
}
#[test]
fn unknown_method_is_method_not_found_and_server_recovers() {
    let mut c = McpClient::spawn(true);
    c.send(json!({"jsonrpc":"2.0","id":1,"method":"resources/list","params":{}}));
    assert_eq!(c.read()["error"]["code"], -32601);
    c.send(json!({"jsonrpc":"2.0","id":2,"method":"tools/call","params":{"name":"search_tools","arguments":{"query":"python"}}}));
    assert_eq!(c.read()["result"]["isError"], false);
}
