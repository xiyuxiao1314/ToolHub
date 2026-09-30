use rusqlite::{params, OptionalExtension, Row};

use crate::db::RegistryDb;
use crate::models::{
    CandidateRow, InstanceRow, ReconcileScope, ScanInstanceInput, UpsertInstanceInput,
};
use crate::RegistryResult;

pub struct Registry {
    pub db: RegistryDb,
}

impl Registry {
    pub fn import_report(
        &mut self,
        environments: &[toolhub_core::Environment],
        instances: &[UpsertInstanceInput],
    ) -> RegistryResult<Vec<String>> {
        if instances.len() > 10000 || environments.len() > 1000 {
            return Err(crate::RegistryError::Validation(
                "report exceeds import limits".into(),
            ));
        }
        let mut imported_envs = vec![];
        let mut env_ids = std::collections::BTreeMap::new();
        for env in environments {
            env_ids.insert(
                env.id.as_str().to_string(),
                toolhub_core::EnvironmentId::new(format!(
                    "env.import.{}",
                    &toolhub_core::path_fingerprint(env.id.as_str())[..20]
                ))
                .map_err(|e| crate::RegistryError::Validation(e.to_string()))?,
            );
        }
        for env in environments {
            let mut env = env.clone();
            let source_id = env.id.as_str().to_string();
            env.id = env_ids[&source_id].clone();
            env.parent_id = env
                .parent_id
                .map(|id| {
                    env_ids.get(id.as_str()).cloned().ok_or_else(|| {
                        crate::RegistryError::Validation("import environment parent missing".into())
                    })
                })
                .transpose()?;
            env.owner = toolhub_core::Owner::unknown();
            env.origin = toolhub_core::Origin::Unknown;
            env.labels.push("imported metadata; size unknown".into());
            imported_envs.push(env);
        }
        let mut normalized = vec![];
        let mut seen = std::collections::BTreeSet::new();
        let mut retained = vec![];
        for input in instances {
            if input.path.is_empty()
                || input.path.len() > 4096
                || input.definition_name.trim().is_empty()
                || input.definition_name.len() > 256
                || toolhub_core::DefinitionId::new(&input.definition_id).is_err()
                || input
                    .version
                    .as_deref()
                    .is_some_and(|v| toolhub_core::compare_versions(v, v).is_err())
                || !seen.insert(identity_key(&input.path))
            {
                return Err(crate::RegistryError::Validation(
                    "invalid or duplicate imported instance".into(),
                ));
            }
            if let Some(existing) = self.find_instance_by_path(&input.path)? {
                if self
                    .get_instance(&existing)?
                    .is_some_and(|i| i.status == "available")
                {
                    retained.push(existing);
                    continue;
                }
            }
            let mut imported = input.clone();
            if !imported.id.starts_with("import") {
                imported.id = format!("imported-{}", imported.id)
            }
            imported.canonical_path = None;
            imported.environment_id = imported
                .environment_id
                .map(|id| {
                    env_ids
                        .get(&id)
                        .map(|id| id.as_str().to_string())
                        .ok_or_else(|| {
                            crate::RegistryError::Validation("import environment missing".into())
                        })
                })
                .transpose()?;
            imported.origin_json = serde_json::to_string(&toolhub_core::Origin::Unknown)?;
            imported.owner_json = serde_json::to_string(&toolhub_core::Owner::unknown())?;
            imported.trust_json = serde_json::to_string(&toolhub_core::TrustRecord::unknown())?;
            imported.status = "missing".into();
            imported.capabilities.clear();
            normalized.push(ScanInstanceInput {
                instance: imported,
                evidence: vec![toolhub_core::Evidence {
                    source: toolhub_core::EvidenceSource::Filesystem,
                    provenance: toolhub_core::Provenance::Imported,
                    confidence: 0.0,
                    summary: "external report metadata; no native identity or execution trust"
                        .into(),
                    detail: None,
                    observed_at: chrono::Utc::now(),
                }],
                interfaces: vec![],
                provider: "report".into(),
                root: "imported metadata".into(),
            });
        }
        let mut ids = self.ingest_scan(&imported_envs, &normalized, &[], &[])?;
        ids.extend(retained);
        Ok(ids)
    }
    /// One transaction owns dependencies, identity resolution, relations and scoped reconciliation.
    pub fn ingest_scan(
        &mut self,
        environments: &[toolhub_core::Environment],
        instances: &[ScanInstanceInput],
        candidates: &[toolhub_core::ScanCandidate],
        scopes: &[ReconcileScope],
    ) -> RegistryResult<Vec<String>> {
        let tx = self.db.conn.transaction()?;
        let mut pending: Vec<_> = environments.iter().collect();
        while !pending.is_empty() {
            let before = pending.len();
            let mut next = vec![];
            for env in pending {
                if let Some(parent) = &env.parent_id {
                    let exists: bool = tx.query_row(
                        "SELECT EXISTS(SELECT 1 FROM environments WHERE id=?1)",
                        [parent.as_str()],
                        |r| r.get(0),
                    )?;
                    if !exists {
                        next.push(env);
                        continue;
                    }
                }
                tx.execute("INSERT INTO environments(id,name,kind,root_path,parent_id,origin_json,owner_json,labels_json) VALUES(?1,?2,?3,?4,?5,?6,?7,?8) ON CONFLICT(id) DO UPDATE SET name=excluded.name,kind=excluded.kind,root_path=excluded.root_path,parent_id=excluded.parent_id,origin_json=excluded.origin_json,labels_json=excluded.labels_json",params![env.id.as_str(),env.name,env.kind.as_str(),env.root_path,env.parent_id.as_ref().map(|p|p.as_str()),serde_json::to_string(&env.origin)?,serde_json::to_string(&env.owner)?,serde_json::to_string(&env.labels)?])?;
            }
            if next.len() == before {
                return Err(crate::RegistryError::Validation(
                    "environment hierarchy has missing parents or a cycle".into(),
                ));
            }
            pending = next;
        }
        let now = chrono::Utc::now().to_rfc3339();
        let mut ids = vec![];
        for input in instances {
            let id = Self::upsert_instance_on(&tx, &input.instance)?;
            for (index, interface) in input.interfaces.iter().enumerate() {
                tx.execute("INSERT INTO interfaces(id,instance_id,kind,executable) VALUES(?1,?2,?3,?4) ON CONFLICT(id) DO UPDATE SET instance_id=excluded.instance_id,kind=excluded.kind,executable=excluded.executable",params![format!("{id}-interface-{index}"),id,interface.kind,interface.executable])?;
            }
            for evidence in &input.evidence {
                evidence
                    .validate()
                    .map_err(|e| crate::RegistryError::Validation(e.to_string()))?;
                tx.execute("INSERT INTO evidence(entity_kind,entity_id,source,provenance,confidence,summary,detail,observed_at) VALUES('instance',?1,?2,?3,?4,?5,?6,?7)",params![id,serde_json::to_value(&evidence.source)?.as_str().unwrap_or("unknown"),serde_json::to_value(&evidence.provenance)?.as_str().unwrap_or("unknown"),evidence.confidence,evidence.summary,evidence.detail,evidence.observed_at.to_rfc3339()])?;
            }
            tx.execute(
                "INSERT OR IGNORE INTO scan_membership(instance_id,provider,root) VALUES(?1,?2,?3)",
                params![id, input.provider, identity_key(&input.root)],
            )?;
            ids.push(id);
        }
        for candidate in candidates {
            let key = identity_key(
                candidate
                    .canonical_path
                    .as_deref()
                    .unwrap_or(&candidate.path),
            );
            let mut query = tx.prepare("SELECT id,path,canonical_path FROM scan_candidates")?;
            let old = query
                .query_map([], |r| {
                    Ok((
                        r.get::<_, String>(0)?,
                        r.get::<_, String>(1)?,
                        r.get::<_, Option<String>>(2)?,
                    ))
                })?
                .collect::<Result<Vec<_>, _>>()?;
            let id = old
                .into_iter()
                .find(|(_, p, c)| identity_key(c.as_deref().unwrap_or(p)) == key)
                .map(|r| r.0)
                .unwrap_or_else(|| candidate.id.clone());
            drop(query);
            let recognized = instances.iter().any(|i| {
                identity_key(
                    i.instance
                        .canonical_path
                        .as_deref()
                        .unwrap_or(&i.instance.path),
                ) == key
            });
            tx.execute("INSERT INTO scan_candidates(id,path,canonical_path,file_name,size_bytes,sha256,version_hint,recognized,metadata_json,discovered_at) VALUES(?1,?2,?3,?4,?5,?6,?7,?8,?9,?10) ON CONFLICT(id) DO UPDATE SET canonical_path=excluded.canonical_path,file_name=excluded.file_name,size_bytes=excluded.size_bytes,sha256=excluded.sha256,version_hint=excluded.version_hint,recognized=excluded.recognized,metadata_json=excluded.metadata_json,discovered_at=excluded.discovered_at",params![id,candidate.path,candidate.canonical_path,candidate.file_name,candidate.size_bytes.map(|s|s as i64),candidate.sha256,candidate.version_hint,recognized as i32,serde_json::to_string(&candidate.metadata)?,now])?;
        }
        // If this scan saw a path but did not recognize it as a tool executable
        // (e.g. uninstall InstallLocation folder), retire any stale instance on that path.
        let mut seen_not_tool: std::collections::HashSet<String> = std::collections::HashSet::new();
        for candidate in candidates {
            let key = identity_key(
                candidate
                    .canonical_path
                    .as_deref()
                    .unwrap_or(&candidate.path),
            );
            let recognized = instances.iter().any(|i| {
                identity_key(
                    i.instance
                        .canonical_path
                        .as_deref()
                        .unwrap_or(&i.instance.path),
                ) == key
            });
            if !recognized {
                seen_not_tool.insert(key);
            }
        }
        if !seen_not_tool.is_empty() {
            let mut stmt = tx.prepare("SELECT id, path, canonical_path FROM tool_instances WHERE status='available'")?;
            let stale: Vec<(String, String, Option<String>)> = stmt
                .query_map([], |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)))?
                .collect::<Result<Vec<_>, _>>()?;
            drop(stmt);
            for (id, path, canonical) in stale {
                let key = identity_key(canonical.as_deref().unwrap_or(&path));
                if seen_not_tool.contains(&key) && !ids.contains(&id) {
                    tx.execute(
                        "UPDATE tool_instances SET status='missing' WHERE id=?1",
                        [&id],
                    )?;
                }
            }
        }
        for scope in scopes.iter().filter(|s| s.complete) {
            let mut query=tx.prepare("SELECT m.instance_id FROM scan_membership m JOIN tool_instances i ON i.id=m.instance_id WHERE m.provider=?1 AND m.root=?2 AND i.status='available'")?;
            let members = query
                .query_map(params![scope.provider, identity_key(&scope.root)], |r| {
                    r.get::<_, String>(0)
                })?
                .collect::<Result<Vec<_>, _>>()?;
            drop(query);
            for member in members {
                if !ids.contains(&member) {
                    tx.execute(
                        "UPDATE tool_instances SET status='missing' WHERE id=?1",
                        [&member],
                    )?;
                }
            }
        }
        tx.execute(
            "UPDATE authorization_revision SET revision=revision+1 WHERE id=1",
            [],
        )?;
        tx.commit()?;
        Ok(ids)
    }

    pub fn load_environment_graph(&mut self) -> RegistryResult<Vec<toolhub_core::Environment>> {
        let mut query=self.db.conn.prepare("SELECT id,name,kind,root_path,parent_id,origin_json,owner_json,labels_json FROM environments ORDER BY id")?;
        let rows = query
            .query_map([], |r| {
                Ok((
                    r.get::<_, String>(0)?,
                    r.get::<_, String>(1)?,
                    r.get::<_, String>(2)?,
                    r.get::<_, Option<String>>(3)?,
                    r.get::<_, Option<String>>(4)?,
                    r.get::<_, String>(5)?,
                    r.get::<_, String>(6)?,
                    r.get::<_, String>(7)?,
                ))
            })?
            .collect::<Result<Vec<_>, _>>()?;
        rows.into_iter().map(|(id,name,kind,root,parent,origin,owner,labels)| Ok(serde_json::from_value(serde_json::json!({"id":id,"name":name,"kind":kind,"root_path":root,"parent_id":parent,"origin":serde_json::from_str::<serde_json::Value>(&origin)?,"owner":serde_json::from_str::<serde_json::Value>(&owner)?,"labels":serde_json::from_str::<serde_json::Value>(&labels)?}))?)).collect()
    }

    pub fn get_setting(&mut self, key: &str) -> RegistryResult<Option<serde_json::Value>> {
        let raw: Option<String> = self
            .db
            .conn
            .query_row("SELECT value_json FROM settings WHERE key=?1", [key], |r| {
                r.get(0)
            })
            .optional()?;
        raw.map(|s| serde_json::from_str(&s).map_err(Into::into))
            .transpose()
    }
    pub fn set_setting(&mut self, key: &str, value: &serde_json::Value) -> RegistryResult<()> {
        self.db.conn.execute("INSERT INTO settings(key,value_json) VALUES(?1,?2) ON CONFLICT(key) DO UPDATE SET value_json=excluded.value_json",params![key,serde_json::to_string(value)?])?;
        Ok(())
    }
    pub fn revision(&mut self) -> RegistryResult<u64> {
        Ok(self.db.conn.query_row(
            "SELECT revision FROM authorization_revision WHERE id=1",
            [],
            |r| r.get::<_, i64>(0),
        )? as u64)
    }
    pub fn bump_revision(&mut self) -> RegistryResult<u64> {
        self.db.conn.execute(
            "UPDATE authorization_revision SET revision=revision+1 WHERE id=1",
            [],
        )?;
        self.revision()
    }
    pub fn open_path(path: &std::path::Path) -> RegistryResult<Self> {
        Ok(Self {
            db: crate::db::open_path(path)?,
        })
    }

    pub fn open_memory() -> RegistryResult<Self> {
        Ok(Self {
            db: crate::db::open_memory()?,
        })
    }

    pub fn ensure_capability(
        &mut self,
        id: &str,
        description: &str,
        is_alias: bool,
        canonical: Option<&str>,
    ) -> RegistryResult<()> {
        self.db.conn.execute(
            "INSERT INTO capabilities(id, description, is_alias, canonical_id)
             VALUES (?1, ?2, ?3, ?4)
             ON CONFLICT(id) DO UPDATE SET description=excluded.description",
            params![id, description, is_alias as i32, canonical],
        )?;
        Ok(())
    }

    /// F06: idempotent upsert keyed by (definition_id, path); preserves identity across rescans.
    /// Returns the persisted instance id (may differ from input.id after path identity).
    pub fn upsert_instance(&mut self, input: &UpsertInstanceInput) -> RegistryResult<String> {
        let tx = self.db.conn.transaction()?;
        let id = Self::upsert_instance_on(&tx, input)?;
        tx.commit()?;
        Ok(id)
    }

    fn upsert_instance_on(
        tx: &rusqlite::Connection,
        input: &UpsertInstanceInput,
    ) -> RegistryResult<String> {
        let now = chrono::Utc::now().to_rfc3339();
        tx.execute(
            "INSERT INTO tool_definitions(id, name, vendor, categories_json, description, created_at, updated_at)
             VALUES (?1, ?2, NULL, '[]', NULL, ?3, ?3)
             ON CONFLICT(id) DO UPDATE SET name=excluded.name, updated_at=excluded.updated_at",
            params![input.definition_id, input.definition_name, now],
        )?;
        // Resolve existing row by stable path identity
        let key = identity_key(input.canonical_path.as_deref().unwrap_or(&input.path));
        let mut query = tx.prepare(
            "SELECT id, path, canonical_path FROM tool_instances ORDER BY first_seen, id",
        )?;
        let rows = query
            .query_map([], |r| {
                Ok((
                    r.get::<_, String>(0)?,
                    r.get::<_, String>(1)?,
                    r.get::<_, Option<String>>(2)?,
                ))
            })?
            .collect::<Result<Vec<_>, _>>()?;
        let existing = rows
            .into_iter()
            .find(|(_, p, c)| identity_key(c.as_deref().unwrap_or(p)) == key)
            .map(|r| r.0);
        drop(query);
        let id = existing.unwrap_or_else(|| input.id.clone());
        // R2-B04: preserve blocked/user-corrected trust across rescans.
        let prev_trust: Option<String> = tx
            .query_row(
                "SELECT trust_json FROM tool_instances WHERE id = ?1",
                params![id],
                |r| r.get(0),
            )
            .optional()?;
        let trust_json = match prev_trust {
            Some(prev) => {
                let prev_level = serde_json::from_str::<serde_json::Value>(&prev)
                    .ok()
                    .and_then(|v| {
                        v.get("level")
                            .and_then(|l| l.as_str())
                            .map(|s| s.to_string())
                    });
                if prev_level.as_deref() == Some("blocked")
                    || prev_level.as_deref() == Some("user_trusted")
                    || has_user_decision(&prev)
                {
                    prev
                } else {
                    input.trust_json.clone()
                }
            }
            None => input.trust_json.clone(),
        };
        let previous_owner: Option<String> = tx
            .query_row(
                "SELECT owner_json FROM tool_instances WHERE id=?1",
                [&id],
                |r| r.get(0),
            )
            .optional()?;
        let owner_json = previous_owner
            .filter(|s| has_user_decision(s))
            .unwrap_or_else(|| input.owner_json.clone());
        tx.execute(
            "INSERT INTO tool_instances(
                id, definition_id, version, platform, arch, path, canonical_path,
                environment_id, origin_json, owner_json, trust_json, status, first_seen, last_seen)
             VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11,?12,?13,?13)
             ON CONFLICT(id) DO UPDATE SET
                definition_id=excluded.definition_id,
                version=excluded.version,
                platform=excluded.platform,
                arch=excluded.arch,
                canonical_path=excluded.canonical_path,
                environment_id=excluded.environment_id,
                origin_json=excluded.origin_json,
                owner_json=excluded.owner_json,
                trust_json=excluded.trust_json,
                status=excluded.status,
                last_seen=excluded.last_seen",
            params![
                id,
                input.definition_id,
                input.version,
                input.platform,
                input.arch,
                input.path,
                input.canonical_path,
                input.environment_id,
                input.origin_json,
                owner_json,
                trust_json,
                input.status,
                now
            ],
        )?;
        for cap in &input.capabilities {
            tx.execute(
                "INSERT INTO tool_capabilities(definition_id, capability_id) VALUES (?1, ?2)
                 ON CONFLICT DO NOTHING",
                params![input.definition_id, cap],
            )?;
        }
        Ok(id)
    }

    pub fn find_instance_by_path(&mut self, path: &str) -> RegistryResult<Option<String>> {
        let id = self
            .db
            .conn
            .query_row(
                "SELECT id FROM tool_instances WHERE path = ?1 OR canonical_path = ?1 LIMIT 1",
                params![path],
                |r| r.get::<_, String>(0),
            )
            .optional()?;
        Ok(id)
    }

    pub fn get_instance(&mut self, id: &str) -> RegistryResult<Option<InstanceRow>> {
        let row = self
            .db
            .conn
            .query_row(
                "SELECT i.id, i.definition_id, d.name, i.version, i.path, i.canonical_path,
                        i.environment_id, i.trust_json, i.status, i.arch, i.platform
                 FROM tool_instances i
                 JOIN tool_definitions d ON d.id = i.definition_id
                 WHERE i.id = ?1",
                params![id],
                |r| {
                    Ok(InstanceRow {
                        id: r.get(0)?,
                        definition_id: r.get(1)?,
                        name: r.get(2)?,
                        version: r.get(3)?,
                        path: r.get(4)?,
                        canonical_path: r.get(5)?,
                        environment_id: r.get(6)?,
                        trust: r.get(7)?,
                        status: r.get(8)?,
                        arch: r.get(9)?,
                        platform: r.get(10)?,
                    })
                },
            )
            .optional()?;
        Ok(row)
    }

    pub fn search(&mut self, query: &str) -> RegistryResult<Vec<InstanceRow>> {
        let like = format!("%{}%", query);
        let mut stmt = self.db.conn.prepare(
            "SELECT i.id, i.definition_id, d.name, i.version, i.path, i.canonical_path,
                    i.environment_id, i.trust_json, i.status, i.arch, i.platform
             FROM tool_instances i
             JOIN tool_definitions d ON d.id = i.definition_id
             WHERE i.status = 'available'
               AND (d.name LIKE ?1 OR i.path LIKE ?1 OR d.id LIKE ?1)
             ORDER BY d.name, i.path",
        )?;
        let rows = stmt
            .query_map(params![like], |r| {
                Ok(InstanceRow {
                    id: r.get(0)?,
                    definition_id: r.get(1)?,
                    name: r.get(2)?,
                    version: r.get(3)?,
                    path: r.get(4)?,
                    canonical_path: r.get(5)?,
                    environment_id: r.get(6)?,
                    trust: r.get(7)?,
                    status: r.get(8)?,
                    arch: r.get(9)?,
                    platform: r.get(10)?,
                })
            })?
            .collect::<Result<Vec<_>, _>>()?;
        Ok(rows)
    }

    pub fn list_instances(&mut self) -> RegistryResult<Vec<InstanceRow>> {
        self.search("")
    }

    pub fn capabilities_for_definition(
        &mut self,
        definition_id: &str,
    ) -> RegistryResult<Vec<String>> {
        let mut stmt = self
            .db
            .conn
            .prepare("SELECT capability_id FROM tool_capabilities WHERE definition_id = ?1")?;
        let rows = stmt
            .query_map(params![definition_id], |r| r.get::<_, String>(0))?
            .collect::<Result<Vec<_>, _>>()?;
        Ok(rows)
    }

    pub fn instances_for_capability(
        &mut self,
        capability_id: &str,
    ) -> RegistryResult<Vec<InstanceRow>> {
        let mut stmt = self.db.conn.prepare(
            "SELECT i.id, i.definition_id, d.name, i.version, i.path, i.canonical_path,
                    i.environment_id, i.trust_json, i.status, i.arch, i.platform
             FROM tool_instances i
             JOIN tool_definitions d ON d.id = i.definition_id
             JOIN tool_capabilities tc ON tc.definition_id = i.definition_id
             WHERE tc.capability_id = ?1 AND i.status = 'available'
             ORDER BY i.path",
        )?;
        let rows = stmt
            .query_map(params![capability_id], |r| {
                Ok(InstanceRow {
                    id: r.get(0)?,
                    definition_id: r.get(1)?,
                    name: r.get(2)?,
                    version: r.get(3)?,
                    path: r.get(4)?,
                    canonical_path: r.get(5)?,
                    environment_id: r.get(6)?,
                    trust: r.get(7)?,
                    status: r.get(8)?,
                    arch: r.get(9)?,
                    platform: r.get(10)?,
                })
            })?
            .collect::<Result<Vec<_>, _>>()?;
        Ok(rows)
    }

    /// F06: candidate identity is the normalized path; repeated scans do not grow rows.
    pub fn upsert_candidate(&mut self, c: &CandidateRow) -> RegistryResult<()> {
        let existing: Option<String> = self
            .db
            .conn
            .query_row(
                "SELECT id FROM scan_candidates WHERE path = ?1 LIMIT 1",
                params![c.path],
                |r| r.get(0),
            )
            .optional()?;
        let id = existing.unwrap_or_else(|| c.id.clone());
        self.db.conn.execute(
            "INSERT INTO scan_candidates(id, path, canonical_path, file_name, recognized, metadata_json, discovered_at)
             VALUES (?1, ?2, NULL, ?3, ?4, '{}', ?5)
             ON CONFLICT(id) DO UPDATE SET recognized=excluded.recognized, file_name=excluded.file_name",
            params![
                id,
                c.path,
                c.file_name,
                c.recognized as i32,
                chrono::Utc::now().to_rfc3339()
            ],
        )?;
        Ok(())
    }

    pub fn list_candidates(&mut self) -> RegistryResult<Vec<CandidateRow>> {
        let mut stmt = self
            .db
            .conn
            .prepare("SELECT id, path, recognized, file_name FROM scan_candidates ORDER BY path")?;
        let rows = stmt
            .query_map([], |r| {
                Ok(CandidateRow {
                    id: r.get(0)?,
                    path: r.get(1)?,
                    recognized: r.get::<_, i32>(2)? != 0,
                    file_name: r.get(3)?,
                })
            })?
            .collect::<Result<Vec<_>, _>>()?;
        Ok(rows)
    }

    pub fn record_activity(
        &mut self,
        kind: &str,
        summary: &str,
        agent: Option<&str>,
    ) -> RegistryResult<()> {
        self.db.conn.execute(
            "INSERT INTO activity(ts, kind, summary, agent, payload_json) VALUES (?1, ?2, ?3, ?4, '{}')",
            params![chrono::Utc::now().to_rfc3339(), kind, summary, agent],
        )?;
        Ok(())
    }

    #[allow(clippy::too_many_arguments)]
    pub fn insert_execution_record(
        &mut self,
        id: &str,
        agent_id: Option<&str>,
        instance_id: Option<&str>,
        capability: Option<&str>,
        executable: &str,
        args_redacted: &str,
        cwd_redacted: &str,
        duration_ms: u64,
        exit_code: Option<i32>,
        status: &str,
        approval_id: Option<&str>,
    ) -> RegistryResult<()> {
        self.db.conn.execute(
            "INSERT INTO execution_records(
                id, agent_id, instance_id, capability, executable, args_redacted, cwd_redacted,
                started_at, duration_ms, exit_code, status, approval_id)
             VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11,?12)",
            params![
                id,
                agent_id,
                instance_id,
                capability,
                executable,
                args_redacted,
                cwd_redacted,
                chrono::Utc::now().to_rfc3339(),
                duration_ms as i64,
                exit_code,
                status,
                approval_id
            ],
        )?;
        Ok(())
    }

    #[allow(clippy::too_many_arguments)]
    pub fn save_approval(
        &mut self,
        id: &str,
        agent_id: &str,
        session_id: &str,
        instance_id: &str,
        executable_sha256: &str,
        canonical_executable: &str,
        args_digest: &str,
        cwd_digest: &str,
        env_digest: &str,
        expires_at: &str,
    ) -> RegistryResult<()> {
        self.db.conn.execute(
            "INSERT INTO execution_approvals(
                id, agent_id, session_id, instance_id, executable_sha256, canonical_executable,
                args_digest, cwd_digest, env_digest, expires_at, consumed, revoked)
             VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,0,0)",
            params![
                id,
                agent_id,
                session_id,
                instance_id,
                executable_sha256,
                canonical_executable,
                args_digest,
                cwd_digest,
                env_digest,
                expires_at
            ],
        )?;
        Ok(())
    }

    pub fn consume_approval(&mut self, id: &str) -> RegistryResult<bool> {
        let n = self.db.conn.execute(
            "UPDATE execution_approvals SET consumed = 1 WHERE id = ?1 AND consumed = 0 AND revoked = 0",
            params![id],
        )?;
        Ok(n == 1)
    }

    pub fn revoke_approval(&mut self, id: &str) -> RegistryResult<()> {
        self.db.conn.execute(
            "UPDATE execution_approvals SET revoked = 1 WHERE id = ?1",
            params![id],
        )?;
        Ok(())
    }

    pub fn set_policy(&mut self, scope: &str, subject: &str, action: &str) -> RegistryResult<()> {
        let tx = self.db.conn.transaction()?;
        tx.execute(
            "DELETE FROM policy_rules WHERE scope = ?1 AND subject = ?2",
            params![scope, subject],
        )?;
        tx.execute(
            "INSERT INTO policy_rules(scope, subject, action, created_at) VALUES (?1,?2,?3,?4)",
            params![scope, subject, action, chrono::Utc::now().to_rfc3339()],
        )?;
        tx.execute(
            "UPDATE authorization_revision SET revision=revision+1 WHERE id=1",
            [],
        )?;
        tx.commit()?;
        Ok(())
    }

    pub fn list_policy(&mut self) -> RegistryResult<Vec<(String, String, String)>> {
        let mut stmt = self
            .db
            .conn
            .prepare("SELECT scope, subject, action FROM policy_rules ORDER BY scope, subject")?;
        let rows = stmt
            .query_map([], |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)))?
            .collect::<Result<Vec<_>, _>>()?;
        Ok(rows)
    }

    pub fn save_discovery_session(
        &mut self,
        id: &str,
        agent_id: &str,
        expires_at: &str,
        scopes_json: &str,
    ) -> RegistryResult<()> {
        self.db.conn.execute(
            "INSERT INTO discovery_sessions(id, agent_id, created_at, expires_at, revoked, scopes_json)
             VALUES (?1,?2,?3,?4,0,?5)",
            params![id, agent_id, chrono::Utc::now().to_rfc3339(), expires_at, scopes_json],
        )?;
        Ok(())
    }

    pub fn disclose_candidates(
        &mut self,
        session_id: &str,
        candidate_ids: &[String],
    ) -> RegistryResult<()> {
        let tx = self.db.conn.transaction()?;
        let valid:bool=tx.query_row("SELECT EXISTS(SELECT 1 FROM discovery_sessions WHERE id=?1 AND revoked=0 AND expires_at>?2)",params![session_id,chrono::Utc::now().to_rfc3339()],|r|r.get(0))?;
        if !valid {
            return Err(crate::RegistryError::Validation(
                "discovery session unavailable".into(),
            ));
        }
        for id in candidate_ids {
            tx.execute("INSERT OR IGNORE INTO discovery_disclosures(session_id,candidate_id) VALUES(?1,?2)",params![session_id,id])?;
        }
        tx.commit()?;
        Ok(())
    }

    pub fn revoke_discovery_session(&mut self, id: &str) -> RegistryResult<()> {
        self.db.conn.execute(
            "UPDATE discovery_sessions SET revoked = 1 WHERE id = ?1",
            params![id],
        )?;
        Ok(())
    }

    pub fn upsert_skill(
        &mut self,
        id: &str,
        name: &str,
        schema: &str,
        kind: &str,
        manifest_json: &str,
        path: Option<&str>,
    ) -> RegistryResult<()> {
        let value: serde_json::Value = serde_json::from_str(manifest_json)?;
        let manifest = toolhub_core::SkillManifest::validate_json(&value)
            .map_err(|e| crate::RegistryError::Validation(e.to_string()))?;
        let manifest_kind = serde_json::to_value(manifest.kind)?;
        if manifest.id.as_str() != id
            || manifest.name != name
            || manifest.schema != schema
            || manifest_kind.as_str() != Some(kind)
        {
            return Err(crate::RegistryError::Validation(
                "skill row disagrees with authoritative manifest".into(),
            ));
        }
        if let Some(root) = path {
            for asset in [
                &manifest.instruction_file,
                &manifest.mcp_config,
                &manifest.package_path,
            ]
            .into_iter()
            .flatten()
            {
                toolhub_core::SkillManifest::resolve_package_path(
                    std::path::Path::new(root),
                    asset,
                )
                .map_err(|e| crate::RegistryError::Validation(e.to_string()))?;
            }
        }
        self.db.conn.execute(
            "INSERT INTO skills(id, name, schema, kind, manifest_json, path)
             VALUES (?1,?2,?3,?4,?5,?6)
             ON CONFLICT(id) DO UPDATE SET name=excluded.name, manifest_json=excluded.manifest_json",
            params![id, name, schema, kind, manifest_json, path],
        )?;
        Ok(())
    }

    pub fn upsert_agent(
        &mut self,
        id: &str,
        name: &str,
        kind: &str,
        health: &str,
    ) -> RegistryResult<()> {
        self.db.conn.execute(
            "INSERT INTO agents(id, name, kind, health, config_json) VALUES (?1,?2,?3,?4,'{}')
             ON CONFLICT(id) DO UPDATE SET health=excluded.health, name=excluded.name",
            params![id, name, kind, health],
        )?;
        Ok(())
    }

    /// F06: start a scan session; returns session id.
    pub fn begin_scan(&mut self, mode: &str) -> RegistryResult<String> {
        let id = uuid::Uuid::new_v4().to_string();
        self.db.conn.execute(
            "INSERT INTO scan_sessions(id, mode, started_at, status, coverage_json, errors_json)
             VALUES (?1, ?2, ?3, 'running', '{}', '[]')",
            params![id, mode, chrono::Utc::now().to_rfc3339()],
        )?;
        Ok(id)
    }

    pub fn finish_scan(
        &mut self,
        id: &str,
        status: &str,
        coverage_json: &str,
        errors_json: &str,
    ) -> RegistryResult<()> {
        self.db.conn.execute(
            "UPDATE scan_sessions SET finished_at=?2, status=?3, coverage_json=?4, errors_json=?5 WHERE id=?1",
            params![
                id,
                chrono::Utc::now().to_rfc3339(),
                status,
                coverage_json,
                errors_json
            ],
        )?;
        Ok(())
    }

    /// F06: mark instances not seen in this scan (only when scan fully covered roots).
    pub fn mark_missing_except(&mut self, seen_ids: &[String]) -> RegistryResult<usize> {
        let mut n = 0;
        let mut stmt = self
            .db
            .conn
            .prepare("SELECT id FROM tool_instances WHERE status = 'available'")?;
        let all: Vec<String> = stmt
            .query_map([], |r| r.get(0))?
            .collect::<Result<Vec<_>, _>>()?;
        drop(stmt);
        for id in all {
            if !seen_ids.contains(&id) {
                self.db.conn.execute(
                    "UPDATE tool_instances SET status = 'missing' WHERE id = ?1",
                    params![id],
                )?;
                n += 1;
            }
        }
        Ok(n)
    }

    /// F06: upsert interfaces for an instance.
    pub fn upsert_interface(
        &mut self,
        id: &str,
        instance_id: &str,
        kind: &str,
        executable: Option<&str>,
    ) -> RegistryResult<()> {
        self.db.conn.execute(
            "INSERT INTO interfaces(id, instance_id, kind, executable, supports_stdin, supports_stdout, supports_batch)
             VALUES (?1, ?2, ?3, ?4, 0, 0, 0)
             ON CONFLICT(id) DO UPDATE SET kind=excluded.kind, executable=excluded.executable",
            params![id, instance_id, kind, executable],
        )?;
        Ok(())
    }

    /// F14/F15: durable skill registration
    pub fn list_skills(&mut self) -> RegistryResult<Vec<(String, String, String, String)>> {
        let mut stmt = self
            .db
            .conn
            .prepare("SELECT id, name, schema, manifest_json FROM skills ORDER BY name")?;
        let rows = stmt
            .query_map([], |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?)))?
            .collect::<Result<Vec<_>, _>>()?;
        Ok(rows)
    }

    /// F14: store untrusted classification evidence (does not elevate trust).
    pub fn store_classification(
        &mut self,
        session_id: &str,
        candidate_id: &str,
        label: &str,
        confidence: f64,
    ) -> RegistryResult<()> {
        toolhub_core::validate_confidence(confidence)
            .map_err(|e| crate::RegistryError::Validation(e.to_string()))?;
        if label.trim().is_empty() || label.len() > 8192 {
            return Err(crate::RegistryError::Validation(
                "classification label empty or over limit".into(),
            ));
        }
        let tx = self.db.conn.transaction()?;
        let (scopes, revoked, expires): (String, bool, String) = tx.query_row(
            "SELECT scopes_json,revoked,expires_at FROM discovery_sessions WHERE id=?1",
            [session_id],
            |r| Ok((r.get(0)?, r.get::<_, i64>(1)? != 0, r.get(2)?)),
        )?;
        let scopes: Vec<String> = serde_json::from_str(&scopes)?;
        let expires = chrono::DateTime::parse_from_rfc3339(&expires)
            .map_err(|_| crate::RegistryError::Validation("malformed discovery expiry".into()))?;
        if revoked
            || expires <= chrono::Utc::now()
            || !scopes.iter().any(|s| s == "classification.submit")
        {
            return Err(crate::RegistryError::Validation(
                "discovery session cannot classify".into(),
            ));
        }
        let disclosed:bool=tx.query_row("SELECT EXISTS(SELECT 1 FROM discovery_disclosures d JOIN scan_candidates c ON c.id=d.candidate_id WHERE d.session_id=?1 AND d.candidate_id=?2)",params![session_id,candidate_id],|r|r.get(0))?;
        if !disclosed {
            return Err(crate::RegistryError::Validation(
                "candidate was not disclosed to this discovery session".into(),
            ));
        }
        tx.execute(
            "INSERT INTO evidence(entity_kind, entity_id, source, provenance, confidence, summary, detail, observed_at)
             VALUES ('candidate', ?1, 'ai_classification', 'ai', ?2, ?3, ?4, ?5)",
            params![
                candidate_id,
                confidence,
                format!("classification from session {session_id}"),
                label,
                chrono::Utc::now().to_rfc3339()
            ],
        )?;
        tx.commit()?;
        Ok(())
    }

    pub fn count_evidence(&mut self) -> RegistryResult<u64> {
        let n: i64 = self
            .db
            .conn
            .query_row("SELECT COUNT(*) FROM evidence", [], |r| r.get(0))?;
        Ok(n as u64)
    }

    pub fn list_activity(&mut self, limit: u32) -> RegistryResult<Vec<(String, String, String)>> {
        let mut stmt = self
            .db
            .conn
            .prepare("SELECT ts, kind, summary FROM activity ORDER BY id DESC LIMIT ?1")?;
        let rows = stmt
            .query_map(params![limit], |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)))?
            .collect::<Result<Vec<_>, _>>()?;
        Ok(rows)
    }

    pub fn count_instances(&mut self) -> RegistryResult<u64> {
        let n: i64 = self
            .db
            .conn
            .query_row("SELECT COUNT(*) FROM tool_instances", [], |r| r.get(0))?;
        Ok(n as u64)
    }

    pub fn count_candidates(&mut self) -> RegistryResult<u64> {
        let n: i64 = self
            .db
            .conn
            .query_row("SELECT COUNT(*) FROM scan_candidates", [], |r| r.get(0))?;
        Ok(n as u64)
    }

    /// Preserve separate instances for the same definition (duplicates stay distinct).
    pub fn duplicate_groups(&mut self) -> RegistryResult<Vec<(String, usize)>> {
        let mut stmt = self.db.conn.prepare(
            "SELECT d.name, COUNT(*) FROM tool_instances i
             JOIN tool_definitions d ON d.id = i.definition_id
             GROUP BY d.id HAVING COUNT(*) > 1
             ORDER BY COUNT(*) DESC",
        )?;
        let rows = stmt
            .query_map([], |r| Ok((r.get(0)?, r.get(1)?)))?
            .collect::<Result<Vec<_>, _>>()?;
        Ok(rows)
    }

    pub fn ensure_builtin_environments(&mut self) -> RegistryResult<()> {
        for env in [
            toolhub_core::Environment::system(),
            toolhub_core::Environment::user(),
        ] {
            self.db.conn.execute("INSERT INTO environments(id,name,kind,origin_json,owner_json,labels_json) VALUES(?1,?2,?3,?4,?5,?6) ON CONFLICT(id) DO UPDATE SET origin_json=excluded.origin_json",params![env.id.as_str(),env.name,env.kind.as_str(),serde_json::to_string(&env.origin)?,serde_json::to_string(&env.owner)?,serde_json::to_string(&env.labels)?])?;
        }
        Ok(())
    }

    #[allow(clippy::too_many_arguments)]
    pub fn insert_environment(
        &mut self,
        id: &str,
        name: &str,
        kind: &str,
        root_path: Option<&str>,
        parent_id: Option<&str>,
        origin_json: &str,
        owner_json: &str,
    ) -> RegistryResult<()> {
        self.db.conn.execute(
            "INSERT INTO environments(id, name, kind, root_path, parent_id, origin_json, owner_json, labels_json)
             VALUES (?1,?2,?3,?4,?5,?6,?7,'[]')
             ON CONFLICT(id) DO UPDATE SET name=excluded.name, kind=excluded.kind, root_path=excluded.root_path",
            params![id, name, kind, root_path, parent_id, origin_json, owner_json],
        )?;
        Ok(())
    }

    pub fn list_environments(&mut self) -> RegistryResult<Vec<(String, String, String)>> {
        let mut stmt = self
            .db
            .conn
            .prepare("SELECT id, name, kind FROM environments ORDER BY name")?;
        let rows = stmt
            .query_map([], |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)))?
            .collect::<Result<Vec<_>, _>>()?;
        Ok(rows)
    }
}

