use serde::{Deserialize, Serialize};

use thiserror::Error;

/// Stable protocol error codes (JSON-RPC compatible where applicable).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ErrorCode {
    ParseError,
    InvalidRequest,
    MethodNotFound,
    InvalidParams,
    InternalError,
    Unauthorized,
    Denied,
    Expired,
    NotFound,
    Conflict,
    Unavailable,
    Timeout,
    Cancelled,
    PayloadTooLarge,
    DaemonError,
    PolicyDenied,
    TrustBlocked,
    SessionInvalid,
    ApprovalInvalid,
}

impl ErrorCode {
    pub fn rpc_code(&self) -> i64 {
        match self {
            ErrorCode::ParseError => -32700,
            ErrorCode::InvalidRequest => -32600,
            ErrorCode::MethodNotFound => -32601,
            ErrorCode::InvalidParams => -32602,
            ErrorCode::InternalError => -32603,
            other => -32000 - (other.numeric() as i64),
        }
    }

    fn numeric(&self) -> u16 {
        match self {
            ErrorCode::Unauthorized => 1,
            ErrorCode::Denied => 2,
            ErrorCode::Expired => 3,
            ErrorCode::NotFound => 4,
            ErrorCode::Conflict => 5,
            ErrorCode::Unavailable => 6,
            ErrorCode::Timeout => 7,
            ErrorCode::Cancelled => 8,
            ErrorCode::PayloadTooLarge => 9,
            ErrorCode::DaemonError => 10,
            ErrorCode::PolicyDenied => 11,
            ErrorCode::TrustBlocked => 12,
            ErrorCode::SessionInvalid => 13,
            ErrorCode::ApprovalInvalid => 14,
            _ => 99,
        }
    }

    pub fn as_str(&self) -> &'static str {
        match self {
            ErrorCode::ParseError => "parse_error",
            ErrorCode::InvalidRequest => "invalid_request",
            ErrorCode::MethodNotFound => "method_not_found",
            ErrorCode::InvalidParams => "invalid_params",
            ErrorCode::InternalError => "internal_error",
            ErrorCode::Unauthorized => "unauthorized",
            ErrorCode::Denied => "denied",
            ErrorCode::Expired => "expired",
            ErrorCode::NotFound => "not_found",
            ErrorCode::Conflict => "conflict",
            ErrorCode::Unavailable => "unavailable",
            ErrorCode::Timeout => "timeout",
            ErrorCode::Cancelled => "cancelled",
            ErrorCode::PayloadTooLarge => "payload_too_large",
            ErrorCode::DaemonError => "daemon_error",
            ErrorCode::PolicyDenied => "policy_denied",
            ErrorCode::TrustBlocked => "trust_blocked",
            ErrorCode::SessionInvalid => "session_invalid",
            ErrorCode::ApprovalInvalid => "approval_invalid",
        }
    }
}

impl std::fmt::Display for ErrorCode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

#[derive(Debug, Error, Clone, Serialize, Deserialize)]
#[error("{code}: {message}")]
pub struct ProtocolError {
    pub code: ErrorCode,
    pub message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub fallback_allowed: Option<bool>,
}

impl ProtocolError {
    pub fn new(code: ErrorCode, message: impl Into<String>) -> Self {
        Self {
            code,
            message: message.into(),
            fallback_allowed: None,
        }
    }

    pub fn unavailable(message: impl Into<String>) -> Self {
        Self {
            code: ErrorCode::Unavailable,
            message: message.into(),
            // Information for the caller; not permission to bypass host controls.
            fallback_allowed: Some(true),
        }
    }

    pub fn denied(message: impl Into<String>) -> Self {
        Self {
            code: ErrorCode::Denied,
            message: message.into(),
            fallback_allowed: Some(false),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fallback_is_informational() {
        let e = ProtocolError::unavailable("daemon_error");
        assert_eq!(e.fallback_allowed, Some(true));
        let d = ProtocolError::denied("policy");
        assert_eq!(d.fallback_allowed, Some(false));
    }
}
