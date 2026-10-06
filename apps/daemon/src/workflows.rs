use super::execution_jobs::{invalid, policy_action, policy_scope, text};
use super::*;

pub(super) fn is_extended(name: &str) -> bool {
    matches!(
        name,
        "protocol.negotiate"
            | "capability.list"
            | "registry.correct"
            | "scan.cancel"
            | "import.report"
            | "execute.approval_request"
            | "execute.approvals"
            | "skill.register"
            | "settings.get"
            | "settings.set"
            | "resource.list"
            | "resource.activate"
            | "resource.rollback"
            | "update.verify"
            | "update.apply"
            | "update.cancel"
            | "agent.launch"
            | "agent.status"
            | "agent.cancel"
            | "credentials.set"
            | "credentials.clear"
            | "credentials.status"
            | "credentials.session"
            | "events.subscribe"
            | "events.poll"
            | "events.unsubscribe"
            | "program.search"
            | "program.propose"
            | "program.proposals"
            | "program.proposal_dismiss"
            | "program.list"
            | "program.scan_start"
            | "program.scan_status"
            | "program.scan_cancel"
            | "program.select"
            | "program.save"
            | "program.remove"
            | "program.launch"
    )
}
impl DaemonService {
    pub(super) fn correct_instance(&mut self, params: &Value) -> Result<Value, ProtocolError> {
        self.require_controller()?;
        let id = text(params, "id")?;
        let row = self
            .registry
            .get_instance(id)
            .map_err(db_err)?
            .ok_or_else(|| ProtocolError::new(ErrorCode::NotFound, "instance unavailable"))?;
        let trust = params
            .get("trust")
            .map(|v| {
                v.as_str()
                    .filter(|v| matches!(*v, "unknown" | "user_trusted" | "blocked"))
                    .ok_or_else(|| invalid("manual trust must be unknown, user_trusted or blocked"))
            })
            .transpose()?;
        let name = params
            .get("name")
            .map(|v| {
                v.as_str()
                    .filter(|s| !s.trim().is_empty() && s.len() <= 256)
                    .ok_or_else(|| invalid("bounded name required"))
            })
            .transpose()?;
        if trust.is_none() && name.is_none() {
            return Err(invalid("name or trust correction required"));
        }
        let tx = self.registry.db.conn.transaction().map_err(sql_err)?;
        if let Some(trust) = trust {
            tx.execute(
                "UPDATE tool_instances SET trust_json=?1 WHERE id=?2",
                rusqlite::params![json!({"level":trust}).to_string(), id],
            )
            .map_err(sql_err)?;
        }
        if let Some(name) = name {
            tx.execute(
                "UPDATE tool_definitions SET name=?1,updated_at=?2 WHERE id=?3",
                rusqlite::params![name, chrono::Utc::now().to_rfc3339(), row.definition_id],
            )
            .map_err(sql_err)?;
        }
        tx.execute(
            "UPDATE authorization_revision SET revision=revision+1 WHERE id=1",
            [],
        )
        .map_err(sql_err)?;
        tx.commit().map_err(sql_err)?;
        self.event(
            "registry.corrected",
            "explicit user correction recorded",
            json!({"instance_id":id}),
        )?;
        Ok(json!({"updated":true,"id":id}))
    }
    pub(super) fn dispatch_extended(
        &mut self,
        name: &str,
        params: &Value,
    ) -> Result<Value, ProtocolError> {
        match name {
            name if name.starts_with("program.") => self.dispatch_program(name, params),
            "scan.cancel" => self.cancel_scan(params),
            "capability.list" => {
                let registry = self.resources.capabilities();
                Ok(json!(registry
                    .list_canonical()
                    .into_iter()
                    .map(|id| json!({"id":id,"aliases":registry.aliases_for(id)}))
                    .collect::<Vec<_>>()))
            }
            "registry.correct" => self.correct_instance(params),
            "protocol.negotiate" => {
                let versions = params
                    .get("versions")
                    .and_then(Value::as_array)
                    .ok_or_else(|| invalid("versions array required"))?;
                if !versions
                    .iter()
                    .any(|v| v.as_str() == Some(toolhub_protocol::PROTOCOL_VERSION))
                {
                    return Err(ProtocolError::new(
                        ErrorCode::InvalidRequest,
                        "unsupported protocol version",
                    ));
                }
                Ok(
                    json!({"protocol_version":toolhub_protocol::PROTOCOL_VERSION,"supported":[toolhub_protocol::PROTOCOL_VERSION]}),
                )
            }
            "execute.approval_request" => self.request_approval(params),
            "execute.approvals" => self.list_approval_requests(),
            "skill.register" => self.register_skill(params),
            "settings.get" => self.settings(),
            "settings.set" => self.save_settings(params),
            "import.report" => self.import_report(
                params
                    .get("report")
                    .ok_or_else(|| invalid("report required"))?,
            ),
            "events.subscribe" => {
                let cursor = params
                    .get("after_seq")
                    .map(|v| {
                        v.as_u64()
                            .ok_or_else(|| invalid("after_seq must be integer"))
                    })
                    .transpose()?
                    .unwrap_or(0);
                let id = uuid::Uuid::new_v4().to_string();
                self.subscriptions
                    .insert(id.clone(), (self.peer_principal(), cursor));
                Ok(json!({"subscription_id":id,"after_seq":cursor,"retained":512}))
            }
            "events.poll" => self.poll_events(params),
            "events.unsubscribe" => {
                let id = text(params, "subscription_id")?;
                self.check_subscription(id)?;
                self.subscriptions.remove(id);
                Ok(json!({"unsubscribed":true}))
            }
            "resource.list" => Ok(json!({"packages":self.resources.list()})),
            "resource.activate" | "resource.rollback" => {
                self.require_controller()?;
                let package = if name == "resource.activate" {
                    self.resources.activate(Path::new(text(params, "path")?))
                } else {
                    self.resources.rollback()
                }
                .map_err(|_| invalid("resource validation or activation failed"))?;
                self.registry.bump_revision().map_err(db_err)?;
                self.event(
                    "resource",
                    "recognition resource changed",
                    json!({"version":package.version}),
                )?;
                Ok(json!(package))
            }
            "update.verify" => {
                self.require_controller()?;
                let package = toolhub_agent_bridge::update::verify_package(Path::new(text(
                    params,
                    "manifest_path",
                )?))
                .map_err(|_| invalid("update package verification failed"))?;
                let id = uuid::Uuid::new_v4().to_string();
                let view = json!(package);
                self.verified_updates.insert(id.clone(), package);
                Ok(json!({"update_id":id,"package":view,"requires_explicit_confirmation":true}))
            }
            "update.apply" => {
                self.require_controller()?;
                if params.get("confirmed") != Some(&Value::Bool(true)) {
                    return Err(ProtocolError::denied(
                        "explicit product update confirmation required",
                    ));
                }
                let id = text(params, "update_id")?;
                let package = self.verified_updates.remove(id).ok_or_else(|| {
                    ProtocolError::new(ErrorCode::NotFound, "verified update not found")
                })?;
                let dest = Path::new(text(params, "destination")?);
                // Only ToolHub's own staged artifact directory, never discovered tools.
                let stage = Path::new(&self.registry_path)
                    .parent()
                    .unwrap_or(Path::new("."))
                    .join("toolhub-updates");
                std::fs::create_dir_all(&stage)
                    .map_err(|_| invalid("update staging unavailable"))?;
                let parent = dest
                    .parent()
                    .ok_or_else(|| invalid("destination directory required"))?;
                if std::fs::canonicalize(parent).map_err(|_| invalid("destination unavailable"))?
                    != std::fs::canonicalize(&stage).map_err(|_| invalid("stage unavailable"))?
                {
                    return Err(ProtocolError::denied(
                        "updates limited to ToolHub staging directory",
                    ));
                }
                toolhub_agent_bridge::update::apply_update(&package, dest, &|| false).map_err(
                    |_| {
                        ProtocolError::new(
                            ErrorCode::InternalError,
                            "update failed; previous artifact restored",
                        )
                    },
                )?;
                self.event(
                    "update",
                    "verified ToolHub artifact staged",
                    json!({"update_id":id}),
                )?;
                Ok(
                    json!({"applied":true,"activation":"explicit user restart/install required","discovered_tools_modified":false}),
                )
            }
            "update.cancel" => {
                self.require_controller()?;
                let id = text(params, "update_id")?;
                let removed = self.verified_updates.remove(id).is_some();
                Ok(json!({"cancelled":removed}))
            }
            "agent.launch" => self.launch_agent(params),
            "agent.status" => self.agent_state(params, false),
            "agent.cancel" => self.agent_state(params, true),
            "credentials.set" => self.configure_credentials(params),
            "credentials.status" => {
                let owner = self.peer_principal();
                let id = text(params, "provider_session_id")?;
                self.credentials
                    .describe(&owner, id)
                    .map(|v| json!(v))
                    .map_err(|_| ProtocolError::denied("provider session unavailable"))
            }
            "credentials.clear" => {
                let owner = self.peer_principal();
                self.credentials
                    .cancel(&owner, text(params, "provider_session_id")?)
                    .map_err(|_| ProtocolError::denied("provider session unavailable"))?;
                Ok(json!({"cleared":true}))
            }
            "credentials.session" => self.run_temporary_provider(params),
            _ => Err(ProtocolError::new(
                ErrorCode::MethodNotFound,
                "unknown method",
            )),
        }
    }
    pub(super) fn require_controller(&self) -> Result<(), ProtocolError> {
        if self.is_admin() {
            Ok(())
        } else {
            Err(ProtocolError::denied(
                "verified desktop controller required",
            ))
        }
    }
    pub(super) fn event(
        &mut self,
        kind: &str,
        summary: &str,
        data: Value,
    ) -> Result<(), ProtocolError> {
        let summary = toolhub_executor::redact_text(summary);
        self.registry
            .record_activity(kind, &summary, Some(&self.peer_principal()))
            .map_err(db_err)?;
        self.registry
            .db
            .conn
            .execute(
                "INSERT INTO event_log(ts,kind,summary,data_json) VALUES(?1,?2,?3,?4)",
                rusqlite::params![
                    chrono::Utc::now().to_rfc3339(),
                    kind,
                    summary,
                    serde_json::to_string(&data).map_err(json_err)?
                ],
            )
            .map_err(sql_err)?;
        self.registry.db.conn.execute("DELETE FROM event_log WHERE seq < (SELECT COALESCE(MAX(seq),0)-511 FROM event_log)",[]).map_err(sql_err)?;
        Ok(())
    }
    fn check_subscription(&self, id: &str) -> Result<u64, ProtocolError> {
        let (owner, cursor) = self.subscriptions.get(id).ok_or_else(|| {
            ProtocolError::new(
                ErrorCode::NotFound,
                "subscription not found; reconnect with last_seq",
            )
        })?;
        if owner != &self.peer_principal() {
            return Err(ProtocolError::denied(
                "subscription belongs to another caller",
            ));
        }
        Ok(*cursor)
    }
    fn poll_events(&mut self, params: &Value) -> Result<Value, ProtocolError> {
        let id = text(params, "subscription_id")?;
        let cursor = self.check_subscription(id)?;
        let limit = params
            .get("limit")
            .and_then(Value::as_u64)
            .unwrap_or(100)
            .clamp(1, 100);
        let first: u64 = self
            .registry
            .db
            .conn
            .query_row("SELECT COALESCE(MIN(seq),0) FROM event_log", [], |row| {
                row.get(0)
            })
            .map_err(sql_err)?;
        let mut stmt=self.registry.db.conn.prepare("SELECT seq,ts,kind,summary,data_json FROM event_log WHERE seq>?1 ORDER BY seq LIMIT ?2").map_err(sql_err)?;
        let rows = stmt
            .query_map(rusqlite::params![cursor, limit], |r| {
                Ok((
                    r.get::<_, u64>(0)?,
                    r.get::<_, String>(1)?,
                    r.get::<_, String>(2)?,
                    r.get::<_, String>(3)?,
                    r.get::<_, String>(4)?,
                ))
            })
            .map_err(sql_err)?
            .collect::<Result<Vec<_>, _>>()
            .map_err(sql_err)?;
        let last = rows.last().map(|row| row.0).unwrap_or(cursor);
        let events=rows.into_iter().map(|(seq,ts,kind,summary,data)|json!({"jsonrpc":"2.0","method":"event","params":{"seq":seq,"ts":ts,"kind":kind,"summary":summary,"data":serde_json::from_str::<Value>(&data).unwrap_or(Value::Null)}})).collect::<Vec<_>>();
        drop(stmt);
        if let Some(subscription) = self.subscriptions.get_mut(id) {
            subscription.1 = last;
        }
        Ok(
            json!({"events":events,"last_seq":last,"gap":first>0&&cursor<first.saturating_sub(1),"oldest_seq":first}),
        )
    }
    fn list_approval_requests(&mut self) -> Result<Value, ProtocolError> {
        let owner = self.peer_principal();
        let admin = self.is_admin();
        let mut stmt=self.registry.db.conn.prepare("SELECT id,owner,session_id,params_json,expires_at,status FROM execution_requests ORDER BY expires_at DESC LIMIT 100").map_err(sql_err)?;
        let requests = stmt
            .query_map([], |r| {
                Ok((
                    r.get::<_, String>(0)?,
                    r.get::<_, String>(1)?,
                    r.get::<_, String>(2)?,
                    r.get::<_, String>(3)?,
                    r.get::<_, String>(4)?,
                    r.get::<_, String>(5)?,
                ))
            })
            .map_err(sql_err)?
            .collect::<Result<Vec<_>, _>>()
            .map_err(sql_err)?;
        let rows=requests.into_iter().filter(|row|admin||row.1==owner).map(|row|{
            let mut disclosure=serde_json::from_str::<Value>(&row.3).unwrap_or(Value::Null);
            let available=self.pending_approvals.get(&row.0);
            if let Some(pending)=available {if let Some(args)=pending.params.get("args").and_then(Value::as_array){disclosure["args"]=json!(toolhub_executor::redact_args(&args.iter().filter_map(Value::as_str).map(str::to_owned).collect::<Vec<_>>()));}disclosure["executable"]=pending.params.get("_executable").cloned().unwrap_or(Value::Null);disclosure["cwd"]=pending.params.get("cwd").cloned().unwrap_or(Value::Null);disclosure["capability"]=pending.params.get("capability").cloned().unwrap_or(Value::Null);disclosure["environment_digest"]=json!(pending.env_digest);disclosure["stdin_digest"]=json!(toolhub_core::execution::digest_strings(&[pending.params.get("stdin").and_then(Value::as_str).unwrap_or("").to_string()]));}
            json!({"request_id":row.0,"owner":row.1,"session_id":row.2,"request":disclosure,"expires_at":row.4,"status":row.5,"body_available":available.is_some()})
        }).collect::<Vec<_>>();
        Ok(json!(rows))
    }
    pub(super) fn set_policy(&mut self, params: &Value) -> Result<Value, ProtocolError> {
        self.require_controller()?;
        let scope = text(params, "scope")?;
        let subject = text(params, "subject")?;
        let action = text(params, "action")?;
        let scope_enum = policy_scope(scope)?;
        let act = policy_action(action)?;
        self.registry
            .set_policy(scope, subject, action)
            .map_err(db_err)?;
        self.policy.set(toolhub_policy::PolicyRule {
            scope: scope_enum,
            subject: subject.into(),
            action: act,
        });
        let revision = self.registry.revision().map_err(db_err)?;
        self.event(
            "policy",
            "effective policy changed",
            json!({"revision":revision}),
        )?;
        Ok(json!({"ok":true}))
    }
    fn settings(&mut self) -> Result<Value, ProtocolError> {
        let mut settings=self.registry.get_setting("user_preferences").map_err(db_err)?.unwrap_or_else(||json!({"scan_mode":"quick","scan_roots":[],"resolver_preferences":{},"privacy":{"redact_exports":true}}));
        settings["versions"] = json!({"app":env!("CARGO_PKG_VERSION"),"core":env!("CARGO_PKG_VERSION"),"protocol":toolhub_protocol::PROTOCOL_VERSION,"resources":self.resources.list().iter().map(|p|p.version.clone()).collect::<Vec<_>>()});
        Ok(settings)
    }
    fn save_settings(&mut self, params: &Value) -> Result<Value, ProtocolError> {
        self.require_controller()?;
        let settings = params.get("settings").unwrap_or(params);
        let object = settings
            .as_object()
            .ok_or_else(|| invalid("settings object required"))?;
        for key in object.keys() {
            if !matches!(
                key.as_str(),
                "scan_mode" | "scan_roots" | "resolver_preferences" | "privacy" | "extensions"
            ) {
                return Err(invalid("unsupported or secret setting"));
            }
        }
        if let Some(mode) = settings.get("scan_mode") {
            if !matches!(mode.as_str(), Some("quick" | "full" | "custom")) {
                return Err(invalid("invalid scan mode"));
            }
        }
        if let Some(roots) = settings.get("scan_roots") {
            let roots = roots
                .as_array()
                .ok_or_else(|| invalid("scan_roots array required"))?;
            if roots.len() > 64
                || roots
                    .iter()
                    .any(|r| r.as_str().is_none_or(|s| !Path::new(s).is_absolute()))
            {
                return Err(invalid("scan roots must be bounded absolute paths"));
            }
        }
        if let Some(prefs) = settings.get("resolver_preferences") {
            let prefs = prefs
                .as_object()
                .ok_or_else(|| invalid("resolver_preferences object required"))?;
            for (key, value) in prefs {
                if !matches!(
                    key.as_str(),
                    "cwd" | "environment" | "trust" | "arch" | "version"
                ) || value.as_str().is_none_or(|v| v.len() > 4096)
                {
                    return Err(invalid("unsupported resolver preference"));
                }
            }
        }
        if let Some(privacy) = settings.get("privacy") {
            let privacy = privacy
                .as_object()
                .ok_or_else(|| invalid("privacy object required"))?;
            if privacy
                .iter()
                .any(|(key, value)| key != "redact_exports" || !value.is_boolean())
            {
                return Err(invalid("unsupported privacy preference"));
            }
            if privacy.get("redact_exports") == Some(&Value::Bool(false)) {
                return Err(invalid("export redaction must remain enabled"));
            }
        }
        if let Some(extensions) = settings.get("extensions") {
            let extensions = serde_json::from_value::<Vec<toolhub_scanner::DeclarativeExtension>>(
                extensions.clone(),
            )
            .map_err(json_err)?;
            if extensions.len() > 16 {
                return Err(invalid("at most 16 scanner extensions"));
            }
            for extension in extensions {
                extension
                    .validate()
                    .map_err(|_| invalid("invalid scanner extension"))?;
            }
        }
        self.registry
            .set_setting("user_preferences", settings)
            .map_err(db_err)?;
        self.event("settings", "nonsecret preferences saved", json!({}))?;
        self.settings()
    }
    pub(super) fn register_skill(&mut self, params: &Value) -> Result<Value, ProtocolError> {
        let (manifest, source) = if let Some(path) = params.get("path") {
            let path = std::fs::canonicalize(
                path.as_str()
                    .ok_or_else(|| invalid("path must be string"))?,
            )
            .map_err(|_| invalid("manifest unavailable"))?;
            let metadata = std::fs::metadata(&path).map_err(|_| invalid("manifest unavailable"))?;
            if metadata.len() > 1024 * 1024 {
                return Err(invalid("manifest too large"));
            }
            let manifest = serde_json::from_slice::<Value>(
                &std::fs::read(&path).map_err(|_| invalid("manifest read failed"))?,
            )
            .map_err(json_err)?;
            (manifest, Some(path))
        } else {
            (params.get("manifest").unwrap_or(params).clone(), None)
        };
        let parsed = toolhub_core::SkillManifest::parse_yaml_like(&manifest)
            .map_err(|e| invalid(&e.to_string()))?;
        if let Some(source) = &source {
            let root = source
                .parent()
                .ok_or_else(|| invalid("manifest directory unavailable"))?;
            for path in [
                &parsed.instruction_file,
                &parsed.mcp_config,
                &parsed.package_path,
            ]
            .into_iter()
            .flatten()
            {
                let target = std::fs::canonicalize(root.join(path))
                    .map_err(|_| invalid("declared content unavailable"))?;
                if !target.starts_with(root) {
                    return Err(ProtocolError::denied("package symlink escapes root"));
                }
            }
        }
        let root = source.as_ref().and_then(|s| s.parent());
        let instructions = Self::read_skill_instructions(&parsed, root);
        let compatibility =
            parsed.library_compatibility(std::env::consts::OS, instructions.as_deref());
        if matches!(
            compatibility.status.as_str(),
            "needs_review" | "host_specific"
        ) {
            return Err(invalid(&compatibility.issues.join("；")));
        }
        if params["if_absent"] == true {
            let root = root.ok_or_else(|| invalid("if_absent requires a manifest file"))?;
            if !self
                .registry
                .insert_skill_if_absent(&parsed, root)
                .map_err(db_err)?
            {
                return Err(invalid(
                    "Skill ID already registered; existing content preserved",
                ));
            }
        } else {
            self.registry
                .upsert_skill(
                    parsed.id.as_str(),
                    &parsed.name,
                    &parsed.schema,
                    match parsed.kind {
                        toolhub_core::skill::SkillKind::Instruction => "instruction",
                        toolhub_core::skill::SkillKind::Mcp => "mcp",
                        toolhub_core::skill::SkillKind::Package => "package",
                    },
                    &serde_json::to_string(&manifest).map_err(json_err)?,
                    // Registry asset validation is relative to the package directory,
                    // whereas the CLI supplies the skill.json manifest file.
                    source
                        .as_ref()
                        .and_then(|s| s.parent())
                        .and_then(|s| s.to_str()),
                )
                .map_err(db_err)?;
        }
        self.event(
            "skill",
            "declarative skill registered",
            json!({"id":parsed.id}),
        )?;
        self.resolve_skill(&json!({"id":parsed.id}))
    }
    pub(super) fn resolve_skill(&mut self, params: &Value) -> Result<Value, ProtocolError> {
        let id = text(params, "id")?;
        let row = self
            .registry
            .list_skills()
            .map_err(db_err)?
            .into_iter()
            .find(|r| r.0 == id)
            .ok_or_else(|| ProtocolError::new(ErrorCode::NotFound, "skill not registered"))?;
        let manifest: Value = serde_json::from_str(&row.3).map_err(json_err)?;
        let skill = toolhub_core::SkillManifest::parse_yaml_like(&manifest)
            .map_err(|e| invalid(&e.to_string()))?;
        let source: Option<String> = self
            .registry
            .db
            .conn
            .query_row("SELECT path FROM skills WHERE id=?1", [id], |r| r.get(0))
            .map_err(sql_err)?;
        let instructions = Self::read_skill_instructions(&skill, source.as_deref().map(Path::new));
        self.skill_view(&skill, manifest, instructions, params)
    }
    pub(super) fn preview_skill(&mut self, params: &Value) -> Result<Value, ProtocolError> {
        let manifest = params
            .get("manifest")
            .ok_or_else(|| invalid("manifest required"))?
            .clone();
        let skill = toolhub_core::SkillManifest::validate_json(&manifest)
            .map_err(|e| invalid(&e.to_string()))?;
        if skill.requires.len() + skill.optional.len() > 32 || manifest.to_string().len() > 65536 {
            return Err(invalid("preview manifest exceeds bounds"));
        }
        let instructions = params
            .get("instructions")
            .and_then(Value::as_str)
            .filter(|s| !s.is_empty() && s.len() <= 65536)
            .ok_or_else(|| invalid("bounded instructions required"))?
            .to_string();
        self.skill_view(&skill, manifest, Some(instructions), params)
    }
    fn skill_view(
        &mut self,
        skill: &toolhub_core::SkillManifest,
        manifest: Value,
        instructions: Option<String>,
        params: &Value,
    ) -> Result<Value, ProtocolError> {
        let id = skill.id.as_str();
        let mut providers = BTreeMap::new();
        let capabilities = self.resources.capabilities();
        for requirement in skill.requires.iter().chain(&skill.optional) {
            let cap = capabilities
                .canonical_of(requirement.capability.as_str())
                .unwrap_or(requirement.capability.as_str());
            let rows = self
                .registry
                .instances_for_capability(cap)
                .map_err(db_err)?;
            providers.insert(
                cap.to_string(),
                rows.into_iter()
                    .filter(|r| Path::new(&r.path).is_file())
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
                    .collect::<Vec<_>>(),
            );
        }
        let prefs = self.resolver_preferences(params)?;
        let result = toolhub_skills::resolve_availability_with_resolver(
            skill,
            &capabilities,
            &providers,
            &prefs,
        );
        let mut result = serde_json::to_value(result).map_err(json_err)?;
        result["id"] = json!(id);
        result["manifest"] = manifest;
        let compatibility =
            skill.library_compatibility(std::env::consts::OS, instructions.as_deref());
        if !compatibility.reusable {
            result["status"] = json!(compatibility.status);
        }
        result["instructions"] = json!(if compatibility.reusable {
            instructions
        } else {
            None
        });
        result["compatibility"] = serde_json::to_value(compatibility).map_err(json_err)?;
        result["instruction_authority"] = json!(false);
        Ok(result)
    }
    fn read_skill_instructions(
        skill: &toolhub_core::SkillManifest,
        root: Option<&Path>,
    ) -> Option<String> {
        let path = toolhub_core::SkillManifest::resolve_package_path(
            root?,
            skill.instruction_file.as_deref()?,
        )
        .ok()?;
        let metadata = path.metadata().ok()?;
        if !metadata.is_file() || metadata.len() > 65536 {
            return None;
        }
        std::fs::read_to_string(path).ok()
    }
    pub(super) fn list_skill_inventory(&mut self) -> Result<Value, ProtocolError> {
        let rows = self.registry.list_skills().map_err(db_err)?;
        let mut list = vec![];
        for (id, name, schema, manifest) in rows {
            let m: Value = serde_json::from_str(&manifest).unwrap_or(json!({}));
            let compatibility = self
                .resolve_skill(&json!({"id":id}))
                .map(|v| v["compatibility"].clone())
                .unwrap_or(
                    json!({"status":"malformed","reusable":false,"issues":["历史条目格式不完整"]}),
                );
            list.push(json!({"id":id,"name":name,"schema":schema,"manifest":m,"compatibility":compatibility}));
        }
        Ok(json!(list))
    }
    pub(super) fn resolver_preferences(
        &mut self,
        params: &Value,
    ) -> Result<ResolvePrefs, ProtocolError> {
        let settings = self
            .registry
            .get_setting("user_preferences")
            .map_err(db_err)?
            .unwrap_or_else(|| json!({}));
        let saved = settings
            .get("resolver_preferences")
            .cloned()
            .unwrap_or_else(|| json!({}));
        let field = |name: &str, alias: &str| {
            params
                .get(name)
                .or_else(|| params.get(alias))
                .or_else(|| saved.get(name))
                .and_then(Value::as_str)
                .filter(|s| !s.is_empty())
                .map(str::to_owned)
        };
        let preferred = self
            .registry
            .get_setting("preferred_instances")
            .map_err(db_err)?
            .unwrap_or_else(|| json!({}));
        let mut preferred_instances: Vec<String> = preferred
            .as_object()
            .into_iter()
            .flat_map(|m| m.values())
            .filter_map(Value::as_str)
            .map(str::to_owned)
            .collect();
        if let Some(cwd) = field("cwd", "cwd") {
            let projects = self
                .registry
                .get_setting("preferred_project_instances")
                .map_err(db_err)?
                .unwrap_or_else(|| json!({}));
            let cwd = toolhub_core::path_norm::canonicalize_best_effort(&cwd);
            if let Some((_, choices)) = projects
                .as_object()
                .into_iter()
                .flat_map(|m| m.iter())
                .filter(|(root, _)| cwd == **root || cwd.starts_with(&format!("{root}/")))
                .max_by_key(|(root, _)| root.len())
            {
                let mut project_ids: Vec<String> = choices
                    .as_object()
                    .into_iter()
                    .flat_map(|m| m.values())
                    .filter_map(Value::as_str)
                    .map(str::to_owned)
                    .collect();
                project_ids.extend(preferred_instances);
                preferred_instances = project_ids;
            }
        }
        if let Some(id) = params.get("preferred_instance").and_then(Value::as_str) {
            preferred_instances.insert(0, id.into());
        }
        Ok(ResolvePrefs {
            preferred_instances,
            cwd: field("cwd", "cwd"),
            prefer_environment: field("environment", "preferred_environment"),
            min_version: field("version", "version"),
            require_trust: field("trust", "require_trust"),
            require_arch: field("arch", "require_arch"),
        })
    }
    pub(super) fn export_report(&mut self) -> Result<Value, ProtocolError> {
        let mut tools = Vec::new();
        for row in self.registry.list_instances().map_err(db_err)? {
            let caps = self
                .registry
                .capabilities_for_definition(&row.definition_id)
                .map_err(db_err)?;
            tools.push(json!({"name":row.name,"version":row.version,"platform":row.platform,"arch":row.arch,"path":toolhub_audit::redact_path_for_export(&row.path),"environment":row.environment_id,"trust":parse_trust(&row.trust),"capabilities":caps,"provenance":"native_registry"}));
        }
        let environments=self.registry.load_environment_graph().map_err(db_err)?.into_iter().map(|env|json!({"id":env.id,"name":env.name,"kind":env.kind,"root_path":env.root_path.map(|p|toolhub_audit::redact_path_for_export(&p)),"parent_id":env.parent_id})).collect::<Vec<_>>();
        Ok(
            json!({"schema":"toolhub.report/v1","generated_at":chrono::Utc::now(),"tools":tools,"environments":environments,"redaction":["home_path","username","secrets","sensitive_args"]}),
        )
    }
    pub(super) fn import_report(&mut self, report: &Value) -> Result<Value, ProtocolError> {
        if report.get("schema").and_then(Value::as_str) != Some("toolhub.report/v1") {
            return Err(invalid("unsupported report schema"));
        }
        let tools = report
            .get("tools")
            .and_then(Value::as_array)
            .ok_or_else(|| invalid("report tools array required"))?;
        if tools.len() > 500 {
            return Err(invalid("report exceeds 500 tools"));
        }
        let mut inputs = Vec::new();
        for tool in tools {
            let name = text(tool, "name")?;
            let path = text(tool, "path")?;
            if name.len() > 256 || path.len() > 4096 {
                return Err(invalid("report field too large"));
            }
            let version = tool
                .get("version")
                .filter(|v| !v.is_null())
                .map(|v| {
                    v.as_str()
                        .map(str::to_owned)
                        .ok_or_else(|| invalid("version must be string or null"))
                })
                .transpose()?;
            inputs.push(UpsertInstanceInput {
                id: format!(
                    "import-{}",
                    toolhub_core::path_fingerprint(&format!("{name}:{path}"))
                ),
                definition_id: format!("imported.{}", toolhub_core::path_fingerprint(name)),
                definition_name: name.into(),
                version,
                platform: "foreign".into(),
                arch: "unknown".into(),
                path: path.into(),
                canonical_path: None,
                environment_id: None,
                origin_json: "{\"unknown\":{}}".into(),
                owner_json: "{\"kind\":\"unknown\",\"certainty\":\"unknown\",\"evidence\":[]}"
                    .into(),
                trust_json: "{\"level\":\"unknown\"}".into(),
                status: "missing".into(),
                capabilities: vec![],
            });
        }
        let imported = self.registry.import_report(&[], &inputs).map_err(db_err)?;
        self.event(
            "report.import",
            "foreign metadata imported without execution trust",
            json!({"imported":inputs.len()}),
        )?;
        Ok(json!({"imported":imported.len(),"execution_trust":false}))
    }
}
pub(super) fn json_err(error: serde_json::Error) -> ProtocolError {
    ProtocolError::new(ErrorCode::InvalidParams, error.to_string())
}
pub(super) fn sql_err(_error: rusqlite::Error) -> ProtocolError {
    ProtocolError::new(ErrorCode::InternalError, "persistent operation failed")
}

