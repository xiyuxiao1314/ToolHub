use serde::{Deserialize, Serialize};

/// Lightweight row views used by queries.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InstanceRow {
    pub id: String,
    pub definition_id: String,
    pub name: String,
    pub version: Option<String>,
    pub path: String,
    pub canonical_path: Option<String>,
    pub environment_id: Option<String>,
    pub trust: String,
    pub status: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CandidateRow {
    pub id: String,
    pub path: String,
    pub recognized: bool,
    pub file_name: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpsertInstanceInput {
    pub id: String,
    pub definition_id: String,
    pub definition_name: String,
    pub version: Option<String>,
    pub platform: String,
    pub arch: String,
    pub path: String,
    pub canonical_path: Option<String>,
    pub environment_id: Option<String>,
    pub origin_json: String,
    pub owner_json: String,
    pub trust_json: String,
    pub status: String,
    pub capabilities: Vec<String>,
}
