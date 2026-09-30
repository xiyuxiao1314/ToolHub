use serde::{Deserialize, Serialize};

/// Shared result DTOs. Field names match wire contracts used by CLI/MCP/daemon.

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StatusResult {
    pub protocol_version: String,
    pub daemon: String,
    pub registry_path: String,
    pub tool_count: u64,
    pub candidate_count: u64,
    pub last_scan: Option<chrono::DateTime<chrono::Utc>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchHit {
    pub id: String,
    pub definition_id: String,
    pub name: String,
    pub version: Option<String>,
    pub path: String,
    pub canonical_path: Option<String>,
    pub environment_id: Option<String>,
    pub trust: String,
    pub status: String,
    pub arch: String,
    pub platform: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResolveResult {
    pub capability: String,
    pub canonical: String,
    pub selected: Option<ResolvedInstance>,
    pub alternatives: Vec<ResolvedInstance>,
    pub explanation: String,
    pub error: Option<String>,
    pub eligibility_error: Option<String>,
    pub rejected: Vec<RejectedCandidate>,
    pub fallback_allowed: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RejectedCandidate {
    pub candidate: ResolvedInstance,
    pub reasons: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResolvedInstance {
    pub instance_id: String,
    pub definition_id: String,
    pub name: String,
    pub version: Option<String>,
    pub path: String,
    pub environment: Option<String>,
    pub trust: String,
    pub arch: String,
    pub cwd_match: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecuteParams {
    pub capability: Option<String>,
    pub instance_id: Option<String>,
    #[serde(default)]
    pub args: Vec<String>,
    pub cwd: Option<String>,
    pub approval_id: Option<String>,
    pub session_id: Option<String>,
    pub execution_id: Option<String>,
    pub timeout_ms: Option<u64>,
    pub max_output_bytes: Option<u64>,
    pub stdin: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PolicyRuleView {
    pub scope: String,
    pub subject: String,
    pub action: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ActivityItem {
    pub ts: chrono::DateTime<chrono::Utc>,
    pub kind: String,
    pub summary: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub agent: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiscoverySessionView {
    pub session_id: String,
    pub agent_id: String,
    pub expires_at: chrono::DateTime<chrono::Utc>,
    pub scopes: Vec<String>,
    pub revoked: bool,
}

/// Redacted machine report shape (export).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MachineReport {
    pub generated_at: chrono::DateTime<chrono::Utc>,
    pub tools: Vec<ReportTool>,
    pub environments: Vec<ReportEnv>,
    pub redaction: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReportTool {
    pub name: String,
    pub version: Option<String>,
    pub environment: Option<String>,
    pub trust: String,
    pub capabilities: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReportEnv {
    pub name: String,
    pub kind: String,
    pub tool_count: usize,
}
