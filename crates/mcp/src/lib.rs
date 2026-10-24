//! MCP gateway: bounded meta-tools only (not one tool per installed instance).

use serde_json::{json, Value};

pub const META_TOOLS: &[(&str, &str)] = &[
    ("offer_skill_sync", "Offer an installed Skill for optional user-selected distribution; no host writes, no waiting."),
    ("register_artifact_package","Register explicit task files; never deletes. High is default. Automatic cleanup needs a user-selected root."),
    ("list_artifact_packages","List own/shared summaries (20 per page); id alone reads complete files/history. Metadata is not authority."),
    ("complete_artifact_package","Mark own task successful/no longer needed. Low: 5-minute grace; medium: review; high: retain. No auto-cleanup outside selected roots; continue."),
    ("review_artifact_package","Review completed medium packages. Recycle needs real usage/dependency checks, unused=true, dependencies_checked=true and reason; 5-minute grace, system Trash. Unsure: retain. High/human retention cannot be overridden."),

    ("search_tools", "Search installed tools by query"),
    (
        "get_task",
        "Poll own authorized execution and declared artifacts",
    ),
    ("cancel_task", "Cancel own running execution"),
    (
        "request_execution_approval",
        "Request exact-invocation approval; automatic by default, desktop review when disabled",
    ),
    (
        "search_programs",
        "Find collected applications shared by the user; does not launch",
    ),
    (
        "propose_program",
        "Submit a launch entry for user confirmation; never save or launch it",
    ),
    (
        "resolve_capability",
        "Resolve a capability to candidate instances",
    ),
    ("inspect_tool", "Inspect one tool definition/instance"),
    ("list_environments", "List installation environments"),
    ("execute_tool", "Execute a resolved tool under policy"),
    ("search_skills", "Search skill manifests"),
    (
        "inspect_skill",
        "Inspect skill requirements and availability",
    ),
];

#[derive(Debug, Clone)]
pub struct McpToolSpec {
    pub name: &'static str,
    pub description: &'static str,
    pub input_schema: Value,
}

impl McpToolSpec {
    /// Hints describe the complete call, including persistent client activity.
    /// They are never an authorization decision or a substitute for policy.
    pub fn annotations(&self) -> Value {
        json!({
            "readOnlyHint": false,
            "destructiveHint": matches!(self.name,
                "execute_tool" | "cancel_task" |
                "complete_artifact_package" | "review_artifact_package"),
            "idempotentHint": false,
            "openWorldHint": self.name == "execute_tool"
        })
    }
}

