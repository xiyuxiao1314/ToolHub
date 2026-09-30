//! Daemon service: one shared registry and domain pipeline for all clients.

use std::collections::BTreeMap;
use std::path::Path;
use std::sync::atomic::AtomicU64;

use serde_json::{json, Value};

use toolhub_agent_bridge::{discovery_task_prompt, DiscoverySession};
use toolhub_core::{AgentId, ExecutionRequest, InstanceId};
use toolhub_ipc::{error_response, PeerIdentity};
use toolhub_policy::{PolicyAction, PolicyContext, PolicyEngine};
use toolhub_protocol::{ErrorCode, JsonRpcRequest, JsonRpcResponse, Method, ProtocolError};
use toolhub_registry::{Registry, UpsertInstanceInput};
use toolhub_resolver::{CandidateInstance, ResolvePrefs};

pub struct DaemonService {
    pub registry: Registry,
    pub policy: PolicyEngine,
    pub sessions: Vec<DiscoverySession>,
    pub approvals: BTreeMap<String, toolhub_core::ExecutionApproval>,
    pub env_graph: toolhub_environment::EnvironmentGraph,
    pub last_scan: Option<chrono::DateTime<chrono::Utc>>,
    #[allow(dead_code)]
    pub seq: AtomicU64,
    pub registry_path: String,
}

impl DaemonService {
    pub fn open(path: &Path) -> anyhow::Result<Self> {
        let mut registry = Registry::open_path(path)?;
        registry.ensure_builtin_environments()?;
        // Seed capability taxonomy.
        for (id, desc) in toolhub_core::capability::CORE_CAPABILITIES {
            registry.ensure_capability(id, desc, false, None)?;
        }
        for (alias, canonical) in toolhub_core::capability::CORE_ALIASES {
            registry.ensure_capability(alias, "", true, Some(canonical))?;
        }
        let sessions = load_sessions(&mut registry).unwrap_or_default();
        let mut policy = PolicyEngine::with_defaults();
        // F03: load persisted rules on startup
        if let Ok(stored) = registry.list_policy() {
            for (scope, subject, action) in stored {
                let scope_enum = match scope.as_str() {
                    "command" => toolhub_policy::PolicyScope::Command,
                    "capability" => toolhub_policy::PolicyScope::Capability,
                    "agent" => toolhub_policy::PolicyScope::Agent,
                    "directory" => toolhub_policy::PolicyScope::Directory,
                    "environment" => toolhub_policy::PolicyScope::Environment,
                    _ => toolhub_policy::PolicyScope::Tool,
                };
                let act = match action.as_str() {
                    "allow" => PolicyAction::Allow,
                    "deny" => PolicyAction::Deny,
                    _ => PolicyAction::Ask,
                };
                policy.set(toolhub_policy::PolicyRule {
                    scope: scope_enum,
                    subject,
                    action: act,
                });
            }
        }
        let approvals = load_approvals(&mut registry).unwrap_or_default();
        Ok(Self {
            registry,
            policy,
            sessions,
            approvals,
            env_graph: toolhub_environment::EnvironmentGraph::new(),
            last_scan: None,
            seq: AtomicU64::new(1),
            registry_path: path.to_string_lossy().to_string(),
        })
    }

    /// F02: principal comes from transport-bound env, not caller-supplied labels.
    fn peer_principal(&self) -> String {
        std::env::var("TOOLHUB_PRINCIPAL")
            .ok()
            .filter(|s| !s.trim().is_empty())
            .unwrap_or_else(|| "local.stdio".to_string())
    }

    pub fn handle(&mut self, req: &JsonRpcRequest) -> JsonRpcResponse {
        let _peer = PeerIdentity::local_stdio();
        // F12: validate JSON-RPC envelope
        if req.jsonrpc != "2.0" {
            return error_response(
                req.id.clone(),
                &ProtocolError::new(ErrorCode::InvalidRequest, "jsonrpc must be \"2.0\""),
            );
        }
        if let Some(id) = &req.id {
            if !id.is_string() && !id.is_number() && !id.is_null() {
                return error_response(
                    req.id.clone(),
                    &ProtocolError::new(
                        ErrorCode::InvalidRequest,
                        "id must be string, number, or null",
                    ),
                );
            }
        }
        let id = req.id.clone();
        match Method::from_name(&req.method) {
            None => error_response(
                id,
                &ProtocolError::new(
                    ErrorCode::MethodNotFound,
                    format!("unknown method {}", req.method),
                ),
            ),
            Some(method) => match self.dispatch(method, &req.params) {
                Ok(value) => JsonRpcResponse {
                    jsonrpc: "2.0".into(),
                    id,
                    result: Some(value),
                    error: None,
                },
                Err(e) => error_response(id, &e),
            },
        }
    }

