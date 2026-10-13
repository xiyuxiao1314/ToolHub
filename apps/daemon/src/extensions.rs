//! Persistent versioned extensions; frontend payloads are opaque to the daemon.
use super::execution_jobs::{invalid, text};
use super::*;
use sha2::{Digest, Sha256};
use toolhub_skills::{bundle::SkillBundle, extension::ExtensionPackage};
type StoredPackage = (ExtensionPackage, String, bool);
fn hash(p: &ExtensionPackage) -> Result<String, ProtocolError> {
    Ok(format!(
        "{:x}",
        Sha256::digest(serde_json::to_vec(p).map_err(json_err)?)
    ))
}
impl DaemonService {
    pub(super) fn extension_packages(
        &self,
        only_enabled: bool,
    ) -> Result<Vec<StoredPackage>, ProtocolError> {
        let mut stmt = self
            .registry
            .db
            .conn
            .prepare("SELECT package_json,digest,enabled FROM extension_packages ORDER BY id")
            .map_err(sql_err)?;
        let rows = stmt
            .query_map([], |r| {
                Ok((
                    r.get::<_, String>(0)?,
                    r.get::<_, String>(1)?,
                    r.get::<_, bool>(2)?,
                ))
            })
            .map_err(sql_err)?;
        let mut out = vec![];
        for row in rows {
            let (json, digest, enabled) = row.map_err(sql_err)?;
            if only_enabled && !enabled {
                continue;
            }
            let p = ExtensionPackage::parse(json.as_bytes()).map_err(|e| invalid(&e))?;
            if hash(&p)? != digest {
                return Err(invalid("extension snapshot integrity failure"));
            }
            out.push((p, digest, enabled));
        }
        Ok(out)
    }
    pub(super) fn extension_capabilities(
        &self,
    ) -> Result<toolhub_core::CapabilityRegistry, ProtocolError> {
        let mut caps = self.resources.capabilities();
        for (p, _, _) in self.extension_packages(true)? {
            if let Some(r) = p.recognition {
                for cap in r.capabilities {
                    caps.register(&cap, "active extension capability")
                        .map_err(|e| invalid(&e.to_string()))?;
                }
            }
        }
        Ok(caps)
    }
    pub(super) fn extension_skills(&self) -> Result<Vec<(String, SkillBundle)>, ProtocolError> {
        Ok(self
            .extension_packages(true)?
            .into_iter()
            .flat_map(|(p, _, _)| p.skills.into_iter().map(move |s| (p.id.clone(), s)))
            .collect())
    }
    pub(super) fn extension_skill(
        &self,
        id: &str,
    ) -> Result<Option<(String, SkillBundle)>, ProtocolError> {
        Ok(self
            .extension_skills()?
            .into_iter()
            .find(|(_, s)| s.manifest.id.as_str() == id))
    }
    pub(super) fn recognize_extensions(
        &self,
        candidate: &toolhub_core::ScanCandidate,
        rules: &[toolhub_recognizer::resources::ResourceTool],
    ) -> Result<toolhub_recognizer::RecognitionResult, ProtocolError> {
        // Existing native and user-loaded resource definitions retain precedence.
        let native = toolhub_recognizer::recognize_with_resources(candidate, &self.resources);
        if native.recognized {
            return Ok(native);
        }
        let mut rec = toolhub_recognizer::recognize_with_rules(candidate, rules);
        if let Some(inst) = rec.instance.as_mut() {
            let existing: Option<(String, String)> = self
                .registry
                .db
                .conn
                .query_row(
                    "SELECT i.definition_id,i.status FROM tool_instances i WHERE i.id=?1",
                    [inst.id.as_str()],
                    |r| Ok((r.get(0)?, r.get(1)?)),
                )
                .optional()
                .map_err(sql_err)?;
            if let Some((definition, status)) = existing {
                let owned: bool = self
                    .registry
                    .db
                    .conn
                    .query_row(
                        "SELECT EXISTS(SELECT 1 FROM extension_definitions WHERE definition_id=?1)",
                        [&definition],
                        |r| r.get(0),
                    )
                    .map_err(sql_err)?;
                if !owned && status == "available" {
                    // Keep existing native identity; skip this extension hypothesis.
                    rec.definition = None;
                    rec.instance = None;
                    rec.recognized = false;
                    rec.ambiguity = Some("existing native record retained".into());
                    return Ok(rec);
                }
            }
            // Publisher strings supplied by a third-party package are not trust evidence.
            inst.trust = toolhub_core::TrustRecord::unknown();
            rec.confidence = rec.confidence.min(0.4);
            rec.ambiguity = Some("extension hypothesis; user trust confirmation required".into());
        }
        Ok(rec)
    }
    fn check_extension_conflicts(&mut self, p: &ExtensionPackage) -> Result<(), ProtocolError> {
        self.check_workbench_dependencies(p)?;
        let rules = p.rules();
        let skills = self.registry.list_skills().map_err(db_err)?;
        for skill in &p.skills {
            if skills.iter().any(|s| s.0 == skill.manifest.id.as_str()) {
                return Err(invalid("Skill ID already belongs to the user's library"));
            }
        }
        for rule in &rules {
            let owner: Option<String> = self
                .registry
                .db
                .conn
                .query_row(
                    "SELECT extension_id FROM extension_definitions WHERE definition_id=?1",
                    [&rule.definition_id],
                    |r| r.get(0),
                )
                .optional()
                .map_err(sql_err)?;
            let exists: bool = self
                .registry
                .db
                .conn
                .query_row(
                    "SELECT EXISTS(SELECT 1 FROM tool_definitions WHERE id=?1)",
                    [&rule.definition_id],
                    |r| r.get(0),
                )
                .map_err(sql_err)?;
            if owner.as_ref().is_some_and(|o| o != &p.id)
                || (exists && owner.as_deref() != Some(&p.id))
            {
                return Err(invalid("tool definition belongs to another source"));
            }
            if self.resources.rules().iter().any(|other| {
                other.definition_id == rule.definition_id
                    || other
                        .file_names
                        .iter()
                        .any(|f| rule.file_names.iter().any(|n| n.eq_ignore_ascii_case(f)))
            }) {
                return Err(invalid(
                    "filename conflicts with active recognition resource",
                ));
            }
        }
        for (other, _, _) in self.extension_packages(false)? {
            if other.id == p.id {
                continue;
            }
            if other
                .skills
                .iter()
                .any(|s| p.skills.iter().any(|n| n.manifest.id == s.manifest.id))
                || other.rules().iter().any(|s| {
                    rules.iter().any(|r| {
                        r.definition_id == s.definition_id
                            || r.file_names
                                .iter()
                                .any(|n| s.file_names.iter().any(|f| f.eq_ignore_ascii_case(n)))
                    })
                })
            {
                return Err(invalid(
                    "ID or filename conflicts with another installed extension",
                ));
            }
        }
        // Capability aliases cannot be redefined as canonical extension capabilities.
        let mut caps = self.resources.capabilities();
        for cap in p.recognition.iter().flat_map(|r| &r.capabilities) {
            caps.register(cap, "extension")
                .map_err(|e| invalid(&e.to_string()))?;
        }
        Ok(())
    }
    pub(super) fn dispatch_extension(
        &mut self,
        method: &str,
        params: &Value,
    ) -> Result<Value, ProtocolError> {
        if matches!(method, "extension.preferences" | "extension.invoke") {
            return self.dispatch_workbench(method, params);
        }
        if method == "extension.list" {
            let mut list = vec![];
            for (p, digest, enabled) in self.extension_packages(false)? {
                let (source,previous,installed,updated):(String,Option<String>,String,String)=self.registry.db.conn.query_row(
                    "SELECT source,previous_digest,installed_at,updated_at FROM extension_packages WHERE id=?1",[&p.id],|r|Ok((r.get(0)?,r.get(1)?,r.get(2)?,r.get(3)?))).map_err(sql_err)?;
                list.push(json!({"package":p,"digest":digest,"enabled":enabled,"source":source,"can_rollback":previous.is_some(),"installed_at":installed,"updated_at":updated}));
            }
            return Ok(json!(list));
        }
        if method == "extension.preview" || method == "extension.install" {
            let p = ExtensionPackage::parse(params["package"].to_string().as_bytes())
                .map_err(|e| invalid(&e))?;
            let digest = hash(&p)?;
            let compatible = p.compatible(std::env::consts::OS, env!("CARGO_PKG_VERSION"));
            let conflict = self.check_extension_conflicts(&p).err().map(|e| e.message);
            let old = self
                .extension_packages(false)?
                .into_iter()
                .find(|(old, _, _)| old.id == p.id);
            if method == "extension.preview" {
                let dependencies = p
                    .skills
                    .iter()
                    .map(|s| {
                        self.preview_skill(
                            &json!({"manifest":s.manifest,"instructions":s.instructions}),
                        )
                    })
                    .collect::<Result<Vec<_>, _>>()?;
                return Ok(
                    json!({"package":p,"digest":digest,"compatible":compatible,"conflict":conflict,
                    "installed_digest":old.as_ref().map(|(_,d,_)|d),"dependencies":dependencies,
                    "permissions":p.permissions,"instruction_authority":false}),
                );
            }
            self.require_controller()?;
            self.extension_idle()?;
            if !compatible {
                return Err(invalid(
                    "extension platform, Skill or minimum ToolHub version incompatible",
                ));
            }
            if let Some(conflict) = conflict {
                return Err(invalid(&conflict));
            }
            if params["digest"].as_str() != Some(digest.as_str()) {
                return Err(invalid("extension preview digest changed"));
            }
            if let Some((old, old_digest, _)) = &old {
                if params["expected_digest"].as_str() != Some(old_digest.as_str()) {
                    return Err(invalid(
                        "installed extension changed; refresh before update",
                    ));
                }
                if old_digest == &digest {
                    return Ok(json!({"status":"already_installed","id":p.id}));
                }
                if toolhub_core::compare_versions(&p.version, &old.version)
                    .map_err(|_| invalid("invalid version"))?
                    != std::cmp::Ordering::Greater
                {
                    return Err(invalid(
                        "updates require a newer version; use rollback for previous version",
                    ));
                }
            } else if !params["expected_digest"].is_null() {
                return Err(invalid("extension was uninstalled; preview again"));
            }
            if old.is_none() && self.extension_packages(false)?.len() >= 32 {
                return Err(invalid("32 installed extensions limit reached"));
            }
            let source = params["source"].as_str().unwrap_or("local");
            if !["builtin", "local"].contains(&source) {
                return Err(invalid(
                    "source must be builtin or local (publisher identity is unverified)",
                ));
            }
            let enabled = old.as_ref().map(|(_, _, e)| *e).unwrap_or(true);
            self.save_extension(&p, &digest, source, enabled, old.as_ref(), false)?;
            return Ok(
                json!({"status":if old.is_some(){"updated"}else{"installed"},"id":p.id,"enabled":enabled,"rescan_required":true}),
            );
        }
        self.require_controller()?;
        self.extension_idle()?;
        let id = text(params, "id")?;
        let old = self
            .extension_packages(false)?
            .into_iter()
            .find(|(p, _, _)| p.id == id)
            .ok_or_else(|| ProtocolError::new(ErrorCode::NotFound, "extension not installed"))?;
        if params["expected_digest"].as_str() != Some(old.1.as_str()) {
            return Err(invalid("installed extension changed; refresh first"));
        }
        match method {
            "extension.set_enabled" => {
                let enabled = params["enabled"]
                    .as_bool()
                    .ok_or_else(|| invalid("enabled must be boolean"))?;
                if enabled {
                    self.check_extension_conflicts(&old.0)?;
                } else {
                    self.check_workbench_dependents(id)?;
                }
                self.save_extension(&old.0, &old.1, "local", enabled, Some(&old), true)?;
                Ok(json!({"enabled":enabled,"id":id,"rescan_required":enabled}))
            }
            "extension.rollback" => {
                let (previous, previous_digest): (Option<String>, Option<String>) = self
                    .registry
                    .db
                    .conn
                    .query_row(
                        "SELECT previous_json,previous_digest FROM extension_packages WHERE id=?1",
                        [id],
                        |r| Ok((r.get(0)?, r.get(1)?)),
                    )
                    .map_err(sql_err)?;
                let p = ExtensionPackage::parse(
                    previous
                        .ok_or_else(|| invalid("no previous version"))?
                        .as_bytes(),
                )
                .map_err(|e| invalid(&e))?;
                if previous_digest.as_deref() != Some(hash(&p)?.as_str()) {
                    return Err(invalid("previous extension snapshot integrity failure"));
                }
                if !p.compatible(std::env::consts::OS, env!("CARGO_PKG_VERSION")) {
                    return Err(invalid("previous version incompatible"));
                }
                self.check_extension_conflicts(&p)?;
                self.save_extension(&p, &hash(&p)?, "local", old.2, Some(&old), false)?;
                Ok(
                    json!({"status":"rolled_back","id":id,"version":p.version,"rescan_required":true}),
                )
            }
            "extension.remove" => {
                self.check_workbench_dependents(id)?;
                if old.0.host_module.as_deref() == Some("artifacts") {
                    self.pause_artifact_extension()?;
                }
                let tx = self.registry.db.conn.transaction().map_err(sql_err)?;
                tx.execute("UPDATE tool_instances SET status='extension_disabled' WHERE status='available' AND definition_id IN (SELECT definition_id FROM extension_definitions WHERE extension_id=?1)",[id]).map_err(sql_err)?;
                tx.execute("DELETE FROM extension_packages WHERE id=?1", [id])
                    .map_err(sql_err)?;
                tx.execute(
                    "UPDATE authorization_revision SET revision=revision+1 WHERE id=1",
                    [],
                )
                .map_err(sql_err)?;
                tx.commit().map_err(sql_err)?;
                self.event(
                    "extension.removed",
                    "declarative extension removed; actual files and user records retained",
                    json!({"id":id}),
                )?;
                Ok(json!({"removed":true,"files_retained":true}))
            }
            _ => Err(invalid("unknown extension method")),
        }
    }
    fn extension_idle(&self) -> Result<(), ProtocolError> {
        if !self.running_scans.is_empty() || !self.running_jobs.is_empty() {
            return Err(ProtocolError::new(
                ErrorCode::Unavailable,
                "scan or execution in progress; finish or cancel before changing extensions",
            ));
        }
        Ok(())
    }
    fn save_extension(
        &mut self,
        p: &ExtensionPackage,
        digest: &str,
        source: &str,
        enabled: bool,
        old: Option<&StoredPackage>,
        state_only: bool,
    ) -> Result<(), ProtocolError> {
        if p.host_module.as_deref() == Some("artifacts") && !enabled {
            self.pause_artifact_extension()?;
        }
        let rules = p.rules();
        let previous_rules = old.map(|(o, _, _)| o.rules()).unwrap_or_default();
        let restore = rules
            .iter()
            .filter(|r| {
                previous_rules
                    .iter()
                    .any(|o| serde_json::to_value(o).ok() == serde_json::to_value(r).ok())
            })
            .map(|r| r.definition_id.clone())
            .collect::<Vec<_>>();
        let mut query=self.registry.db.conn.prepare("SELECT id,definition_id,path,status FROM tool_instances WHERE definition_id IN (SELECT definition_id FROM extension_definitions WHERE extension_id=?1)").map_err(sql_err)?;
        let instances = query
            .query_map([&p.id], |r| {
                Ok((
                    r.get::<_, String>(0)?,
                    r.get::<_, String>(1)?,
                    r.get::<_, String>(2)?,
                    r.get::<_, String>(3)?,
                ))
            })
            .map_err(sql_err)?
            .collect::<Result<Vec<_>, _>>()
            .map_err(sql_err)?;
        drop(query);
        let tx = self.registry.db.conn.transaction().map_err(sql_err)?;
        let now = chrono::Utc::now().to_rfc3339();
        if state_only {
            tx.execute(
                "UPDATE extension_packages SET enabled=?2,updated_at=?3 WHERE id=?1",
                rusqlite::params![p.id, enabled, now],
            )
            .map_err(sql_err)?;
        } else {
            tx.execute("INSERT INTO extension_packages(id,package_json,digest,source,enabled,previous_json,previous_digest,installed_at,updated_at) VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?8) ON CONFLICT(id) DO UPDATE SET package_json=excluded.package_json,digest=excluded.digest,source=excluded.source,enabled=excluded.enabled,previous_json=excluded.previous_json,previous_digest=excluded.previous_digest,updated_at=excluded.updated_at",
                rusqlite::params![p.id,serde_json::to_string(p).map_err(json_err)?,digest,source,enabled,old.map(|(o,_,_)|serde_json::to_string(o)).transpose().map_err(json_err)?,old.map(|(_,d,_)|d),now]).map_err(sql_err)?;
        }
        tx.execute("UPDATE tool_instances SET status='extension_disabled' WHERE status='available' AND definition_id IN (SELECT definition_id FROM extension_definitions WHERE extension_id=?1)",[&p.id]).map_err(sql_err)?;
        for rule in &rules {
            tx.execute("INSERT INTO extension_definitions(definition_id,extension_id) VALUES(?1,?2) ON CONFLICT DO NOTHING",rusqlite::params![rule.definition_id,p.id]).map_err(sql_err)?;
            if !restore.contains(&rule.definition_id) {
                tx.execute(
                    "DELETE FROM tool_capabilities WHERE definition_id=?1",
                    [&rule.definition_id],
                )
                .map_err(sql_err)?;
            }
        }
        if enabled {
            for (id, definition, path, status) in &instances {
                if restore.contains(definition)
                    && ["available", "extension_disabled"].contains(&status.as_str())
                {
                    tx.execute(
                        "UPDATE tool_instances SET status=?2 WHERE id=?1",
                        rusqlite::params![
                            id,
                            if Path::new(path).is_file() {
                                "available"
                            } else {
                                "missing"
                            }
                        ],
                    )
                    .map_err(sql_err)?;
                }
            }
        }
        for cap in p.recognition.iter().flat_map(|r| &r.capabilities) {
            tx.execute("INSERT INTO capabilities(id,description,is_alias,canonical_id) VALUES(?1,'extension capability',0,NULL) ON CONFLICT DO NOTHING",[cap]).map_err(sql_err)?;
        }
        tx.execute(
            "UPDATE authorization_revision SET revision=revision+1 WHERE id=1",
            [],
        )
        .map_err(sql_err)?;
        tx.commit().map_err(sql_err)?;
        self.event(
            "extension.changed",
            "declarative extension snapshot changed",
            json!({"id":p.id,"version":p.version,"digest":digest,"enabled":enabled}),
        )?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn example() -> Value {
        serde_json::from_str(include_str!(
            "../../../extensions/examples/blender.toolhub-extension.json"
        ))
        .unwrap()
    }
    fn call(
        s: &mut DaemonService,
        name: &str,
        p: Value,
        admin: bool,
    ) -> Result<Value, toolhub_protocol::JsonRpcErrorObject> {
        let r = s.handle_authenticated(&JsonRpcRequest::new(1, name, p), "fixture", admin);
        match r.error {
            Some(e) => Err(e),
            None => Ok(r.result.unwrap()),
        }
    }
    fn install(s: &mut DaemonService, p: Value, old: Option<&str>) -> Value {
        let view = call(s, "extension.preview", json!({"package":p}), false).unwrap();
        let mut params = json!({"package":view["package"],"digest":view["digest"]});
        if let Some(old) = old {
            params["expected_digest"] = json!(old);
        }
        call(s, "extension.install", params, true).unwrap();
        view
    }
    #[test]
    fn lifecycle_reaches_shared_queries_and_preserves_files_across_restart() {
        let temp = tempfile::tempdir().unwrap();
        let db = temp.path().join("registry.sqlite");
        let binary = temp.path().join(if cfg!(windows) {
            "blender.exe"
        } else {
            "blender"
        });
        std::fs::write(&binary, b"fixture; never execute").unwrap();
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            std::fs::set_permissions(&binary, std::fs::Permissions::from_mode(0o755)).unwrap();
        }
        let mut s = DaemonService::open(&db).unwrap();
        let preview = install(&mut s, example(), None);
        let digest = preview["digest"].as_str().unwrap();
        let report = toolhub_scanner::run_scan(
            toolhub_scanner::ScanMode::Quick,
            Some(vec![temp.path().to_string_lossy().into_owned()]),
        );
        assert!(report
            .candidates
            .iter()
            .any(|c| c.path == binary.to_string_lossy()));
        s.ingest_scan(&report, "custom").unwrap();
        let rows = call(&mut s, "registry.search", json!({"query":"Blender"}), false).unwrap();
        assert_eq!(rows.as_array().unwrap().len(), 1);
        let id = rows[0]["id"].as_str().unwrap().to_owned();
        assert_eq!(parse_trust(rows[0]["trust"].as_str().unwrap()), "unknown");
        let v = call(
            &mut s,
            "skill.inspect",
            json!({"id":"community.blender.render"}),
            false,
        )
        .unwrap();
        assert!(v["instructions"]
            .as_str()
            .unwrap()
            .contains("--disable-autoexec"));
        assert_eq!(v["extension_id"], "community.blender");
        call(
            &mut s,
            "registry.correct",
            json!({"id":id,"trust":"user_trusted"}),
            true,
        )
        .unwrap();
        let v = call(
            &mut s,
            "resolve.capability",
            json!({"capability":"graphics.scene.render"}),
            false,
        )
        .unwrap();
        assert_eq!(v["selected"]["instance_id"], id);
        let revision = s.registry.revision().unwrap();
        call(
            &mut s,
            "extension.set_enabled",
            json!({"id":"community.blender","enabled":false,"expected_digest":digest}),
            true,
        )
        .unwrap();
        assert!(s.registry.revision().unwrap() > revision);
        assert!(call(
            &mut s,
            "skill.inspect",
            json!({"id":"community.blender.render"}),
            false
        )
        .is_err());
        assert_eq!(
            call(&mut s, "registry.search", json!({"query":"Blender"}), false).unwrap(),
            json!([])
        );
        assert!(call(
            &mut s,
            "execute.approval_request",
            json!({"instance_id":id,"args":[],"cwd":temp.path()}),
            false
        )
        .is_err());
        drop(s);
        let mut s = DaemonService::open(&db).unwrap();
        assert_eq!(
            call(&mut s, "extension.list", json!({}), false).unwrap()[0]["enabled"],
            false
        );
        assert_eq!(
            s.registry.get_instance(&id).unwrap().unwrap().status,
            "extension_disabled"
        );
        call(
            &mut s,
            "extension.set_enabled",
            json!({"id":"community.blender","enabled":true,"expected_digest":digest}),
            true,
        )
        .unwrap();
        assert!(call(
            &mut s,
            "resolve.capability",
            json!({"capability":"graphics.scene.render"}),
            false
        )
        .unwrap()["selected"]
            .is_object());
        // Keep an ordinary user's Skill intact when the extension is uninstalled.
        s.registry.upsert_skill("personal.notes","Notes","toolhub.skill/v1","instruction",r#"{"schema":"toolhub.skill/v1","id":"personal.notes","name":"Notes","kind":"instruction","instruction_file":"SKILL.md"}"#,None).unwrap();
        call(
            &mut s,
            "extension.remove",
            json!({"id":"community.blender","expected_digest":digest}),
            true,
        )
        .unwrap();
        assert!(binary.is_file());
        assert!(s.registry.get_instance(&id).unwrap().is_some());
        assert_eq!(s.registry.list_skills().unwrap().len(), 1);
        assert_eq!(
            call(&mut s, "extension.list", json!({}), false)
                .unwrap()
                .as_array()
                .unwrap()
                .iter()
                .filter(|r| r["package"]["host_module"].is_null())
                .count(),
            0
        );
        assert_eq!(
            call(&mut s, "registry.search", json!({"query":"Blender"}), false).unwrap(),
            json!([])
        );
    }
    #[test]
    fn extension_does_not_reclassify_existing_user_record() {
        let temp = tempfile::tempdir().unwrap();
        let mut s = DaemonService::open(&temp.path().join("r.sqlite")).unwrap();
        let binary = temp.path().join(if cfg!(windows) {
            "blender.exe"
        } else {
            "blender"
        });
        std::fs::write(&binary, b"fixture").unwrap();
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            std::fs::set_permissions(&binary, std::fs::Permissions::from_mode(0o755)).unwrap();
        }
        let path = binary.to_string_lossy().to_string();
        let id = toolhub_recognizer::stable_instance_id(&path);
        let input = UpsertInstanceInput {
            id: id.clone(),
            definition_id: "personal.blender".into(),
            definition_name: "User Blender".into(),
            version: Some("4.0".into()),
            platform: "windows".into(),
            arch: "x86_64".into(),
            path: path.clone(),
            canonical_path: Some(path),
            environment_id: None,
            origin_json: json!({"type":"unknown"}).to_string(),
            owner_json: json!({}).to_string(),
            trust_json: json!({"level":"user_trusted"}).to_string(),
            status: "available".into(),
            capabilities: vec![],
        };
        s.registry.upsert_instance(&input).unwrap();
        install(&mut s, example(), None);
        let report = toolhub_scanner::run_scan(
            toolhub_scanner::ScanMode::Quick,
            Some(vec![temp.path().to_string_lossy().into_owned()]),
        );
        s.ingest_scan(&report, "custom").unwrap();
        let row = s.registry.get_instance(&id).unwrap().unwrap();
        assert_eq!(row.definition_id, "personal.blender");
        assert_eq!(row.name, "User Blender");
        assert_eq!(row.status, "available");
        assert_eq!(parse_trust(&row.trust), "user_trusted");
    }
    #[test]
    fn controller_digest_conflicts_and_atomic_update_rollback() {
        let temp = tempfile::tempdir().unwrap();
        let mut s = DaemonService::open(&temp.path().join("r.sqlite")).unwrap();
        let p = example();
        let preview = call(&mut s, "extension.preview", json!({"package":p}), false).unwrap();
        assert!(call(
            &mut s,
            "extension.install",
            json!({"package":p,"digest":preview["digest"]}),
            false
        )
        .is_err());
        assert!(call(
            &mut s,
            "extension.install",
            json!({"package":p,"digest":"bad"}),
            true
        )
        .is_err());
        let v = install(&mut s, p.clone(), None);
        let old = v["digest"].as_str().unwrap();
        let mut newer = p.clone();
        newer["version"] = json!("1.1.0");
        newer["recognition"]["version"] = json!("1.1.0");
        newer["recognition"]["capabilities"] =
            json!(["graphics.scene.render", "graphics.scene.animate"]);
        newer["recognition"]["tools"][0]["capabilities"] = json!(["graphics.scene.animate"]);
        let nv = call(&mut s, "extension.preview", json!({"package":newer}), false).unwrap();
        assert!(call(
            &mut s,
            "extension.install",
            json!({"package":newer,"digest":nv["digest"],"expected_digest":"stale"}),
            true
        )
        .is_err());
        s.registry.db.conn.execute_batch("CREATE TRIGGER extension_abort BEFORE INSERT ON capabilities WHEN NEW.id='graphics.scene.animate' BEGIN SELECT RAISE(ABORT,'fixture failed write'); END;").unwrap();
        assert!(call(
            &mut s,
            "extension.install",
            json!({"package":newer,"digest":nv["digest"],"expected_digest":old}),
            true
        )
        .is_err());
        assert_eq!(
            call(&mut s, "extension.list", json!({}), false).unwrap()[0]["digest"],
            old
        );
        s.registry
            .db
            .conn
            .execute_batch("DROP TRIGGER extension_abort;")
            .unwrap();
        let updated = install(&mut s, newer, Some(old));
        s.running_scans.insert(
            "fixture".into(),
            ("fixture".into(), Arc::new(AtomicBool::new(false))),
        );
        assert!(call(
            &mut s,
            "extension.set_enabled",
            json!({"id":"community.blender","expected_digest":updated["digest"],"enabled":false}),
            true
        )
        .is_err());
        s.running_scans.clear();
        s.registry
            .db
            .conn
            .execute(
                "UPDATE extension_packages SET previous_digest='tampered'",
                [],
            )
            .unwrap();
        assert!(call(
            &mut s,
            "extension.rollback",
            json!({"id":"community.blender","expected_digest":updated["digest"]}),
            true
        )
        .is_err());
        s.registry
            .db
            .conn
            .execute("UPDATE extension_packages SET previous_digest=?1", [old])
            .unwrap();
        call(
            &mut s,
            "extension.rollback",
            json!({"id":"community.blender","expected_digest":updated["digest"]}),
            true,
        )
        .unwrap();
        assert_eq!(
            call(&mut s, "extension.list", json!({}), false).unwrap()[0]["digest"],
            old
        );
        let mut collision = p;
        collision["id"] = json!("community.other");
        collision["recognition"]["tools"][0]["definition_id"] = json!("community.other.cli");
        collision["skills"][0]["manifest"]["id"] = json!("community.other.render");
        assert!(call(
            &mut s,
            "extension.preview",
            json!({"package":collision}),
            false
        )
        .unwrap()["conflict"]
            .is_string());
        assert!(call(
            &mut s,
            "extension.remove",
            json!({"id":"community.blender","expected_digest":old,"path":"untrusted"}),
            true
        )
        .is_err());
    }
}