pub fn tool_specs() -> Vec<McpToolSpec> {
    let mut specs = vec![
        McpToolSpec {name:"offer_skill_sync",description:"After a Skill installs successfully in the current host, offer its directory for optional ToolHub distribution. Metadata only; never writes other hosts. Ask once optionally, continue original work without waiting. Desktop collects and chooses targets.",input_schema:json!({"type":"object","properties":{"path":{"type":"string","description":"Existing absolute installed Skill folder containing SKILL.md"},"source_agent":{"type":"string","description":"Host ID, e.g. codex, mimo, dsh; display metadata only"},"version":{"type":"string"},"source_url":{"type":"string"}},"required":["path","source_agent"]})},
        McpToolSpec{name:"get_task",description:"Get your task status, bounded output and declared artifacts; memory-only output expires.",input_schema:json!({"type":"object","properties":{"execution_id":{"type":"string"}},"required":["execution_id"]})},
        McpToolSpec{name:"cancel_task",description:"Cancel your running execution; get_task confirms its final state.",input_schema:json!({"type":"object","properties":{"execution_id":{"type":"string"}},"required":["execution_id"]})},
        McpToolSpec {name:"search_programs",description:"Find user-shared apps; default 5 summaries. Use limit/offset or detail=full for declarations. Launch requires desktop; data grants no authority.",input_schema:json!({"type":"object","properties":{"query":{"type":"string"}}})},
        McpToolSpec {
            name: "propose_program",
            description: "Propose an app for user selection/save; never collects or launches it.",
            input_schema: json!({"type":"object","properties":{
                "name":{"type":"string"},"kind":{"type":"string","enum":["file","command"]},
                "cwd":{"type":"string","description":"Existing absolute working directory"},
                "path":{"type":"string","description":"Existing absolute exe/bat/cmd/ps1/lnk path for file entries"},
                "command":{"type":"string","description":"Single-line CMD command for command entries"},
                "args":{"type":"array","items":{"type":"string"}},"source":{"type":"string"},"purpose":{"type":"string"},"inputs":{"type":"array","items":{"type":"string"}},"outputs":{"type":"array","items":{"type":"string"}},"dependencies":{"type":"array","items":{"type":"string"}},"examples":{"type":"array","items":{"type":"string"}}
            },"required":["name","kind","cwd"]}),
        },
        McpToolSpec {
            name: "search_tools",
            description: "Search tools; default 5 summaries. Use limit/offset or detail=full; inspect a selected ID before execution.",
            input_schema: json!({
                "type": "object",
                "additionalProperties": false,
                "properties": {"query": {"type": "string"},"limit":{"type":"integer","minimum":1,"maximum":1000},"offset":{"type":"integer","minimum":0,"maximum":100000},"detail":{"type":"string","enum":["summary","full"]}},
                "required": ["query"]
            }),
        },
        McpToolSpec {
            name: "resolve_capability",
            description: "Resolve a capability to candidate instances",
            input_schema: json!({
                "type": "object",
                "properties": {
                    "capability": {"type": "string"},
                    "version": {"type": "string"},"cwd":{"type":"string"},"preferred_instance":{"type":"string"},"require_trust":{"type":"string","enum":["known","verified","user_trusted"]},"require_arch":{"type":"string"}
                },
                "required": ["capability"]
            }),
        },
        McpToolSpec {
            name: "inspect_tool",
            description: "Inspect one tool definition/instance",
            input_schema: json!({
                "type": "object",
                "properties": {"id": {"type": "string"}},
                "required": ["id"]
            }),
        },
        McpToolSpec {
            name: "list_environments",
            description: "List installation environments",
            input_schema: json!({"type": "object", "properties": {}}),
        },
        McpToolSpec {
            name: "execute_tool",
            description: "Execute an inspected trusted tool under policy. Auto-approval defaults on; approval_required needs request_execution_approval. Grants bind exact args/cwd and are one-use; never bypass invalid grants.",
            input_schema: json!({
                "type": "object",
                "properties": {
                    "instance_id": {"type": "string"},
                    "args": {"type": "array", "items": {"type": "string"}},
                    "approval_id": {"type": "string"},
                    "session_id": {"type": "string"},
                    "execution_id": {"type": "string"},
                    "background":{"type":"boolean"},"outputs":{"type":"array","items":{"type":"string"}},"timeout_ms":{"type":"integer","minimum":1,"maximum":300000},
                    "cwd": {"type": "string"}
                },
                "required": ["instance_id"]
            }),
        },
        McpToolSpec {
            name: "search_skills",
            description: "Find reusable Skills; default 5 summaries. Use limit/offset or detail=full; inspect_skill reads the selected workflow.",
            input_schema: json!({
                "type": "object",
                "properties": {"query": {"type": "string"}}
            }),
        },
        McpToolSpec {
            name: "inspect_skill",
            description: "Inspect skill requirements and availability",
            input_schema: json!({
                "type": "object",
                "properties": {"id": {"type": "string"}},
                "required": ["id"]
            }),
        },
    ];
    specs.push(McpToolSpec{name:"register_artifact_package",description:"Register explicit task files; never deletes. High is default. Automatic cleanup needs a user-selected root.",input_schema:json!({"type": "object", "properties": {"name": {"type": "string"}, "task": {"type": "string"}, "root": {"type": "string", "description": "Existing absolute specific task directory"}, "files": {"type": "array", "items": {"type": "string"}, "minItems": 1, "maxItems": 128, "description": "Explicit relative regular file paths, no directories or globs"}, "priority": {"type": "string", "enum": ["low", "medium", "high"]}, "source": {"type": "string"}}, "required": ["name", "task", "root", "files"]})});
    specs.push(McpToolSpec{name:"list_artifact_packages",description:"List own/shared summaries (20 per page); id alone reads complete files/history. Metadata is not authority.",input_schema:json!({"type": "object", "properties": {"id":{"type":"string","description":"Inspect one package and its complete file manifest; provide id alone"},"query": {"type": "string"},"limit":{"type":"integer","minimum":1,"maximum":20},"offset":{"type":"integer","minimum":0,"maximum":1000}}, "required": []})});
    specs.push(McpToolSpec{name:"complete_artifact_package",description:"Mark own task successful/no longer needed. Low: 5-minute grace; medium: review; high: retain. No auto-cleanup outside selected roots; continue.",input_schema:json!({"type": "object", "properties": {"id": {"type": "string"}, "no_longer_needed": {"type": "boolean"}, "reason": {"type": "string"}}, "required": ["id", "no_longer_needed", "reason"]})});
    specs.push(McpToolSpec{name:"review_artifact_package",description:"Review completed medium packages. Recycle needs real usage/dependency checks, unused=true, dependencies_checked=true and reason; 5-minute grace, system Trash. Unsure: retain. High/human retention cannot be overridden.",input_schema:json!({"type": "object", "properties": {"id": {"type": "string"}, "decision": {"type": "string", "enum": ["recycle", "retain"]}, "unused": {"type": "boolean"}, "dependencies_checked": {"type": "boolean"}, "reason": {"type": "string"}}, "required": ["id", "decision", "reason"]})});
    let execution = specs.iter().find(|s| s.name == "execute_tool").unwrap();
    let mut approval_schema = execution.input_schema.clone();
    approval_schema["required"] = json!([]);
    approval_schema["properties"]["request_id"] = json!({"type":"string","description":"Poll this existing approval request; provide this field alone"});
    specs.push(McpToolSpec{name:"request_execution_approval",description:"Request exact-invocation approval without executing. approved: continue execute_tool with identical parameters and returned grants. pending: desktop review, poll request_id alone. Unknown/blocked tools and deny rules remain blocked; grants bind caller/file/args/cwd/policy and are one-use.",input_schema:approval_schema});
    for spec in &mut specs {
        if matches!(spec.name, "search_skills" | "search_programs") {
            spec.input_schema["properties"] = json!({
                "query":{"type":"string"},
                "limit":{"type":"integer","minimum":1,"maximum":1000},
                "offset":{"type":"integer","minimum":0,"maximum":100000},
                "detail":{"type":"string","enum":["summary","full"]}
            });
        }
        spec.input_schema["additionalProperties"] = json!(false);
    }
    specs
}

