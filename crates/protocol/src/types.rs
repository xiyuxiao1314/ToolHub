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
    pub definition_id: String,
    pub name: String,
    pub instance_count: usize,
    pub capabilities: Vec<String>,
    pub trust: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResolveResult {
    pub capability: String,
    pub canonical_capability: String,
    pub selected: Option<ResolvedInstance>,
    pub alternatives: Vec<ResolvedInstance>,
    pub explanation: String,
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
    pub score: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecuteParams {
    pub capability: Option<String>,
    pub instance_id: Option<String>,
    pub args: Vec<String>,
    pub cwd: Option<String>,
    pub approval_id: Option<String>,
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
