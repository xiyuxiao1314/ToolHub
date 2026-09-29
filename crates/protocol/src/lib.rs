//! ToolHub Protocol (THP): JSON-RPC method names, error model, envelopes.

pub mod error;
pub mod methods;
pub mod types;

pub use error::{ErrorCode, ProtocolError};
pub use methods::{Method, METHOD_NAMES};
pub use types::*;

pub const PROTOCOL_VERSION: &str = "1.0";
pub const MANIFEST_VERSION: &str = "toolhub.manifest/v1";

/// JSON-RPC 2.0 request envelope.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
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

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
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
}
