use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize)]
pub struct AgentOperation {
    pub id: String,
    pub owner: String,
    pub agent_id: String,
    pub session_id: String,
    pub state: String,
    pub pid: Option<u32>,
    pub exit_code: Option<i32>,
    pub disclosed_candidates: usize,
    pub boundary: String,
}
struct RunningAgent {
    operation: AgentOperation,
    child: std::process::Child,
    tree: toolhub_executor::OwnedProcessTree,
    expires_at: chrono::DateTime<chrono::Utc>,
}
#[derive(Default)]
pub struct AgentRuntime {
    operations: std::collections::BTreeMap<String, std::sync::Arc<std::sync::Mutex<RunningAgent>>>,
}
impl AgentRuntime {
    pub fn new() -> Self {
        Self::default()
    }
    pub fn launch(
        &mut self,
        owner: &str,
        agent: &DetectedAgent,
        session: &crate::DiscoverySession,
        disclosure: &serde_json::Value,
        confirmed: bool,
    ) -> Result<AgentOperation, String> {
        if !confirmed {
            return Err("explicit metadata disclosure confirmation required".into());
        }
        if session.agent_id != owner
            || !session.is_usable(chrono::Utc::now())
            || !session.allows("candidate.inspect")
        {
            return Err("discovery session does not authorize caller metadata disclosure".into());
        }
        let executable = agent
            .executable
            .as_deref()
            .ok_or("adapter executable unavailable")?;
        let path =
            std::fs::canonicalize(executable).map_err(|_| "adapter executable unavailable")?;
        let mut command = std::process::Command::new(&path);
        match agent.id.as_str() {
            "owned-fixture" => {
                command.arg("--agent-fixture");
            }
            "opencode" => {
                command.arg("run").arg(format!("{}\nApproved metadata follows on stdin; use only this disclosed candidate set.",crate::discovery_task_prompt(&agent.id,&session.id)));
            }
            _ => return Err("adapter has no supported automatic launch contract".into()),
        }
        self.launch_command(owner, agent, session, disclosure, command)
    }
    fn launch_command(
        &mut self,
        owner: &str,
        agent: &DetectedAgent,
        session: &crate::DiscoverySession,
        disclosure: &serde_json::Value,
        mut command: std::process::Command,
    ) -> Result<AgentOperation, String> {
        let data = serde_json::to_vec(disclosure).map_err(|_| "invalid disclosure")?;
        if data.len() > 65536 {
            return Err("metadata disclosure exceeds 64 KiB".into());
        }
        command.env_clear();
        for name in [
            "PATH",
            "SystemRoot",
            "WINDIR",
            "HOME",
            "USERPROFILE",
            "TEMP",
            "TMP",
        ] {
            if let Some(value) = std::env::var_os(name) {
                command.env(name, value);
            }
        }
        command
            .stdin(std::process::Stdio::piped())
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null());
        let (mut child, tree) = toolhub_executor::OwnedProcessTree::spawn(&mut command)
            .map_err(|_| "adapter launch failed")?;
        let stdin = child.stdin.take().ok_or("adapter stdin unavailable")?;
        let operation=AgentOperation {id:uuid::Uuid::new_v4().to_string(),owner:owner.into(),agent_id:agent.id.clone(),session_id:session.id.clone(),state:"running".into(),pid:Some(child.id()),exit_code:None,disclosed_candidates:disclosure.get("candidates").and_then(|v|v.as_array()).map(|a|a.len()).unwrap_or(0),boundary:"ToolHub restricts disclosed metadata and its own operations; the external agent host shell is not sandboxed by ToolHub".into()};
        let running = std::sync::Arc::new(std::sync::Mutex::new(RunningAgent {
            operation: operation.clone(),
            child,
            tree,
            expires_at: session.expires_at,
        }));
        self.operations
            .insert(operation.id.clone(), running.clone());
        std::thread::spawn(move || {
            use std::io::Write;
            let mut stdin = stdin;
            let _ = stdin.write_all(&data);
        });
        std::thread::spawn(move || loop {
            let Ok(mut state) = running.lock() else { break };
            if state.operation.state != "running" {
                break;
            }
            let result = {
                let RunningAgent { child, tree, .. } = &mut *state;
                tree.poll_child(child)
            };
            match result {
                Ok(Some(exit)) => {
                    state.tree.terminate();
                    state.operation.state = if exit.success() {
                        "completed"
                    } else {
                        "failed"
                    }
                    .into();
                    state.operation.exit_code = exit.code();
                    state.operation.pid = None;
                    break;
                }
                Err(_) => {
                    state.tree.terminate();
                    state.operation.state = "failed".into();
                    state.operation.pid = None;
                    break;
                }
                _ => {}
            }
            if chrono::Utc::now() >= state.expires_at {
                state.tree.terminate();
                state.operation.state = "expired".into();
                state.operation.pid = None;
                break;
            }
            drop(state);
            std::thread::sleep(std::time::Duration::from_millis(10));
        });
        Ok(operation)
    }
    pub fn status(&mut self, owner: &str, id: &str) -> Result<AgentOperation, String> {
        let state = self
            .operations
            .get(id)
            .ok_or("agent operation not found")?
            .lock()
            .map_err(|_| "agent operation unavailable")?;
        if state.operation.owner != owner {
            return Err("agent operation owner mismatch".into());
        }
        Ok(state.operation.clone())
    }
    pub fn cancel_discovery(&mut self, owner: &str, session_id: &str) -> usize {
        let ids: Vec<_> = self
            .operations
            .iter()
            .filter_map(|(id, s)| {
                s.lock()
                    .ok()
                    .filter(|s| {
                        s.operation.owner == owner
                            && s.operation.session_id == session_id
                            && s.operation.state == "running"
                    })
                    .map(|_| id.clone())
            })
            .collect();
        for id in &ids {
            let _ = self.cancel(owner, id);
        }
        ids.len()
    }
    pub fn cancel(&mut self, owner: &str, id: &str) -> Result<AgentOperation, String> {
        let mut state = self
            .operations
            .get(id)
            .ok_or("agent operation not found")?
            .lock()
            .map_err(|_| "agent operation unavailable")?;
        if state.operation.owner != owner {
            return Err("agent operation owner mismatch".into());
        }
        if state.operation.state == "running" {
            state.tree.terminate();
            state.operation.state = "cancelled".into();
            state.operation.pid = None;
        }
        Ok(state.operation.clone())
    }
}
impl Drop for AgentRuntime {
    fn drop(&mut self) {
        for operation in self.operations.values() {
            if let Ok(mut state) = operation.lock() {
                if state.operation.state == "running" {
                    state.tree.terminate();
                    state.operation.state = "cancelled".into();
                    state.operation.pid = None;
                }
            }
        }
    }
}

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

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    #[ignore = "finite owned child, invoked only by runtime lifecycle test"]
    fn owned_agent_fixture_child() {
        use std::io::Read;
        let mut data = vec![];
        std::io::stdin().take(65537).read_to_end(&mut data).unwrap();
        assert_eq!(
            serde_json::from_slice::<serde_json::Value>(&data).unwrap()["candidates"]
                .as_array()
                .unwrap()
                .len(),
            1
        );
        std::thread::sleep(std::time::Duration::from_millis(400));
    }
    #[test]
    fn runtime_launch_monitor_cancel_has_real_owned_processes() {
        let mut runtime = AgentRuntime::new();
        let session =
            crate::DiscoverySession::issue("owner", 1, &["candidate.inspect".into()]).unwrap();
        let exe = std::env::current_exe().unwrap();
        let agent = DetectedAgent {
            id: "owned-fixture".into(),
            name: "Owned fixture".into(),
            kind: "owned_fixture".into(),
            executable: Some(exe.to_string_lossy().into_owned()),
            version: None,
        };
        let disclosure = serde_json::json!({"candidates":[{"id":"synthetic","name":"fixture"}]});
        assert!(runtime
            .launch("owner", &agent, &session, &disclosure, false)
            .is_err());
        assert!(runtime
            .launch("other", &agent, &session, &disclosure, true)
            .is_err());
        for cancelled in [false, true] {
            let mut command = std::process::Command::new(&exe);
            command.args([
                "--exact",
                "adapters::tests::owned_agent_fixture_child",
                "--ignored",
            ]);
            let operation = runtime
                .launch_command("owner", &agent, &session, &disclosure, command)
                .unwrap();
            assert!(operation.pid.is_some());
            assert!(runtime.status("other", &operation.id).is_err());
            if cancelled {
                assert_eq!(
                    runtime.cancel("owner", &operation.id).unwrap().state,
                    "cancelled"
                );
            } else {
                let deadline = std::time::Instant::now() + std::time::Duration::from_secs(3);
                loop {
                    let state = runtime.status("owner", &operation.id).unwrap();
                    if state.state != "running" {
                        assert_eq!(state.state, "completed");
                        assert_eq!(state.exit_code, Some(0));
                        break;
                    }
                    assert!(std::time::Instant::now() < deadline);
                    std::thread::sleep(std::time::Duration::from_millis(10));
                }
            }
        }
        assert!(runtime.cancel("owner", "nonexistent").is_err());
    }
}
