use crate::{ErrorCode, JsonRpcRequest, Method, ProtocolError};
use serde_json::Value;

/// THP application methods use named object parameters. JSON-RPC itself also permits arrays.
pub fn validate_method_params(request: &JsonRpcRequest) -> Result<(), ProtocolError> {
    let Some(method) = Method::from_name(&request.method) else {
        return Ok(());
    };
    let invalid = |message: String| ProtocolError::new(ErrorCode::InvalidParams, message);
    let empty = serde_json::Map::new();
    let params = if request.params.is_null() {
        &empty
    } else {
        request
            .params
            .as_object()
            .ok_or_else(|| invalid("named object params required".into()))?
    };
    let required: &[&str] = match method {
        Method::RegistryCorrect
        | Method::InspectTool
        | Method::InspectInstance
        | Method::PreferInstance
        | Method::HealthInstance
        | Method::SkillInspect
        | Method::SkillResolve
        | Method::DiscoveryInspect
        | Method::ProgramProposalDismiss
        | Method::ProgramRemove
        | Method::ProgramLaunch => &["id"],
        Method::ResolveCapability => &["capability"],
        Method::SearchTools => &["query"],
        Method::RequestApproval => &["instance_id"],
        Method::ApproveExecution | Method::ApprovalStatus => &["request_id"],
        Method::RevokeApproval => &["approval_id"],
        Method::DiscoveryRevoke => &["session_id"],
        Method::AgentLaunch => &["agent_id"],
        Method::AgentObserved => &["name", "stage"],
        Method::AgentStatus | Method::AgentCancel => &["operation_id"],
        Method::ExecuteCancel | Method::ExecuteStatus => &["execution_id"],
        Method::PolicySet => &["scope", "subject", "action"],
        _ => &[],
    };
    for key in required {
        match params.get(*key).and_then(Value::as_str) {
            Some(value) if !value.is_empty() || *key == "query" => {}
            _ => return Err(invalid(format!("{key} must be a string"))),
        }
    }
    if method == Method::SkillPreview
        && (!params.get("manifest").is_some_and(Value::is_object)
            || !params.get("instructions").is_some_and(|v| {
                v.as_str()
                    .is_some_and(|s| !s.is_empty() && s.len() <= 65536)
            }))
    {
        return Err(invalid(
            "manifest object and bounded instruction text required".into(),
        ));
    }
    if params.get("if_absent").is_some_and(|v| !v.is_boolean()) {
        return Err(invalid("if_absent must be boolean".into()));
    }
    if method == Method::SkillRegister
        && !params.contains_key("path")
        && !params.get("manifest").is_some_and(Value::is_object)
        && !params.contains_key("schema")
    {
        return Err(invalid("path or declarative manifest required".into()));
    }
    if method == Method::Negotiate
        && !params.get("versions").is_some_and(|v| {
            v.as_array().is_some_and(|versions| {
                !versions.is_empty()
                    && versions.len() <= 16
                    && versions
                        .iter()
                        .all(|v| v.as_str().is_some_and(|s| s.len() <= 32))
            })
        })
    {
        return Err(invalid(
            "versions must be a bounded nonempty string array".into(),
        ));
    }
    let strings = [
        "id",
        "query",
        "capability",
        "version",
        "cwd",
        "instance_id",
        "approval_id",
        "request_id",
        "session_id",
        "execution_id",
        "agent_id",
        "operation_id",
        "protocol_version",
        "mode",
        "environment",
        "preferred_environment",
        "prefer_environment",
        "trust",
        "require_trust",
        "arch",
        "require_arch",
        "min_version",
        "scope",
        "subject",
        "action",
        "format",
        "path",
        "name",
        "stage",
        "detail",
        "project",
        "preferred_instance",
    ];
    for key in strings {
        if let Some(value) = params.get(key) {
            match value {
                Value::Null => {}
                Value::String(text) if text.len() <= 16384 => {}
                _ => return Err(invalid(format!("{key} must be a bounded string"))),
            }
        }
    }
    if let Some(value) = params.get("args") {
        if !value.as_array().is_some_and(|items| {
            items.len() <= 1024
                && items
                    .iter()
                    .all(|v| v.as_str().is_some_and(|s| s.len() <= 16384))
        }) {
            return Err(invalid("args must be a bounded string array".into()));
        }
    }
    if method == Method::ProgramLaunch && params.get("terminal").is_some_and(|v| !v.is_boolean()) {
        return Err(invalid("terminal must be a boolean".into()));
    }
    for (key, max) in [("limit", 1000), ("offset", 100000)] {
        if let Some(value) = params.get(key) {
            if value
                .as_u64()
                .is_none_or(|v| v > max || (key == "limit" && v == 0))
            {
                return Err(invalid(format!("{key} out of bounds")));
            }
        }
    }
    for key in ["clear", "background"] {
        if params.get(key).is_some_and(|v| !v.is_boolean()) {
            return Err(invalid(format!("{key} must be boolean")));
        }
    }
    if let Some(value) = params.get("include_missing") {
        if !value.is_boolean() {
            return Err(invalid("include_missing must be a boolean".into()));
        }
    }
    for (key, max) in [
        ("timeout_ms", crate::limits::MAX_TIMEOUT_MS),
        ("max_output_bytes", crate::limits::MAX_OUTPUT_BYTES),
    ] {
        if let Some(value) = params.get(key) {
            if !matches!(value.as_u64(),Some(n) if n>0 && n<=max) {
                return Err(invalid(format!("{key} out of bounds")));
            }
        }
    }
    if let Some(value) = params.get("mode") {
        if !value
            .as_str()
            .is_some_and(|mode| matches!(mode, "quick" | "full" | "custom"))
        {
            return Err(invalid("unknown scan mode".into()));
        }
    }
    if let Some(value) = params.get("stdin") {
        match value {
            Value::Null => {}
            Value::String(text) if text.len() <= crate::limits::MAX_OUTPUT_BYTES as usize => {}
            _ => return Err(invalid("stdin must be a bounded string".into())),
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn program_terminal_requires_an_explicit_boolean() {
        for params in [
            serde_json::json!({"id":"x"}),
            serde_json::json!({"id":"x","terminal":true}),
            serde_json::json!({"id":"x","terminal":false}),
        ] {
            assert!(
                validate_method_params(&JsonRpcRequest::new(1, "program.launch", params)).is_ok()
            );
        }
        for value in [
            serde_json::json!("false"),
            serde_json::json!(null),
            serde_json::json!(1),
        ] {
            assert!(validate_method_params(&JsonRpcRequest::new(
                1,
                "program.launch",
                serde_json::json!({"id":"x","terminal":value})
            ))
            .is_err());
        }
    }
    #[test]
    fn missing_filter_requires_an_explicit_boolean() {
        for value in [serde_json::json!(true), serde_json::json!(false)] {
            assert!(validate_method_params(&JsonRpcRequest::new(
                1,
                "registry.search",
                serde_json::json!({"query":"", "include_missing":value})
            ))
            .is_ok());
        }
        for value in [
            serde_json::json!("true"),
            serde_json::json!(null),
            serde_json::json!(1),
        ] {
            assert!(validate_method_params(&JsonRpcRequest::new(
                1,
                "registry.search",
                serde_json::json!({"query":"", "include_missing":value})
            ))
            .is_err());
        }
    }
    #[test]
    fn no_silent_default_for_wrong_parameter_types() {
        for params in [
            serde_json::json!({"instance_id":"x","args":[false]}),
            serde_json::json!({"instance_id":"x","timeout_ms":"1"}),
            serde_json::json!([]),
        ] {
            assert!(
                validate_method_params(&JsonRpcRequest::new(1, "execute.tool", params)).is_err()
            );
        }
    }
}