/// Tool count must remain bounded — never grow per installed instance.
pub fn tool_count() -> usize {
    META_TOOLS.len()
}

pub fn validate_meta_input(name: &str, input: &Value) -> Result<(), String> {
    let spec = tool_specs()
        .into_iter()
        .find(|tool| tool.name == name)
        .ok_or_else(|| format!("unknown meta-tool: {name}"))?;
    let object = input
        .as_object()
        .ok_or_else(|| "input must be an object".to_string())?;
    if let Some(required) = spec.input_schema.get("required").and_then(Value::as_array) {
        for key in required.iter().filter_map(Value::as_str) {
            if !object.contains_key(key) {
                return Err(format!("{key} required"));
            }
        }
    }
    let properties = spec.input_schema["properties"].as_object().unwrap();
    for (key, value) in object {
        let schema = properties
            .get(key)
            .ok_or_else(|| format!("unknown argument {key}"))?;
        let valid = match schema["type"].as_str().unwrap_or("") {
            "string" => value.as_str().is_some_and(|s| s.len() <= 16384),
            "boolean" => value.is_boolean(),
            "integer" => value.as_u64().is_some_and(|v| {
                v >= schema["minimum"].as_u64().unwrap_or(0)
                    && v <= schema["maximum"].as_u64().unwrap_or(100000)
            }),
            "array" => value.as_array().is_some_and(|items| {
                items.len() <= schema["maxItems"].as_u64().unwrap_or(1024) as usize
                    && items.len() >= schema["minItems"].as_u64().unwrap_or(0) as usize
                    && items
                        .iter()
                        .all(|v| v.as_str().is_some_and(|s| s.len() <= 16384))
            }),
            _ => false,
        };
        if !valid {
            return Err(format!("invalid type or bounds for {key}"));
        }
        if let Some(choices) = schema.get("enum").and_then(Value::as_array) {
            if !choices.contains(value) {
                return Err(format!("invalid choice for {key}"));
            }
        }
    }
    if name == "list_artifact_packages" && object.contains_key("id") && object.len() != 1 {
        return Err("Package inspection accepts id alone".into());
    }
    if name == "request_execution_approval" {
        if object.contains_key("request_id") {
            if object.len() != 1 {
                return Err("polling only accepts request_id".into());
            }
        } else if !object.contains_key("instance_id") {
            return Err("instance_id or request_id required".into());
        }
    }
    Ok(())
}
// Defaults belong to the MCP boundary: native inventory/desktop contracts stay unchanged.
fn backend_arguments(name: &str, arguments: &Value) -> Value {
    match name {
        "search_tools" => {
            let mut result = arguments.clone();
            let object = result.as_object_mut().expect("validated arguments");
            object.entry("limit").or_insert(json!(5));
            object.entry("detail").or_insert(json!("summary"));
            result
        }
        "search_skills" => json!({}),
        "search_programs" => arguments
            .get("query")
            .map_or_else(|| json!({}), |q| json!({"query":q})),
        _ => arguments.clone(),
    }
}