fn _row_unused(r: &Row) -> rusqlite::Result<String> {
    r.get(0)
}

fn identity_key(path: &str) -> String {
    toolhub_core::normalize_path(
        toolhub_core::path_norm::canonicalize_best_effort(path)
            .replace('\\', "/")
            .trim_start_matches("//?/"),
    )
}
fn has_user_decision(raw: &str) -> bool {
    serde_json::from_str::<serde_json::Value>(raw)
        .ok()
        .and_then(|v| v.get("evidence").and_then(|e| e.as_array()).cloned())
        .map(|e| {
            e.iter().any(|v| {
                v.get("source").and_then(|s| s.as_str()) == Some("user_decision")
                    || v.get("provenance").and_then(|s| s.as_str()) == Some("user")
            })
        })
        .unwrap_or(false)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rescan_preserves_owner_correction_without_fabricated_evidence() {
        let mut reg = Registry::open_memory().unwrap();
        reg.ensure_builtin_environments().unwrap();
        let mut first = sample("legacy", "C:/py/python.exe");
        first.owner_json = r#"{"kind":"user","certainty":"known","evidence":[{"source":"user_decision","provenance":"user","confidence":1.0,"summary":"corrected"}]}"#.into();
        reg.upsert_instance(&first).unwrap();
        let second = sample("generated", "C:/py/python.exe");
        assert_eq!(reg.upsert_instance(&second).unwrap(), "legacy");
        let owner: String = reg
            .db
            .conn
            .query_row(
                "SELECT owner_json FROM tool_instances WHERE id='legacy'",
                [],
                |r| r.get(0),
            )
            .unwrap();
        assert_eq!(owner, first.owner_json);
        assert_eq!(reg.count_evidence().unwrap(), 0);
    }

    fn sample(id: &str, path: &str) -> UpsertInstanceInput {
        UpsertInstanceInput {
            id: id.into(),
            definition_id: "org.python.python".into(),
            definition_name: "Python".into(),
            version: Some("3.13.0".into()),
            platform: "windows".into(),
            arch: "x86_64".into(),
            path: path.into(),
            canonical_path: Some(path.into()),
            environment_id: Some("env.system".into()),
            origin_json: "{}".into(),
            owner_json: "{}".into(),
            trust_json: r#"{"level":"known"}"#.into(),
            status: "available".into(),
            capabilities: vec!["language.python.execute".into()],
        }
    }

    #[test]
    fn atomic_scan_returns_legacy_identity_and_scopes_missing() {
        let mut reg = Registry::open_memory().unwrap();
        let envs = vec![
            toolhub_core::Environment::system(),
            toolhub_core::Environment::user(),
        ];
        let scan = |id: &str, path: &str, root: &str| ScanInstanceInput {
            instance: sample(id, path),
            evidence: vec![toolhub_core::Evidence::native(
                toolhub_core::EvidenceSource::Filesystem,
                0.4,
                "observed fixture metadata",
            )
            .unwrap()],
            interfaces: vec![crate::ScanInterfaceInput {
                kind: "cli".into(),
                executable: Some(path.into()),
            }],
            provider: "custom".into(),
            root: root.into(),
        };
        let first = vec![
            scan("legacy", "C:/first/python.exe", "C:/first"),
            scan("other", "C:/other/python.exe", "C:/other"),
        ];
        assert_eq!(
            reg.ingest_scan(&envs, &first, &[], &[]).unwrap(),
            vec!["legacy", "other"]
        );
        let second = vec![scan("new-id", "C:/first/python.exe", "C:/first")];
        assert_eq!(
            reg.ingest_scan(
                &envs,
                &second,
                &[],
                &[ReconcileScope {
                    provider: "custom".into(),
                    root: "C:/first".into(),
                    complete: true,
                    recursive: true
                }]
            )
            .unwrap(),
            vec!["legacy"]
        );
        assert_eq!(
            reg.get_instance("other").unwrap().unwrap().status,
            "available"
        );
        let relations: i64 = reg
            .db
            .conn
            .query_row(
                "SELECT COUNT(*) FROM interfaces WHERE instance_id='legacy'",
                [],
                |r| r.get(0),
            )
            .unwrap();
        assert_eq!(relations, 1);
        reg.ingest_scan(
            &[],
            &[],
            &[],
            &[ReconcileScope {
                provider: "custom".into(),
                root: "C:/first".into(),
                complete: false,
                recursive: true,
            }],
        )
        .unwrap();
        assert_eq!(
            reg.get_instance("legacy").unwrap().unwrap().status,
            "available"
        );
        reg.ingest_scan(
            &[],
            &[],
            &[],
            &[ReconcileScope {
                provider: "custom".into(),
                root: "C:/first".into(),
                complete: true,
                recursive: true,
            }],
        )
        .unwrap();
        assert_eq!(
            reg.get_instance("legacy").unwrap().unwrap().status,
            "missing"
        );
        assert_eq!(
            reg.get_instance("other").unwrap().unwrap().status,
            "available"
        );
    }

    #[test]
    fn related_write_failure_rolls_back_entire_scan() {
        let mut reg = Registry::open_memory().unwrap();
        reg.db.conn.execute_batch("CREATE TRIGGER fixture_fail BEFORE INSERT ON interfaces BEGIN SELECT RAISE(ABORT,'owned fixture failure'); END;").unwrap();
        let input = ScanInstanceInput {
            instance: sample("first", "C:/first/python.exe"),
            evidence: vec![],
            interfaces: vec![crate::ScanInterfaceInput {
                kind: "cli".into(),
                executable: None,
            }],
            provider: "custom".into(),
            root: "C:/first".into(),
        };
        assert!(reg
            .ingest_scan(&[toolhub_core::Environment::system()], &[input], &[], &[])
            .is_err());
        assert_eq!(reg.count_instances().unwrap(), 0);
        assert!(reg.list_environments().unwrap().is_empty());
    }

    #[test]
    fn persisted_environment_hierarchy_reloads_with_native_metadata() {
        let mut reg = Registry::open_memory().unwrap();
        let mut child = toolhub_core::Environment::user();
        child.id = toolhub_core::EnvironmentId::new("env.project.fixture").unwrap();
        child.root_path = Some("C:/project/.venv".into());
        child.parent_id = Some(toolhub_core::EnvironmentId::new("env.user").unwrap());
        child.labels = vec!["size unknown".into()];
        let envs = vec![child.clone(), toolhub_core::Environment::user()];
        reg.ingest_scan(&envs, &[], &[], &[]).unwrap();
        assert_eq!(
            reg.load_environment_graph()
                .unwrap()
                .into_iter()
                .find(|e| e.id == child.id)
                .unwrap(),
            child
        );
    }

    #[test]
    fn classification_requires_durable_disclosure_and_current_revocation() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("private.db");
        let mut reg = Registry::open_path(&path).unwrap();
        let candidate = toolhub_core::ScanCandidate::from_path("C:/fixture/unknown.exe");
        reg.ingest_scan(&[], &[], std::slice::from_ref(&candidate), &[])
            .unwrap();
        let expiry = (chrono::Utc::now() + chrono::Duration::minutes(1)).to_rfc3339();
        reg.save_discovery_session(
            "session",
            "owner",
            &expiry,
            "[\"classification.submit\",\"candidate.inspect\"]",
        )
        .unwrap();
        assert!(reg
            .store_classification("session", &candidate.id, "hypothesis", 0.5)
            .is_err());
        reg.disclose_candidates("session", std::slice::from_ref(&candidate.id))
            .unwrap();
        assert!(reg
            .store_classification("session", &candidate.id, "hypothesis", 2.0)
            .is_err());
        assert!(reg
            .store_classification("session", "nonexistent", "hypothesis", 0.5)
            .is_err());
        reg.store_classification("session", &candidate.id, "hypothesis", 0.5)
            .unwrap();
        let mut other = Registry::open_path(&path).unwrap();
        other.revoke_discovery_session("session").unwrap();
        assert!(reg
            .store_classification("session", &candidate.id, "hypothesis", 0.5)
            .is_err());
        assert_eq!(reg.count_evidence().unwrap(), 1);
    }

    #[test]
    fn import_validates_entire_batch_and_does_not_elevate_trust() {
        let mut reg = Registry::open_memory().unwrap();
        let mut first = sample("fixture", "foreign/python.exe");
        first.environment_id = None;
        let mut malformed = first.clone();
        malformed.id = "bad".into();
        malformed.path = "different".into();
        malformed.version = Some("banana".into());
        assert!(reg.import_report(&[], &[first.clone(), malformed]).is_err());
        assert_eq!(reg.count_instances().unwrap(), 0);
        let ids = reg.import_report(&[], &[first]).unwrap();
        let row = reg.get_instance(&ids[0]).unwrap().unwrap();
        assert_eq!(row.status, "missing");
        assert_eq!(
            serde_json::from_str::<serde_json::Value>(&row.trust).unwrap()["level"],
            "unknown"
        );
        assert!(reg
            .capabilities_for_definition(&row.definition_id)
            .unwrap()
            .is_empty());
        assert_eq!(reg.count_evidence().unwrap(), 1);
    }

    #[test]
    fn upsert_and_search() {
        let mut reg = Registry::open_memory().unwrap();
        reg.ensure_builtin_environments().unwrap();
        reg.upsert_instance(&sample("i1", "C:/py/python.exe"))
            .unwrap();
        reg.upsert_instance(&sample("i1", "C:/py/python.exe"))
            .unwrap();
        assert_eq!(reg.count_instances().unwrap(), 1);
        let hits = reg.search("Python").unwrap();
        assert_eq!(hits.len(), 1);
    }

    #[test]
    fn preserves_duplicate_instances() {
        let mut reg = Registry::open_memory().unwrap();
        reg.ensure_builtin_environments().unwrap();
        reg.upsert_instance(&sample("i1", "C:/sys/python.exe"))
            .unwrap();
        reg.upsert_instance(&sample("i2", "C:/conda/python.exe"))
            .unwrap();
        assert_eq!(reg.count_instances().unwrap(), 2);
        let dups = reg.duplicate_groups().unwrap();
        assert_eq!(dups[0].1, 2);
    }

    #[test]
    fn policy_updates_replace_previous_actions_for_one_subject() {
        let mut reg = Registry::open_memory().unwrap();
        reg.set_policy("tool", "other", "deny").unwrap();
        for action in ["deny", "ask", "allow", "deny"] {
            reg.set_policy("tool", "fixture", action).unwrap();
            let rules = reg.list_policy().unwrap();
            let matching: Vec<_> = rules.iter().filter(|r| r.1 == "fixture").collect();
            assert_eq!(matching.len(), 1);
            assert_eq!(matching[0].2, action);
            assert!(rules.iter().any(|r| r.1 == "other" && r.2 == "deny"));
        }
    }

    #[test]
    fn policy_replacement_failure_preserves_previous_rule() {
        let mut reg = Registry::open_memory().unwrap();
        reg.set_policy("tool", "fixture", "deny").unwrap();
        reg.db
            .conn
            .execute_batch(
                "CREATE TRIGGER reject_policy BEFORE INSERT ON policy_rules
             BEGIN SELECT RAISE(ABORT, 'fixture write failure'); END;",
            )
            .unwrap();
        assert!(reg.set_policy("tool", "fixture", "ask").is_err());
        assert_eq!(
            reg.list_policy().unwrap(),
            vec![("tool".into(), "fixture".into(), "deny".into())]
        );
    }
}