    fn dispatch(&mut self, method: Method, params: &Value) -> Result<Value, ProtocolError> {
        match method {
            Method::Ping => Ok(json!({"ok": true, "protocol": toolhub_protocol::PROTOCOL_VERSION})),
            Method::Status => {
                let tools = self.registry.count_instances().map_err(db_err)?;
                let cands = self.registry.count_candidates().map_err(db_err)?;
                Ok(json!({
                    "protocol_version": toolhub_protocol::PROTOCOL_VERSION,
                    "daemon": "toolhubd",
                    "registry_path": self.registry_path,
                    "tool_count": tools,
                    "candidate_count": cands,
                    "last_scan": self.last_scan,
                }))
            }
            Method::ScanStart => {
                let mode_str = params
                    .get("mode")
                    .and_then(|v| v.as_str())
                    .unwrap_or("quick")
                    .to_string();
                let mode = if mode_str == "full" {
                    toolhub_scanner::ScanMode::Full
                } else {
                    toolhub_scanner::ScanMode::Quick
                };
                let sid = self.registry.begin_scan(&mode_str).map_err(db_err)?;
                let report = toolhub_scanner::run_scan(mode, None);
                self.ingest_scan(&report);
                let cov = json!({
                    "roots_ok": report.coverage.roots_ok.len(),
                    "roots_failed": report.coverage.roots_failed.len(),
                    "candidates": report.candidates.len(),
                });
                let errs = json!(report.coverage.roots_failed);
                let _ = self.registry.finish_scan(
                    &sid,
                    if report.coverage.roots_failed.is_empty() {
                        "completed"
                    } else {
                        "partial"
                    },
                    &cov.to_string(),
                    &errs.to_string(),
                );
                Ok(json!({
                    "scan_session_id": sid,
                    "mode": mode_str,
                    "candidates": report.candidates.len(),
                    "roots_ok": report.coverage.roots_ok.len(),
                    "roots_failed": report.coverage.roots_failed,
                    "recognized": self.registry.count_instances().map_err(db_err)?,
                    "unknown_candidates": self.registry.count_candidates().map_err(db_err)?,
                    "evidence": self.registry.count_evidence().map_err(db_err)?,
                }))
            }
            Method::ScanStatus => Ok(json!({
                "last_scan": self.last_scan,
                "tools": self.registry.count_instances().map_err(db_err)?,
                "candidates": self.registry.count_candidates().map_err(db_err)?,
            })),
            Method::SearchTools => {
                let q = params.get("query").and_then(|v| v.as_str()).unwrap_or("");
                let hits = self.registry.search(q).map_err(db_err)?;
                Ok(json!(hits))
            }
            Method::InspectTool | Method::InspectInstance => {
                let id = params
                    .get("id")
                    .and_then(|v| v.as_str())
                    .ok_or_else(|| ProtocolError::new(ErrorCode::InvalidParams, "id required"))?;
                let inst = self.registry.get_instance(id).map_err(db_err)?;
                match inst {
                    Some(row) => {
                        let caps = self
                            .registry
                            .capabilities_for_definition(&row.definition_id)
                            .unwrap_or_default();
                        Ok(json!({
                            "instance": row,
                            "capabilities": caps,
                        }))
                    }
                    None => Err(ProtocolError::new(
                        ErrorCode::NotFound,
                        format!("instance {id} not found"),
                    )),
                }
            }
            Method::ListEnvironments => {
                let envs = self.registry.list_environments().map_err(db_err)?;
                Ok(json!(envs))
            }
            Method::ListDuplicates => {
                let dups = self.registry.duplicate_groups().map_err(db_err)?;
                Ok(json!(dups))
            }
            Method::ResolveCapability => {
                let cap = params
                    .get("capability")
                    .and_then(|v| v.as_str())
                    .ok_or_else(|| {
                        ProtocolError::new(ErrorCode::InvalidParams, "capability required")
                    })?;
                let reg = toolhub_core::CapabilityRegistry::with_core_taxonomy();
                let canonical = reg
                    .canonical_of(cap)
                    .ok_or_else(|| {
                        ProtocolError::new(ErrorCode::NotFound, format!("unknown capability {cap}"))
                    })?
                    .to_string();
                let rows = self
                    .registry
                    .instances_for_capability(&canonical)
                    .map_err(db_err)?;
                let cands: Vec<CandidateInstance> = rows
                    .into_iter()
                    .map(|r| CandidateInstance {
                        instance_id: r.id,
                        definition_id: r.definition_id,
                        name: r.name,
                        version: r.version,
                        path: r.path,
                        environment: r.environment_id,
                        trust: parse_trust(&r.trust),
                        arch: "x86_64".into(),
                        cwd_match: false,
                    })
                    .collect();
                let req = toolhub_core::CapabilityRequirement {
                    capability: toolhub_core::CapabilityId::new(canonical.clone()).unwrap(),
                    version: params
                        .get("version")
                        .and_then(|v| v.as_str())
                        .map(|s| s.to_string()),
                    optional: false,
                };
                let out = toolhub_resolver::resolve(&reg, &req, cands, &ResolvePrefs::default());
                Ok(json!(out))
            }
            Method::ExecuteTool => {
                let instance_id = params
                    .get("instance_id")
                    .and_then(|v| v.as_str())
                    .ok_or_else(|| {
                        ProtocolError::new(ErrorCode::InvalidParams, "instance_id required")
                    })?;
                let args: Vec<String> = params
                    .get("args")
                    .and_then(|v| v.as_array())
                    .map(|a| {
                        a.iter()
                            .filter_map(|x| x.as_str().map(|s| s.to_string()))
                            .collect()
                    })
                    .unwrap_or_default();
                let approval_id = params
                    .get("approval_id")
                    .and_then(|v| v.as_str())
                    .map(|s| s.to_string());

                let row = self
                    .registry
                    .get_instance(instance_id)
                    .map_err(db_err)?
                    .ok_or_else(|| ProtocolError::new(ErrorCode::NotFound, "instance not found"))?;

                // F03: blocked/missing availability prevents launch regardless of trust.
                if row.status == "blocked" || row.status == "missing" {
                    return Err(ProtocolError::denied(format!(
                        "instance status {} does not permit execution",
                        row.status
                    )));
                }
                let trust = parse_trust(&row.trust);
                if trust == "blocked" || trust == "unknown" {
                    return Err(ProtocolError::denied(format!(
                        "trust level {trust} does not permit execution without elevated approval"
                    )));
                }

                let principal = self.peer_principal();
                let _agent_for_policy = params
                    .get("agent_id")
                    .and_then(|v| v.as_str())
                    .map(|s| s.to_string())
                    .unwrap_or_else(|| principal.clone());
                // F02: caller labels cannot choose security principal for approval binding.
                let agent_id = AgentId::new(principal.clone())
                    .or_else(|_| AgentId::new("local.stdio"))
                    .map_err(|e| ProtocolError::new(ErrorCode::InvalidParams, e.to_string()))?;

                let req = ExecutionRequest {
                    instance_id: InstanceId::new(instance_id)
                        .map_err(|e| ProtocolError::new(ErrorCode::InvalidParams, e.to_string()))?,
                    executable: row.path.clone(),
                    args: args.clone(),
                    cwd: params
                        .get("cwd")
                        .and_then(|v| v.as_str())
                        .map(|s| s.to_string()),
                    env_overrides: BTreeMap::new(),
                    timeout_ms: params
                        .get("timeout_ms")
                        .and_then(|v| v.as_u64())
                        .unwrap_or(30_000),
                    max_output_bytes: toolhub_protocol::limits::MAX_OUTPUT_BYTES,
                    stdin: params
                        .get("stdin")
                        .and_then(|v| v.as_str())
                        .map(|s| s.to_string()),
                    agent_id: Some(agent_id.clone()),
                };

                let env_s = toolhub_executor::sanitize_env(&req.env_overrides, &[]);
                let env_dig = toolhub_executor::env_digest(&env_s);
                let tool_name = Path::new(&row.path)
                    .file_stem()
                    .map(|s| s.to_string_lossy().to_string())
                    .unwrap_or_default();
                let ctx = PolicyContext {
                    tool: Some(tool_name),
                    agent: Some(principal.clone()),
                    directory: req.cwd.clone(),
                    environment: row.environment_id.clone(),
                    ..Default::default()
                };
                let decision = self.policy.decide(&ctx);

                if decision.action == PolicyAction::Deny {
                    return Err(ProtocolError::denied(decision.explanation));
                }

                // F01: authoritative approval validation before launch.
                let mut approval_validated = false;
                if decision.action == PolicyAction::Ask {
                    let aid = approval_id.as_deref().ok_or_else(|| {
                        ProtocolError::new(ErrorCode::Denied, "approval_required")
                    })?;
                    let approval = self.approvals.get(aid).ok_or_else(|| {
                        ProtocolError::new(ErrorCode::ApprovalInvalid, "unknown approval")
                    })?;
                    let hash = toolhub_executor::hash_file(&row.path).map_err(|e| {
                        ProtocolError::new(ErrorCode::InternalError, format!("hash failed: {e}"))
                    })?;
                    let canon = row
                        .canonical_path
                        .clone()
                        .unwrap_or_else(|| row.path.clone());
                    toolhub_executor::validate_approval(
                        approval,
                        &req,
                        &hash,
                        &canon,
                        &req.args,
                        req.cwd.as_deref(),
                        req.stdin.as_deref(),
                        &env_dig,
                        chrono::Utc::now(),
                    )
                    .map_err(|st| match st {
                        toolhub_core::ExecutionStatus::Expired => ProtocolError::new(
                            ErrorCode::Expired,
                            "approval expired, consumed, or revoked",
                        ),
                        _ => ProtocolError::new(
                            ErrorCode::ApprovalInvalid,
                            "approval does not bind this request",
                        ),
                    })?;
                    // atomic consume
                    if !self.registry.consume_approval(aid).map_err(db_err)? {
                        return Err(ProtocolError::new(
                            ErrorCode::ApprovalInvalid,
                            "approval already consumed",
                        ));
                    }
                    if let Some(a) = self.approvals.get_mut(aid) {
                        a.consume();
                    }
                    approval_validated = true;
                }

                let result =
                    toolhub_executor::execute(&req, &self.policy, &[], &ctx, approval_validated);

                let audit = toolhub_audit::AuditRecord::from_execution(
                    Some(&principal),
                    Some(instance_id),
                    None,
                    &req.executable,
                    &req.args,
                    req.cwd.as_deref(),
                    result.duration_ms,
                    result.exit_code,
                    result.status.as_str(),
                    approval_id.as_deref(),
                );
                let _ = self.registry.insert_execution_record(
                    &audit.id,
                    audit.agent_id.as_deref(),
                    audit.instance_id.as_deref(),
                    audit.capability.as_deref(),
                    &audit.executable,
                    &audit
                        .args_redacted
                        .iter()
                        .map(|s| s.as_str())
                        .collect::<Vec<_>>()
                        .join(" "),
                    audit.cwd_redacted.as_deref().unwrap_or(""),
                    audit.duration_ms,
                    audit.exit_code,
                    &audit.status,
                    audit.approval_id.as_deref(),
                );
                // Response includes stdout for the caller; audit does not persist it.
                Ok(json!({
                    "status": result.status.as_str(),
                    "exit_code": result.exit_code,
                    "stdout": result.stdout,
                    "stderr": result.stderr,
                    "duration_ms": result.duration_ms,
                    "truncated": result.truncated,
                    "error_code": result.error_code,
                    "fallback_allowed": result.fallback_allowed,
                }))
            }
            Method::ApproveExecution => {
                // R2-B01: only a trusted approver channel may mint execution approvals.
                let principal = self.peer_principal();
                let admin = std::env::var("TOOLHUB_ADMIN").ok().as_deref() == Some("1")
                    || principal.starts_with("local.admin");
                if !admin {
                    return Err(ProtocolError::denied(
                        "execute.approve requires trusted approver (TOOLHUB_ADMIN=1 or local.admin)",
                    ));
                }
                let instance_id = params
                    .get("instance_id")
                    .and_then(|v| v.as_str())
                    .ok_or_else(|| {
                        ProtocolError::new(ErrorCode::InvalidParams, "instance_id required")
                    })?;
                let args: Vec<String> = params
                    .get("args")
                    .and_then(|v| v.as_array())
                    .map(|a| {
                        a.iter()
                            .filter_map(|x| x.as_str().map(|s| s.to_string()))
                            .collect()
                    })
                    .unwrap_or_default();
                let row = self
                    .registry
                    .get_instance(instance_id)
                    .map_err(db_err)?
                    .ok_or_else(|| ProtocolError::new(ErrorCode::NotFound, "instance not found"))?;
                let hash = toolhub_executor::hash_file(&row.path).unwrap_or_default();
                let env = toolhub_executor::sanitize_env(&BTreeMap::new(), &[]);
                let _agent_id = params
                    .get("agent_id")
                    .and_then(|v| v.as_str())
                    .unwrap_or("local.stdio");
                let approval = toolhub_executor::build_approval(
                    &uuid::Uuid::new_v4().to_string(),
                    AgentId::new(self.peer_principal())
                        .unwrap_or_else(|_| AgentId::new("local.stdio").unwrap()),
                    "cli-session",
                    &ExecutionRequest {
                        instance_id: InstanceId::new(instance_id).unwrap(),
                        executable: row.path.clone(),
                        args: args.clone(),
                        cwd: params
                            .get("cwd")
                            .and_then(|v| v.as_str())
                            .map(|s| s.to_string()),
                        env_overrides: BTreeMap::new(),
                        timeout_ms: 30_000,
                        max_output_bytes: 1024 * 256,
                        stdin: params
                            .get("stdin")
                            .and_then(|v| v.as_str())
                            .map(|s| s.to_string()),
                        agent_id: Some(
                            AgentId::new(self.peer_principal())
                                .unwrap_or_else(|_| AgentId::new("local.stdio").unwrap()),
                        ),
                    },
                    &hash,
                    row.canonical_path.as_deref().unwrap_or(&row.path),
                    &toolhub_executor::env_digest(&env),
                    300,
                );
                let aid = approval.approval_id.clone();
                let principal = self.peer_principal();
                let _ = self.registry.save_approval(
                    &aid,
                    &principal,
                    "cli-session",
                    instance_id,
                    &hash,
                    row.canonical_path.as_deref().unwrap_or(&row.path),
                    &format!("{}|{}", approval.args_digest, approval.stdin_digest),
                    &approval.cwd_digest,
                    &approval.env_digest,
                    &approval.expires_at.to_rfc3339(),
                );
                self.approvals.insert(aid.clone(), approval);
                Ok(json!({"approval_id": aid, "expires_in_seconds": 300}))
            }
            Method::RevokeApproval => {
                let aid = params
                    .get("approval_id")
                    .and_then(|v| v.as_str())
                    .ok_or_else(|| {
                        ProtocolError::new(ErrorCode::InvalidParams, "approval_id required")
                    })?;
                if let Some(a) = self.approvals.get_mut(aid) {
                    a.revoke();
                }
                let _ = self.registry.revoke_approval(aid);
                Ok(json!({"revoked": true}))
            }
            Method::SkillList => {
                // F15: optional declarative registration via params
                if let Some(manifest) = params.get("register") {
                    let id = manifest.get("id").and_then(|v| v.as_str()).ok_or_else(|| {
                        ProtocolError::new(ErrorCode::InvalidParams, "id required")
                    })?;
                    let name = manifest.get("name").and_then(|v| v.as_str()).unwrap_or(id);
                    let schema = manifest
                        .get("schema")
                        .and_then(|v| v.as_str())
                        .unwrap_or("toolhub.skill/v1");
                    if schema != "toolhub.skill/v1" {
                        return Err(ProtocolError::new(
                            ErrorCode::InvalidParams,
                            "unsupported skill schema",
                        ));
                    }
                    // reject hook/installer payloads
                    if manifest.get("hooks").is_some() || manifest.get("install").is_some() {
                        return Err(ProtocolError::new(
                            ErrorCode::Denied,
                            "skill hooks/installers are not permitted",
                        ));
                    }
                    let kind = manifest
                        .get("kind")
                        .and_then(|v| v.as_str())
                        .unwrap_or("instruction");
                    let json_s = serde_json::to_string(manifest).unwrap_or_else(|_| "{}".into());
                    self.registry
                        .upsert_skill(id, name, schema, kind, &json_s, None)
                        .map_err(db_err)?;
                }
                let rows = self.registry.list_skills().map_err(db_err)?;
                let skills: Vec<Value> = rows
                    .into_iter()
                    .map(|(id, name, schema, manifest)| {
                        let m: Value = serde_json::from_str(&manifest).unwrap_or(json!({}));
                        json!({"id": id, "name": name, "schema": schema, "manifest": m})
                    })
                    .collect();
                Ok(json!(skills))
            }
            Method::SkillInspect | Method::SkillResolve => {
                let id = params.get("id").and_then(|v| v.as_str()).unwrap_or("");
                let rows = self.registry.list_skills().map_err(db_err)?;
                if let Some((_, name, schema, manifest)) =
                    rows.into_iter().find(|(sid, _, _, _)| sid == id)
                {
                    let m: Value = serde_json::from_str(&manifest).unwrap_or(json!({}));
                    // F15: resolve requirements against registry capability providers
                    let mut requires = vec![];
                    if let Some(reqs) = m.get("requires").and_then(|v| v.as_array()) {
                        for r in reqs {
                            let cap = r.get("capability").and_then(|v| v.as_str()).unwrap_or("");
                            let canon = toolhub_core::CapabilityRegistry::with_core_taxonomy()
                                .canonical_of(cap)
                                .unwrap_or(cap)
                                .to_string();
                            let providers = self
                                .registry
                                .instances_for_capability(&canon)
                                .map_err(db_err)?;
                            requires.push(json!({
                                "capability": cap,
                                "canonical": canon,
                                "satisfied": !providers.is_empty(),
                                "providers": providers.len(),
                            }));
                        }
                    }
                    let status = if requires.iter().all(|r| r["satisfied"] == json!(true)) {
                        "available"
                    } else {
                        "missing_capabilities"
                    };
                    Ok(json!({
                        "id": id,
                        "name": name,
                        "schema": schema,
                        "status": status,
                        "requires": requires,
                        "manifest": m
                    }))
                } else {
                    Ok(json!({"id": id, "status": "malformed", "note": "skill not registered"}))
                }
            }
            Method::DiscoveryStart => {
                let agent_id = params
                    .get("agent_id")
                    .and_then(|v| v.as_str())
                    .unwrap_or("local.agent");
                let scopes: Vec<String> = params
                    .get("scopes")
                    .and_then(|v| v.as_array())
                    .map(|a| {
                        a.iter()
                            .filter_map(|x| x.as_str().map(|s| s.to_string()))
                            .collect()
                    })
                    .unwrap_or_else(|| {
                        toolhub_agent_bridge::DISCOVERY_SCOPES
                            .iter()
                            .map(|s| s.to_string())
                            .collect()
                    });
                let session = DiscoverySession::issue(agent_id, 30, &scopes)
                    .map_err(|e| ProtocolError::new(ErrorCode::InvalidParams, e))?;
                let _ = self.registry.save_discovery_session(
                    &session.id,
                    agent_id,
                    &session.expires_at.to_rfc3339(),
                    &serde_json::to_string(&session.scopes).unwrap_or_else(|_| "[]".into()),
                );
                let prompt = discovery_task_prompt(agent_id, &session.id);
                self.sessions.push(session.clone());
                Ok(json!({
                    "session_id": session.id,
                    "agent_id": session.agent_id,
                    "expires_at": session.expires_at,
                    "scopes": session.scopes,
                    "launch_instructions": prompt,
                }))
            }
            Method::DiscoveryList => {
                let now = chrono::Utc::now();
                let list: Vec<_> = self
                    .sessions
                    .iter()
                    .map(|s| {
                        json!({
                            "session_id": s.id,
                            "agent_id": s.agent_id,
                            "expires_at": s.expires_at,
                            "scopes": s.scopes,
                            "revoked": s.revoked,
                            "usable": s.is_usable(now),
                        })
                    })
                    .collect();
                Ok(json!(list))
            }
            Method::DiscoveryInspect => {
                let id = params
                    .get("id")
                    .and_then(|v| v.as_str())
                    .ok_or_else(|| ProtocolError::new(ErrorCode::InvalidParams, "id required"))?;
                let now = chrono::Utc::now();
                let s =
                    self.sessions.iter().find(|s| s.id == id).ok_or_else(|| {
                        ProtocolError::new(ErrorCode::NotFound, "session not found")
                    })?;
                if !s.is_usable(now) {
                    return Err(ProtocolError::new(
                        ErrorCode::Expired,
                        "discovery session expired or revoked",
                    ));
                }
                if !s.allows("candidate.inspect") && !s.allows("metadata.read") {
                    return Err(ProtocolError::denied("scope not permitted"));
                }
                // Return candidate list only (metadata), never execute.
                let cands = self.registry.list_candidates().map_err(db_err)?;
                Ok(json!({"session_id": id, "candidates": cands}))
            }
            Method::DiscoveryClassify => {
                let id = params
                    .get("session_id")
                    .and_then(|v| v.as_str())
                    .unwrap_or("");
                let now = chrono::Utc::now();
                let s = self.sessions.iter().find(|s| s.id == id).ok_or_else(|| {
                    ProtocolError::new(ErrorCode::SessionInvalid, "session not found")
                })?;
                if !s.is_usable(now) {
                    return Err(ProtocolError::new(ErrorCode::Expired, "session expired"));
                }
                if !s.allows("classification.submit") {
                    return Err(ProtocolError::denied("classification.submit not allowed"));
                }
                // F14: require candidate + non-empty label; durable untrusted enrichment
                let cand = params
                    .get("candidate_id")
                    .and_then(|v| v.as_str())
                    .filter(|s| !s.is_empty())
                    .ok_or_else(|| {
                        ProtocolError::new(ErrorCode::InvalidParams, "candidate_id required")
                    })?;
                let label = params
                    .get("label")
                    .and_then(|v| v.as_str())
                    .filter(|s| !s.trim().is_empty())
                    .ok_or_else(|| {
                        ProtocolError::new(ErrorCode::InvalidParams, "label required")
                    })?;
                let conf = params
                    .get("confidence")
                    .and_then(|v| v.as_f64())
                    .unwrap_or(0.5)
                    .clamp(0.0, 1.0);
                self.registry
                    .store_classification(id, cand, label, conf)
                    .map_err(db_err)?;
                Ok(json!({
                    "accepted": true,
                    "stored": true,
                    "provenance": "ai",
                    "evidence_count": self.registry.count_evidence().map_err(db_err)?,
                    "note": "classification stored as untrusted enrichment only; no execution trust"
                }))
            }
            Method::DiscoveryRevoke => {
                let id = params
                    .get("session_id")
                    .and_then(|v| v.as_str())
                    .unwrap_or("");
                for s in &mut self.sessions {
                    if s.id == id {
                        s.revoke();
                    }
                }
                let _ = self.registry.revoke_discovery_session(id);
                Ok(json!({"revoked": true}))
            }
            Method::AgentList => {
                // F16: detect + health + typed MCP config
                let agents = toolhub_agent_bridge::detect_all();
                let endpoint = "toolhub";
                let view: Vec<Value> = agents
                    .into_iter()
                    .map(|a| {
                        let cfg = serde_json::json!({
                            "mcpServers": {
                                "toolhub": {
                                    "command": "toolhub",
                                    "args": ["mcp", "serve"],
                                    "env": {"TOOLHUB_ENDPOINT": endpoint}
                                }
                            }
                        });
                        json!({
                            "id": a.id,
                            "name": a.name,
                            "kind": a.kind,
                            "executable": a.executable,
                            "version": a.version,
                            "health": "detected",
                            "mcp_config": cfg,
                            "launch": toolhub_agent_bridge::discovery_task_prompt(&a.id, "<session>")
                        })
                    })
                    .collect();
                Ok(json!(view))
            }
            Method::PolicyGet => {
                let rules = self.registry.list_policy().map_err(db_err)?;
                Ok(json!({
                    "defaults": self.policy.rules,
                    "stored": rules,
                }))
            }
            Method::PolicySet => {
                let scope = params
                    .get("scope")
                    .and_then(|v| v.as_str())
                    .unwrap_or("tool");
                let subject = params
                    .get("subject")
                    .and_then(|v| v.as_str())
                    .ok_or_else(|| {
                        ProtocolError::new(ErrorCode::InvalidParams, "subject required")
                    })?;
                let action = params
                    .get("action")
                    .and_then(|v| v.as_str())
                    .unwrap_or("ask");
                let act = match action {
                    "allow" => PolicyAction::Allow,
                    "deny" => PolicyAction::Deny,
                    _ => PolicyAction::Ask,
                };
                let scope_enum = match scope {
                    "command" => toolhub_policy::PolicyScope::Command,
                    "capability" => toolhub_policy::PolicyScope::Capability,
                    "agent" => toolhub_policy::PolicyScope::Agent,
                    "directory" => toolhub_policy::PolicyScope::Directory,
                    "environment" => toolhub_policy::PolicyScope::Environment,
                    _ => toolhub_policy::PolicyScope::Tool,
                };
                // F01/F02: granting Allow is privileged; Ask/Deny are safer defaults.
                if act == PolicyAction::Allow {
                    let principal = self.peer_principal();
                    let admin = std::env::var("TOOLHUB_ADMIN").ok().as_deref() == Some("1")
                        || principal.starts_with("local.admin");
                    if !admin {
                        return Err(ProtocolError::denied(
                            "policy.set allow requires admin principal (TOOLHUB_ADMIN=1 or local.admin)",
                        ));
                    }
                }
                self.registry
                    .set_policy(scope, subject, action)
                    .map_err(db_err)?;
                self.policy.set(toolhub_policy::PolicyRule {
                    scope: scope_enum,
                    subject: subject.to_string(),
                    action: act,
                });
                Ok(json!({"ok": true}))
            }
            Method::ActivityList => {
                let rows = self.registry.list_activity(100).map_err(db_err)?;
                let items: Vec<Value> = rows
                    .into_iter()
                    .map(|(ts, kind, summary)| json!({"ts": ts, "kind": kind, "summary": summary}))
                    .collect();
                Ok(json!(items))
            }
            Method::ExportReport => {
                let instances = self.registry.list_instances().map_err(db_err)?;
                let tools: Vec<_> = instances
                    .into_iter()
                    .map(|r| {
                        json!({
                            "name": r.name,
                            "version": r.version,
                            "trust": parse_trust(&r.trust),
                            "path": toolhub_audit::redact_path_for_export(&r.path),
                        })
                    })
                    .collect();
                Ok(json!({
                    "generated_at": chrono::Utc::now(),
                    "tools": tools,
                    "redaction": ["home_path", "username", "secrets", "sensitive_args"],
                }))
            }
        }
    }

