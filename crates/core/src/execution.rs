use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::id::{AgentId, InstanceId};

/// Binds approval to caller/session, instance identity, executable hash,
/// args/cwd/environment digest, policy state, and expiry.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ExecutionApproval {
    pub approval_id: String,
    pub agent_id: AgentId,
    pub session_id: String,
    pub instance_id: InstanceId,
    pub executable_sha256: String,
    pub canonical_executable: String,
    pub args_digest: String,
    pub cwd_digest: String,
    pub stdin_digest: String,
    pub env_digest: String,
    /// Digest of policy+trust state at mint time (R2-B01 residual).
    pub policy_trust_digest: String,
    pub expires_at: DateTime<Utc>,
    pub consumed: bool,
    #[serde(default)]
    pub revoked: bool,
}

impl ExecutionApproval {
    pub fn is_usable(&self, now: DateTime<Utc>) -> bool {
        !self.consumed && !self.revoked && now < self.expires_at
    }

    pub fn consume(&mut self) {
        self.consumed = true;
    }

    pub fn revoke(&mut self) {
        self.revoked = true;
    }
}

/// Sanitized request from a client. No implicit shell interpretation.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ExecutionRequest {
    pub instance_id: InstanceId,
    pub executable: String,
    pub args: Vec<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub cwd: Option<String>,
    #[serde(default)]
    pub env_overrides: std::collections::BTreeMap<String, String>,
    #[serde(default = "default_timeout")]
    pub timeout_ms: u64,
    #[serde(default = "default_max_output")]
    pub max_output_bytes: u64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub stdin: Option<String>,
    #[serde(default)]
    pub agent_id: Option<AgentId>,
}

fn default_timeout() -> u64 {
    30_000
}

fn default_max_output() -> u64 {
    1024 * 256
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ExecutionResult {
    pub status: ExecutionStatus,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub exit_code: Option<i32>,
    #[serde(default)]
    pub stdout: String,
    #[serde(default)]
    pub stderr: String,
    pub duration_ms: u64,
    #[serde(default)]
    pub truncated: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub error_code: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub fallback_allowed: Option<bool>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ExecutionStatus {
    Success,
    Failed,
    Denied,
    Expired,
    TimedOut,
    Cancelled,
    Unavailable,
    InvalidRequest,
}

impl ExecutionStatus {
    pub fn as_str(&self) -> &'static str {
        match self {
            ExecutionStatus::Success => "success",
            ExecutionStatus::Failed => "failed",
            ExecutionStatus::Denied => "denied",
            ExecutionStatus::Expired => "expired",
            ExecutionStatus::TimedOut => "timed_out",
            ExecutionStatus::Cancelled => "cancelled",
            ExecutionStatus::Unavailable => "unavailable",
            ExecutionStatus::InvalidRequest => "invalid_request",
        }
    }
}

/// Digest helpers used for approval binding.
pub fn digest_strings(items: &[String]) -> String {
    use sha2::{Digest, Sha256};
    let mut h = Sha256::new();
    h.update(b"toolhub.digest.strings/v2");
    h.update((items.len() as u64).to_le_bytes());
    for i in items {
        let b = i.as_bytes();
        h.update((b.len() as u64).to_le_bytes());
        h.update(b);
    }
    format!("v2:{}", hex::encode(h.finalize()))
}

pub fn digest_map(map: &std::collections::BTreeMap<String, String>) -> String {
    use sha2::{Digest, Sha256};
    let mut h = Sha256::new();
    h.update(b"toolhub.digest.map/v2");
    h.update((map.len() as u64).to_le_bytes());
    for (k, v) in map {
        for item in [k, v] {
            h.update((item.len() as u64).to_le_bytes());
            h.update(item.as_bytes());
        }
    }
    format!("v2:{}", hex::encode(h.finalize()))
}

pub fn hash_bytes(data: &[u8]) -> String {
    use sha2::{Digest, Sha256};
    let mut h = Sha256::new();
    h.update(data);
    hex::encode(h.finalize())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn map_digest_is_unambiguous_and_versioned() {
        let first = std::collections::BTreeMap::from([("a".into(), "b\u{1f}c\u{1e}d".into())]);
        let second =
            std::collections::BTreeMap::from([("a".into(), "b".into()), ("c".into(), "d".into())]);
        assert_ne!(digest_map(&first), digest_map(&second));
        assert!(digest_map(&first).starts_with("v2:"));
        assert!(digest_strings(&[]).starts_with("v2:"));
    }

    #[test]
    fn approval_consumed_once() {
        let mut a = ExecutionApproval {
            approval_id: "a1".into(),
            agent_id: AgentId::new("agent.test").unwrap(),
            session_id: "s1".into(),
            instance_id: InstanceId::new("inst-1").unwrap(),
            executable_sha256: "aa".into(),
            canonical_executable: "C:/tools/x.exe".into(),
            args_digest: "bb".into(),
            cwd_digest: "cc".into(),
            stdin_digest: "ee".into(),
            env_digest: "dd".into(),
            policy_trust_digest: "ff".into(),
            expires_at: Utc::now() + chrono::Duration::minutes(5),
            consumed: false,
            revoked: false,
        };
        assert!(a.is_usable(Utc::now()));
        a.consume();
        assert!(!a.is_usable(Utc::now()));
    }
}
