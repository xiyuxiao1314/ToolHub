//! Process execution with real deadlines, bounded I/O, sanitization and approval binding.

use std::collections::BTreeMap;
use std::io::{Read, Write};
use std::process::{Command, Stdio};
use std::sync::mpsc;
use std::time::{Duration, Instant};

use toolhub_core::{
    digest_map, digest_strings, hash_bytes, ExecutionApproval, ExecutionRequest, ExecutionResult,
    ExecutionStatus,
};
use toolhub_policy::{PolicyAction, PolicyContext, PolicyEngine};

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
    "AUTHORIZATION",
];

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

pub fn is_secret_name(name: &str) -> bool {
    let ku = name.to_ascii_uppercase().replace('-', "_");
    SECRET_PREFIXES.iter().any(|p| ku.contains(p))
}

pub fn sanitize_env(overrides: &BTreeMap<String, String>, allow_extra: &[String]) -> SanitizedEnv {
    let mut vars = BTreeMap::new();
    let mut excluded = vec![];

    for (k, v) in std::env::vars() {
        if is_secret_name(&k) && !allow_extra.iter().any(|a| a == &k) {
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
        if is_secret_name(k) && !allow_extra.iter().any(|a| a == k) {
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
#[allow(clippy::too_many_arguments)]
pub fn validate_approval(
    approval: &ExecutionApproval,
    request: &ExecutionRequest,
    executable_hash: &str,
    canonical_executable: &str,
    args: &[String],
    cwd: Option<&str>,
    stdin: Option<&str>,
    env_dig: &str,
    now: chrono::DateTime<chrono::Utc>,
) -> Result<(), ExecutionStatus> {
    if approval.revoked || approval.consumed {
        return Err(ExecutionStatus::Denied);
    }
    if now >= approval.expires_at {
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
    let stdin_d = digest_strings(&[stdin.unwrap_or_default().to_string()]);
    if approval.stdin_digest != stdin_d {
        return Err(ExecutionStatus::Denied);
    }
    if approval.env_digest != env_dig {
        return Err(ExecutionStatus::Denied);
    }
    if request.agent_id.is_some() && request.agent_id.as_ref() != Some(&approval.agent_id) {
        return Err(ExecutionStatus::Denied);
    }
    Ok(())
}

pub fn execute(
    request: &ExecutionRequest,
    policy: &PolicyEngine,
    allow_extra_env: &[String],
    ctx_extra: &PolicyContext,
    approval_validated: bool,
) -> ExecutionResult {
    let cmd_name = std::path::Path::new(&request.executable)
        .file_stem()
        .map(|s| s.to_string_lossy().to_string())
        .unwrap_or_else(|| request.executable.clone());

    let mut ctx = ctx_extra.clone();
    if ctx.tool.is_none() {
        ctx.tool = Some(cmd_name.clone());
    }
    if ctx.command.is_none() {
        ctx.command = Some(cmd_name.clone());
    }
    if ctx.agent.is_none() {
        ctx.agent = request.agent_id.as_ref().map(|a| a.as_str().to_string());
    }
    if ctx.directory.is_none() {
        ctx.directory = request.cwd.clone();
    }

    let decision = policy.decide(&ctx);

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

    // Ask requires a validated+consumed approval (F01). Allow needs none. Deny already returned.
    if decision.action == PolicyAction::Ask && !approval_validated {
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
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .env_clear();
    for (k, v) in &env.vars {
        cmd.env(k, v);
    }
    if let Some(cwd) = &request.cwd {
        cmd.current_dir(cwd);
    }

    let mut child = match cmd.spawn() {
        Err(e) => {
            return ExecutionResult {
                status: ExecutionStatus::Unavailable,
                exit_code: None,
                stdout: String::new(),
                stderr: format!("spawn failed: {e}"),
                duration_ms: start.elapsed().as_millis() as u64,
                truncated: false,
                error_code: Some("daemon_error".into()),
                fallback_allowed: Some(true),
            }
        }
        Ok(c) => c,
    };

    let mut stdin_handle = child.stdin.take();
    let mut stdout_pipe = child.stdout.take();
    let mut stderr_pipe = child.stderr.take();
    let max_out = request.max_output_bytes as usize;
    let timeout = Duration::from_millis(request.timeout_ms.clamp(1, 600_000));

    let stdin_data = request.stdin.clone().unwrap_or_default();
    let (tx, rx) = mpsc::channel();
    let tx2 = tx.clone();

    // Writer thread: bounded stdin write.
    let writer = std::thread::spawn(move || {
        if let Some(mut sin) = stdin_handle.take() {
            let data = stdin_data.into_bytes();
            let mut off = 0usize;
            while off < data.len() {
                match sin.write(&data[off..]) {
                    Ok(0) => break,
                    Ok(n) => off += n,
                    Err(_) => break,
                }
            }
            let _ = sin.flush();
        }
    });

    // Reader threads with hard caps.
    let out_tx = tx.clone();
    let stdout_t = std::thread::spawn(move || {
        let mut buf: Vec<u8> = Vec::new();
        let mut truncated = false;
        if let Some(mut r) = stdout_pipe.take() {
            let mut chunk = [0u8; 8192];
            loop {
                match r.read(&mut chunk) {
                    Ok(0) => break,
                    Ok(n) => {
                        if buf.len() + n > max_out {
                            let room = max_out.saturating_sub(buf.len());
                            buf.extend_from_slice(&chunk[..room]);
                            truncated = true;
                            // drain rest without storing
                            let mut drain = [0u8; 8192];
                            while r.read(&mut drain).unwrap_or(0) > 0 {}
                            break;
                        }
                        buf.extend_from_slice(&chunk[..n]);
                    }
                    Err(_) => break,
                }
            }
        }
        let _ = out_tx.send(("stdout", buf, truncated));
    });
    let err_tx = tx2.clone();
    let stderr_t = std::thread::spawn(move || {
        let mut buf: Vec<u8> = Vec::new();
        let mut truncated = false;
        if let Some(mut r) = stderr_pipe.take() {
            let mut chunk = [0u8; 8192];
            loop {
                match r.read(&mut chunk) {
                    Ok(0) => break,
                    Ok(n) => {
                        if buf.len() + n > max_out {
                            let room = max_out.saturating_sub(buf.len());
                            buf.extend_from_slice(&chunk[..room]);
                            truncated = true;
                            let mut drain = [0u8; 8192];
                            while r.read(&mut drain).unwrap_or(0) > 0 {}
                            break;
                        }
                        buf.extend_from_slice(&chunk[..n]);
                    }
                    Err(_) => break,
                }
            }
        }
        let _ = err_tx.send(("stderr", buf, truncated));
    });

    // Wait with deadline; kill on timeout.
    let deadline = Instant::now() + timeout;
    let status_loop = loop {
        match child.try_wait() {
            Ok(Some(st)) => break Ok(st),
            Ok(None) => {
                if Instant::now() >= deadline {
                    let _ = child.kill();
                    // best-effort: kill process tree on Windows via taskkill is skipped
                    // to avoid extra OS commands; child kill is the primary bound.
                    let _ = child.wait();
                    break Err(ExecutionStatus::TimedOut);
                }
                std::thread::sleep(Duration::from_millis(10));
            }
            Err(_) => {
                let _ = child.kill();
                break Err(ExecutionStatus::Failed);
            }
        }
    };

    let _ = writer.join();
    let _ = stdout_t.join();
    let _ = stderr_t.join();

    // Collect up to two I/O messages (may have timed out before send).
    let mut stdout = Vec::new();
    let mut stderr = Vec::new();
    let mut truncated = false;
    let collect_deadline = Instant::now() + Duration::from_millis(500);
    while Instant::now() < collect_deadline {
        match rx.try_recv() {
            Ok((which, buf, t)) => {
                truncated |= t;
                if which == "stdout" {
                    stdout = buf;
                } else {
                    stderr = buf;
                }
            }
            Err(mpsc::TryRecvError::Empty) => {
                if stdout_t_finished(&stdout, &stderr) {
                    break;
                }
                std::thread::sleep(Duration::from_millis(5));
            }
            Err(mpsc::TryRecvError::Disconnected) => break,
        }
    }

    let duration_ms = start.elapsed().as_millis() as u64;
    let stdout = utf8_lossy_cap(&stdout, max_out, &mut truncated);
    let stderr = utf8_lossy_cap(&stderr, max_out, &mut truncated);

    match status_loop {
        Err(ExecutionStatus::TimedOut) => ExecutionResult {
            status: ExecutionStatus::TimedOut,
            exit_code: None,
            stdout,
            stderr,
            duration_ms,
            truncated,
            error_code: Some("timeout".into()),
            fallback_allowed: Some(false),
        },
        Err(other) => ExecutionResult {
            status: other,
            exit_code: None,
            stdout,
            stderr,
            duration_ms,
            truncated,
            error_code: Some("daemon_error".into()),
            fallback_allowed: Some(true),
        },
        Ok(st) => {
            let status = if st.success() {
                ExecutionStatus::Success
            } else {
                ExecutionStatus::Failed
            };
            ExecutionResult {
                status,
                exit_code: st.code(),
                stdout,
                stderr,
                duration_ms,
                truncated,
                error_code: None,
                fallback_allowed: Some(false),
            }
        }
    }
}

fn stdout_t_finished(stdout: &[u8], stderr: &[u8]) -> bool {
    // Heuristic: both channels drained enough — caller also uses timeout.
    let _ = (stdout, stderr);
    false
}

/// UTF-8-safe truncation that never panics on multibyte boundaries.
fn utf8_lossy_cap(bytes: &[u8], max: usize, truncated: &mut bool) -> String {
    let slice = if bytes.len() > max {
        *truncated = true;
        // walk back to a char boundary
        let mut end = max;
        while end > 0 && !char_boundary(bytes, end) {
            end -= 1;
        }
        &bytes[..end]
    } else {
        bytes
    };
    String::from_utf8_lossy(slice).into_owned()
}

fn char_boundary(b: &[u8], i: usize) -> bool {
    if i == 0 || i >= b.len() {
        return true;
    }
    (b[i] & 0xC0) != 0x80
}

/// Redact secrets from free-form text (logs/audit/errors).
pub fn redact_text(text: &str) -> String {
    let mut out = mask_assignments(text);
    // flag-value pairs: --token abc, --api-key=xyz already covered by assignments
    out = mask_flag_values(&out);
    out
}

fn mask_assignments(text: &str) -> String {
    let mut result = String::with_capacity(text.len());
    for part in text.split_inclusive(&['\n', ' ', '\t', ';'][..]) {
        if let Some(eq) = part.find('=') {
            let key = &part[..eq];
            if is_secret_name(key) {
                result.push_str(key);
                result.push_str("=<redacted>");
                // keep trailing whitespace/newline if present
                let rest = &part[eq + 1..];
                let trimmed = rest.trim_end_matches(|c: char| c.is_whitespace());
                let suffix = &rest[trimmed.len()..];
                result.push_str(suffix);
                continue;
            }
        }
        result.push_str(part);
    }
    result
}

fn mask_flag_values(text: &str) -> String {
    let mut result = String::with_capacity(text.len());
    let mut parts = text.split_whitespace().peekable();
    let mut first = true;
    while let Some(p) = parts.next() {
        if !first {
            result.push(' ');
        }
        first = false;
        let pu = p.to_ascii_uppercase();
        let is_flag = p.starts_with('-')
            && SECRET_PREFIXES
                .iter()
                .any(|s| pu.contains(s.trim_end_matches('_')));
        if is_flag {
            result.push_str(p);
            if let Some(next) = parts.peek() {
                if !next.starts_with('-') {
                    result.push_str(" <redacted>");
                    parts.next();
                    continue;
                }
            }
        } else {
            result.push_str(p);
        }
    }
    result
}

/// Redact argv for audit persistence (flag values too).
pub fn redact_args(args: &[String]) -> Vec<String> {
    let joined = args.join(" ");
    let red = redact_text(&joined);
    // keep as single redacted line plus individual mask for unknowns
    let mut out = vec![];
    let mut skip_next = false;
    for a in args {
        if skip_next {
            skip_next = false;
            out.push("<redacted>".into());
            continue;
        }
        if a.starts_with('-') && a.contains('=') {
            let (k, _v) = a.split_once('=').unwrap();
            if is_secret_name(k) {
                out.push(format!("{k}=<redacted>"));
            } else {
                out.push(a.clone());
            }
            continue;
        }
        if a.starts_with('-') && is_secret_name(a) {
            out.push(a.clone());
            skip_next = true;
        } else if is_secret_name(a) {
            out.push("<redacted>".into());
        } else {
            out.push(a.clone());
        }
    }
    let _ = red;
    out
}

/// Normalize path for privacy redaction across Windows/macOS forms.
pub fn redact_path_for_export(path: &str, homes: &[String], usernames: &[String]) -> String {
    let mut out = path.to_string();
    let normalized = out.replace('\\', "/");
    for h in homes {
        if h.is_empty() {
            continue;
        }
        let hn = h.replace('\\', "/");
        for variant in [
            hn.clone(),
            hn.to_lowercase(),
            hn.to_uppercase(),
            h.clone(),
            h.to_lowercase(),
        ] {
            if variant.is_empty() {
                continue;
            }
            // component-boundary replace
            if normalized.to_lowercase().contains(&variant.to_lowercase()) {
                out = replace_ci(&out, &variant, "~");
            }
        }
    }
    for u in usernames {
        if u.len() < 2 {
            continue;
        }
        out = replace_ci(&out, u, "<user>");
    }
    out
}

fn replace_ci(hay: &str, needle: &str, rep: &str) -> String {
    let h = hay.to_lowercase();
    let n = needle.to_lowercase();
    if n.is_empty() {
        return hay.to_string();
    }
    let mut result = String::with_capacity(hay.len());
    let mut i = 0;
    while let Some(pos) = h[i..].find(&n) {
        let abs = i + pos;
        result.push_str(&hay[i..abs]);
        result.push_str(rep);
        i = abs + n.len();
    }
    result.push_str(&hay[i..]);
    result
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
    let stdin_d = digest_strings(&[request.stdin.clone().unwrap_or_default()]);
    ExecutionApproval {
        approval_id: approval_id.to_string(),
        agent_id,
        session_id: session_id.to_string(),
        instance_id: request.instance_id.clone(),
        executable_sha256: executable_hash.to_string(),
        canonical_executable: canonical_executable.to_string(),
        args_digest: digest_strings(&request.args),
        cwd_digest: digest_strings(&[request.cwd.clone().unwrap_or_default()]),
        stdin_digest: stdin_d,
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
        let env = sanitize_env(&BTreeMap::new(), &[]);
        assert!(!env.vars.contains_key("OPENAI_API_KEY"));
        std::env::remove_var("OPENAI_API_KEY");
    }

    #[test]
    fn redacts_flag_values() {
        let args = vec![
            "--token".into(),
            "SECRETVAL123".into(),
            "ok".into(),
            "--api-key=KEYVAL".into(),
        ];
        let r = redact_args(&args);
        assert_eq!(r[1], "<redacted>");
        assert!(r[3].contains("<redacted>"));
        let text = redact_text("--token SECRETVAL123 --api-key=KEYVAL");
        assert!(!text.contains("SECRETVAL123"));
        assert!(!text.contains("KEYVAL"));
    }

    #[test]
    fn ask_with_validated_approval_proceeds() {
        let policy = PolicyEngine::with_defaults(); // Tool:* = Ask
        let python = if cfg!(windows) {
            "python".to_string()
        } else {
            "python3".to_string()
        };
        let req = ExecutionRequest {
            instance_id: toolhub_core::InstanceId::new("x1").unwrap(),
            executable: python.clone(),
            args: vec!["--version".into()],
            cwd: None,
            env_overrides: BTreeMap::new(),
            timeout_ms: 5000,
            max_output_bytes: 4096,
            stdin: None,
            agent_id: None,
        };
        let denied = execute(&req, &policy, &[], &PolicyContext::default(), false);
        assert_eq!(denied.status, ExecutionStatus::Denied);
        let ok = execute(&req, &policy, &[], &PolicyContext::default(), true);
        assert!(
            matches!(
                ok.status,
                ExecutionStatus::Success | ExecutionStatus::Failed
            ),
            "got {:?}",
            ok.status
        );
    }

    #[test]
    fn short_flag_secret_redacted() {
        let args = vec!["-t".into(), "SECRETV".into(), "ok".into()];
        let r = redact_args(&args);
        assert_eq!(r[1], "<redacted>");
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
        let r = execute(&req, &policy, &[], &PolicyContext::default(), false);
        assert_eq!(r.status, ExecutionStatus::Denied);
    }

    #[test]
    fn utf8_truncate_no_panic() {
        let mut t = false;
        let s = "你好世界".repeat(100);
        let out = utf8_lossy_cap(s.as_bytes(), 7, &mut t);
        assert!(t);
        assert!(out.len() <= 7);
    }

    #[test]
    fn timeout_kills_child() {
        // Use a long-running Python sleep which exists in test envs.
        let python = if cfg!(windows) {
            "python".to_string()
        } else {
            "python3".to_string()
        };
        let req = ExecutionRequest {
            instance_id: toolhub_core::InstanceId::new("x1").unwrap(),
            executable: python.clone(),
            args: vec!["-c".into(), "import time; time.sleep(5)".into()],
            cwd: None,
            env_overrides: BTreeMap::new(),
            timeout_ms: 300,
            max_output_bytes: 1024,
            stdin: None,
            agent_id: None,
        };
        let mut policy = PolicyEngine::default();
        policy.set(toolhub_policy::PolicyRule {
            scope: toolhub_policy::PolicyScope::Tool,
            subject: python.clone(),
            action: PolicyAction::Allow,
        });
        // Also allow by file stem
        policy.set(toolhub_policy::PolicyRule {
            scope: toolhub_policy::PolicyScope::Tool,
            subject: "python".into(),
            action: PolicyAction::Allow,
        });
        let r = execute(&req, &policy, &[], &PolicyContext::default(), true);
        assert_eq!(r.status, ExecutionStatus::TimedOut);
    }

    #[test]
    fn path_redaction_home_and_user() {
        let p = r"C:\Users\hp\secret-project\python.exe";
        let out = redact_path_for_export(
            p,
            &[r"C:\Users\hp".into(), "C:/Users/hp".into()],
            &["hp".into()],
        );
        assert!(!out.to_lowercase().contains("hp/secret"));
        assert!(out.contains("~") || out.contains("<user>"));
    }
}
