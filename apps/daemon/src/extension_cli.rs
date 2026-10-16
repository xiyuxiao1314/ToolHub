//! Declarative CLI forms are rendered and invoked by the trusted host, never by frame scripts.
use super::execution_jobs::{invalid, text, PreparedExecution};
use super::*;

impl DaemonService {
    pub(super) fn cli_request(&mut self, params: &Value) -> Result<(Value, Value), ProtocolError> {
        self.require_feature("registry.search")?;
        let id = text(params, "id")?;
        let digest = text(params, "expected_digest")?;
        let package = self
            .extension_packages(true)?
            .into_iter()
            .find(|(p, d, _)| p.id == id && d == digest)
            .map(|(p, _, _)| p)
            .ok_or_else(|| invalid("extension disabled, removed or updated"))?;
        let cli = package
            .cli
            .as_ref()
            .ok_or_else(|| invalid("no CLI contribution"))?;
        let action = cli
            .actions
            .iter()
            .find(|a| a.id == params["action_id"].as_str().unwrap_or(""))
            .ok_or_else(|| invalid("unknown CLI action"))?;
        let instance_id = text(params, "instance_id")?;
        let instance = self
            .registry
            .get_instance(instance_id)
            .map_err(db_err)?
            .ok_or_else(|| invalid("instance not found"))?;
        if instance.definition_id != cli.tool_definition || instance.status != "available" {
            return Err(ProtocolError::denied(
                "wrong or unavailable tool for extension",
            ));
        }
        let values = params["values"]
            .as_object()
            .ok_or_else(|| invalid("values object required"))?;
        if values.len() != action.parameters.len()
            || values
                .keys()
                .any(|k| !action.parameters.iter().any(|p| &p.id == k))
        {
            return Err(invalid("exact declared CLI parameter set required"));
        }
        let mut substitutions = BTreeMap::new();
        let mut input = None;
        let mut output = None;
        for p in &action.parameters {
            let raw = values
                .get(&p.id)
                .and_then(Value::as_str)
                .filter(|s| !s.is_empty() && s.len() <= 4096 && !s.chars().any(char::is_control))
                .ok_or_else(|| invalid("bounded CLI parameter string required"))?;
            let value = if p.kind == "choice" {
                if !p.choices.iter().any(|c| c.value == raw) {
                    return Err(invalid("undeclared CLI choice"));
                }
                raw.to_owned()
            } else {
                let path = Path::new(raw);
                if !path.is_absolute()
                    || path.components().any(|c| {
                        matches!(
                            c,
                            std::path::Component::ParentDir | std::path::Component::CurDir
                        )
                    })
                {
                    return Err(invalid("select an absolute local file path"));
                }
                #[cfg(windows)]
                if raw.starts_with(['\\', '/']) || raw.get(3..).is_some_and(|s| s.contains(':')) {
                    return Err(invalid(
                        "network, device and alternate-stream paths unavailable",
                    ));
                }
                if !path
                    .extension()
                    .and_then(|v| v.to_str())
                    .is_some_and(|e| p.extensions.iter().any(|v| v.eq_ignore_ascii_case(e)))
                {
                    return Err(invalid("file suffix not accepted by action"));
                }
                let canonical = if p.kind == "input_file" {
                    let file = std::fs::canonicalize(path)
                        .map_err(|_| invalid("input file unavailable"))?;
                    if !file.is_file() {
                        return Err(invalid("regular input file required"));
                    }
                    input = Some(file.clone());
                    file
                } else {
                    if std::fs::symlink_metadata(path).is_ok() {
                        return Err(invalid("output already exists; select a new filename"));
                    }
                    let parent = std::fs::canonicalize(
                        path.parent()
                            .ok_or_else(|| invalid("output directory required"))?,
                    )
                    .map_err(|_| invalid("output directory unavailable"))?;
                    if !parent.is_dir() {
                        return Err(invalid("output directory required"));
                    }
                    let name = path
                        .file_name()
                        .ok_or_else(|| invalid("output filename required"))?;
                    let name_str = name.to_string_lossy();
                    #[cfg(windows)]
                    if name_str.ends_with(['.', ' '])
                        || name_str.contains(['<', '>', '"', '|', '?', '*', ':'])
                        || matches!(
                            name_str
                                .split('.')
                                .next()
                                .unwrap_or("")
                                .to_ascii_uppercase()
                                .as_str(),
                            "CON"
                                | "PRN"
                                | "AUX"
                                | "NUL"
                                | "COM1"
                                | "COM2"
                                | "COM3"
                                | "COM4"
                                | "COM5"
                                | "COM6"
                                | "COM7"
                                | "COM8"
                                | "COM9"
                                | "LPT1"
                                | "LPT2"
                                | "LPT3"
                                | "LPT4"
                                | "LPT5"
                                | "LPT6"
                                | "LPT7"
                                | "LPT8"
                                | "LPT9"
                        )
                    {
                        return Err(invalid("reserved output filename"));
                    }
                    let file = parent.join(name);
                    output = Some(file.clone());
                    file
                };
                #[cfg(windows)]
                let canonical = {
                    // FFmpeg builds do not all understand Win32 extended path prefixes.
                    let s = canonical.to_string_lossy();
                    if s.starts_with(r"\\?\UNC\") || !s.starts_with(r"\\?\") {
                        return Err(invalid("local drive file required"));
                    }
                    std::path::PathBuf::from(&s[4..])
                };
                canonical.to_string_lossy().into_owned()
            };
            substitutions.insert(p.id.clone(), value);
        }
        if input == output {
            return Err(invalid("input and output must differ"));
        }
        let output = output.ok_or_else(|| invalid("output file required"))?;
        let cwd = output.parent().unwrap();
        let args: Vec<String> = action
            .args
            .iter()
            .map(|arg| {
                arg.strip_prefix('{')
                    .and_then(|s| s.strip_suffix('}'))
                    .and_then(|id| substitutions.get(id))
                    .unwrap_or(arg)
                    .clone()
            })
            .collect();
        let execution = json!({"instance_id":instance_id,"args":args,"cwd":cwd,"timeout_ms":action.timeout_ms,"outputs":[output.file_name().unwrap().to_string_lossy()]});
        let disclosure = json!({"id":id,"expected_digest":digest,"action_id":action.id,"label":action.label,"executable":instance.path,"instance_id":instance_id,"args":args,"cwd":cwd,"output":output,"timeout_ms":action.timeout_ms,"trust":instance.trust,"supported":toolhub_executor::PINNED_EXECUTION_SUPPORTED});
        Ok((execution, disclosure))
    }
    pub(super) fn prepare_cli_job(
        &mut self,
        req: &JsonRpcRequest,
    ) -> Result<PreparedExecution, ProtocolError> {
        self.require_controller()?;
        let (mut execution, disclosure) = self.cli_request(&req.params)?;
        // The confirmation binds canonical paths, the exact argument array, instance and package digest.
        if req.params.get("confirmation") != Some(&disclosure) {
            return Err(invalid("CLI preview changed; review again"));
        }
        let principal = format!("extension:{}", text(&req.params, "id")?);
        let previous = self.connection_principal.replace(principal.clone());
        let was_controller = self.controller_verified;
        let result = (|| {
            // This controller-only command represents the user's explicit single-run approval.
            // Policy denial, unknown trust and executable pinning remain mandatory.
            let requested = self.request_approval(&execution)?;
            let approved = if requested["status"] == "approved" {
                requested
            } else {
                self.approve_pending(&json!({"request_id":requested["request_id"]}))?
            };
            execution["approval_id"] = approved["approval_id"].clone();
            execution["session_id"] = approved["session_id"].clone();
            self.controller_verified = false;
            self.prepare_job(&execution, &principal, false, req.id.clone())
        })();
        self.connection_principal = previous;
        self.controller_verified = was_controller;
        result
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn call(
        s: &mut DaemonService,
        method: &str,
        params: Value,
        controller: bool,
    ) -> JsonRpcResponse {
        s.handle_authenticated(
            &JsonRpcRequest::new(1, method, params),
            "fixture.desktop",
            controller,
        )
    }
    fn fixture() -> (tempfile::TempDir, DaemonService, Value) {
        let dir = tempfile::tempdir().unwrap();
        let mut s = DaemonService::open(&dir.path().join("registry.sqlite")).unwrap();
        let package: Value = serde_json::from_str(include_str!(
            "../../../extensions/examples/ffmpeg.toolhub-extension.json"
        ))
        .unwrap();
        let preview = call(
            &mut s,
            "extension.preview",
            json!({"package":package}),
            false,
        )
        .result
        .unwrap();
        assert!(preview["conflict"].is_null(), "{preview}");
        let result = call(
            &mut s,
            "extension.install",
            json!({"package":package,"digest":preview["digest"]}),
            true,
        );
        assert!(result.error.is_none(), "{:?}", result.error);
        // Owned test executable for validation/pinning only: never launch an untrusted system binary.
        let exe = dir.path().join(if cfg!(windows) {
            "ffmpeg.exe"
        } else {
            "ffmpeg"
        });
        std::fs::copy(std::env::current_exe().unwrap(), &exe).unwrap();
        s.registry
            .upsert_instance(&UpsertInstanceInput {
                id: "fixture.ffmpeg".into(),
                definition_id: "org.ffmpeg.ffmpeg".into(),
                definition_name: "FFmpeg".into(),
                version: None,
                platform: std::env::consts::OS.into(),
                arch: "x86_64".into(),
                path: exe.to_string_lossy().into_owned(),
                canonical_path: None,
                environment_id: None,
                origin_json: serde_json::to_string(&toolhub_core::Origin::Unknown).unwrap(),
                owner_json: serde_json::to_string(&toolhub_core::Owner::unknown()).unwrap(),
                trust_json: json!({"level":"user_trusted"}).to_string(),
                status: "available".into(),
                capabilities: vec!["media.video.transcode".into()],
            })
            .unwrap();
        let input = dir.path().join("中文 & ' [input].mp4");
        std::fs::write(&input, "owned fixture; not real media").unwrap();
        let params = json!({"id":"community.ffmpeg","expected_digest":preview["digest"],"action_id":"compress","instance_id":"fixture.ffmpeg","values":{"input":input,"output":dir.path().join("中文 & ' [out].mp4"),"quality":"23","speed":"medium"}});
        (dir, s, params)
    }
    #[test]
    fn canonical_preview_uses_arg_array_and_rejects_mutated_values_or_paths() {
        let (dir, mut s, p) = fixture();
        let preview = call(&mut s, "extension.cli_preview", p.clone(), true)
            .result
            .unwrap();
        let args = preview["args"].as_array().unwrap();
        assert!(args.iter().any(|v| v
            .as_str()
            .is_some_and(|v| v.ends_with("中文 & ' [input].mp4"))));
        assert!(args.iter().any(|v| v == "-n"));
        assert!(args.iter().any(|v| v == "file,pipe"));
        for (key, value) in [
            ("quality", "23 & arbitrary-command"),
            ("output", "../outside.mp4"),
            ("input", "https://example.invalid/input.mp4"),
        ] {
            let mut other = p.clone();
            other["values"][key] = json!(value);
            assert!(call(&mut s, "extension.cli_preview", other, true)
                .error
                .is_some());
        }
        let mut bad = p.clone();
        bad["values"]["unexpected"] = json!("x");
        assert!(call(&mut s, "extension.cli_preview", bad, true)
            .error
            .is_some());
        let output = dir.path().join("中文 & ' [out].mp4");
        std::fs::write(&output, "keep").unwrap();
        assert!(call(&mut s, "extension.cli_preview", p, true)
            .error
            .is_some());
        assert_eq!(std::fs::read_to_string(output).unwrap(), "keep");
    }
    #[test]
    fn cli_authority_cannot_be_inherited_by_agents_frames_or_other_tools() {
        let (_dir, mut s, p) = fixture();
        assert!(call(&mut s, "extension.cli_preview", p.clone(), false)
            .error
            .is_some());
        let mut confirmed = p.clone();
        confirmed["confirmation"] = call(&mut s, "extension.cli_preview", p.clone(), true)
            .result
            .unwrap();
        assert!(s
            .prepare_execution(
                &JsonRpcRequest::new(1, "extension.cli_start", confirmed.clone()),
                "fixture.agent",
                false
            )
            .is_err());
        for method in [
            "extension.cli_start",
            "extension.cli_preview",
            "execute.tool",
            "execute.approve",
            "execute.cancel",
        ] {
            assert!(call(&mut s,"extension.invoke",json!({"id":p["id"],"expected_digest":p["expected_digest"],"method":method,"params":confirmed}),true).error.is_some());
        }
        let mut bad = p.clone();
        bad["instance_id"] = json!("missing");
        assert!(call(&mut s, "extension.cli_preview", bad, true)
            .error
            .is_some());
        s.registry.db.conn.execute("INSERT INTO tool_definitions(id,name,vendor,description,created_at,updated_at) SELECT 'other.tool',name,vendor,description,created_at,updated_at FROM tool_definitions WHERE id='org.ffmpeg.ffmpeg'",[]).unwrap();
        s.registry
            .db
            .conn
            .execute(
                "UPDATE tool_instances SET definition_id='other.tool' WHERE id='fixture.ffmpeg'",
                [],
            )
            .unwrap();
        assert!(call(&mut s, "extension.cli_preview", p, true)
            .error
            .is_some());
        assert!(s.running_jobs.is_empty());
    }
    #[test]
    fn stale_disabled_and_changed_confirmation_fail_before_approval() {
        let (_dir, mut s, p) = fixture();
        let mut confirmed = p.clone();
        confirmed["confirmation"] = call(&mut s, "extension.cli_preview", p.clone(), true)
            .result
            .unwrap();
        confirmed["values"]["quality"] = json!("28");
        assert!(s
            .prepare_execution(
                &JsonRpcRequest::new(1, "extension.cli_start", confirmed),
                "fixture.desktop",
                true
            )
            .is_err());
        let count: i64 = s
            .registry
            .db
            .conn
            .query_row("SELECT count(*) FROM execution_requests", [], |r| r.get(0))
            .unwrap();
        assert_eq!(count, 0);
        let mut stale = p.clone();
        stale["expected_digest"] = json!("stale");
        assert!(call(&mut s, "extension.cli_preview", stale, true)
            .error
            .is_some());
        assert!(call(
            &mut s,
            "extension.set_enabled",
            json!({"id":p["id"],"expected_digest":p["expected_digest"],"enabled":false}),
            true
        )
        .error
        .is_none());
        assert!(call(&mut s, "extension.cli_preview", p, true)
            .error
            .is_some());
    }
    #[test]
    #[cfg(any(windows, target_os = "linux"))]
    fn explicit_native_run_still_obeys_trust_policy_and_is_extension_owned() {
        let (_dir, mut s, p) = fixture();
        let mut confirmed = p.clone();
        confirmed["confirmation"] = call(&mut s, "extension.cli_preview", p.clone(), true)
            .result
            .unwrap();
        s.registry.db.conn.execute("UPDATE tool_instances SET trust_json='{\"level\":\"unknown\"}' WHERE id='fixture.ffmpeg'",[]).unwrap();
        // Refresh disclosure so rejection is actually trust-based, not a changed preview.
        confirmed["confirmation"] = call(&mut s, "extension.cli_preview", p.clone(), true)
            .result
            .unwrap();
        let error = s
            .prepare_execution(
                &JsonRpcRequest::new(1, "extension.cli_start", confirmed.clone()),
                "fixture.desktop",
                true,
            )
            .err()
            .unwrap();
        assert!(error.error.unwrap().message.contains("unknown"));
        s.registry.db.conn.execute("UPDATE tool_instances SET trust_json='{\"level\":\"user_trusted\"}' WHERE id='fixture.ffmpeg'",[]).unwrap();
        confirmed["confirmation"] = call(&mut s, "extension.cli_preview", p, true)
            .result
            .unwrap();
        s.registry
            .set_policy("agent", "extension:community.ffmpeg", "deny")
            .unwrap();
        let error = s
            .prepare_execution(
                &JsonRpcRequest::new(1, "extension.cli_start", confirmed.clone()),
                "fixture.desktop",
                true,
            )
            .err()
            .unwrap();
        assert!(error.error.unwrap().message.contains("denied"));
        s.registry
            .set_policy("agent", "extension:community.ffmpeg", "ask")
            .unwrap();
        call(
            &mut s,
            "settings.set",
            json!({"auto_approve_execution":false}),
            true,
        );
        let job = s
            .prepare_execution(
                &JsonRpcRequest::new(1, "extension.cli_start", confirmed),
                "fixture.desktop",
                true,
            )
            .unwrap_or_else(|e| panic!("{:?}", e.error));
        assert_eq!(
            s.running_jobs.get(job.id()).unwrap().0,
            "extension:community.ffmpeg"
        );
        assert_eq!(s.connection_principal, None);
        assert!(!s.controller_verified);
        let count: i64 = s
            .registry
            .db
            .conn
            .query_row(
                "SELECT count(*) FROM execution_approvals WHERE consumed=1",
                [],
                |r| r.get(0),
            )
            .unwrap();
        assert_eq!(count, 1);
        // Do not run a media operation in a unit test; native acceptance uses selected real FFmpeg.
        drop(job);
    }
}
