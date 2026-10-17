//! Read-only observations, never execution grants.
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProjectDependency {
    pub project_root: String,
    pub project_name: String,
    pub package_name: String,
    pub declared_version: Option<String>,
    pub package_version: Option<String>,
    pub package_root: String,
    pub executable: Option<String>,
    #[serde(default)]
    pub distribution_override: Option<String>,
    pub binary_version: Option<String>,
    pub instance_id: Option<String>,
    pub status: String,
    pub source: String,
    pub issue: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ProjectDistOverride {
    pub project_root: String,
    pub electron_dist: String,
}