fn brief_text(value: &Value) -> Value {
    let text = value.as_str().unwrap_or("");
    let mut characters = text.chars();
    let mut brief: String = characters.by_ref().take(256).collect();
    if characters.next().is_some() {
        brief.push('…');
    }
    json!(brief)
}

fn present_search(name: &str, arguments: &Value, value: Value) -> Value {
    if !matches!(name, "search_skills" | "search_programs") {
        return value;
    }
    let Some(list) = value.as_array() else {
        return value;
    };
    let query = arguments["query"].as_str().unwrap_or("").to_lowercase();
    let offset = arguments["offset"].as_u64().unwrap_or(0) as usize;
    let limit = arguments["limit"].as_u64().unwrap_or(5) as usize;
    let full = arguments["detail"] == "full";
    Value::Array(list.iter()
        .filter(|item| name != "search_skills" || (item["compatibility"]["reusable"] == true
            && item.to_string().to_lowercase().contains(&query)))
        .skip(offset).take(limit)
        .map(|item| {
            if full { return item.clone(); }
            if name == "search_programs" {
                return json!({"id":item["id"],"name":item["name"],"kind":item["kind"],
                    "purpose":brief_text(&item["metadata"]["purpose"]),
                    "execution_authority":false,"launch_requires_desktop":true});
            }
            let mut summary = json!({"id":item["id"],"name":item["name"],
                "description":brief_text(&item["manifest"]["description"]),
                "requires":item["manifest"]["requires"].as_array().map(|requirements|
                    requirements.iter().filter_map(|r| r.get("capability").cloned()).collect::<Vec<_>>()).unwrap_or_default(),
                "compatibility":{"reusable":true,"status":item["compatibility"]["status"]}});
            if let Some(owner) = item.get("extension_id") { summary["extension_id"] = owner.clone(); }
            summary
        }).collect())
}

