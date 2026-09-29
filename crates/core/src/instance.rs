use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::evidence::Evidence;
use crate::id::{CapabilityId, DefinitionId, EnvironmentId, InstanceId};
use crate::owner::{Origin, Owner};
use crate::trust::TrustRecord;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ToolCategory {
    Cli,
    Gui,
    Runtime,
    Compiler,
    Sdk,
    Decompiler,
    Debugger,
    PackageManager,
    Container,
    Database,
    Media,
    Ai,
    Other,
}

/// What a software product is (identity), independent of installs.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ToolDefinition {
    pub id: DefinitionId,
    pub name: String,
    #[serde(default)]
    pub categories: Vec<ToolCategory>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub vendor: Option<String>,
    #[serde(default)]
    pub capabilities: Vec<CapabilityId>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
}

/// One installed copy on this machine. Multiple instances are preserved.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ToolInstance {
    pub id: InstanceId,
    pub definition_id: DefinitionId,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub version: Option<String>,
    pub platform: String,
    pub arch: String,
    /// Original discovery path (may be a symlink/shim).
    pub path: String,
    /// Canonical executable path after link resolution.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub canonical_path: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub environment_id: Option<EnvironmentId>,
    #[serde(default)]
    pub origin: Origin,
    #[serde(default)]
    pub owner: Owner,
    pub trust: TrustRecord,
    #[serde(default)]
    pub interfaces: Vec<Interface>,
    #[serde(default)]
    pub evidence: Vec<Evidence>,
    pub status: InstanceStatus,
    #[serde(default = "Utc::now")]
    pub first_seen: DateTime<Utc>,
    #[serde(default = "Utc::now")]
    pub last_seen: DateTime<Utc>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum InstanceStatus {
    Available,
    Missing,
    Blocked,
    Unknown,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum InterfaceKind {
    Cli,
    Gui,
    Api,
    Http,
    LocalSocket,
    Mcp,
    PythonApi,
    Shell,
    Plugin,
    Library,
    FileHandler,
    Automation,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Interface {
    pub id: String,
    pub kind: InterfaceKind,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub executable: Option<String>,
    #[serde(default)]
    pub supports_stdin: bool,
    #[serde(default)]
    pub supports_stdout: bool,
    #[serde(default)]
    pub supports_batch: bool,
}

/// Discovered item awaiting recognition. Never auto-promoted to ToolDefinition.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ScanCandidate {
    pub id: String,
    pub path: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub canonical_path: Option<String>,
    #[serde(default)]
    pub file_name: Option<String>,
    #[serde(default)]
    pub size_bytes: Option<u64>,
    #[serde(default)]
    pub sha256: Option<String>,
    #[serde(default)]
    pub version_hint: Option<String>,
    pub recognized: bool,
    #[serde(default)]
    pub metadata: serde_json::Value,
    #[serde(default = "Utc::now")]
    pub discovered_at: DateTime<Utc>,
}

impl ScanCandidate {
    pub fn from_path(path: impl Into<String>) -> Self {
        let path = path.into();
        let file_name = std::path::Path::new(&path)
            .file_name()
            .map(|s| s.to_string_lossy().to_string());
        Self {
            id: uuid::Uuid::new_v4().to_string(),
            path,
            canonical_path: None,
            file_name,
            size_bytes: None,
            sha256: None,
            version_hint: None,
            recognized: false,
            metadata: serde_json::json!({}),
            discovered_at: Utc::now(),
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::id::DefinitionId;

    #[test]
    fn multiple_instances_are_distinct() {
        let def = DefinitionId::new("org.python.python").unwrap();
        assert_eq!(def.as_str(), "org.python.python");
    }
}
