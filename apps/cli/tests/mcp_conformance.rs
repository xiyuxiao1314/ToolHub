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
    first_response: bool,
    _dir: tempfile::TempDir,
}
impl McpClient {
    fn spawn(ready: bool) -> Self {
        let dir = tempfile::tempdir().unwrap();
        let db = dir.path().join("mcp.sqlite");
        let daemon = Command::new(binary("toolhubd"))
            .arg("--listen")
            // This is an ordinary-client fixture. Do not pin/hash an unrelated
            // adjacent desktop image or accept a real desktop as its controller.
            .arg("--controller-image")
            .arg(dir.path().join("no-controller-image"))
            .env("TOOLHUB_REGISTRY", &db)
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::from(
                std::fs::File::create(dir.path().join("daemon.stderr")).unwrap(),
            ))
            .spawn()
            .unwrap();
        std::thread::sleep(Duration::from_millis(150));
        let mut child = Command::new(binary("toolhub"))
            .args(["mcp", "serve"])
            .env("TOOLHUB_REGISTRY", &db)
            .env("TOOLHUBD_BIN", binary("toolhubd"))
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::from(
                std::fs::File::create(dir.path().join("client.stderr")).unwrap(),
            ))
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
            first_response: true,
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
        // The first response also includes native process launch and the
        // production five-second managed-service readiness window. Allow cold
        // CI startup overhead separately; subsequent protocol replies keep the
        // original five-second bound and every semantic assertion is unchanged.
        let timeout = if self.first_response { 15 } else { 5 };
        self.first_response = false;
        let line = self.lines.recv_timeout(Duration::from_secs(timeout))
            .unwrap_or_else(|error| {
                let log = |name| std::fs::read_to_string(self._dir.path().join(name))
                    .unwrap_or_default().chars().take(4096).collect::<String>();
                panic!("bounded MCP response: {error:?}; client={:?}; daemon={:?}; client stderr={}; daemon stderr={}",
                    self.child.try_wait(), self.daemon.try_wait(), log("client.stderr"), log("daemon.stderr"));
            });
        serde_json::from_str(&line).unwrap()
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
    assert_eq!(c.read()["result"]["protocolVersion"], "2025-11-25");
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
    assert_eq!(value["result"]["tools"].as_array().unwrap().len(), 19);
}
#[test]
fn schema_types_enforced_and_recovery_after_malformed_json() {
    let mut c = McpClient::spawn(true);
    c.raw("{bad");
    assert_eq!(c.read()["error"]["code"], -32700);
    c.send(json!({"jsonrpc":"2.0","id":2,"method":"tools/call","params":{"name":"search_tools","arguments":{"query":false}}}));
    assert_eq!(c.read()["error"]["code"], -32602);
    c.send(json!({"jsonrpc":"2.0","id":3,"method":"tools/list","params":{}}));
    assert_eq!(c.read()["result"]["tools"].as_array().unwrap().len(), 19);
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

#[test]
fn all_meta_tools_have_complete_behavior_hints() {
    let mut client = McpClient::spawn(true);
    client.send(json!({"jsonrpc":"2.0","id":1,"method":"tools/list","params":{}}));
    let response = client.read();
    let tools = response["result"]["tools"].as_array().unwrap();
    let names: std::collections::BTreeSet<_> = tools
        .iter()
        .map(|tool| tool["name"].as_str().unwrap())
        .collect();
    let declared: std::collections::BTreeSet<_> = toolhub_mcp::META_TOOLS
        .iter()
        .map(|(name, _)| *name)
        .collect();
    assert_eq!(names, declared);
    assert_eq!(tools.len(), names.len(), "duplicate MCP name");
    for tool in tools {
        let hints = &tool["annotations"];
        for name in [
            "readOnlyHint",
            "destructiveHint",
            "idempotentHint",
            "openWorldHint",
        ] {
            assert!(hints[name].is_boolean(), "{}: {name}", tool["name"]);
        }
        // Every successful gateway call persists self-reported agent activity.
        assert_eq!(hints["readOnlyHint"], false);
        assert_eq!(hints["idempotentHint"], false);
        assert_eq!(hints["openWorldHint"], tool["name"] == "execute_tool");
        assert_eq!(
            hints["destructiveHint"],
            matches!(
                tool["name"].as_str().unwrap(),
                "execute_tool"
                    | "cancel_task"
                    | "complete_artifact_package"
                    | "review_artifact_package"
            )
        );
    }
}

fn call_tool(client: &mut McpClient, name: &str, arguments: Value, is_error: bool) -> Value {
    client.send(json!({"jsonrpc":"2.0","id":200,"method":"tools/call","params":{"name":name,"arguments":arguments}}));
    let response = client.read();
    assert!(response.get("error").is_none(), "{name}: {response}");
    let result = &response["result"];
    assert_eq!(result["isError"], is_error, "{name}: {response}");
    let text: Value = serde_json::from_str(result["content"][0]["text"].as_str().unwrap()).unwrap();
    assert_eq!(
        text, result["structuredContent"]["result"],
        "{name}: result formats differ"
    );
    text
}

#[test]
fn all_meta_tools_route_through_isolated_daemon_and_preserve_authority() {
    let mut client = McpClient::spawn(true);
    let artifact_dir = tempfile::Builder::new()
        .prefix("mcp-artifact-fixture-")
        .tempdir_in(std::env::current_dir().unwrap())
        .unwrap();
    let task_root = artifact_dir.path().join("task-output");
    std::fs::create_dir(&task_root).unwrap();
    let protected = task_root.join("fixture.txt");
    std::fs::write(&protected, "retain this task output").unwrap();

    assert_eq!(
        call_tool(
            &mut client,
            "search_tools",
            json!({"query":"absent-conformance-tool"}),
            false
        ),
        json!([])
    );
    // Unknown capability is a semantic error, never permission to execute.
    let resolution = call_tool(
        &mut client,
        "resolve_capability",
        json!({"capability":"conformance.absent"}),
        true,
    );
    assert!(resolution.is_object() || resolution.is_array());
    call_tool(
        &mut client,
        "inspect_tool",
        json!({"id":"missing-instance"}),
        true,
    );
    assert!(call_tool(&mut client, "list_environments", json!({}), false).is_array());
    assert_eq!(
        call_tool(&mut client, "search_programs", json!({}), false),
        json!([])
    );
    let proposal = call_tool(
        &mut client,
        "propose_program",
        json!({
            "name":"Conformance proposal", "kind":"command", "cwd":task_root,
            "command":"echo submitted-but-never-executed", "source":"conformance"
        }),
        false,
    );
    assert_eq!(proposal["status"], "pending");
    // Submitting metadata neither collects nor launches a program.
    assert_eq!(
        call_tool(&mut client, "search_programs", json!({}), false),
        json!([])
    );
    call_tool(
        &mut client,
        "get_task",
        json!({"execution_id":"missing-task"}),
        true,
    );
    call_tool(
        &mut client,
        "cancel_task",
        json!({"execution_id":"missing-task"}),
        true,
    );
    call_tool(
        &mut client,
        "execute_tool",
        json!({"instance_id":"missing-instance"}),
        true,
    );
    assert!(call_tool(&mut client, "search_skills", json!({}), false).is_array());
    call_tool(
        &mut client,
        "inspect_skill",
        json!({"id":"missing-skill"}),
        true,
    );
    // Metadata offers neither distribute to another host nor grant service control.
    call_tool(&mut client, "offer_skill_sync", json!({"path":task_root.join("absent-skill"),"source_agent":"conformance"}), true);
    let services = call_tool(&mut client, "list_services", json!({}), !cfg!(windows));
    if cfg!(windows) { assert_eq!(services["total"], 0); }
    call_tool(&mut client, "register_service", json!({"name":"Invalid PID fixture","pid":4294967294u32,"project_dir":task_root}), true);
    let package = call_tool(
        &mut client,
        "register_artifact_package",
        json!({
            "name":"Protected fixture", "task":"MCP conformance", "root":task_root,
            "files":["fixture.txt"]
        }),
        false,
    );
    let id = package["id"].as_str().unwrap();
    assert_eq!(package["package"]["priority"], "high");
    assert_eq!(package["auto_scope"], false);
    let listed = call_tool(&mut client, "list_artifact_packages", json!({}), false);
    assert!(listed
        .as_array()
        .unwrap()
        .iter()
        .any(|item| item["id"] == id));
    let inspected = call_tool(
        &mut client,
        "list_artifact_packages",
        json!({"id":id}),
        false,
    );
    assert_eq!(inspected["package"]["files"][0]["status"], "present");
    let completed = call_tool(
        &mut client,
        "complete_artifact_package",
        json!({
            "id":id, "no_longer_needed":true, "reason":"Fixture task succeeded"
        }),
        false,
    );
    assert_eq!(completed["status"], "retained");
    assert!(completed["cleanup_after"].is_null());
    call_tool(
        &mut client,
        "review_artifact_package",
        json!({
            "id":id, "decision":"recycle", "unused":true, "dependencies_checked":true,
            "reason":"Agent must not override high priority"
        }),
        true,
    );
    call_tool(
        &mut client,
        "request_execution_approval",
        json!({"instance_id":"missing-instance"}),
        true,
    );
    call_tool(
        &mut client,
        "request_execution_approval",
        json!({"request_id":"missing-request"}),
        true,
    );
    assert_eq!(
        std::fs::read_to_string(&protected).unwrap(),
        "retain this task output"
    );
}
