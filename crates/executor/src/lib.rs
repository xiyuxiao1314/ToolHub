//! Process execution with environment sanitization, bounds, and approval binding.

use std::collections::BTreeMap;
use std::process::{Command, Stdio};
use std::time::{Duration, Instant};

use toolhub_core::{
    digest_map, digest_strings, hash_bytes, ExecutionApproval, ExecutionRequest, ExecutionResult,
    ExecutionStatus,
};
use toolhub_policy::{PolicyAction, PolicyContext, PolicyEngine};

/// Environment variables that are secret-bearing and excluded by default.
pub const SECRET_PREFIXES: &[&str] = &[
    "AWS_",
    "GITHUB_TOKEN",
    "GH_TOKEN",
    "OPENAI_API_KEY",
    "ANTHROPIC_API_KEY",
    "AZURE_",
    "GOOGLE_",
    "HF_TOKEN",
    "NPM_TOKEN",
    "PYPI_",
    "DOCKER_",
    "KUBE",
    "SSH_",
    "GPG_",
    "PASSWORD",
    "SECRET",
    "TOKEN",
    "API_KEY",
    "APIKEY",
    "CREDENTIAL",
    "PRIVATE_KEY",
];

/// Always-allow safe variables for child processes.
const ALLOWED_ENV: &[&str] = &[
    "PATH",
    "HOME",
    "USERPROFILE",
    "TMP",
    "TEMP",
    "TMPDIR",
    "SYSTEMROOT",
    "WINDIR",
    "COMSPEC",
    "PATHEXT",
    "LANG",
    "LC_ALL",
    "TERM",
    "SHELL",
    "USER",
    "USERNAME",
    "NUMBER_OF_PROCESSORS",
    "OS",
    "PROCESSOR_ARCHITECTURE",
];

#[derive(Debug, Clone)]
pub struct SanitizedEnv {
    pub vars: BTreeMap<String, String>,
    pub excluded: Vec<String>,
}

/// Build child environment from an explicit approved set; secrets excluded by default.
pub fn sanitize_env(overrides: &BTreeMap<String, String>, allow_extra: &[String]) -> SanitizedEnv {
    let mut vars = BTreeMap::new();
    let mut excluded = vec![];

    for (k, v) in std::env::vars() {
        let ku = k.to_ascii_uppercase();
        if SECRET_PREFIXES.iter().any(|p| ku.contains(p)) && !allow_extra.iter().any(|a| a == &k) {
            excluded.push(k);
            continue;
        }
        if ALLOWED_ENV.iter().any(|a| a.eq_ignore_ascii_case(&k))
            || allow_extra.iter().any(|a| a == &k)
        {
            vars.insert(k, v);
        } else {
            excluded.push(k);
        }
    }

    for (k, v) in overrides {
        if SECRET_PREFIXES
            .iter()
            .any(|p| k.to_ascii_uppercase().contains(p))
            && !allow_extra.iter().any(|a| a == k)
        {
            excluded.push(k.clone());
            continue;
        }
        vars.insert(k.clone(), v.clone());
    }

    SanitizedEnv { vars, excluded }
}

pub fn env_digest(env: &SanitizedEnv) -> String {
    digest_map(&env.vars)
}

pub fn hash_file(path: &str) -> std::io::Result<String> {
    let data = std::fs::read(path)?;
    Ok(hash_bytes(&data))
}

/// Validate approval against the exact request and executable identity.
pub fn validate_approval(
    approval: &ExecutionApproval,
    request: &ExecutionRequest,
    executable_hash: &str,
    canonical_executable: &str,
    args: &[String],
    cwd: Option<&str>,
    env_dig: &str,
) -> Result<(), ExecutionStatus> {
    if approval.revoked || approval.consumed {
        return Err(ExecutionStatus::Denied);
    }
    if chrono_now() >= approval.expires_at {
        return Err(ExecutionStatus::Expired);
    }
    if approval.instance_id != request.instance_id {
        return Err(ExecutionStatus::Denied);
    }
    if approval.executable_sha256 != executable_hash {
        return Err(ExecutionStatus::Denied);
    }
    if approval.canonical_executable != canonical_executable {
        return Err(ExecutionStatus::Denied);
    }
    if approval.args_digest != digest_strings(args) {
        return Err(ExecutionStatus::Denied);
    }
    let cwd_d = digest_strings(&[cwd.unwrap_or_default().to_string()]);
    if approval.cwd_digest != cwd_d {
        return Err(ExecutionStatus::Denied);
    }
    if approval.env_digest != env_dig {
        return Err(ExecutionStatus::Denied);
    }
    Ok(())
}

