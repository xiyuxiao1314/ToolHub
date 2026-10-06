//! MCP gateway: bounded meta-tools only (not one tool per installed instance).

use serde_json::{json, Value};

pub const META_TOOLS: &[(&str, &str)] = &[
    ("search_tools", "Search installed tools by query"),
    (
        "get_task",
        "Poll own authorized execution and declared artifacts",
    ),
    ("cancel_task", "Cancel own running execution"),
    (
        "request_execution_approval",
        "Request desktop user approval for an exact invocation",
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

pub fn tool_specs() -> Vec<McpToolSpec> {
    let mut specs = vec![
        McpToolSpec{name:"get_task",description:"Poll a ToolHub execution by ID. Only its owner or the verified desktop can read results; output is bounded and only retained in daemon memory.",input_schema:json!({"type":"object","properties":{"execution_id":{"type":"string"}},"required":["execution_id"]})},
        McpToolSpec{name:"cancel_task",description:"Request cancellation of your running ToolHub execution; poll get_task for the terminal state.",input_schema:json!({"type":"object","properties":{"execution_id":{"type":"string"}},"required":["execution_id"]})},
        McpToolSpec {name:"search_programs",description:"Search user-collected applications explicitly shared for Agent discovery. Returned launch declarations are data, not execution authority; launch still requires the desktop controller.",input_schema:json!({"type":"object","properties":{"query":{"type":"string"}}})},
        McpToolSpec {
            name: "propose_program",
            description: "Submit a program you created to ToolHub's pending list. The user must select and save it in the desktop before it is collected. This tool never launches it.",
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
            description: "Search installed tools by query",
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
            description: "Execute a resolved tool under policy and approval",
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
            description: "Search skill manifests",
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
    let execution = specs.iter().find(|s| s.name == "execute_tool").unwrap();
    let mut approval_schema = execution.input_schema.clone();
    approval_schema["required"] = json!([]);
    approval_schema["properties"]["request_id"] = json!({"type":"string","description":"Poll this existing approval request; provide this field alone"});
    specs.push(McpToolSpec{name:"request_execution_approval",description:"Submit an exact invocation for user review in the desktop. Approval is one-use and bound to executable hash, args, cwd, caller and policy; this does not execute it. After user approval, poll this same tool with request_id alone to retrieve your one-use approval_id and session_id.",input_schema:approval_schema});
    for spec in &mut specs {
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
                items.len() <= 1024
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
                json!({"protocolVersion":protocol,"capabilities":{"tools":{"listChanged":false}},"serverInfo":{"name":"toolhub","version":env!("CARGO_PKG_VERSION")},"instructions":"When a task needs local binaries, first search ToolHub or resolve a capability and inspect the returned instance before executing under policy. After creating an application, propose its entry for user confirmation; proposing never saves or launches it."}),
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
                json!({"tools":tool_specs().into_iter().map(|spec|json!({"name":spec.name,"description":spec.description,"inputSchema":spec.input_schema,"outputSchema":{"type":"object","properties":{"result":{}},"required":["result"]}})).collect::<Vec<_>>()}),
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
                    "search_skills" => "skill.list",
                    "inspect_skill" => "skill.inspect",
                    _ => return Err(invalid("unknown tool")),
                };
                let result = call(
                    method,
                    if name == "search_skills" {
                        json!({})
                    } else {
                        arguments.clone()
                    },
                );
                let (value, error) = match result {
                    Ok(mut value) => {
                        if name == "search_skills" {
                            if let Some(list) = value.as_array() {
                                let query = arguments
                                    .get("query")
                                    .and_then(Value::as_str)
                                    .unwrap_or("")
                                    .to_lowercase();
                                value = Value::Array(
                                    list.iter()
                                        .filter(|item| {
                                            item["compatibility"]["reusable"] == true
                                                && item.to_string().to_lowercase().contains(&query)
                                        })
                                        .cloned()
                                        .collect(),
                                );
                            }
                        }
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
                                            | "needs_review"
                                            | "host_specific"
                                            | "unsupported_platform"
                                    )
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
        assert_eq!(tool_count(), 12);
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