    fn ingest_scan(&mut self, report: &toolhub_scanner::ScanReport) {
        let mut seen = Vec::new();
        for cand in &report.candidates {
            let rec = toolhub_recognizer::recognize(cand);
            if rec.recognized {
                if let (Some(def), Some(inst)) = (rec.definition.as_ref(), rec.instance.as_ref()) {
                    let env_id = self.env_graph.ensure_detected(&inst.path);
                    self.env_graph.place(inst.id.clone(), env_id.clone());
                    let owner = toolhub_environment::attribute_owner(&inst.path);
                    let input = UpsertInstanceInput {
                        id: inst.id.as_str().to_string(),
                        definition_id: def.id.as_str().to_string(),
                        definition_name: def.name.clone(),
                        version: inst.version.clone(),
                        platform: inst.platform.clone(),
                        arch: inst.arch.clone(),
                        path: inst.path.clone(),
                        canonical_path: inst.canonical_path.clone(),
                        environment_id: Some(env_id.as_str().to_string()),
                        origin_json: serde_json::to_string(&inst.origin)
                            .unwrap_or_else(|_| "{}".into()),
                        owner_json: serde_json::to_string(&owner).unwrap_or_else(|_| "{}".into()),
                        trust_json: serde_json::to_string(&inst.trust)
                            .unwrap_or_else(|_| "{}".into()),
                        status: "available".into(),
                        capabilities: def
                            .capabilities
                            .iter()
                            .map(|c| c.as_str().to_string())
                            .collect(),
                    };
                    if self.registry.upsert_instance(&input).is_ok() {
                        seen.push(inst.id.as_str().to_string());
                    }
                    let _ = self.registry.db.conn.execute(
                        "INSERT INTO environments(id, name, kind, root_path, parent_id, origin_json, owner_json, labels_json)
                         VALUES (?1, ?2, ?3, NULL, NULL, '{}', '{}', '[]')
                         ON CONFLICT(id) DO NOTHING",
                        rusqlite::params![env_id.as_str(), env_id.as_str(), "detected"],
                    );
                    let _ = self.registry.upsert_interface(
                        &format!("if-{}", inst.id.as_str()),
                        inst.id.as_str(),
                        "cli",
                        Some(inst.path.as_str()),
                    );
                }
            } else {
                let _ = self
                    .registry
                    .upsert_candidate(&toolhub_registry::CandidateRow {
                        id: cand.id.clone(),
                        path: cand.path.clone(),
                        recognized: false,
                        file_name: cand.file_name.clone(),
                    });
            }
        }
        if report.coverage.roots_failed.is_empty() {
            let _ = self.registry.mark_missing_except(&seen);
        }
        self.last_scan = Some(chrono::Utc::now());
        let _ = self
            .registry
            .record_activity("scan", "native scan completed", None);
    }
}