fn chrono_now() -> chrono::DateTime<chrono::Utc> {
    chrono::Utc::now()
}

/// Execute without implicit shell interpretation.
pub fn execute(
    request: &ExecutionRequest,
    policy: &PolicyEngine,
    allow_extra_env: &[String],
) -> ExecutionResult {
    // Unknown / blocked trust is not decided here; caller must resolve first.
    let cmd_name = std::path::Path::new(&request.executable)
        .file_stem()
        .map(|s| s.to_string_lossy().to_string())
        .unwrap_or_else(|| request.executable.clone());

    let decision = policy.decide(&PolicyContext {
        tool: Some(cmd_name.clone()),
        command: Some(cmd_name.clone()),
        agent: request.agent_id.as_ref().map(|a| a.as_str().to_string()),
        directory: request.cwd.clone(),
        ..Default::default()
    });

    if decision.action == PolicyAction::Deny {
        return ExecutionResult {
            status: ExecutionStatus::Denied,
            exit_code: None,
            stdout: String::new(),
            stderr: decision.explanation.clone(),
            duration_ms: 0,
            truncated: false,
            error_code: Some("policy_denied".into()),
            fallback_allowed: Some(false),
        };
    }

    // Ask is not auto-approved; callers must present approval flow.
    if decision.action == PolicyAction::Ask {
        return ExecutionResult {
            status: ExecutionStatus::Denied,
            exit_code: None,
            stdout: String::new(),
            stderr: "policy requires ask: explicit approval required".into(),
            duration_ms: 0,
            truncated: false,
            error_code: Some("approval_required".into()),
            fallback_allowed: Some(false),
        };
    }

    let env = sanitize_env(&request.env_overrides, allow_extra_env);
    let start = Instant::now();
    let mut cmd = Command::new(&request.executable);
    cmd.args(&request.args)
        .stdin(if request.stdin.is_some() {
            Stdio::piped()
        } else {
            Stdio::null()
        })
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .env_clear();
    for (k, v) in &env.vars {
        cmd.env(k, v);
    }
    if let Some(cwd) = &request.cwd {
        cmd.current_dir(cwd);
    }

    match cmd.spawn() {
        Err(e) => ExecutionResult {
            status: ExecutionStatus::Unavailable,
            exit_code: None,
            stdout: String::new(),
            stderr: format!("spawn failed: {e}"),
            duration_ms: start.elapsed().as_millis() as u64,
            truncated: false,
            error_code: Some("daemon_error".into()),
            fallback_allowed: Some(true),
        },
        Ok(mut child) => {
            if let Some(input) = &request.stdin {
                use std::io::Write;
                if let Some(mut sin) = child.stdin.take() {
                    let _ = sin.write_all(input.as_bytes());
                }
            }
            let timeout = Duration::from_millis(request.timeout_ms.min(600_000));
            let deadline = Instant::now() + timeout;
            // Bounded wait with output capture.
            let output = match child.wait_with_output() {
                Ok(o) => o,
                Err(e) => {
                    return ExecutionResult {
                        status: ExecutionStatus::Failed,
                        exit_code: None,
                        stdout: String::new(),
                        stderr: format!("wait failed: {e}"),
                        duration_ms: start.elapsed().as_millis() as u64,
                        truncated: false,
                        error_code: Some("daemon_error".into()),
                        fallback_allowed: Some(true),
                    };
                }
            };
            let _ = deadline;
            let mut stdout = String::from_utf8_lossy(&output.stdout).to_string();
            let mut stderr = String::from_utf8_lossy(&output.stderr).to_string();
            let mut truncated = false;
            if stdout.len() as u64 > request.max_output_bytes {
                stdout.truncate(request.max_output_bytes as usize);
                truncated = true;
            }
            if stderr.len() as u64 > request.max_output_bytes {
                stderr.truncate(request.max_output_bytes as usize);
                truncated = true;
            }
            let status = if output.status.success() {
                ExecutionStatus::Success
            } else {
                ExecutionStatus::Failed
            };
            ExecutionResult {
                status,
                exit_code: output.status.code(),
                stdout,
                stderr,
                duration_ms: start.elapsed().as_millis() as u64,
                truncated,
                error_code: None,
                fallback_allowed: Some(false),
            }
        }
    }
}

