//! Daemon service: one shared registry and domain pipeline for all clients.

use std::collections::BTreeMap;
use std::path::Path;
use std::sync::atomic::{AtomicBool, AtomicU64};
use std::sync::Arc;
use rusqlite::OptionalExtension;
#[path = "workflows.rs"] mod workflows;
#[path = "execution_jobs.rs"] mod execution_jobs;
pub use execution_jobs::PreparedExecution;
#[path="scan_jobs.rs"] mod scan_jobs;
pub use scan_jobs::PreparedScan;
use workflows::{json_err,sql_err};
#[path = "connected_flows.rs"] mod connected_flows;

use serde_json::{json, Value};

use toolhub_agent_bridge::{discovery_task_prompt, DiscoverySession};
use toolhub_core::{AgentId, ExecutionRequest, InstanceId};
use toolhub_ipc::error_response;
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
    /// Per-connection principal override (named pipe / unix socket peers).
    pub connection_principal: Option<String>,
    /// R3-F04: running operation ids -> owner principal.
    pub controller_verified: bool,
    pub running_scans: BTreeMap<String, (String, Arc<AtomicBool>)>,
    pub running_jobs: BTreeMap<String, (String, Arc<AtomicBool>)>,
    pub subscriptions: BTreeMap<String, (String, u64)>,
    pub pending_approvals: BTreeMap<String, execution_jobs::PendingApproval>,
    pub resources: toolhub_recognizer::resources::ResourceStore,
    pub agents: toolhub_agent_bridge::adapters::AgentRuntime,
    pub credentials: toolhub_agent_bridge::temp_credentials::TempProviderStore,
    pub verified_updates: BTreeMap<String,toolhub_agent_bridge::update::UpdatePackage>,
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
        let sessions = load_sessions(&mut registry).map_err(anyhow::Error::msg)?;
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
        let approvals = load_approvals(&mut registry).map_err(anyhow::Error::msg)?;
        registry.db.conn.execute_batch("CREATE TABLE IF NOT EXISTS execution_requests(id TEXT PRIMARY KEY,owner TEXT NOT NULL,session_id TEXT NOT NULL,params_json TEXT NOT NULL,expires_at TEXT NOT NULL,status TEXT NOT NULL); CREATE TABLE IF NOT EXISTS discovery_disclosures(session_id TEXT NOT NULL,candidate_id TEXT NOT NULL,PRIMARY KEY(session_id,candidate_id)); CREATE TABLE IF NOT EXISTS event_log(seq INTEGER PRIMARY KEY AUTOINCREMENT,ts TEXT NOT NULL,kind TEXT NOT NULL,summary TEXT NOT NULL,data_json TEXT NOT NULL);")?;
        let environments=registry.load_environment_graph()?;
        let mut env_graph=toolhub_environment::EnvironmentGraph::new();
        for environment in environments {env_graph.insert(environment);}
        let resource_dir=path.parent().unwrap_or(Path::new(".")).join("toolhub-resources");
        let resources=toolhub_recognizer::resources::ResourceStore::open(&resource_dir)?;
        Ok(Self {
            registry,
            policy,
            sessions,
            approvals,
            env_graph,
            last_scan: None,
            connection_principal: None,
            controller_verified: false,
            running_jobs: BTreeMap::new(),
            running_scans: BTreeMap::new(),
            subscriptions: BTreeMap::new(),
            pending_approvals: BTreeMap::new(),
            resources,
            agents: toolhub_agent_bridge::adapters::AgentRuntime::new(),
            credentials: toolhub_agent_bridge::temp_credentials::TempProviderStore::new(),
            verified_updates: BTreeMap::new(),
            seq: AtomicU64::new(1),
            registry_path: path.to_string_lossy().to_string(),
        })
    }

    pub fn peer_principal(&self) -> String {
        self.connection_principal.clone().unwrap_or_else(|| "local.stdio".into())
    }
    pub fn is_admin(&self) -> bool {self.controller_verified}
    pub fn handle_authenticated(&mut self, req: &JsonRpcRequest, principal: &str, controller: bool) -> JsonRpcResponse {
        let previous=self.connection_principal.replace(principal.into());
        let previous_controller=std::mem::replace(&mut self.controller_verified,controller);
        let result=self.handle(req);
        self.connection_principal=previous;
        self.controller_verified=previous_controller;
        result
    }
    pub fn handle_with_principal(&mut self, req:&JsonRpcRequest, principal:Option<String>)->JsonRpcResponse {
        self.handle_authenticated(req,principal.as_deref().unwrap_or("local.stdio"),false)
    }

    pub fn handle(&mut self, req: &JsonRpcRequest) -> JsonRpcResponse {
        if !req.params.is_object() {return error_response(req.id.clone(),&ProtocolError::new(ErrorCode::InvalidParams,"named object params required"));}
        if let Err(error)=self.refresh_authority(){return error_response(req.id.clone(),&error);}
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
                    Some(Value::Null),
                    &ProtocolError::new(
                        ErrorCode::InvalidRequest,
                        "id must be string, number, or null",
                    ),
                );
            }
        }
        if let Err(error)=toolhub_protocol::validate_method_params(req){return error_response(req.id.clone(),&error);}
        if req.method=="protocol.negotiate" || workflows::is_extended(&req.method) {
            return match self.dispatch_extended(&req.method,&req.params) {Ok(result)=>JsonRpcResponse{jsonrpc:"2.0".into(),id:req.id.clone(),result:Some(result),error:None},Err(error)=>error_response(req.id.clone(),&error)};
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
                Err(e) => {
                    if req.method.starts_with("execute.") {
                        if let Err(audit_error)=self.record_denial(&req.params,&e) { return error_response(id,&audit_error); }
                    }
                    error_response(id, &e)
                },
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
            Method::ScanStart => self.synchronous_scan(params),
            Method::ScanStatus => Ok(json!({
                "running": self.running_scans.keys().collect::<Vec<_>>(),
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
                            .map_err(db_err)?;
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
                let envs = self.registry.load_environment_graph().map_err(db_err)?;
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
                let reg = self.resources.capabilities();
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
                        arch: r.arch,
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
                // R2-B06: wire preferences from request
                let prefs=self.resolver_preferences(params)?;
                let out = toolhub_resolver::resolve(&reg, &req, cands, &prefs);
                Ok(json!(out))
            }
            Method::ExecuteTool => {
                let job=self.prepare_job(params,&self.peer_principal(),self.is_admin(),None)?;
                let result=job.run();
                self.finish_job(job,result)
            }
            Method::ExecuteCancel => self.cancel_execution(params),
            Method::ApproveExecution => self.approve_pending(params),
            Method::RevokeApproval => self.revoke_approval(params),
            Method::SkillList => {
                // F15: optional declarative registration via params
                if let Some(manifest) = params.get("register") {
                    // R3-F09: one authoritative model at the public boundary.
                    let parsed = toolhub_core::SkillManifest::parse_yaml_like(manifest)
                        .map_err(|e| ProtocolError::new(ErrorCode::InvalidParams, e.to_string()))?;
                    parsed
                        .assert_safe_paths()
                        .map_err(|e| ProtocolError::new(ErrorCode::InvalidParams, e.to_string()))?;
                    let id = parsed.id.as_str().to_string();
                    let name = parsed.name.clone();
                    let schema = parsed.schema.clone();
                    if schema != "toolhub.skill/v1" {
                        return Err(ProtocolError::new(
                            ErrorCode::InvalidParams,
                            "unsupported skill schema",
                        ));
                    }
                    // R2-B09: reject hook/installer payloads and unsafe paths
                    if manifest.get("hooks").is_some()
                        || manifest.get("install").is_some()
                        || manifest.get("postinstall").is_some()
                    {
                        return Err(ProtocolError::new(
                            ErrorCode::Denied,
                            "skill hooks/installers are not permitted",
                        ));
                    }
                    for key in ["instruction_file", "mcp_config", "package_path"] {
                        if let Some(p) = manifest.get(key).and_then(|v| v.as_str()) {
                            if p.contains("..") || p.starts_with('/') || p.contains(':') {
                                return Err(ProtocolError::new(
                                    ErrorCode::InvalidParams,
                                    format!("unsafe skill path: {p}"),
                                ));
                            }
                        }
                    }
                    let kind = manifest
                        .get("kind")
                        .and_then(|v| v.as_str())
                        .unwrap_or("instruction");
                    let json_s = serde_json::to_string(manifest).unwrap_or_else(|_| "{}".into());
                    self.registry
                        .upsert_skill(&id, &name, &schema, kind, &json_s, None)
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
            Method::SkillInspect | Method::SkillResolve => self.resolve_skill(params),
            Method::DiscoveryStart => {
                // R2-B08: session is bound to the authenticated peer principal.
                let principal = self.peer_principal();
                let agent_id = principal.as_str();
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
                self.registry.save_discovery_session(
                    &session.id,
                    agent_id,
                    &session.expires_at.to_rfc3339(),
                    &serde_json::to_string(&session.scopes).map_err(json_err)?,
                ).map_err(db_err)?;
                let available=self.registry.list_candidates().map_err(db_err)?;
                let ids=if let Some(requested)=params.get("candidate_ids") {let ids=requested.as_array().ok_or_else(||ProtocolError::new(ErrorCode::InvalidParams,"candidate_ids must be array"))?.iter().map(|v|v.as_str().map(str::to_owned).ok_or_else(||ProtocolError::new(ErrorCode::InvalidParams,"candidate id must be string"))).collect::<Result<Vec<_>,_>>()?;if ids.iter().any(|id|!available.iter().any(|c|&c.id==id)){return Err(ProtocolError::new(ErrorCode::NotFound,"candidate not found"));}ids}else{available.iter().map(|c|c.id.clone()).collect()};
                self.registry.disclose_candidates(&session.id,&ids).map_err(db_err)?;
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
                let principal = self.peer_principal();
                let list: Vec<_> = self
                    .sessions
                    .iter()
                    .filter(|s| s.agent_id == principal)
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
                let principal = self.peer_principal();
                let s =
                    self.sessions.iter().find(|s| s.id == id).ok_or_else(|| {
                        ProtocolError::new(ErrorCode::NotFound, "session not found")
                    })?;
                if s.agent_id != principal {
                    return Err(ProtocolError::denied(
                        "session belongs to another principal",
                    ));
                }
                if !s.is_usable(now) {
                    return Err(ProtocolError::new(
                        ErrorCode::Expired,
                        "discovery session expired or revoked",
                    ));
                }
                if !s.allows("candidate.inspect") && !s.allows("metadata.read") && !s.allows("candidate.list") {
                    return Err(ProtocolError::denied("scope not permitted"));
                }
                // Return candidate list only (metadata), never execute.
                let mut cands=Vec::new();
                for candidate in self.registry.list_candidates().map_err(db_err)? {if self.registry.db.conn.query_row("SELECT 1 FROM discovery_disclosures WHERE session_id=?1 AND candidate_id=?2",rusqlite::params![id,candidate.id],|row|row.get::<_,u8>(0)).optional().map_err(sql_err)?.is_some(){cands.push(candidate);}}
                Ok(json!({"session_id": id, "candidates": cands}))
            }
            Method::DiscoveryClassify => {
                let id = params
                    .get("session_id")
                    .and_then(|v| v.as_str())
                    .unwrap_or("");
                let now = chrono::Utc::now();
                let principal = self.peer_principal();
                let s = self.sessions.iter().find(|s| s.id == id).ok_or_else(|| {
                    ProtocolError::new(ErrorCode::SessionInvalid, "session not found")
                })?;
                if s.agent_id != principal {
                    return Err(ProtocolError::denied(
                        "session belongs to another principal",
                    ));
                }
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
                // R3-F08: strict finite confidence in [0,1]; no silent clamp/default.
                let conf = params
                    .get("confidence")
                    .and_then(|v| v.as_f64())
                    .ok_or_else(|| {
                        ProtocolError::new(ErrorCode::InvalidParams, "confidence required")
                    })?;
                if !(0.0..=1.0).contains(&conf) || !conf.is_finite() {
                    return Err(ProtocolError::new(
                        ErrorCode::InvalidParams,
                        "confidence must be finite in [0,1]",
                    ));
                }
                // R3-F08: candidate must exist
                let cands = self.registry.list_candidates().map_err(db_err)?;
                if !cands.iter().any(|c| c.id == cand) {
                    return Err(ProtocolError::new(
                        ErrorCode::NotFound,
                        "candidate not found",
                    ));
                }
                let disclosed=self.registry.db.conn.query_row("SELECT 1 FROM discovery_disclosures WHERE session_id=?1 AND candidate_id=?2",rusqlite::params![id,cand],|row|row.get::<_,u8>(0)).optional().map_err(sql_err)?;
                if disclosed.is_none(){return Err(ProtocolError::denied("candidate not disclosed to this session"));}
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
                let principal = self.peer_principal();
                let mut owned = false;
                for s in &mut self.sessions {
                    if s.id == id {
                        if s.agent_id != principal {
                            return Err(ProtocolError::denied(
                                "session belongs to another principal",
                            ));
                        }
                        s.revoke();
                        owned = true;
                    }
                }
                if !owned {
                    return Err(ProtocolError::new(ErrorCode::NotFound, "session not found"));
                }
                self.registry.revoke_discovery_session(id).map_err(db_err)?;
                self.credentials.revoke_discovery(&principal,id);
                self.agents.cancel_discovery(&principal,id);
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
            Method::PolicySet => self.set_policy(params),
            Method::ActivityList => {
                let rows = self.registry.list_activity(100).map_err(db_err)?;
                let items: Vec<Value> = rows
                    .into_iter()
                    .map(|(ts, kind, summary)| json!({"ts": ts, "kind": kind, "summary": summary}))
                    .collect();
                Ok(json!(items))
            }
            Method::ExportReport => {
                if let Some(report)=params.get("import") {self.import_report(report)} else {self.export_report()}
            }
            _ => Err(ProtocolError::new(ErrorCode::MethodNotFound,"method not implemented")),
        }
    }

    fn ingest_scan(&mut self, report:&toolhub_scanner::ScanReport,_mode:&str)->Result<(),ProtocolError> {
        let mut instances=Vec::new(); let mut candidates=Vec::new();
        for cand in &report.candidates {
            let rec=toolhub_recognizer::recognize_with_resources(cand,&self.resources);
            if let (Some(def),Some(inst))=(rec.definition.as_ref(),rec.instance.as_ref()) {
                let env_id=self.env_graph.ensure_detected(&inst.path);
                let owner=toolhub_environment::attribute_owner(&inst.path);
                let scope=report.coverage.scopes.iter().find(|scope|toolhub_core::normalize_path(&inst.path).starts_with(&toolhub_core::normalize_path(&scope.root)));
                instances.push(toolhub_registry::ScanInstanceInput{
                    instance:UpsertInstanceInput{id:inst.id.as_str().into(),definition_id:def.id.as_str().into(),definition_name:def.name.clone(),version:inst.version.clone(),platform:inst.platform.clone(),arch:inst.arch.clone(),path:inst.path.clone(),canonical_path:inst.canonical_path.clone(),environment_id:Some(env_id.as_str().into()),origin_json:serde_json::to_string(&inst.origin).map_err(json_err)?,owner_json:serde_json::to_string(&owner).map_err(json_err)?,trust_json:serde_json::to_string(&inst.trust).map_err(json_err)?,status:"available".into(),capabilities:def.capabilities.iter().map(|cap|cap.as_str().into()).collect()},
                    evidence:rec.evidence.clone(),interfaces:vec![toolhub_registry::ScanInterfaceInput{kind:"cli".into(),executable:Some(inst.path.clone())}],provider:cand.metadata.get("scanner_scope").and_then(|s|s.get("provider")).and_then(Value::as_str).map(str::to_owned).unwrap_or_else(||scope.map(|scope|scope.provider.clone()).unwrap_or_else(||"unknown".into())),root:cand.metadata.get("scanner_scope").and_then(|s|s.get("root")).and_then(Value::as_str).map(str::to_owned).unwrap_or_else(||scope.map(|scope|scope.root.clone()).unwrap_or_default())
                });
            } else {candidates.push(cand.clone());}
        }
        let scopes=report.coverage.scopes.iter().map(|scope|toolhub_registry::ReconcileScope{provider:scope.provider.clone(),root:scope.root.clone(),complete:scope.complete&&!report.cancelled,recursive:scope.recursive}).collect::<Vec<_>>();
        self.registry.ingest_scan(&self.env_graph.environments,&instances,&candidates,&scopes).map_err(db_err)?;
        self.last_scan=Some(chrono::Utc::now());
        self.event("scan","native scan ingested",json!({"candidates":report.candidates.len()}))?;
        Ok(())
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
        let (env_digest, policy_trust_digest) = match env_d.split_once('|') {
            Some((e, p)) => (e.to_string(), p.to_string()),
            None => (env_d, String::new()),
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
                env_digest,
                policy_trust_digest,
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
        assert!(svc.handle_authenticated(&request(1, "deny"),"fixture.controller",true).error.is_none());
        svc.registry
            .db
            .conn
            .execute_batch(
                "CREATE TRIGGER reject_policy BEFORE INSERT ON policy_rules
             BEGIN SELECT RAISE(ABORT, 'fixture write failure'); END;",
            )
            .unwrap();
        assert!(svc.handle_authenticated(&request(2, "ask"),"fixture.controller",true).error.is_some());
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

#[cfg(test)]
mod final_regressions {
    use super::*;
    fn service() -> (tempfile::TempDir, DaemonService) {
        let dir = tempfile::tempdir().unwrap();
        let svc = DaemonService::open(&dir.path().join("private.sqlite")).unwrap();
        (dir, svc)
    }
    #[test]
    fn principal_label_cannot_grant_controller_role() {
        let (_dir, mut svc) = service();
        svc.connection_principal = Some("local.admin.spoof".into());
        assert!(!svc.is_admin(), "unverified label grants controller role");
    }
    #[test]
    fn scalar_params_are_rejected() {
        let (_dir, mut svc) = service();
        let resp = svc.handle(&JsonRpcRequest::new(1,"registry.search",json!(7)));
        assert!(resp.error.is_some(), "scalar params silently accepted");
    }
    #[test]
    fn malformed_report_is_rejected_atomically() {
        let (_dir, mut svc) = service();
        let report=json!({"schema":"toolhub.report/v1","tools":[{"name":"valid","path":"foreign"},true]});
        let resp=svc.handle(&JsonRpcRequest::new(1,"export.report",json!({"import":report})));
        assert!(resp.error.is_some(), "malformed import reported success");
        assert_eq!(svc.registry.count_instances().unwrap(),0,"partial import escaped validation");
    }
    #[test]
    fn persisted_revocation_is_observed_by_open_service() {
        let (_dir, mut svc) = service();
        let start=svc.handle(&JsonRpcRequest::new(1,"discovery.start",json!({"scopes":["candidate.inspect"]})));
        let id=start.result.unwrap()["session_id"].as_str().unwrap().to_string();
        svc.registry.revoke_discovery_session(&id).unwrap();
        let resp=svc.handle(&JsonRpcRequest::new(2,"discovery.inspect",json!({"id":id})));
        assert!(resp.error.is_some(), "open process used stale session cache");
    }
}
