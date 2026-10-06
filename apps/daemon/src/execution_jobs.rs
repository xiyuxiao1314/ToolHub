use super::*;
use std::sync::atomic::Ordering;
use toolhub_core::ExecutionResult;

pub struct PreparedExecution {
    pin: toolhub_executor::ExecutablePin,
    request: ExecutionRequest,
    context: PolicyContext,
    policy: PolicyEngine,
    cancelled: Arc<AtomicBool>,
    operation_id: String,
    principal: String,
    approval_id: Option<String>,
    rpc_id: Option<Value>,
    outputs: Vec<String>,
}
pub struct PendingApproval {
    pub params: Value,
    pub hash: String,
    pub authorization_digest: String,
    pub env_digest: String,
    pub expires_at: chrono::DateTime<chrono::Utc>,
}
impl PreparedExecution {
    pub fn id(&self) -> &str {
        &self.operation_id
    }
    pub fn run(&self) -> ExecutionResult {
        toolhub_executor::execute_with_pin(
            &self.request,
            &self.policy,
            &[],
            &self.context,
            true,
            &self.pin,
            Arc::clone(&self.cancelled),
        )
    }
}
impl DaemonService {
    #[allow(clippy::result_large_err)]
    pub fn prepare_execution(
        &mut self,
        req: &JsonRpcRequest,
        principal: &str,
        controller: bool,
    ) -> Result<PreparedExecution, JsonRpcResponse> {
        let previous = self.connection_principal.replace(principal.into());
        let old = std::mem::replace(&mut self.controller_verified, controller);
        let result = toolhub_protocol::validate_method_params(req)
            .and_then(|()| self.refresh_authority())
            .and_then(|()| self.prepare_job(&req.params, principal, controller, req.id.clone()));
        let result = match result {
            Ok(job) => Ok(job),
            Err(error) => {
                let error = self
                    .record_denial(&req.params, &error)
                    .err()
                    .unwrap_or(error);
                Err(error_response(req.id.clone(), &error))
            }
        };
        self.connection_principal = previous;
        self.controller_verified = old;
        result
    }
    pub fn finish_execution(
        &mut self,
        job: PreparedExecution,
        result: ExecutionResult,
    ) -> JsonRpcResponse {
        let id = job.rpc_id.clone();
        match self.finish_job(job, result) {
            Ok(value) => JsonRpcResponse {
                jsonrpc: "2.0".into(),
                id,
                result: Some(value),
                error: None,
            },
            Err(error) => error_response(id, &error),
        }
    }
    pub(super) fn refresh_authority(&mut self) -> Result<(), ProtocolError> {
        let mut policy = PolicyEngine::with_defaults();
        for (scope, subject, action) in self.registry.list_policy().map_err(db_err)? {
            let scope = policy_scope(&scope)?;
            let action = policy_action(&action)?;
            policy.set(toolhub_policy::PolicyRule {
                scope,
                subject,
                action,
            });
        }
        self.policy = policy;
        self.approvals = load_approvals(&mut self.registry)
            .map_err(|e| ProtocolError::new(ErrorCode::InternalError, e))?;
        self.sessions = load_sessions(&mut self.registry)
            .map_err(|e| ProtocolError::new(ErrorCode::InternalError, e))?;
        self.pending_approvals
            .retain(|_, pending| pending.expires_at > chrono::Utc::now());
        Ok(())
    }
    fn canonical_request(
        &mut self,
        params: &Value,
        principal: &str,
    ) -> Result<(ExecutionRequest, PolicyContext, String, String, String), ProtocolError> {
        if !params.is_object() {
            return Err(invalid("named object params required"));
        }
        let id = text(params, "instance_id")?;
        let row = self
            .registry
            .get_instance(id)
            .map_err(db_err)?
            .ok_or_else(|| ProtocolError::new(ErrorCode::NotFound, "instance not found"))?;
        if row.status != "available" {
            return Err(ProtocolError::denied("instance unavailable"));
        }
        let trust = parse_trust(&row.trust);
        if !matches!(trust.as_str(), "known" | "user_trusted" | "verified") {
            return Err(ProtocolError::denied(
                "unknown or blocked executable cannot run",
            ));
        }
        let executable = std::fs::canonicalize(&row.path)
            .map_err(|_| ProtocolError::new(ErrorCode::Unavailable, "executable unavailable"))?
            .to_string_lossy()
            .into_owned();
        let cwd = match params.get("cwd") {
            Some(Value::String(path)) => std::fs::canonicalize(path),
            None => std::env::current_dir().and_then(std::fs::canonicalize),
            _ => return Err(invalid("cwd must be string")),
        }
        .map_err(|_| invalid("working directory unavailable"))?;
        if !cwd.is_dir() {
            return Err(invalid("cwd must be directory"));
        }
        let args = match params.get("args") {
            None => vec![],
            Some(Value::Array(args)) => args
                .iter()
                .map(|v| {
                    v.as_str()
                        .map(str::to_owned)
                        .ok_or_else(|| invalid("every arg must be string"))
                })
                .collect::<Result<Vec<_>, _>>()?,
            _ => return Err(invalid("args must be array")),
        };
        let timeout = params
            .get("timeout_ms")
            .map(|v| {
                v.as_u64()
                    .ok_or_else(|| invalid("timeout_ms must be positive integer"))
            })
            .transpose()?
            .unwrap_or(30_000);
        if timeout == 0 || timeout > 300_000 {
            return Err(invalid("timeout_ms outside 1..300000"));
        }
        let stdin = params
            .get("stdin")
            .map(|v| {
                v.as_str()
                    .map(str::to_owned)
                    .ok_or_else(|| invalid("stdin must be string"))
            })
            .transpose()?;
        let overrides = params
            .get("env")
            .map(|v| {
                serde_json::from_value::<BTreeMap<String, String>>(v.clone())
                    .map_err(|_| invalid("env must contain string values"))
            })
            .transpose()?
            .unwrap_or_default();
        let request = ExecutionRequest {
            instance_id: InstanceId::new(id).map_err(|e| invalid(&e.to_string()))?,
            executable: executable.clone(),
            args,
            cwd: Some(cwd.to_string_lossy().into_owned()),
            env_overrides: overrides,
            timeout_ms: timeout,
            max_output_bytes: toolhub_protocol::limits::MAX_OUTPUT_BYTES,
            stdin,
            agent_id: Some(
                AgentId::new(principal).map_err(|_| invalid("invalid authenticated identity"))?,
            ),
        };
        let tool = Path::new(&row.path)
            .file_stem()
            .map(|v| v.to_string_lossy().to_lowercase())
            .unwrap_or_default();
        let capability = params
            .get("capability")
            .map(|v| {
                v.as_str()
                    .map(str::to_owned)
                    .ok_or_else(|| invalid("capability must be string"))
            })
            .transpose()?;
        if let Some(cap) = &capability {
            if !self
                .registry
                .capabilities_for_definition(&row.definition_id)
                .map_err(db_err)?
                .contains(cap)
            {
                return Err(ProtocolError::denied(
                    "instance does not provide requested capability",
                ));
            }
        }
        let context = PolicyContext {
            command: Some(if request.args == ["--version"] && trust == "known" {
                "version-probe".into()
            } else {
                tool.clone()
            }),
            tool: Some(tool),
            capability,
            agent: Some(principal.into()),
            directory: request.cwd.clone(),
            environment: row.environment_id,
            trust: Some(trust),
        };
        let revision = self.registry.revision().map_err(db_err)?;
        let digest = toolhub_core::execution::digest_strings(&[
            "authorization-revision/v2".into(),
            revision.to_string(),
            serde_json::to_string(&self.policy.rules).map_err(json_err)?,
            serde_json::to_string(&context).map_err(json_err)?,
            row.trust,
            row.status,
            id.into(),
            request.timeout_ms.to_string(),
            request.max_output_bytes.to_string(),
        ]);
        if !toolhub_executor::PINNED_EXECUTION_SUPPORTED {
            return Err(ProtocolError::new(
                ErrorCode::Unavailable,
                "macOS preview does not support identity-pinned tool execution or its approvals; use discovery only",
            ));
        }
        let hash = toolhub_executor::hash_file(&executable)
            .map_err(|_| ProtocolError::new(ErrorCode::Unavailable, "cannot hash executable"))?;
        let env_digest = toolhub_executor::env_digest(&toolhub_executor::sanitize_env(
            &request.env_overrides,
            &[],
        ));
        Ok((request, context, digest, hash, env_digest))
    }
    pub(super) fn prepare_job(
        &mut self,
        params: &Value,
        principal: &str,
        _controller: bool,
        rpc_id: Option<Value>,
    ) -> Result<PreparedExecution, ProtocolError> {
        let (request, context, digest, hash, env_digest) =
            self.canonical_request(params, principal)?;
        let pin = toolhub_executor::ExecutablePin::open(Path::new(&request.executable))
            .map_err(|_| ProtocolError::new(ErrorCode::Unavailable, "cannot pin executable"))?;
        if pin.hash().map_err(|_| {
            ProtocolError::new(ErrorCode::Unavailable, "cannot hash pinned executable")
        })? != hash
        {
            return Err(ProtocolError::new(
                ErrorCode::ApprovalInvalid,
                "executable changed while preparing execution",
            ));
        }
        let decision = self.policy.decide(&context);
        if decision.action == PolicyAction::Deny {
            return Err(ProtocolError::denied("denied by effective policy"));
        }
        let operation_id = params
            .get("execution_id")
            .map(|v| {
                v.as_str()
                    .map(str::to_owned)
                    .ok_or_else(|| invalid("execution_id must be string"))
            })
            .transpose()?
            .unwrap_or_else(|| uuid::Uuid::new_v4().to_string());
        if operation_id.is_empty()
            || operation_id.len() > 128
            || self.running_jobs.contains_key(&operation_id)
        {
            return Err(invalid("invalid or running execution_id"));
        }
        if self.running_jobs.len() >= 32 {
            return Err(invalid("at most 32 active executions"));
        }
        let used: bool = self
            .registry
            .db
            .conn
            .query_row(
                "SELECT EXISTS(SELECT 1 FROM managed_tasks WHERE id=?1)",
                [&operation_id],
                |r| r.get(0),
            )
            .map_err(sql_err)?;
        if used {
            return Err(invalid("execution_id already used"));
        }
        let outputs = super::managed_tasks::output_declarations(params)?;
        let approval_id = params
            .get("approval_id")
            .map(|v| {
                v.as_str()
                    .map(str::to_owned)
                    .ok_or_else(|| invalid("approval_id must be string"))
            })
            .transpose()?;
        if decision.action == PolicyAction::Ask {
            let id = approval_id.as_deref().ok_or_else(|| {
                ProtocolError::denied("approval_required: submit execute.approval_request")
            })?;
            let approval = self.approvals.get(id).ok_or_else(|| {
                ProtocolError::new(ErrorCode::ApprovalInvalid, "unknown approval")
            })?;
            if approval.session_id != text(params, "session_id")? {
                return Err(ProtocolError::new(
                    ErrorCode::ApprovalInvalid,
                    "execution session mismatch",
                ));
            }
            toolhub_executor::validate_approval(
                approval,
                &request,
                &hash,
                &request.executable,
                &request.args,
                request.cwd.as_deref(),
                request.stdin.as_deref(),
                &env_digest,
                &digest,
                chrono::Utc::now(),
            )
            .map_err(|_| {
                ProtocolError::new(
                    ErrorCode::ApprovalInvalid,
                    "approval does not bind current request or revision",
                )
            })?;
            if !self.registry.consume_approval(id).map_err(db_err)? {
                return Err(ProtocolError::new(
                    ErrorCode::ApprovalInvalid,
                    "approval consumed or revoked",
                ));
            }
        }
        let cancelled = Arc::new(AtomicBool::new(false));
        self.running_jobs.insert(
            operation_id.clone(),
            (principal.into(), Arc::clone(&cancelled)),
        );
        if let Err(error) = self.event(
            "execution.started",
            "authorized execution started",
            json!({"execution_id":operation_id}),
        ) {
            self.running_jobs.remove(&operation_id);
            return Err(error);
        }
        let view = json!({"execution_id":operation_id,"instance_id":request.instance_id,"name":Path::new(&request.executable).file_name().map(|s|s.to_string_lossy()),"status":"running","started_at":chrono::Utc::now(),"cwd":request.cwd,"timeout_ms":request.timeout_ms,"artifacts":[]});
        if let Err(e) = self.persist_task(&operation_id, principal, &view) {
            self.running_jobs.remove(&operation_id);
            return Err(e);
        }
        Ok(PreparedExecution {
            outputs,
            pin,
            request,
            context,
            policy: self.policy.clone(),
            cancelled,
            operation_id,
            principal: principal.into(),
            approval_id,
            rpc_id,
        })
    }
    pub(super) fn finish_job(
        &mut self,
        job: PreparedExecution,
        result: ExecutionResult,
    ) -> Result<Value, ProtocolError> {
        self.running_jobs.remove(&job.operation_id);
        let artifacts = super::managed_tasks::existing_artifacts(
            job.request.cwd.as_deref().unwrap_or(""),
            &job.outputs,
        );
        let view = json!({"execution_id":job.operation_id,"instance_id":job.request.instance_id,"name":Path::new(&job.request.executable).file_name().map(|s|s.to_string_lossy()),"status":result.status.as_str(),"finished_at":chrono::Utc::now(),"executable_sha256":job.pin.hash().ok(),"exit_code":result.exit_code,"duration_ms":result.duration_ms,"artifacts":artifacts,"output_retained_in_memory":true});
        self.persist_task(&job.operation_id, &job.principal, &view)?;
        if self.task_results.len() >= 64 {
            if let Some(id) = self.task_results.keys().next().cloned() {
                self.task_results.remove(&id);
            }
        }
        self.task_results.insert(job.operation_id.clone(),json!({"stdout":result.stdout.chars().take(32768).collect::<String>(),"stderr":result.stderr.chars().take(32768).collect::<String>(),"truncated":result.truncated||result.stdout.chars().count()>32768||result.stderr.chars().count()>32768}));
        let audit = toolhub_audit::AuditRecord::from_execution(
            Some(&job.principal),
            Some(job.request.instance_id.as_str()),
            job.context.capability.as_deref(),
            &job.request.executable,
            &job.request.args,
            job.request.cwd.as_deref(),
            result.duration_ms,
            result.exit_code,
            result.status.as_str(),
            job.approval_id.as_deref(),
        );
        self.persist_audit(&audit)?;
        self.event(
            "execution",
            &format!("execution {}", result.status.as_str()),
            json!({"execution_id":job.operation_id,"status":result.status.as_str()}),
        )?;
        Ok(
            json!({"execution_id":job.operation_id,"artifacts":artifacts,"status":result.status.as_str(),"exit_code":result.exit_code,"stdout":result.stdout,"stderr":result.stderr,"duration_ms":result.duration_ms,"truncated":result.truncated,"error_code":result.error_code,"fallback_allowed":result.fallback_allowed}),
        )
    }
    pub(super) fn request_approval(&mut self, params: &Value) -> Result<Value, ProtocolError> {
        let owner = self.peer_principal();
        let (request, context, digest, hash, env_digest) =
            self.canonical_request(params, &owner)?;
        if self.policy.decide(&context).action == PolicyAction::Deny {
            return Err(ProtocolError::denied("denied requests cannot be approved"));
        }
        let id = uuid::Uuid::new_v4().to_string();
        let session = uuid::Uuid::new_v4().to_string();
        let mut snapshot = params.clone();
        snapshot["cwd"] = json!(request.cwd);
        snapshot["_executable"] = json!(request.executable);
        snapshot["session_id"] = json!(session);
        let expires_at = chrono::Utc::now() + chrono::Duration::minutes(5);
        let metadata = json!({"instance_id":request.instance_id,"cwd":request.cwd.as_ref().map(|p|toolhub_audit::redact_path_for_export(p)),"args_count":request.args.len(),"request_digest":toolhub_core::execution::digest_strings(&request.args),"executable_sha256":hash});
        self.registry.db.conn.execute("INSERT INTO execution_requests(id,owner,session_id,params_json,expires_at,status) VALUES(?1,?2,?3,?4,?5,'pending')",rusqlite::params![id,owner,session,serde_json::to_string(&metadata).map_err(json_err)?,expires_at.to_rfc3339()]).map_err(sql_err)?;
        self.pending_approvals.insert(
            id.clone(),
            PendingApproval {
                params: snapshot,
                hash,
                authorization_digest: digest,
                env_digest,
                expires_at,
            },
        );
        self.event(
            "approval.request",
            "execution approval requested",
            json!({"request_id":id}),
        )?;
        Ok(json!({"request_id":id,"session_id":session,"expires_in_seconds":300}))
    }
    pub(super) fn approve_pending(&mut self, params: &Value) -> Result<Value, ProtocolError> {
        if !self.is_admin() {
            return Err(ProtocolError::denied(
                "verified desktop controller required",
            ));
        }
        let id = text(params, "request_id")?;
        let row=self.registry.db.conn.query_row("SELECT owner,session_id,params_json,expires_at,status FROM execution_requests WHERE id=?1",[id],|r|Ok((r.get::<_,String>(0)?,r.get::<_,String>(1)?,r.get::<_,String>(2)?,r.get::<_,String>(3)?,r.get::<_,String>(4)?))).optional().map_err(sql_err)?.ok_or_else(||ProtocolError::new(ErrorCode::NotFound,"approval request not found"))?;
        if row.4 != "pending"
            || chrono::DateTime::parse_from_rfc3339(&row.3)
                .map_err(|_| invalid("invalid expiry"))?
                <= chrono::Utc::now()
        {
            return Err(ProtocolError::new(
                ErrorCode::Expired,
                "request expired or already decided",
            ));
        }
        let pending = self.pending_approvals.get(id).ok_or_else(|| {
            ProtocolError::new(
                ErrorCode::Expired,
                "request body expired or daemon restarted; resubmit",
            )
        })?;
        let snapshot = pending.params.clone();
        let original_hash = pending.hash.clone();
        let original_digest = pending.authorization_digest.clone();
        let original_env = pending.env_digest.clone();
        let (request, context, digest, hash, env_digest) =
            self.canonical_request(&snapshot, &row.0)?;
        if original_hash != hash || original_digest != digest || original_env != env_digest {
            return Err(ProtocolError::new(
                ErrorCode::ApprovalInvalid,
                "request executable, environment or authorization changed since disclosure",
            ));
        }
        if self.policy.decide(&context).action == PolicyAction::Deny {
            return Err(ProtocolError::denied("current policy denies request"));
        }
        let approval = toolhub_executor::build_approval(
            &uuid::Uuid::new_v4().to_string(),
            request
                .agent_id
                .clone()
                .ok_or_else(|| invalid("missing caller"))?,
            &row.1,
            &request,
            &hash,
            &request.executable,
            &env_digest,
            &digest,
            300,
        );
        self.registry
            .db
            .conn
            .execute_batch("SAVEPOINT issue_approval")
            .map_err(sql_err)?;
        let result = (|| {
            self.registry
                .save_approval(
                    &approval.approval_id,
                    &row.0,
                    &row.1,
                    request.instance_id.as_str(),
                    &hash,
                    &request.executable,
                    &format!("{}|{}", approval.args_digest, approval.stdin_digest),
                    &approval.cwd_digest,
                    &format!("{}|{}", approval.env_digest, approval.policy_trust_digest),
                    &approval.expires_at.to_rfc3339(),
                )
                .map_err(db_err)?;
            self.registry.db.conn.execute("UPDATE execution_requests SET status='approved' WHERE id=?1 AND status='pending'",[id]).map_err(sql_err)?;
            Ok::<(), ProtocolError>(())
        })();
        if result.is_err() {
            self.registry
                .db
                .conn
                .execute_batch("ROLLBACK TO issue_approval; RELEASE issue_approval")
                .map_err(sql_err)?;
            result?;
        }
        self.registry
            .db
            .conn
            .execute_batch("RELEASE issue_approval")
            .map_err(sql_err)?;
        self.pending_approvals.remove(id);
        self.event(
            "approval.issued",
            "bound approval issued",
            json!({"request_id":id}),
        )?;
        Ok(
            json!({"approval_id":approval.approval_id,"request_id":id,"session_id":row.1,"expires_in_seconds":300}),
        )
    }
    pub(super) fn cancel_execution(&mut self, params: &Value) -> Result<Value, ProtocolError> {
        let id = text(params, "execution_id")?;
        if let Some((owner, token)) = self.running_jobs.get(id) {
            if owner != &self.peer_principal() && !self.is_admin() {
                return Err(ProtocolError::denied("operation owned by another caller"));
            }
            token.store(true, Ordering::SeqCst);
            self.event(
                "execution.cancel_requested",
                "cancellation requested",
                json!({"execution_id":id}),
            )?;
            Ok(json!({"execution_id":id,"cancel_requested":true,"cancelled":false}))
        } else {
            Err(ProtocolError::new(
                ErrorCode::NotFound,
                "execution is not running",
            ))
        }
    }
    pub(super) fn revoke_approval(&mut self, params: &Value) -> Result<Value, ProtocolError> {
        let id = text(params, "approval_id")?;
        let approval = self
            .approvals
            .get(id)
            .ok_or_else(|| ProtocolError::new(ErrorCode::NotFound, "approval not found"))?;
        if approval.agent_id.as_str() != self.peer_principal() && !self.is_admin() {
            return Err(ProtocolError::denied(
                "approval owner or controller required",
            ));
        }
        self.registry.revoke_approval(id).map_err(db_err)?;
        self.event(
            "approval.revoked",
            "execution approval revoked",
            json!({"approval_id":id}),
        )?;
        Ok(json!({"revoked":true}))
    }
    pub(super) fn record_denial(
        &mut self,
        params: &Value,
        error: &ProtocolError,
    ) -> Result<(), ProtocolError> {
        let owner = self.peer_principal();
        let audit = toolhub_audit::AuditRecord::from_execution(
            Some(&owner),
            None,
            None,
            "<unlaunched>",
            &[],
            None,
            0,
            None,
            "denied",
            None,
        );
        self.persist_audit(&audit)?;
        self.event("execution.denied","execution rejected before launch",json!({"instance_id":params.get("instance_id").and_then(Value::as_str),"error":error.code}))?;
        Ok(())
    }
    fn persist_audit(&mut self, a: &toolhub_audit::AuditRecord) -> Result<(), ProtocolError> {
        self.registry
            .insert_execution_record(
                &a.id,
                a.agent_id.as_deref(),
                a.instance_id.as_deref(),
                a.capability.as_deref(),
                &a.executable,
                &serde_json::to_string(&a.args_redacted).map_err(json_err)?,
                a.cwd_redacted.as_deref().unwrap_or(""),
                a.duration_ms,
                a.exit_code,
                &a.status,
                a.approval_id.as_deref(),
            )
            .map_err(db_err)
    }
}
pub(super) fn text<'a>(v: &'a Value, key: &str) -> Result<&'a str, ProtocolError> {
    v.get(key)
        .and_then(Value::as_str)
        .filter(|s| !s.is_empty())
        .ok_or_else(|| invalid(&format!("{key} must be nonempty string")))
}
pub(super) fn invalid(message: &str) -> ProtocolError {
    ProtocolError::new(ErrorCode::InvalidParams, message)
}
pub(super) fn policy_scope(value: &str) -> Result<toolhub_policy::PolicyScope, ProtocolError> {
    use toolhub_policy::PolicyScope::*;
    match value {
        "command" => Ok(Command),
        "tool" => Ok(Tool),
        "capability" => Ok(Capability),
        "agent" => Ok(Agent),
        "directory" => Ok(Directory),
        "environment" => Ok(Environment),
        _ => Err(invalid("invalid policy scope")),
    }
}
pub(super) fn policy_action(value: &str) -> Result<PolicyAction, ProtocolError> {
    match value {
        "allow" => Ok(PolicyAction::Allow),
        "ask" => Ok(PolicyAction::Ask),
        "deny" => Ok(PolicyAction::Deny),
        _ => Err(invalid("invalid policy action")),
    }
}
