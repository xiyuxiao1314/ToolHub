//! Process execution with real deadlines, bounded I/O, sanitization and approval binding.

use std::collections::BTreeMap;
use std::io::{Read, Write};
use std::process::{Command, Stdio};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{mpsc, Arc};
mod process_tree;
pub use process_tree::Tree as OwnedProcessTree;
mod executable_pin;
pub use executable_pin::ExecutablePin;
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

/// Digest of effective policy decision + trust + status at mint/revalidate time.
pub fn policy_trust_digest(
    decision: &str,
    trust_level: &str,
    status: &str,
    instance_id: &str,
) -> String {
    digest_strings(&[
        decision.to_string(),
        trust_level.to_string(),
        status.to_string(),
        instance_id.to_string(),
    ])
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
    policy_trust_digest: &str,
    now: chrono::DateTime<chrono::Utc>,
) -> Result<(), ExecutionStatus> {
    if approval.revoked || approval.consumed {
        return Err(ExecutionStatus::Denied);
    }
    if now >= approval.expires_at {
        return Err(ExecutionStatus::Expired);
    }
    if approval.session_id.is_empty() {
        return Err(ExecutionStatus::Denied);
    }
    // R3-F03: fail closed on missing digests (legacy approvals invalidated).
    if approval.executable_sha256.is_empty()
        || [
            &approval.args_digest,
            &approval.cwd_digest,
            &approval.stdin_digest,
            &approval.env_digest,
            &approval.policy_trust_digest,
        ]
        .iter()
        .any(|d| !d.starts_with("v2:"))
    {
        return Err(ExecutionStatus::Denied);
    }
    if request.agent_id.as_ref() != Some(&approval.agent_id) {
        return Err(ExecutionStatus::Denied);
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
    let cwd_d = digest_optional(cwd);
    if approval.cwd_digest != cwd_d {
        return Err(ExecutionStatus::Denied);
    }
    let stdin_d = digest_optional(stdin);
    if approval.stdin_digest != stdin_d {
        return Err(ExecutionStatus::Denied);
    }
    if approval.env_digest != env_dig {
        return Err(ExecutionStatus::Denied);
    }
    // R2-B01: revalidate policy/trust revision binding when provided.
    if approval.policy_trust_digest != policy_trust_digest {
        return Err(ExecutionStatus::Denied);
    }
    Ok(())
}

fn digest_optional(value: Option<&str>) -> String {
    match value {
        Some(value) => digest_strings(&["some".into(), value.into()]),
        None => digest_strings(&["none".into()]),
    }
}

pub fn execute(
    request: &ExecutionRequest,
    policy: &PolicyEngine,
    allow_extra_env: &[String],
    ctx_extra: &PolicyContext,
    approval_validated: bool,
) -> ExecutionResult {
    execute_with_cancel(
        request,
        policy,
        allow_extra_env,
        ctx_extra,
        approval_validated,
        Arc::new(AtomicBool::new(false)),
    )
}

pub fn execute_with_cancel(
    request: &ExecutionRequest,
    policy: &PolicyEngine,
    allow_extra_env: &[String],
    ctx_extra: &PolicyContext,
    approval_validated: bool,
    cancel: Arc<AtomicBool>,
) -> ExecutionResult {
    execute_internal(
        request,
        policy,
        allow_extra_env,
        ctx_extra,
        approval_validated,
        None,
        cancel,
    )
}

#[allow(clippy::too_many_arguments)]
pub fn execute_with_pin(
    request: &ExecutionRequest,
    policy: &PolicyEngine,
    allow_extra_env: &[String],
    ctx_extra: &PolicyContext,
    approval_validated: bool,
    pin: &ExecutablePin,
    cancel: Arc<AtomicBool>,
) -> ExecutionResult {
    execute_internal(
        request,
        policy,
        allow_extra_env,
        ctx_extra,
        approval_validated,
        Some(pin),
        cancel,
    )
}

#[allow(clippy::too_many_arguments)]
fn execute_internal(
    request: &ExecutionRequest,
    policy: &PolicyEngine,
    allow_extra_env: &[String],
    ctx_extra: &PolicyContext,
    approval_validated: bool,
    pin: Option<&ExecutablePin>,
    cancel: Arc<AtomicBool>,
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
    let fail = |status, code: &str, message: String| ExecutionResult {
        status,
        exit_code: None,
        stdout: String::new(),
        stderr: message,
        duration_ms: start.elapsed().as_millis() as u64,
        truncated: false,
        error_code: Some(code.into()),
        fallback_allowed: Some(false),
    };
    if cancel.load(Ordering::Acquire) {
        return fail(
            ExecutionStatus::Cancelled,
            "cancelled",
            "cancelled before launch".into(),
        );
    }
    let mut cmd = match pin.map(ExecutablePin::command).transpose() {
        Ok(Some(command)) => command,
        Ok(None) => Command::new(&request.executable),
        Err(error) => {
            return fail(
                ExecutionStatus::Unavailable,
                "executable_pin_failed",
                redact_text(&error.to_string()),
            )
        }
    };
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
    let mut tree = match process_tree::Tree::prepare(&mut cmd) {
        Ok(t) => t,
        Err(e) => {
            return fail(
                ExecutionStatus::Unavailable,
                "process_tree_unavailable",
                redact_text(&e.to_string()),
            )
        }
    };
    let mut child = match cmd.spawn() {
        Ok(c) => c,
        Err(e) => {
            return fail(
                ExecutionStatus::Unavailable,
                "spawn_failed",
                redact_text(&e.to_string()),
            )
        }
    };
    if let Err(e) = tree.attach(&mut child) {
        let _ = child.kill();
        tree.terminate();
        return fail(
            ExecutionStatus::Unavailable,
            "process_tree_unavailable",
            redact_text(&e.to_string()),
        );
    }
    if let Err(error) = process_tree::make_pipes_interruptible(&child) {
        tree.terminate();
        let _ = child.kill();
        return fail(
            ExecutionStatus::Unavailable,
            "pipe_setup_failed",
            redact_text(&error.to_string()),
        );
    }
    let max_out = request.max_output_bytes.min(1024 * 256) as usize;
    let (tx, rx) = mpsc::channel();
    let stop_pipes = Arc::new(AtomicBool::new(false));
    let reader =
        |pipe: Box<dyn Read + Send>, which: bool, tx: mpsc::Sender<(bool, Vec<u8>, bool)>| {
            let stop = stop_pipes.clone();
            std::thread::spawn(move || {
                let mut pipe = pipe;
                let mut buf = Vec::new();
                let mut truncated = false;
                let mut chunk = [0u8; 8192];
                loop {
                    if stop.load(Ordering::Acquire) {
                        truncated = true;
                        break;
                    }
                    match pipe.read(&mut chunk) {
                        Ok(0) => break,
                        Ok(n) => {
                            let room = max_out.saturating_sub(buf.len());
                            buf.extend_from_slice(&chunk[..n.min(room)]);
                            truncated |= n > room;
                        }
                        Err(error) if error.kind() == std::io::ErrorKind::WouldBlock => {
                            std::thread::sleep(Duration::from_millis(5))
                        }
                        Err(_) => break,
                    }
                }
                let _ = tx.send((which, buf, truncated));
            })
        };
    let out = reader(Box::new(child.stdout.take().unwrap()), true, tx.clone());
    let err = reader(Box::new(child.stderr.take().unwrap()), false, tx.clone());
    drop(tx);
    let input = request.stdin.clone().unwrap_or_default();
    let stdin = child.stdin.take();
    let writer_stop = stop_pipes.clone();
    let writer = std::thread::spawn(move || {
        if let Some(mut stdin) = stdin {
            let mut offset = 0;
            while offset < input.len() && !writer_stop.load(Ordering::Acquire) {
                match stdin.write(&input.as_bytes()[offset..]) {
                    Ok(0) => break,
                    Ok(n) => offset += n,
                    Err(error) if error.kind() == std::io::ErrorKind::WouldBlock => {
                        std::thread::sleep(Duration::from_millis(5))
                    }
                    Err(_) => break,
                }
            }
        }
    });
    let deadline = start + Duration::from_millis(request.timeout_ms.clamp(1, 600_000));
    let status_loop = loop {
        if cancel.load(Ordering::Acquire) {
            break Err(ExecutionStatus::Cancelled);
        }
        match tree.poll_child(&mut child) {
            Ok(Some(status)) => break Ok(status),
            Ok(None) if Instant::now() >= deadline => break Err(ExecutionStatus::TimedOut),
            Ok(None) => std::thread::sleep(Duration::from_millis(10)),
            Err(_) => break Err(ExecutionStatus::Failed),
        }
    };
    // Close the owned tree on every outcome, including a normally exiting parent whose
    // descendants still retain stdin/stdout/stderr. Never join a pipe before teardown.
    tree.terminate();
    if status_loop.is_err() {
        let _ = child.kill();
    }
    let cleanup_deadline = Instant::now() + Duration::from_millis(500);
    let mut stdout = Vec::new();
    let mut stderr = Vec::new();
    let mut truncated = false;
    let mut received = 0;
    while received < 2 && Instant::now() < cleanup_deadline {
        match rx.recv_timeout(Duration::from_millis(10)) {
            Ok((which, data, t)) => {
                received += 1;
                truncated |= t;
                if which {
                    stdout = data;
                } else {
                    stderr = data;
                }
            }
            Err(mpsc::RecvTimeoutError::Disconnected) => break,
            Err(mpsc::RecvTimeoutError::Timeout) => {}
        }
    }
    stop_pipes.store(true, Ordering::Release);
    // Interruptible Unix collectors finish promptly even if a detached descendant
    // escaped its process group and retained a pipe. Capture their bounded remainder.
    let final_deadline = Instant::now() + Duration::from_millis(20);
    while received < 2 && Instant::now() < final_deadline {
        match rx.recv_timeout(Duration::from_millis(5)) {
            Ok((which, data, t)) => {
                received += 1;
                truncated |= t;
                if which {
                    stdout = data;
                } else {
                    stderr = data;
                }
            }
            Err(mpsc::RecvTimeoutError::Disconnected) => break,
            Err(mpsc::RecvTimeoutError::Timeout) => {}
        }
    }
    for thread in [&out, &err] {
        if !thread.is_finished() {
            process_tree::cancel_io(thread);
        }
    }
    if !writer.is_finished() {
        process_tree::cancel_io(&writer);
    }
    // Join only finished threads. Detached collectors own their handles and fixed-size
    // buffers; an OS pipe failure cannot hold the service response indefinitely.
    if out.is_finished() {
        let _ = out.join();
    } else {
        truncated = true;
    }
    if err.is_finished() {
        let _ = err.join();
    } else {
        truncated = true;
    }
    if writer.is_finished() {
        let _ = writer.join();
    }
    let _ = child.try_wait();
    let mut stdout = utf8_lossy_cap(&stdout, max_out, &mut truncated);
    let mut stderr = utf8_lossy_cap(&stderr, max_out, &mut truncated);
    cap_json_output(&mut stdout, &mut stderr, max_out, &mut truncated);
    let (status, exit_code, error_code) = match status_loop {
        Ok(st) => (
            if st.success() {
                ExecutionStatus::Success
            } else {
                ExecutionStatus::Failed
            },
            st.code(),
            if st.success() {
                None
            } else {
                Some("child_failed".into())
            },
        ),
        Err(st) => (
            st,
            None,
            Some(
                match st {
                    ExecutionStatus::TimedOut => "timeout",
                    ExecutionStatus::Cancelled => "cancelled",
                    _ => "process_failed",
                }
                .into(),
            ),
        ),
    };
    ExecutionResult {
        status,
        exit_code,
        stdout,
        stderr,
        duration_ms: start.elapsed().as_millis() as u64,
        truncated,
        error_code,
        fallback_allowed: Some(false),
    }
}

/// UTF-8-safe truncation that never panics on multibyte boundaries.
fn utf8_lossy_cap(bytes: &[u8], max: usize, truncated: &mut bool) -> String {
    let mut out = String::from_utf8_lossy(bytes).into_owned();
    if out.len() > max {
        *truncated = true;
        let mut end = max;
        while !out.is_char_boundary(end) {
            end -= 1;
        }
        out.truncate(end);
    }
    out
}

/// Aggregate output budget counts UTF-8 JSON escaped bytes, excluding the two quote pairs.
fn cap_json_output(stdout: &mut String, stderr: &mut String, max: usize, truncated: &mut bool) {
    let mut remaining = max;
    for text in [stdout, stderr] {
        let mut end = 0;
        for (offset, c) in text.char_indices() {
            let cost = match c {
                '"' | '\\' | '\n' | '\r' | '\t' | '\u{8}' | '\u{c}' => 2,
                c if c <= '\u{1f}' => 6,
                c => c.len_utf8(),
            };
            if cost > remaining {
                break;
            }
            remaining -= cost;
            end = offset + c.len_utf8();
        }
        if end < text.len() {
            *truncated = true;
            text.truncate(end);
        }
    }
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
        let key = p.trim_matches(['"', '\'', ':']);
        if p.ends_with(':') && is_secret_name(key) {
            result.push_str(key);
            result.push_str(": <redacted>");
            if let Some(next) = parts.next() {
                if key.eq_ignore_ascii_case("authorization")
                    && (next.eq_ignore_ascii_case("bearer") || next.eq_ignore_ascii_case("basic"))
                {
                    parts.next();
                }
            }
            continue;
        }
        let is_flag = is_secret_flag(p);
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
            if is_secret_flag(k) {
                out.push(format!("{k}=<redacted>"));
            } else {
                out.push(a.clone());
            }
            continue;
        }
        if is_secret_flag(a) {
            out.push(a.clone());
            skip_next = true;
        } else if is_secret_name(a) {
            out.push("<redacted>".into());
        } else {
            out.push(a.clone());
        }
    }
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
    let (h, offsets) = fold_with_offsets(hay);
    let (n, _) = fold_with_offsets(needle);
    if n.is_empty() {
        return hay.to_string();
    }
    let mut result = String::with_capacity(hay.len());
    let mut i = 0;
    while let Some(pos) = h[i..].find(&n) {
        let abs = i + pos;
        let end = abs + n.len();
        if let (Some(start), Some(stop), Some(previous)) =
            (offsets.get(&abs), offsets.get(&end), offsets.get(&i))
        {
            result.push_str(&hay[*previous..*start]);
            result.push_str(rep);
            i = end;
            let _ = stop;
        } else {
            let next = h[abs..].chars().next().map(char::len_utf8).unwrap_or(1);
            let boundary = abs + next;
            if let (Some(previous), Some(stop)) = (offsets.get(&i), offsets.get(&boundary)) {
                result.push_str(&hay[*previous..*stop]);
                i = boundary;
            } else {
                return hay.to_string();
            }
        }
    }
    result.push_str(&hay[*offsets.get(&i).unwrap_or(&hay.len())..]);
    result
}

