//! Registered task outputs, independent of executable tools and launch entries.
use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ArtifactPriority {
    Low,
    Medium,
    High,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ArtifactRoot {
    pub id: String,
    pub path: String,
    pub enabled: bool,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ArtifactFile {
    pub relative_path: String,
    pub bytes: u64,
    pub sha256: String,
    pub modified_ns: String,
    pub status: String,
    pub error: Option<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ArtifactPackage {
    pub id: String,
    pub name: String,
    pub task: String,
    pub source: String,
    pub owner: String,
    pub root: String,
    pub priority: ArtifactPriority,
    pub retained: bool,
    pub status: String,
    pub files: Vec<ArtifactFile>,
    pub completed: bool,
    pub no_longer_needed: bool,
    pub cleanup_after: Option<i64>,
    pub reason: String,
    pub reviewer: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}

pub const ARTIFACT_GRACE_SECONDS: i64 = 300;
pub const ARTIFACT_MAX_FILES: usize = 128;
pub const ARTIFACT_MAX_FILE_BYTES: u64 = 256 * 1024 * 1024;
