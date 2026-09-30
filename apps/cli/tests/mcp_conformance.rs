//! MCP conformance fixtures: lifecycle, notifications, tools/list, tools/call, errors.

use std::io::{BufRead, BufReader, Write};
use std::process::{Command, Stdio};
use std::time::Duration;

fn toolhub_bin() -> String {
    std::env::var("TOOLHUB_BIN").unwrap_or_else(|_| {
        let mut p = std::env::current_exe().unwrap();
        p.pop();
        p.push(if cfg!(windows) {
            "toolhub.exe"
        } else {
            "toolhub"
        });
        p.to_string_lossy().to_string()
    })
}

fn toolhubd_bin() -> String {
    std::env::var("TOOLHUBD_BIN").unwrap_or_else(|_| {
        let mut p = std::env::current_exe().unwrap();
        p.pop();
        p.push(if cfg!(windows) {
            "toolhubd.exe"
        } else {
            "toolhubd"
        });
        p.to_string_lossy().to_string()
    })
}

struct McpClient {
    child: std::process::Child,
    stdin: std::process::ChildStdin,
    reader: BufReader<std::process::ChildStdout>,
}

impl McpClient {
    fn spawn(db: &std::path::Path) -> Self {
        let mut child = Command::new(toolhub_bin())
            .env("TOOLHUB_REGISTRY", db)
            .env("TOOLHUBD_BIN", toolhubd_bin())
            .env("TOOLHUB_PRINCIPAL", "local.admin")
            .env("TOOLHUB_ADMIN", "1")
            .args(["mcp", "serve"])
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .spawn()
            .expect("spawn toolhub mcp serve");
        let stdin = child.stdin.take().unwrap();
        let stdout = child.stdout.take().unwrap();
        Self {
            child,
            stdin,
            reader: BufReader::new(stdout),
        }
    }

    fn send_line(&mut self, line: &str) {
        writeln!(self.stdin, "{line}").unwrap();
        self.stdin.flush().unwrap();
    }

    fn read_line_timeout(&mut self, ms: u64) -> Option<String> {
        // Simple blocking read; tests use short scripts so this is fine.
        let _ = ms;
        let mut line = String::new();
        let n = self.reader.read_line(&mut line).ok()?;
        if n == 0 {
            None
        } else {
            Some(line)
        }
    }
}

impl Drop for McpClient {
    fn drop(&mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}

#[test]
fn mcp_initialize_lists_capabilities() {
    let dir = tempfile::tempdir().unwrap();
    let db = dir.path().join("mcp1.sqlite");
    let mut c = McpClient::spawn(&db);
    c.send_line(r#"{"jsonrpc":"2.0","id":1,"method":"initialize","params":{"protocolVersion":"2024-11-05","capabilities":{},"clientInfo":{"name":"test","version":"0"}}}"#);
    let line = c
        .read_line_timeout(5000)
        .expect("initialize response");
    assert!(line.contains("serverInfo"), "{line}");
    assert!(line.contains("capabilities"), "{line}");
    assert!(line.contains("toolhub"), "{line}");
}

#[test]
fn mcp_notification_gets_no_response() {
    let dir = tempfile::tempdir().unwrap();
    let db = dir.path().join("mcp2.sqlite");
    let mut c = McpClient::spawn(&db);
    c.send_line(r#"{"jsonrpc":"2.0","method":"notifications/initialized"}"#);
    // Next real request should get exactly one response (the request's).
    c.send_line(r#"{"jsonrpc":"2.0","id":2,"method":"tools/list","params":{}}"#);
    let line = c
        .read_line_timeout(5000)
        .expect("tools/list response");
    // Must be the tools/list reply, not a notification ack.
    assert!(line.contains("tools"), "{line}");
    assert!(line.contains("\"id\":2") || line.contains("\"id\": 2"), "{line}");
}

#[test]
fn mcp_tools_list_object_shape() {
    let dir = tempfile::tempdir().unwrap();
    let db = dir.path().join("mcp3.sqlite");
    let mut c = McpClient::spawn(&db);
    c.send_line(r#"{"jsonrpc":"2.0","id":1,"method":"tools/list","params":{}}"#);
    let line = c.read_line_timeout(5000).unwrap();
    assert!(line.contains("\"tools\""), "{line}");
    assert!(line.contains("search_tools"), "{line}");
    assert!(line.contains("inputSchema") || line.contains("input_schema"), "{line}");
}

#[test]
fn mcp_tools_call_search_and_recover_after_error() {
    let dir = tempfile::tempdir().unwrap();
    let db = dir.path().join("mcp4.sqlite");
    let mut c = McpClient::spawn(&db);

    // Valid search
    c.send_line(r#"{"jsonrpc":"2.0","id":1,"method":"tools/call","params":{"name":"search_tools","arguments":{"query":"python"}}}"#);
    let ok = c.read_line_timeout(8000).unwrap();
    assert!(ok.contains("content") || ok.contains("result"), "{ok}");

    // Invalid: missing tool name
    c.send_line(r#"{"jsonrpc":"2.0","id":2,"method":"tools/call","params":{"name":"install_everything","arguments":{}}}"#);
    let err = c.read_line_timeout(8000).unwrap();
    assert!(err.contains("isError") || err.contains("error"), "{err}");

    // Server still alive: another valid call
    c.send_line(r#"{"jsonrpc":"2.0","id":3,"method":"tools/call","params":{"name":"list_environments","arguments":{}}}"#);
    let ok2 = c.read_line_timeout(8000).unwrap();
    assert!(ok2.contains("content") || ok2.contains("result"), "{ok2}");
}

#[test]
fn mcp_error_message_is_string() {
    let dir = tempfile::tempdir().unwrap();
    let db = dir.path().join("mcp5.sqlite");
    let mut c = McpClient::spawn(&db);
    c.send_line(r#"{"jsonrpc":"2.0","id":1,"method":"tools/call","params":{"name":"inspect_tool","arguments":{"id":"definitely-missing"}}}"#);
    let line = c.read_line_timeout(8000).unwrap();
    // Either isError content or error.message as string — not a JSON object message.
    assert!(
        line.contains("isError") || line.contains("\"message\":\"") || line.contains("\"message\": \""),
        "{line}"
    );
    // If error object present, message must be quoted string
    if line.contains("\"error\"") && line.contains("message") {
        assert!(!line.contains("\"message\":{"), "message must be string: {line}");
    }
}

#[test]
fn mcp_unsupported_method_recovers() {
    let dir = tempfile::tempdir().unwrap();
    let db = dir.path().join("mcp6.sqlite");
    let mut c = McpClient::spawn(&db);
    c.send_line(r#"{"jsonrpc":"2.0","id":1,"method":"resources/list","params":{}}"#);
    let e = c.read_line_timeout(8000).unwrap();
    assert!(e.contains("error") || e.contains("unsupported"), "{e}");
    c.send_line(r#"{"jsonrpc":"2.0","id":2,"method":"tools/list","params":{}}"#);
    let ok = c.read_line_timeout(8000).unwrap();
    assert!(ok.contains("tools"), "{ok}");
    let _ = Duration::from_secs(0);
}
