use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::evidence::Evidence;

/// Execution trust of a tool instance. Distinct from recognition confidence.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TrustLevel {
    Blocked,
    Unknown,
    UserTrusted,
    Known,
    Verified,
}

impl TrustLevel {
    pub fn as_str(&self) -> &'static str {
        match self {
            TrustLevel::Blocked => "blocked",
            TrustLevel::Unknown => "unknown",
            TrustLevel::UserTrusted => "user_trusted",
            TrustLevel::Known => "known",
            TrustLevel::Verified => "verified",
        }
    }

    pub fn from_str_lossy(s: &str) -> Self {
        match s {
            "blocked" => TrustLevel::Blocked,
            "unknown" => TrustLevel::Unknown,
            "user_trusted" => TrustLevel::UserTrusted,
            "known" => TrustLevel::Known,
            "verified" => TrustLevel::Verified,
            _ => TrustLevel::Unknown,
        }
    }

    /// Whether default policy may consider execution (still gated by policy engine).
    pub fn permits_default_execution(&self) -> bool {
        matches!(
            self,
            TrustLevel::Known | TrustLevel::Verified | TrustLevel::UserTrusted
        )
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TrustRecord {
    pub level: TrustLevel,
    #[serde(default)]
    pub evidence: Vec<Evidence>,
    #[serde(default = "Utc::now")]
    pub updated_at: DateTime<Utc>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub reason: Option<String>,
}

impl TrustRecord {
    pub fn unknown() -> Self {
        Self {
            level: TrustLevel::Unknown,
            evidence: vec![],
            updated_at: Utc::now(),
            reason: Some("default".into()),
        }
    }

    pub fn known(evidence: Vec<Evidence>) -> Self {
        Self {
            level: TrustLevel::Known,
            evidence,
            updated_at: Utc::now(),
            reason: None,
        }
    }

    pub fn blocked(reason: impl Into<String>) -> Self {
        Self {
            level: TrustLevel::Blocked,
            evidence: vec![],
            updated_at: Utc::now(),
            reason: Some(reason.into()),
        }
    }
}

/// Allowed trust transitions. Freezing these prevents silent privilege inflation.
pub fn can_transition(from: TrustLevel, to: TrustLevel) -> bool {
    use TrustLevel::*;
    matches!(
        (from, to),
        (Unknown, Known)
            | (Unknown, UserTrusted)
            | (Unknown, Blocked)
            | (Unknown, Verified)
            | (Known, Verified)
            | (Known, UserTrusted)
            | (Known, Blocked)
            | (Known, Unknown)
            | (UserTrusted, Verified)
            | (UserTrusted, Blocked)
            | (UserTrusted, Known)
            | (UserTrusted, Unknown)
            | (Verified, Blocked)
            | (Verified, Known)
            | (Verified, UserTrusted)
            | (Blocked, Unknown)
            | (Blocked, UserTrusted)
            | (Blocked, Known)
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn trust_transitions() {
        assert!(can_transition(TrustLevel::Unknown, TrustLevel::Known));
        assert!(can_transition(TrustLevel::Known, TrustLevel::Blocked));
        assert!(!can_transition(TrustLevel::Blocked, TrustLevel::Verified));
        assert!(!can_transition(TrustLevel::Unknown, TrustLevel::Unknown));
        // self allowed only if listed
    }
}