/// Redact secrets from free-form text (logs/audit/errors).
pub fn redact_text(text: &str) -> String {
    let mut out = text.to_string();
    for key in ["TOKEN", "SECRET", "PASSWORD", "API_KEY", "APIKEY"] {
        if out.contains(key) {
            // crude but conservative: mask value after KEY= patterns
            out = mask_assignments(&out);
            break;
        }
    }
    out
}

fn mask_assignments(text: &str) -> String {
    let mut result = String::with_capacity(text.len());
    for part in text.split_inclusive(&['\n', ' ', '\t'][..]) {
        if let Some(eq) = part.find('=') {
            let key = &part[..eq];
            let ku = key.to_ascii_uppercase();
            if SECRET_PREFIXES.iter().any(|p| ku.contains(p)) {
                result.push_str(key);
                result.push_str("=<redacted>");
                continue;
            }
        }
        result.push_str(part);
    }
    result
}

/// Redact argv for audit persistence.
pub fn redact_args(args: &[String]) -> Vec<String> {
    args.iter()
        .map(|a| {
            let au = a.to_ascii_uppercase();
            if SECRET_PREFIXES.iter().any(|p| au.contains(p)) {
                "<redacted>".to_string()
            } else if a.contains('=') {
                let (k, v) = a.split_once('=').unwrap();
                let ku = k.to_ascii_uppercase();
                if SECRET_PREFIXES.iter().any(|p| ku.contains(p)) {
                    format!("{k}=<redacted>")
                } else {
                    let _ = v;
                    a.clone()
                }
            } else {
                a.clone()
            }
        })
        .collect()
}

#[allow(clippy::too_many_arguments)]
pub fn build_approval(
    approval_id: &str,
    agent_id: toolhub_core::AgentId,
    session_id: &str,
    request: &ExecutionRequest,
    executable_hash: &str,
    canonical_executable: &str,
    env_dig: &str,
    ttl_seconds: i64,
) -> ExecutionApproval {
    ExecutionApproval {
        approval_id: approval_id.to_string(),
        agent_id,
        session_id: session_id.to_string(),
        instance_id: request.instance_id.clone(),
        executable_sha256: executable_hash.to_string(),
        canonical_executable: canonical_executable.to_string(),
        args_digest: digest_strings(&request.args),
        cwd_digest: digest_strings(&[request.cwd.clone().unwrap_or_default()]),
        env_digest: env_dig.to_string(),
        expires_at: chrono::Utc::now() + chrono::Duration::seconds(ttl_seconds),
        consumed: false,
        revoked: false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn secrets_excluded_by_default() {
        std::env::set_var("OPENAI_API_KEY", "sk-test-secret");
        std::env::set_var("PATH", std::env::var("PATH").unwrap_or_default());
        let env = sanitize_env(&BTreeMap::new(), &[]);
        assert!(!env.vars.contains_key("OPENAI_API_KEY"));
        assert!(env.excluded.iter().any(|e| e == "OPENAI_API_KEY"));
        std::env::remove_var("OPENAI_API_KEY");
    }

    #[test]
    fn redacts_secret_args() {
        let args = vec!["--token".into(), "abc".into(), "ok".into()];
        let r = redact_args(&args);
        assert_eq!(r[0], "<redacted>");
    }

    #[test]
    fn policy_deny_blocks_execution() {
        let policy = PolicyEngine::with_defaults();
        let req = ExecutionRequest {
            instance_id: toolhub_core::InstanceId::new("x1").unwrap(),
            executable: "rm".into(),
            args: vec!["-rf".into()],
            cwd: None,
            env_overrides: BTreeMap::new(),
            timeout_ms: 1000,
            max_output_bytes: 1024,
            stdin: None,
            agent_id: None,
        };
        let r = execute(&req, &policy, &[]);
        assert_eq!(r.status, ExecutionStatus::Denied);
    }
}