#[cfg(test)]
mod skill_registration_tests {
    use super::*;

    #[test]
    fn manifest_file_registers_sibling_assets_from_its_package_directory() {
        let dir = tempfile::tempdir().unwrap();
        let package = dir.path().join("Agent's Skill 空格");
        std::fs::create_dir(&package).unwrap();
        std::fs::write(package.join("SKILL.md"), "instruction fixture").unwrap();
        let manifest = package.join("skill.json");
        let value = json!({"schema":"toolhub.skill/v1","id":"fixture.integration","name":"Fixture", "kind":"instruction","instruction_file":"SKILL.md","description":"Owned workflow","portability":{"scope":"portable","platforms":["windows","macos","linux"],"inputs":["local input"],"outputs":["local output"],"permissions":[],"host_dependencies":[]},"requires":[]});
        std::fs::write(&manifest, serde_json::to_vec(&value).unwrap()).unwrap();
        let mut service = DaemonService::open(&dir.path().join("private.sqlite")).unwrap();
        let response = service.handle(&toolhub_protocol::JsonRpcRequest::new(
            1,
            "skill.register",
            json!({"path":manifest}),
        ));
        assert!(response.error.is_none(), "{:?}", response.error);
        assert_eq!(response.result.unwrap()["status"], "available");
        let stored: String = service
            .registry
            .db
            .conn
            .query_row(
                "SELECT path FROM skills WHERE id='fixture.integration'",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(
            std::path::Path::new(&stored),
            package.canonicalize().unwrap()
        );

        let mut invalid = value;
        invalid["instruction_file"] = json!("missing.md");
        std::fs::write(&manifest, serde_json::to_vec(&invalid).unwrap()).unwrap();
        let response = service.handle(&toolhub_protocol::JsonRpcRequest::new(
            2,
            "skill.register",
            json!({"path":manifest}),
        ));
        assert!(response.error.is_some());
        let rows = service.registry.list_skills().unwrap();
        assert_eq!(rows.len(), 1);
        assert_eq!(
            serde_json::from_str::<Value>(&rows[0].3).unwrap()["instruction_file"],
            "SKILL.md"
        );
    }
}

#[cfg(test)]
mod preference_priority_tests {
    use super::*;
    #[test]
    fn explicit_choice_precedes_project_then_global_defaults() {
        let dir = tempfile::tempdir().unwrap();
        let mut svc = DaemonService::open(&dir.path().join("prefs.sqlite")).unwrap();
        svc.registry
            .set_setting("preferred_instances", &json!({"org.nodejs.node":"global"}))
            .unwrap();
        let project =
            toolhub_core::path_norm::canonicalize_best_effort(dir.path().to_str().unwrap());
        let mut projects = serde_json::Map::new();
        projects.insert(project, json!({"org.nodejs.node":"project"}));
        svc.registry
            .set_setting("preferred_project_instances", &Value::Object(projects))
            .unwrap();
        let prefs = svc
            .resolver_preferences(&json!({"cwd":dir.path(),"preferred_instance":"explicit"}))
            .unwrap();
        assert_eq!(
            prefs.preferred_instances,
            vec!["explicit", "project", "global"]
        );
    }
}

#[cfg(test)]
mod portability_registration_tests {
    use super::*;
    #[test]
    fn failed_import_preserves_previous_skill_and_rechecks_changed_instructions() {
        let dir = tempfile::tempdir().unwrap();
        let manifest = dir.path().join("skill.json");
        let instruction = dir.path().join("SKILL.md");
        std::fs::write(&instruction, "Use FFmpeg via ToolHub; created by ChatGPT").unwrap();
        let mut value = json!({"schema":"toolhub.skill/v1","id":"generic.test","name":"Test","description":"Generic workflow","instruction_file":"SKILL.md","portability":{"scope":"portable","platforms":["windows","macos","linux"],"inputs":["local input"],"outputs":["local output"],"permissions":[],"host_dependencies":[]}});
        std::fs::write(&manifest, value.to_string()).unwrap();
        let mut svc = DaemonService::open(&dir.path().join("db.sqlite")).unwrap();
        svc.register_skill(&json!({"path":manifest})).unwrap();
        let old = svc.registry.list_skills().unwrap()[0].3.clone();
        value["portability"]["host_dependencies"] = json!(["chatgpt.image_gen"]);
        std::fs::write(&manifest, value.to_string()).unwrap();
        assert!(svc.register_skill(&json!({"path":manifest})).is_err());
        assert!(svc
            .handle(&toolhub_protocol::JsonRpcRequest::new(
                1,
                "skill.list",
                json!({"register":value})
            ))
            .error
            .is_some());
        assert_eq!(svc.registry.list_skills().unwrap()[0].3, old);
        std::fs::write(&instruction, "Call image_gen").unwrap();
        let v = svc.resolve_skill(&json!({"id":"generic.test"})).unwrap();
        assert_eq!(v["compatibility"]["status"], "needs_review");
        assert!(v["instructions"].is_null());
        assert_eq!(v["instruction_authority"], false);
    }
}

#[cfg(test)]
mod market_preview_tests {
    use super::*;
    #[test]
    fn preview_is_read_only_and_market_collection_never_overwrites_existing_content() {
        let dir = tempfile::tempdir().unwrap();
        let mut svc = DaemonService::open(&dir.path().join("db.sqlite")).unwrap();
        let bundle = toolhub_skills::bundle::SkillBundle::parse(include_bytes!(
            "../../../skills/market/media-inspect.toolhub-skill.json"
        ))
        .unwrap();
        let manifest = serde_json::to_value(&bundle.manifest).unwrap();
        let preview = svc
            .preview_skill(&json!({"manifest":manifest,"instructions":bundle.instructions}))
            .unwrap();
        assert_eq!(preview["status"], "missing_capabilities");
        assert_eq!(preview["instruction_authority"], false);
        assert!(svc.registry.list_skills().unwrap().is_empty());
        assert!(!svc.controller_verified);
        assert!(svc.approvals.is_empty());
        std::fs::write(dir.path().join("SKILL.md"), &bundle.instructions).unwrap();
        std::fs::write(dir.path().join("skill.json"), manifest.to_string()).unwrap();
        svc.register_skill(&json!({"path":dir.path().join("skill.json"),"if_absent":true}))
            .unwrap();
        let before = svc.registry.list_skills().unwrap();
        let mut changed = manifest;
        changed["description"] = json!("Changed content");
        std::fs::write(dir.path().join("skill.json"), changed.to_string()).unwrap();
        assert!(svc
            .register_skill(&json!({"path":dir.path().join("skill.json"),"if_absent":true}))
            .is_err());
        assert_eq!(svc.registry.list_skills().unwrap(), before);
        assert!(!svc.controller_verified);
        assert!(svc.approvals.is_empty());
    }
}
