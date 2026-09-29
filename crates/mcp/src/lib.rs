//! MCP gateway: bounded meta-tools only (not one tool per installed instance).

use serde_json::{json, Value};

pub const META_TOOLS: &[(&str, &str)] = &[
    ("search_tools", "Search installed tools by query"),
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
    vec![
        McpToolSpec {
            name: "search_tools",
            description: "Search installed tools by query",
            input_schema: json!({
                "type": "object",
                "properties": {"query": {"type": "string"}},
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
                    "version": {"type": "string"}
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
                    "approval_id": {"type": "string"}
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
    ]
}

/// Tool count must remain bounded — never grow per installed instance.
pub fn tool_count() -> usize {
    META_TOOLS.len()
}

pub fn validate_meta_input(name: &str, input: &Value) -> Result<(), String> {
    if !META_TOOLS.iter().any(|(n, _)| *n == name) {
        return Err(format!("unknown meta-tool: {name}"));
    }
    if !input.is_object() {
        return Err("input must be an object".into());
    }
    match name {
        "search_tools" | "resolve_capability" | "inspect_tool" | "inspect_skill" => {
            if input
                .get("query")
                .or(input.get("capability"))
                .or(input.get("id"))
                .is_none()
                && name != "list_environments"
            {
                if name == "search_tools" && input.get("query").is_none() {
                    return Err("query required".into());
                }
                if name == "resolve_capability" && input.get("capability").is_none() {
                    return Err("capability required".into());
                }
                if (name == "inspect_tool" || name == "inspect_skill") && input.get("id").is_none()
                {
                    return Err("id required".into());
                }
            }
            Ok(())
        }
        "execute_tool" => {
            if input.get("instance_id").is_none() {
                return Err("instance_id required".into());
            }
            Ok(())
        }
        _ => Ok(()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bounded_metatools() {
        assert_eq!(tool_count(), 7);
        assert!(validate_meta_input("search_tools", &json!({"query":"py"})).is_ok());
        assert!(validate_meta_input("search_tools", &json!({})).is_err());
        assert!(validate_meta_input("install_everything", &json!({})).is_err());
    }
}