fn load_approvals(
    reg: &mut Registry,
) -> Result<BTreeMap<String, toolhub_core::ExecutionApproval>, String> {
    let mut stmt = reg
        .db
        .conn
        .prepare(
            "SELECT id, agent_id, session_id, instance_id, executable_sha256, canonical_executable,
                    args_digest, cwd_digest, env_digest, expires_at, consumed, revoked
             FROM execution_approvals",
        )
        .map_err(|e| e.to_string())?;
    let rows = stmt
        .query_map([], |r| {
            Ok((
                r.get::<_, String>(0)?,
                r.get::<_, String>(1)?,
                r.get::<_, String>(2)?,
                r.get::<_, String>(3)?,
                r.get::<_, String>(4)?,
                r.get::<_, String>(5)?,
                r.get::<_, String>(6)?,
                r.get::<_, String>(7)?,
                r.get::<_, String>(8)?,
                r.get::<_, String>(9)?,
                r.get::<_, i32>(10)?,
                r.get::<_, i32>(11)?,
            ))
        })
        .map_err(|e| e.to_string())?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e| e.to_string())?;
    let mut out = BTreeMap::new();
    for (
        id,
        agent_id,
        session_id,
        instance_id,
        sha,
        canon,
        args_d,
        cwd_d,
        env_d,
        expires,
        consumed,
        revoked,
    ) in rows
    {
        let expires_at = chrono::DateTime::parse_from_rfc3339(&expires)
            .map(|d| d.with_timezone(&chrono::Utc))
            .unwrap_or_else(|_| chrono::Utc::now());
        let (args_digest, stdin_digest) = match args_d.split_once('|') {
            Some((a, s)) => (a.to_string(), s.to_string()),
            None => (args_d, String::new()),
        };
        out.insert(
            id.clone(),
            toolhub_core::ExecutionApproval {
                approval_id: id,
                agent_id: AgentId::new(&agent_id)
                    .unwrap_or_else(|_| AgentId::new("local.stdio").unwrap()),
                session_id,
                instance_id: InstanceId::new(&instance_id)
                    .unwrap_or_else(|_| InstanceId::new("unknown").unwrap()),
                executable_sha256: sha,
                canonical_executable: canon,
                args_digest,
                cwd_digest: cwd_d,
                stdin_digest,
                env_digest: env_d,
                expires_at,
                consumed: consumed != 0,
                revoked: revoked != 0,
            },
        );
    }
    Ok(out)
}

