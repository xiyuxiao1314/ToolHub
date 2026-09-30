//! ToolHub Protocol (THP): JSON-RPC method names, error model, envelopes.

pub mod error;
pub mod methods;
pub mod types;
mod validation;
pub use validation::validate_method_params;

pub use error::{ErrorCode, ProtocolError};
pub use methods::{Method, METHOD_NAMES};
pub use types::*;

pub const PROTOCOL_VERSION: &str = "1.0";
pub const MANIFEST_VERSION: &str = "toolhub.manifest/v1";

/// JSON-RPC 2.0 request envelope.
#[derive(Debug, Clone, serde::Serialize)]
pub struct JsonRpcRequest {
    pub jsonrpc: String,
    pub id: Option<serde_json::Value>,
    pub method: String,
    #[serde(default, skip_serializing_if = "serde_json::Value::is_null")]
    pub params: serde_json::Value,
}

impl JsonRpcRequest {
    pub fn new(id: impl Into<serde_json::Value>, method: &str, params: serde_json::Value) -> Self {
        Self {
            jsonrpc: "2.0".into(),
            id: Some(id.into()),
            method: method.to_string(),
            params,
        }
    }
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct JsonRpcResponse {
    pub jsonrpc: String,
    pub id: Option<serde_json::Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub result: Option<serde_json::Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<JsonRpcErrorObject>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct JsonRpcErrorObject {
    pub code: i64,
    pub message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data: Option<serde_json::Value>,
}

/// Event envelope for subscriptions / streaming updates.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct EventEnvelope {
    pub seq: u64,
    pub kind: String,
    pub payload: serde_json::Value,
    #[serde(default)]
    pub ts: chrono::DateTime<chrono::Utc>,
}

/// Stable limits enforced at the protocol boundary.
pub mod limits {
    pub const MAX_REQUEST_BYTES: usize = 1024 * 1024;
    pub const MAX_OUTPUT_BYTES: u64 = 1024 * 256;
    pub const DEFAULT_TIMEOUT_MS: u64 = 30_000;
    pub const MAX_TIMEOUT_MS: u64 = 600_000;
}

impl<'de> serde::Deserialize<'de> for JsonRpcResponse {
    fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        use serde::de::Error;
        let value = serde_json::Value::deserialize(deserializer)?;
        let object = value
            .as_object()
            .ok_or_else(|| D::Error::custom("response must be object"))?;
        if object.get("jsonrpc").and_then(|v| v.as_str()) != Some("2.0") {
            return Err(D::Error::custom("invalid response version"));
        }
        let id = object
            .get("id")
            .cloned()
            .ok_or_else(|| D::Error::custom("response id missing"))?;
        if !id.is_null() && !id.is_string() && !id.is_number() {
            return Err(D::Error::custom("invalid response id"));
        }
        let result = object.get("result").cloned();
        let error = object
            .get("error")
            .map(|value| {
                serde_json::from_value::<JsonRpcErrorObject>(value.clone())
                    .map_err(D::Error::custom)
            })
            .transpose()?;
        if result.is_some() == error.is_some() {
            return Err(D::Error::custom(
                "response must have exactly one result or error",
            ));
        }
        Ok(Self {
            jsonrpc: "2.0".into(),
            id: Some(id),
            result,
            error,
        })
    }
}

impl<'de> serde::Deserialize<'de> for JsonRpcRequest {
    fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        use serde::de::Error;
        let value = serde_json::Value::deserialize(deserializer)?;
        let object = value
            .as_object()
            .ok_or_else(|| D::Error::custom("invalid_request: expected object"))?;
        if object.get("jsonrpc").and_then(|v| v.as_str()) != Some("2.0") {
            return Err(D::Error::custom("invalid_request: jsonrpc must be 2.0"));
        }
        let method = object
            .get("method")
            .and_then(|v| v.as_str())
            .filter(|v| !v.is_empty() && v.len() <= 256)
            .ok_or_else(|| D::Error::custom("invalid_request: method must be a bounded string"))?;
        let id = object.get("id").cloned();
        if id
            .as_ref()
            .is_some_and(|v| !v.is_null() && !v.is_string() && !v.is_number())
        {
            return Err(D::Error::custom("invalid_request: invalid id"));
        }
        if id
            .as_ref()
            .and_then(|v| v.as_str())
            .is_some_and(|v| v.len() > 256)
        {
            return Err(D::Error::custom("invalid_request: id too long"));
        }
        let params = object
            .get("params")
            .cloned()
            .unwrap_or(serde_json::Value::Null);
        if object.contains_key("params") && !params.is_object() && !params.is_array() {
            return Err(D::Error::custom(
                "invalid_params: params must be object or array",
            ));
        }
        Ok(Self {
            jsonrpc: "2.0".into(),
            id,
            method: method.into(),
            params,
        })
    }
}

/// Shared envelope parser. Syntax errors are distinguished from invalid envelopes.
pub fn parse_request(bytes: &[u8]) -> Result<JsonRpcRequest, ProtocolError> {
    if bytes.len() > limits::MAX_REQUEST_BYTES {
        return Err(ProtocolError::new(
            ErrorCode::PayloadTooLarge,
            "request too large",
        ));
    }
    let value: serde_json::Value = serde_json::from_slice(bytes)
        .map_err(|_| ProtocolError::new(ErrorCode::ParseError, "invalid JSON"))?;
    serde_json::from_value(value).map_err(|e| {
        ProtocolError::new(
            if e.to_string().starts_with("invalid_params") {
                ErrorCode::InvalidParams
            } else {
                ErrorCode::InvalidRequest
            },
            e.to_string(),
        )
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn request_roundtrip() {
        let req = JsonRpcRequest::new(1, "registry.search", serde_json::json!({"q":"python"}));
        let s = serde_json::to_string(&req).unwrap();
        let back: JsonRpcRequest = serde_json::from_str(&s).unwrap();
        assert_eq!(back.method, "registry.search");
    }

    #[test]
    fn rejects_bad_envelopes_and_scalar_params() {
        for raw in [
            r#"{"jsonrpc":"1.0","id":1,"method":"status"}"#,
            r#"{"jsonrpc":"2.0","id":{},"method":"status"}"#,
            r#"{"jsonrpc":"2.0","id":1,"method":"status","params":false}"#,
        ] {
            assert!(
                serde_json::from_str::<JsonRpcRequest>(raw).is_err(),
                "{raw}"
            );
        }
    }

    #[test]
    fn null_id_is_request_and_missing_id_is_notification() {
        let null: JsonRpcRequest =
            serde_json::from_str(r#"{"jsonrpc":"2.0","id":null,"method":"status"}"#).unwrap();
        assert_eq!(null.id, Some(serde_json::Value::Null));
        let notification: JsonRpcRequest =
            serde_json::from_str(r#"{"jsonrpc":"2.0","method":"status"}"#).unwrap();
        assert_eq!(notification.id, None);
    }

    #[test]
    fn null_result_is_a_valid_response() {
        let response: JsonRpcResponse =
            serde_json::from_value(serde_json::json!({"jsonrpc":"2.0","id":null,"result":null}))
                .unwrap();
        assert_eq!(response.id, Some(serde_json::Value::Null));
        assert_eq!(response.result, Some(serde_json::Value::Null));
        assert!(serde_json::from_value::<JsonRpcResponse>(serde_json::json!({"jsonrpc":"2.0","id":1,"result":false,"error":{"code":-1,"message":"bad"}})).is_err());
    }
}