#[derive(Default)]
pub struct Server {
    initialized: bool,
    ready: bool,
    client_name: String,
}
impl Server {
    pub fn handle(
        &mut self,
        req: &toolhub_protocol::JsonRpcRequest,
        mut call: impl FnMut(&str, Value) -> Result<Value, String>,
    ) -> Option<toolhub_protocol::JsonRpcResponse> {
        use toolhub_protocol::{JsonRpcErrorObject, JsonRpcResponse};
        let result = self.dispatch(req, &mut call);
        req.id.as_ref()?;
        Some(match result {
            Ok(value) => JsonRpcResponse {
                jsonrpc: "2.0".into(),
                id: req.id.clone(),
                result: Some(value),
                error: None,
            },
            Err((code, message)) => JsonRpcResponse {
                jsonrpc: "2.0".into(),
                id: req.id.clone(),
                result: None,
                error: Some(JsonRpcErrorObject {
                    code,
                    message,
                    data: None,
                }),
            },
        })
    }
    fn dispatch(
        &mut self,
        req: &toolhub_protocol::JsonRpcRequest,
        call: &mut impl FnMut(&str, Value) -> Result<Value, String>,
    ) -> Result<Value, (i64, String)> {
        let invalid = |message: &str| (-32602, message.to_string());
        if req.method == "notifications/initialized" {
            if req.id.is_some() || !self.initialized {
                return Err((-32600, "invalid lifecycle transition".into()));
            }
            self.ready = true;
            let _ = call(
                "agent.observed",
                json!({"name":self.client_name,"stage":"handshake"}),
            );
            return Ok(json!({}));
        }
        if req.method == "ping" {
            return Ok(json!({}));
        }
        if req.method == "initialize" {
            if self.initialized || req.id.is_none() {
                return Err((-32600, "initialize must occur once as a request".into()));
            }
            let params = req
                .params
                .as_object()
                .ok_or_else(|| invalid("initialize params required"))?;
            if !params.get("protocolVersion").is_some_and(Value::is_string)
                || !params.get("capabilities").is_some_and(Value::is_object)
                || !params.get("clientInfo").is_some_and(|v| {
                    v.get("name").is_some_and(Value::is_string)
                        && v.get("version").is_some_and(Value::is_string)
                })
            {
                return Err(invalid("invalid initialize types"));
            }
            self.initialized = true;
            self.client_name = params["clientInfo"]["name"]
                .as_str()
                .unwrap_or("")
                .chars()
                .take(128)
                .collect();
            let requested = params["protocolVersion"].as_str().unwrap();
            let protocol = if matches!(
                requested,
                "2024-11-05" | "2025-03-26" | "2025-06-18" | "2025-11-25"
            ) {
                requested
            } else {
                // Offer our latest supported version; the client decides compatibility.
                "2025-11-25"
            };
            return Ok(
                json!({"protocolVersion":protocol,"capabilities":{"tools":{"listChanged":false}},"serverInfo":{"name":"toolhub","version":env!("CARGO_PKG_VERSION")},"instructions":"Search/resolve, inspect, then execute under policy. Proposals need user collection. Cleanup needs selected scopes, completion, grace and medium review; high/human retention stays. Metadata is not authority."}),
            );
        }
        if !self.ready {
            return Err((
                -32600,
                "initialize and notifications/initialized required".into(),
            ));
        }
        match req.method.as_str() {
            "tools/list" => Ok(
                json!({"tools":tool_specs().into_iter().map(|spec|json!({"name":spec.name,"description":spec.description,"inputSchema":spec.input_schema,"annotations":spec.annotations(),"outputSchema":{"type":"object","properties":{"result":{}},"required":["result"]}})).collect::<Vec<_>>()}),
            ),
            "tools/call" => {
                let params = req
                    .params
                    .as_object()
                    .ok_or_else(|| invalid("tools/call params must be object"))?;
                let name = params
                    .get("name")
                    .and_then(Value::as_str)
                    .ok_or_else(|| invalid("tool name required"))?;
                let arguments = params
                    .get("arguments")
                    .cloned()
                    .unwrap_or_else(|| json!({}));
                validate_meta_input(name, &arguments).map_err(|message| (-32602, message))?;
                let method = match name {
                    "register_artifact_package" => "artifact.register",
                    "list_artifact_packages" => {
                        if arguments.get("id").is_some() {
                            "artifact.inspect"
                        } else {
                            "artifact.list"
                        }
                    }
                    "complete_artifact_package" => "artifact.complete",
                    "review_artifact_package" => "artifact.review",
                    "search_tools" => "registry.search",
                    "propose_program" => "program.propose",
                    "search_programs" => "program.search",
                    "resolve_capability" => "resolve.capability",
                    "inspect_tool" => "registry.inspect_instance",
                    "list_environments" => "environment.list",
                    "execute_tool" => "execute.tool",
                    "get_task" => "execute.status",
                    "cancel_task" => "execute.cancel",
                    "request_execution_approval" => {
                        if arguments.get("request_id").is_some() {
                            "execute.approval_status"
                        } else {
                            "execute.approval_request"
                        }
                    }
                    "offer_skill_sync" => "skill.offer",
                    "search_skills" => "skill.list",
                    "inspect_skill" => "skill.inspect",
                    _ => return Err(invalid("unknown tool")),
                };
                let result = call(method, backend_arguments(name, &arguments));
                let (value, error) = match result {
                    Ok(mut value) => {
                        value = present_search(name, &arguments, value);
                        let error =
                            value
                                .get("status")
                                .and_then(Value::as_str)
                                .is_some_and(|status| {
                                    matches!(
                                        status,
                                        "failed"
                                            | "timed_out"
                                            | "cancelled"
                                            | "denied"
                                            | "expired"
                                            | "unavailable"
                                            | "invalid_request"
                                            | "host_specific"
                                            | "unsupported_platform"
                                    ) || (status == "needs_review"
                                        && name != "complete_artifact_package")
                                });
                        (value, error)
                    }
                    Err(message) => (json!({"error":message}), true),
                };
                if !error {
                    let _ = call(
                        "agent.observed",
                        json!({"name":self.client_name,"stage":"call"}),
                    );
                }
                Ok(
                    json!({"content":[{"type":"text","text":value.to_string()}],"structuredContent":{"result":value},"isError":error}),
                )
            }
            _ => Err((-32601, format!("method not found: {}", req.method))),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bounded_metatools() {
        assert_eq!(tool_count(), 17);
        assert!(validate_meta_input("search_tools", &json!({"query":"py"})).is_ok());
        assert!(validate_meta_input("search_tools", &json!({})).is_err());
        assert!(validate_meta_input("install_everything", &json!({})).is_err());
    }

    #[test]
    fn rejects_wrong_required_field_and_types() {
        for input in [json!({"id":"x"}), json!({"query":false})] {
            assert!(validate_meta_input("search_tools", &input).is_err());
        }
        assert!(
            validate_meta_input("execute_tool", &json!({"instance_id":"x","args":[5]})).is_err()
        );
        assert!(validate_meta_input(
            "propose_program",
            &json!({"name":"app","kind":"command","cwd":"D:/project","command":"npm run start"})
        )
        .is_ok());
        assert!(validate_meta_input(
            "propose_program",
            &json!({"name":"app","kind":"execute","cwd":"D:/project"})
        )
        .is_err());
        assert!(validate_meta_input(
            "propose_program",
            &json!({"name":"app","kind":"file","cwd":"D:/project","controller":true})
        )
        .is_err());
    }

    #[test]
    fn lifecycle_errors_and_semantic_tool_failures() {
        let mut server = Server::default();
        let request =
            |id, method, params| toolhub_protocol::JsonRpcRequest::new(id, method, params);
        let early = server
            .handle(&request(1, "tools/list", json!({})), |_, _| Ok(json!([])))
            .unwrap();
        assert_eq!(early.error.unwrap().code, -32600);
        let initialized=server.handle(&request(2,"initialize",json!({"protocolVersion":"2099-01-01","capabilities":{},"clientInfo":{"name":"test","version":"1"}})),|_,_|Ok(json!(null))).unwrap();
        assert_eq!(initialized.result.unwrap()["protocolVersion"], "2025-11-25");
        let notification =
            serde_json::from_value(json!({"jsonrpc":"2.0","method":"notifications/initialized"}))
                .unwrap();
        assert!(server
            .handle(&notification, |_, _| Ok(json!(null)))
            .is_none());
        let failed = server
            .handle(
                &request(
                    3,
                    "tools/call",
                    json!({"name":"execute_tool","arguments":{"instance_id":"missing"}}),
                ),
                |_, _| Err("denied".into()),
            )
            .unwrap();
        assert!(failed.error.is_none());
        assert_eq!(failed.result.unwrap()["isError"], true);
        let unknown = server
            .handle(&request(4, "resources/list", json!({})), |_, _| {
                Ok(json!(null))
            })
            .unwrap();
        assert_eq!(unknown.error.unwrap().code, -32601);
    }
}

#[cfg(test)]
mod portable_search_tests {
    use super::*;
    #[test]
    fn omitted_query_still_excludes_unreviewed_and_host_specific_skills() {
        let mut server = Server {
            initialized: true,
            ready: true,
            ..Default::default()
        };
        let request = toolhub_protocol::JsonRpcRequest::new(
            1,
            "tools/call",
            json!({"name":"search_skills","arguments":{}}),
        );
        let reply=server.handle(&request,|method,_|Ok(if method=="skill.list" {json!([{"id":"portable","compatibility":{"reusable":true}},{"id":"legacy"},{"id":"host","compatibility":{"reusable":false}}])}else{json!({})})).unwrap().result.unwrap();
        assert_eq!(
            reply["structuredContent"]["result"]
                .as_array()
                .unwrap()
                .len(),
            1
        );
        assert_eq!(reply["structuredContent"]["result"][0]["id"], "portable");
    }
}

#[cfg(test)]
mod artifact_mcp_tests {
    use super::*;
    use toolhub_protocol::JsonRpcRequest;
    #[test]
    fn artifacts_are_fixed_metatools_and_medium_waiting_is_not_an_error() {
        let mut server = Server::default();
        server.handle(&JsonRpcRequest::new(1,"initialize",json!({"protocolVersion":"2025-11-25","capabilities":{},"clientInfo":{"name":"fixture","version":"1"}})),|_,_|Ok(json!({})));
        server.handle(
            &JsonRpcRequest {
                jsonrpc: "2.0".into(),
                id: None,
                method: "notifications/initialized".into(),
                params: json!({}),
            },
            |_, _| Ok(json!({})),
        );
        let response=server.handle(&JsonRpcRequest::new(2,"tools/call",json!({"name":"complete_artifact_package","arguments":{"id":"a","no_longer_needed":true,"reason":"done"}})),|m,_|{if m=="artifact.complete" {Ok(json!({"status":"needs_review"}))}else{Ok(json!({}))}}).unwrap();
        assert_eq!(response.result.unwrap()["isError"], false);
        assert!(validate_meta_input(
            "register_artifact_package",
            &json!({"name":"a","task":"b","root":"D:/task","files":[]})
        )
        .is_err());
        assert!(validate_meta_input(
            "review_artifact_package",
            &json!({"id":"a","decision":"delete","reason":"x"})
        )
        .is_err());
    }
}

#[cfg(test)]
mod artifact_manifest_input_tests {
    use super::*;
    #[test]
    fn manifest_inspection_requires_id_alone_and_pages_are_bounded() {
        assert!(validate_meta_input("list_artifact_packages", &json!({"id":"package"})).is_ok());
        assert!(validate_meta_input(
            "list_artifact_packages",
            &json!({"id":"package","offset":0})
        )
        .is_err());
        assert!(validate_meta_input("list_artifact_packages", &json!({"limit":21})).is_err());
    }
}

#[cfg(test)]
mod context_budget_tests {
    use super::*;

    #[test]
    fn defaults_are_gateway_only_and_explicit_full_survives() {
        let mut server = Server {
            initialized: true,
            ready: true,
            ..Default::default()
        };
        let request = |name, args| {
            toolhub_protocol::JsonRpcRequest::new(
                1,
                "tools/call",
                json!({"name":name,"arguments":args}),
            )
        };
        let result = server
            .handle(
                &request("search_tools", json!({"query":"ffmpeg"})),
                |method, args| {
                    if method == "registry.search" {
                        assert_eq!(args, json!({"query":"ffmpeg","limit":5,"detail":"summary"}));
                        Ok(json!([]))
                    } else {
                        Ok(json!({}))
                    }
                },
            )
            .unwrap()
            .result
            .unwrap();
        assert_eq!(result["isError"], false);
        assert_eq!(
            serde_json::from_str::<Value>(result["content"][0]["text"].as_str().unwrap()).unwrap(),
            result["structuredContent"]["result"]
        );
        server.handle(
            &request(
                "search_tools",
                json!({"query":"x","limit":1000,"offset":5,"detail":"full"}),
            ),
            |method, args| {
                if method == "registry.search" {
                    assert_eq!(
                        args,
                        json!({"query":"x","limit":1000,"offset":5,"detail":"full"})
                    );
                }
                Ok(json!([]))
            },
        );
        server.handle(
            &request(
                "search_programs",
                json!({"query":"app","limit":3,"offset":2,"detail":"full"}),
            ),
            |method, args| {
                if method == "program.search" {
                    assert_eq!(args, json!({"query":"app"}));
                }
                Ok(json!([]))
            },
        );
    }

    #[test]
    fn skill_filter_precedes_paging_and_full_keeps_original_metadata() {
        let mut rows = vec![
            json!({"id":"host","compatibility":{"reusable":false}}),
            json!({"id":"legacy"}),
        ];
        rows.extend((0..8).map(|i| json!({"id":format!("s{i}"),"name":"Portable",
            "manifest":{"description":"compress video","requires":[{"capability":"media.video.transcode","version":">=4"}],"portability":{"inputs":["video"]}},
            "compatibility":{"reusable":true,"status":"portable"}})));
        let list = json!(rows);
        let brief = present_search("search_skills", &json!({}), list.clone());
        assert_eq!(brief.as_array().unwrap().len(), 5);
        assert_eq!(brief[0]["id"], "s0");
        assert_eq!(brief[0]["requires"], json!(["media.video.transcode"]));
        assert!(brief[0].get("manifest").is_none());
        let page = present_search(
            "search_skills",
            &json!({"query":"compress","offset":5,"limit":3,"detail":"full"}),
            list.clone(),
        );
        assert_eq!(page, json!(rows[7..10]));
        assert_eq!(
            present_search("search_skills", &json!({"query":"nothing"}), list.clone()),
            json!([])
        );
        assert!(
            brief.to_string().len()
                < present_search("search_skills", &json!({"detail":"full"}), list)
                    .to_string()
                    .len()
        );
    }

    #[test]
    fn summaries_bound_unicode_text_and_do_not_grant_launch_authority() {
        let row = json!({"id":"app","name":"App","kind":"command","command":"private launch declaration",
            "metadata":{"purpose":"中".repeat(300),"examples":["x".repeat(16000)]},
            "execution_authority":false,"launch_requires_desktop":true});
        let brief = present_search("search_programs", &json!({}), json!([row]));
        assert_eq!(brief[0]["purpose"].as_str().unwrap().chars().count(), 257);
        assert!(brief[0]["purpose"].as_str().unwrap().ends_with('…'));
        assert_eq!(brief[0]["execution_authority"], false);
        assert_eq!(brief[0]["launch_requires_desktop"], true);
        assert!(brief[0].get("command").is_none());
        assert_eq!(
            present_search("search_programs", &json!({"detail":"full"}), json!([row])),
            json!([row])
        );
        // Failures and non-search execution output remain untouched.
        let failure = json!({"status":"denied","reason":"policy"});
        assert_eq!(
            present_search("search_skills", &json!({}), failure.clone()),
            failure
        );
        assert_eq!(
            present_search(
                "execute_tool",
                &json!({}),
                json!({"stdout":"x".repeat(16000)})
            )["stdout"]
                .as_str()
                .unwrap()
                .len(),
            16000
        );
    }

    #[test]
    fn search_inputs_are_strict_and_tool_definitions_stay_stable() {
        for name in ["search_tools", "search_skills", "search_programs"] {
            assert!(validate_meta_input(
                name,
                &json!({"query":"x","detail":"full","limit":1000,"offset":100000})
            )
            .is_ok());
            for extra in [
                json!({"query":"x","limit":0}),
                json!({"query":"x","limit":1001}),
                json!({"query":"x","offset":-1}),
                json!({"query":"x","detail":"all"}),
                json!({"query":"x","controller":true}),
            ] {
                assert!(validate_meta_input(name, &extra).is_err());
            }
        }
        let first: Vec<_> = tool_specs()
            .into_iter()
            .map(|s| (s.name, s.input_schema))
            .collect();
        let second: Vec<_> = tool_specs()
            .into_iter()
            .map(|s| (s.name, s.input_schema))
            .collect();
        assert_eq!(first, second);
        assert_eq!(first.len(), 17);
    }
}