fn load_sessions(reg: &mut Registry) -> Result<Vec<DiscoverySession>, String> {
    let mut stmt = reg
        .db
        .conn
        .prepare(
            "SELECT id, agent_id, created_at, expires_at, revoked, scopes_json
             FROM discovery_sessions",
        )
        .map_err(|e| e.to_string())?;
    let rows = stmt
        .query_map([], |r| {
            Ok((
                r.get::<_, String>(0)?,
                r.get::<_, String>(1)?,
                r.get::<_, String>(2)?,
                r.get::<_, String>(3)?,
                r.get::<_, i32>(4)?,
                r.get::<_, String>(5)?,
            ))
        })
        .map_err(|e| e.to_string())?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e| e.to_string())?;
    let mut out = vec![];
    for (id, agent_id, created, expires, revoked, scopes_json) in rows {
        let created_at = chrono::DateTime::parse_from_rfc3339(&created)
            .map(|d| d.with_timezone(&chrono::Utc))
            .unwrap_or_else(|_| chrono::Utc::now());
        let expires_at = chrono::DateTime::parse_from_rfc3339(&expires)
            .map(|d| d.with_timezone(&chrono::Utc))
            .unwrap_or_else(|_| chrono::Utc::now());
        let scopes: std::collections::BTreeSet<String> =
            serde_json::from_str(&scopes_json).unwrap_or_default();
        out.push(DiscoverySession {
            id,
            agent_id,
            created_at,
            expires_at,
            scopes,
            revoked: revoked != 0,
        });
    }
    Ok(out)
}

