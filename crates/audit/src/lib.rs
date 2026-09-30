//! Redacted execution / activity audit records.

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuditRecord {
    pub id: String,
    pub ts: chrono::DateTime<chrono::Utc>,
    pub agent_id: Option<String>,
    pub instance_id: Option<String>,
    pub capability: Option<String>,
    pub executable: String,
    pub args_redacted: Vec<String>,
    pub cwd_redacted: Option<String>,
    pub duration_ms: u64,
    pub exit_code: Option<i32>,
    pub status: String,
    pub approval_id: Option<String>,
    /// Sensitive stdout is not persisted by default.
    pub stdout_stored: bool,
}

impl AuditRecord {
    #[allow(clippy::too_many_arguments)]
    pub fn from_execution(
        agent_id: Option<&str>,
        instance_id: Option<&str>,
        capability: Option<&str>,
        executable: &str,
        args: &[String],
        cwd: Option<&str>,
        duration_ms: u64,
        exit_code: Option<i32>,
        status: &str,
        approval_id: Option<&str>,
    ) -> Self {
        let cwd_redacted = cwd.map(|s| redact_path_for_export(&toolhub_executor::redact_text(s)));
        Self {
            id: uuid_like(),
            ts: chrono::Utc::now(),
            agent_id: agent_id.map(|s| s.to_string()),
            instance_id: instance_id.map(|s| s.to_string()),
            capability: capability.map(|s| s.to_string()),
            executable: redact_path_for_export(&toolhub_executor::redact_text(executable)),
            args_redacted: toolhub_executor::redact_args(args)
                .iter()
                .map(|a| redact_path_for_export(&toolhub_executor::redact_text(a)))
                .collect(),
            cwd_redacted,
            duration_ms,
            exit_code,
            status: status.to_string(),
            approval_id: approval_id.map(|s| s.to_string()),
            stdout_stored: false,
        }
    }
}

fn uuid_like() -> String {
    use std::time::{SystemTime, UNIX_EPOCH};
    let n = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_nanos())
        .unwrap_or_default();
    format!("aud-{n:x}")
}

/// Export policy: hide home/user identity and private paths by default.
pub fn redact_path_for_export(path: &str) -> String {
    let mut homes = vec![];
    if let Some(h) = std::env::var_os("USERPROFILE") {
        homes.push(h.to_string_lossy().to_string());
    }
    if let Some(h) = std::env::var_os("HOME") {
        homes.push(h.to_string_lossy().to_string());
    }
    let mut users = vec![];
    if let Ok(u) = std::env::var("USERNAME") {
        users.push(u);
    }
    if let Ok(u) = std::env::var("USER") {
        users.push(u);
    }
    toolhub_executor::redact_path_for_export(path, &homes, &users)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn audit_does_not_store_stdout_by_default() {
        let a = AuditRecord::from_execution(
            Some("agent.x"),
            Some("i1"),
            Some("language.python.execute"),
            "python",
            &["-c".into(), "print(1)".into()],
            Some("/home/user/project"),
            12,
            Some(0),
            "success",
            None,
        );
        assert!(!a.stdout_stored);
        assert_eq!(a.args_redacted.len(), 2);
    }

    #[test]
    fn audit_redacts_private_paths_in_executable_cwd_and_arguments() {
        let home = std::env::var("USERPROFILE")
            .or_else(|_| std::env::var("HOME"))
            .unwrap();
        let private = format!("{home}/private-fixture/x.exe");
        let record = AuditRecord::from_execution(
            None,
            None,
            None,
            &private,
            &[private.clone(), "-t".into(), "opaque-value".into()],
            Some(&private),
            0,
            None,
            "denied",
            None,
        );
        let json = serde_json::to_string(&record).unwrap();
        assert!(!json.contains(&home.replace('\\', "\\\\")));
        assert!(!json.contains("opaque-value"));
    }
}
