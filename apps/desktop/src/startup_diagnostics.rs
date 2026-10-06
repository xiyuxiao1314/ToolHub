//! Bounded local diagnostic for login launches; no arguments or environment values.
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Clone, Deserialize, Serialize)]
pub(crate) struct StartupRecord {
    pub at_ms: u64,
    pub pid: u32,
    pub stage: String,
    pub error: Option<String>,
}
fn path() -> Option<PathBuf> {
    std::env::var_os("LOCALAPPDATA")
        .map(|p| PathBuf::from(p).join("ToolHub").join("startup-last.json"))
}
pub(crate) fn record(stage: &str, error: Option<&str>) {
    let Some(path) = path() else {
        return;
    };
    let record = StartupRecord {
        at_ms: std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis() as u64,
        pid: std::process::id(),
        stage: stage.into(),
        error: error.map(|s| s.chars().take(1000).collect()),
    };
    // Logging cannot prevent the application from opening. Only the most recent login launch is retained.
    if let (Some(parent), Ok(json)) = (path.parent(), serde_json::to_vec(&record)) {
        if std::fs::create_dir_all(parent).is_ok() {
            let _ = std::fs::write(path, json);
        }
    }
}
pub(crate) fn last() -> Option<StartupRecord> {
    let path = path()?;
    if std::fs::metadata(&path).ok()?.len() > 8192 {
        return None;
    }
    serde_json::from_slice(&std::fs::read(path).ok()?).ok()
}
