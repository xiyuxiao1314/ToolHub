use serde::{Deserialize, Serialize};

/// AgentAdapter contract. Detect/config/health only — ToolHub does not sandbox host shells.
pub trait AgentAdapter {
    fn id(&self) -> &'static str;
    fn detect(&self) -> Option<DetectedAgent>;
    fn get_version(&self) -> Option<String> {
        None
    }
    fn get_capabilities(&self) -> Vec<String> {
        vec![
            "discovery.session".into(),
            "mcp.client".into(),
            "cli.drive".into(),
        ]
    }
    fn generate_mcp_config(&self, daemon_endpoint: &str) -> String {
        format!(
            r#"{{"mcpServers": {{"toolhub": {{"command": "toolhub", "args": ["mcp", "serve"], "env": {{"TOOLHUB_ENDPOINT": "{daemon_endpoint}"}}}}}}}}"#
        )
    }
    fn generate_install_instructions(&self) -> String {
        "Use the agent's own installer. ToolHub never installs agents or tools.".into()
    }
    fn health_check(&self) -> String {
        self.detect()
            .map(|_| "detected".to_string())
            .unwrap_or_else(|| "unsupported".into())
    }
    fn launch_discovery_session(&self, prompt: &str) -> Result<String, String> {
        // Generic adapters do not auto-spawn agents; they return launch instructions.
        Ok(format!(
            "Manual launch required. Model/provider belongs to the agent.\n{prompt}"
        ))
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DetectedAgent {
    pub id: String,
    pub name: String,
    pub kind: String,
    pub executable: Option<String>,
    pub version: Option<String>,
}

pub const KNOWN_AGENTS: &[(&str, &[&str])] = &[
    ("opencode", &["opencode", "opencode.exe"]),
    ("codex", &["codex", "codex.exe"]),
    ("claude-code", &["claude", "claude.exe"]),
    ("cursor", &["cursor", "cursor.exe"]),
];

fn find_on_path(names: &[&str]) -> Option<std::path::PathBuf> {
    for name in names {
        if let Ok(p) = which_path(name) {
            return Some(p);
        }
    }
    None
}

fn which_path(name: &str) -> Result<std::path::PathBuf, ()> {
    let path = std::env::var_os("PATH").ok_or(())?;
    for dir in std::env::split_paths(&path) {
        let candidate = dir.join(name);
        if candidate.is_file() {
            return Ok(candidate);
        }
        #[cfg(windows)]
        {
            let exe = dir.join(format!("{name}.exe"));
            if exe.is_file() {
                return Ok(exe);
            }
        }
    }
    Err(())
}

#[derive(Default)]
pub struct GenericCliAdapter {
    pub id: &'static str,
    pub names: &'static [&'static str],
}

impl AgentAdapter for GenericCliAdapter {
    fn id(&self) -> &'static str {
        self.id
    }

    fn detect(&self) -> Option<DetectedAgent> {
        let exe = find_on_path(self.names)?;
        Some(DetectedAgent {
            id: self.id.to_string(),
            name: self.id.to_string(),
            kind: "cli".into(),
            executable: Some(exe.to_string_lossy().to_string()),
            version: None,
        })
    }
}

#[derive(Default)]
pub struct GenericMcpAdapter {
    pub id: &'static str,
    pub names: &'static [&'static str],
}

impl AgentAdapter for GenericMcpAdapter {
    fn id(&self) -> &'static str {
        self.id
    }

    fn detect(&self) -> Option<DetectedAgent> {
        let exe = find_on_path(self.names)?;
        Some(DetectedAgent {
            id: self.id.to_string(),
            name: self.id.to_string(),
            kind: "mcp".into(),
            executable: Some(exe.to_string_lossy().to_string()),
            version: None,
        })
    }
}

pub fn detect_all() -> Vec<DetectedAgent> {
    let mut out = vec![];
    for (id, names) in KNOWN_AGENTS {
        let adapter = GenericCliAdapter { id, names };
        if let Some(d) = adapter.detect() {
            out.push(d);
        }
    }
    out
}