fn fold_with_offsets(text: &str) -> (String, BTreeMap<usize, usize>) {
    let mut folded = String::new();
    let mut offsets = BTreeMap::new();
    for (i, c) in text.char_indices() {
        offsets.insert(folded.len(), i);
        if c == '\\' {
            folded.push('/');
        } else {
            folded.extend(c.to_lowercase());
        }
    }
    offsets.insert(folded.len(), text.len());
    (folded, offsets)
}

fn is_secret_flag(flag: &str) -> bool {
    flag.starts_with('-')
        && (is_secret_name(flag) || matches!(flag, "-t" | "-p" | "-k" | "-u" | "-H"))
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
    policy_trust_digest: &str,
    ttl_seconds: i64,
) -> ExecutionApproval {
    let stdin_d = digest_optional(request.stdin.as_deref());
    ExecutionApproval {
        approval_id: approval_id.to_string(),
        agent_id,
        session_id: session_id.to_string(),
        instance_id: request.instance_id.clone(),
        executable_sha256: executable_hash.to_string(),
        canonical_executable: canonical_executable.to_string(),
        args_digest: digest_strings(&request.args),
        cwd_digest: digest_optional(request.cwd.as_deref()),
        stdin_digest: stdin_d,
        env_digest: env_dig.to_string(),
        policy_trust_digest: policy_trust_digest.to_string(),
        expires_at: chrono::Utc::now() + chrono::Duration::seconds(ttl_seconds),
        consumed: false,
        revoked: false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fixture_python() -> String {
        if let Some(path) = std::env::var_os("TOOLHUB_TEST_PYTHON") {
            return std::fs::canonicalize(path)
                .expect("TOOLHUB_TEST_PYTHON must point to an installed Python executable")
                .to_string_lossy()
                .into_owned();
        }
        let names: &[&str] = if cfg!(windows) {
            &["python.exe", "python3.exe"]
        } else {
            &["python3", "python"]
        };
        for dir in std::env::split_paths(&std::env::var_os("PATH").unwrap_or_default()) {
            for name in names {
                let path = dir.join(name);
                if path.is_file() {
                    return std::fs::canonicalize(path)
                        .unwrap()
                        .to_string_lossy()
                        .into_owned();
                }
            }
        }
        panic!("Python fixture unavailable; set TOOLHUB_TEST_PYTHON to an installed interpreter")
    }

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
        let python = fixture_python();
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
        let args = vec!["-t".into(), "opaque123".into(), "ok".into()];
        let r = redact_args(&args);
        assert_eq!(r[1], "<redacted>");
        assert!(
            !redact_text("-t opaque123 Authorization: Bearer opaque456 token: opaque789")
                .contains("opaque")
        );
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
    fn invalid_utf8_final_bytes_are_capped() {
        let mut t = false;
        assert!(utf8_lossy_cap(&[255, 255, 255], 3, &mut t).len() <= 3);
    }

    #[test]
    fn unicode_path_redaction_does_not_slice_case_expansion() {
        assert_eq!(
            redact_path_for_export("İ/Users/张三/x", &["/Users/张三".into()], &[]),
            "İ~/x"
        );
    }

    #[test]
    fn normal_parent_exit_closes_owned_descendant_pipes() {
        let python = fixture_python();
        let req = ExecutionRequest {
            instance_id: toolhub_core::InstanceId::new("fixture").unwrap(), executable:python,
            args:vec!["-c".into(),"import subprocess,sys; subprocess.Popen([sys.executable,'-c','import time; time.sleep(2)']); print('parent',flush=True)".into()],
            cwd:None, env_overrides:BTreeMap::new(), timeout_ms:300, max_output_bytes:1024, stdin:None,agent_id:None,
        };
        let started = Instant::now();
        let r = execute(
            &req,
            &PolicyEngine::default(),
            &[],
            &PolicyContext::default(),
            true,
        );
        assert_eq!(r.status, ExecutionStatus::Success);
        assert!(
            started.elapsed() < Duration::from_millis(1200),
            "inherited pipes delayed return {:?}",
            started.elapsed()
        );
    }

    fn fixture_request(script: &str, timeout: u64, cap: u64) -> ExecutionRequest {
        ExecutionRequest {
            instance_id: toolhub_core::InstanceId::new("owned.fixture").unwrap(),
            executable: fixture_python(),
            args: vec!["-c".into(), script.into()],
            cwd: None,
            env_overrides: BTreeMap::new(),
            timeout_ms: timeout,
            max_output_bytes: cap,
            stdin: None,
            agent_id: None,
        }
    }

    #[test]
    fn cancellation_finishes_owned_child_and_reports_cancelled() {
        let token = Arc::new(AtomicBool::new(false));
        let other = token.clone();
        let trigger = std::thread::spawn(move || {
            std::thread::sleep(Duration::from_millis(100));
            other.store(true, Ordering::Release);
        });
        let start = Instant::now();
        let result = execute_with_cancel(
            &fixture_request("import time; time.sleep(2)", 3000, 1024),
            &PolicyEngine::default(),
            &[],
            &PolicyContext::default(),
            true,
            token,
        );
        trigger.join().unwrap();
        assert_eq!(result.status, ExecutionStatus::Cancelled);
        assert_eq!(result.error_code.as_deref(), Some("cancelled"));
        assert!(start.elapsed() < Duration::from_millis(900));
    }

    #[test]
    fn output_budget_counts_json_escaping_and_decoded_utf8() {
        let result=execute(&fixture_request("import sys; sys.stdout.buffer.write(bytes([255])*100+b'\\x00'*100); sys.stderr.write('\\\\'*100)",3000,64),&PolicyEngine::default(),&[],&PolicyContext::default(),true);
        assert_eq!(result.status, ExecutionStatus::Success);
        let cost = serde_json::to_vec(&result.stdout).unwrap().len()
            + serde_json::to_vec(&result.stderr).unwrap().len()
            - 4;
        assert!(cost <= 64, "serialized output {cost}");
        assert!(result.truncated);
    }

    #[test]
    fn finite_failure_preserves_exit_code() {
        let result = execute(
            &fixture_request("import sys; sys.exit(7)", 3000, 64),
            &PolicyEngine::default(),
            &[],
            &PolicyContext::default(),
            true,
        );
        assert_eq!(result.status, ExecutionStatus::Failed);
        assert_eq!(result.exit_code, Some(7));
        assert_eq!(result.error_code.as_deref(), Some("child_failed"));
    }

    #[test]
    fn pinned_known_fixture_executes_same_identity() {
        let request = fixture_request("print('pinned-fixture')", 3000, 128);
        let pin = ExecutablePin::open(std::path::Path::new(&request.executable)).unwrap();
        assert_eq!(pin.hash().unwrap(), hash_file(&request.executable).unwrap());
        let result = execute_with_pin(
            &request,
            &PolicyEngine::default(),
            &[],
            &PolicyContext::default(),
            true,
            &pin,
            Arc::new(AtomicBool::new(false)),
        );
        assert_eq!(result.status, ExecutionStatus::Success);
        assert!(result.stdout.contains("pinned-fixture"));
    }

    #[cfg(windows)]
    #[test]
    fn windows_executable_pin_denies_owned_fixture_write_and_replace() {
        let path = std::env::temp_dir().join(format!(
            "toolhub-pin-owned-{}-{}.bin",
            std::process::id(),
            chrono::Utc::now().timestamp_nanos_opt().unwrap()
        ));
        std::fs::write(&path, "owned bytes").unwrap();
        let pin = ExecutablePin::open(&path).unwrap();
        assert!(std::fs::OpenOptions::new().write(true).open(&path).is_err());
        assert!(std::fs::rename(&path, path.with_extension("replaced")).is_err());
        drop(pin);
        std::fs::remove_file(path).unwrap();
    }

    #[test]
    fn approval_rejects_missing_identity_legacy_digest_and_changed_optional_stdin() {
        let mut request = fixture_request("pass", 3000, 64);
        let agent = toolhub_core::AgentId::new("caller.fixture").unwrap();
        request.agent_id = Some(agent.clone());
        let env = digest_map(&BTreeMap::new());
        let policy = digest_strings(&["context".into()]);
        let approval = build_approval(
            "a",
            agent,
            "session",
            &request,
            "hash",
            &request.executable,
            &env,
            &policy,
            300,
        );
        let validate = |a: &ExecutionApproval, r: &ExecutionRequest| {
            validate_approval(
                a,
                r,
                "hash",
                &r.executable,
                &r.args,
                r.cwd.as_deref(),
                r.stdin.as_deref(),
                &env,
                &policy,
                chrono::Utc::now(),
            )
        };
        assert!(validate(&approval, &request).is_ok());
        request.stdin = Some(String::new());
        assert!(validate(&approval, &request).is_err());
        request.stdin = None;
        request.agent_id = None;
        assert!(validate(&approval, &request).is_err());
        request.agent_id = Some(approval.agent_id.clone());
        let mut legacy = approval;
        legacy.args_digest = legacy.args_digest.trim_start_matches("v2:").into();
        assert!(validate(&legacy, &request).is_err());
    }

    #[test]
    fn owned_descendant_is_terminated_on_normal_exit_and_timeout() {
        for parent in ["", "; time.sleep(2)"] {
            let marker = std::env::temp_dir().join(format!(
                "toolhub-owned-job-{}-{}.txt",
                std::process::id(),
                chrono::Utc::now().timestamp_nanos_opt().unwrap()
            ));
            let child = format!(
                "import time,pathlib; time.sleep(0.8); pathlib.Path({}).write_text('alive')",
                serde_json::to_string(&marker.to_string_lossy()).unwrap()
            );
            let script = format!(
                "import subprocess,sys,time; subprocess.Popen([sys.executable,'-c',{}]){parent}",
                serde_json::to_string(&child).unwrap()
            );
            let result = execute(
                &fixture_request(&script, 300, 1024),
                &PolicyEngine::default(),
                &[],
                &PolicyContext::default(),
                true,
            );
            assert_eq!(
                result.status,
                if parent.is_empty() {
                    ExecutionStatus::Success
                } else {
                    ExecutionStatus::TimedOut
                }
            );
            std::thread::sleep(Duration::from_millis(950));
            assert!(!marker.exists(), "owned descendant survived job close");
        }
    }

    #[test]
    fn timeout_kills_child() {
        // Use a long-running Python sleep which exists in test envs.
        let python = fixture_python();
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