fn parse_trust(trust_json: &str) -> String {
    serde_json::from_str::<Value>(trust_json)
        .ok()
        .and_then(|v| {
            v.get("level")
                .and_then(|l| l.as_str())
                .map(|s| s.to_string())
        })
        .unwrap_or_else(|| "unknown".into())
}

fn db_err(e: toolhub_registry::RegistryError) -> ProtocolError {
    ProtocolError::new(ErrorCode::InternalError, e.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn service_open_and_status() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("r.sqlite");
        let mut svc = DaemonService::open(&path).unwrap();
        let resp = svc.handle(&JsonRpcRequest::new(1, "ping", json!({})));
        assert!(resp.error.is_none());
        let resp = svc.handle(&JsonRpcRequest::new(2, "status", json!({})));
        assert!(resp.error.is_none());
        let body = resp.result.unwrap();
        assert_eq!(body["daemon"], "toolhubd");
    }

    #[test]
    fn unknown_method_is_structured() {
        let dir = tempfile::tempdir().unwrap();
        let mut svc = DaemonService::open(&dir.path().join("r.sqlite")).unwrap();
        let resp = svc.handle(&JsonRpcRequest::new(1, "nope", json!({})));
        assert!(resp.error.is_some());
    }

    #[test]
    fn failed_policy_write_returns_error_and_keeps_effective_rule() {
        let dir = tempfile::tempdir().unwrap();
        let mut svc = DaemonService::open(&dir.path().join("policy.sqlite")).unwrap();
        let request = |id, action| {
            JsonRpcRequest::new(
                id,
                "policy.set",
                json!({
                    "scope": "tool", "subject": "fixture", "action": action,
                }),
            )
        };
        assert!(svc.handle(&request(1, "deny")).error.is_none());
        svc.registry
            .db
            .conn
            .execute_batch(
                "CREATE TRIGGER reject_policy BEFORE INSERT ON policy_rules
             BEGIN SELECT RAISE(ABORT, 'fixture write failure'); END;",
            )
            .unwrap();
        assert!(svc.handle(&request(2, "ask")).error.is_some());
        let context = PolicyContext {
            tool: Some("fixture".into()),
            ..Default::default()
        };
        assert_eq!(svc.policy.decide(&context).action, PolicyAction::Deny);
        drop(svc);
        let svc = DaemonService::open(&dir.path().join("policy.sqlite")).unwrap();
        assert_eq!(svc.policy.decide(&context).action, PolicyAction::Deny);
    }
}
