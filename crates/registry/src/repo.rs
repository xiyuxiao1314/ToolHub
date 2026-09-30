use rusqlite::{params, OptionalExtension, Row};

use crate::db::RegistryDb;
use crate::models::{CandidateRow, InstanceRow, UpsertInstanceInput};
use crate::RegistryResult;

pub struct Registry {
    pub db: RegistryDb,
}

impl Registry {
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
        let now = chrono::Utc::now().to_rfc3339();
        let tx = self.db.conn.transaction()?;
        tx.execute(
            "INSERT INTO tool_definitions(id, name, vendor, categories_json, description, created_at, updated_at)
             VALUES (?1, ?2, NULL, '[]', NULL, ?3, ?3)
             ON CONFLICT(id) DO UPDATE SET name=excluded.name, updated_at=excluded.updated_at",
            params![input.definition_id, input.definition_name, now],
        )?;
        // Resolve existing row by stable path identity
        let existing: Option<String> = tx
            .query_row(
                "SELECT id FROM tool_instances WHERE definition_id = ?1 AND (path = ?2 OR lower(replace(path,'\\','/')) = ?3) LIMIT 1",
                params![
                    input.definition_id,
                    input.path,
                    toolhub_core::normalize_path(&input.path).replace('\\', "/")
                ],
                |r| r.get(0),
            )
            .optional()?;
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
                {
                    prev
                } else {
                    input.trust_json.clone()
                }
            }
            None => input.trust_json.clone(),
        };
        tx.execute(
            "INSERT INTO tool_instances(
                id, definition_id, version, platform, arch, path, canonical_path,
                environment_id, origin_json, owner_json, trust_json, status, first_seen, last_seen)
             VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11,?12,?13,?13)
             ON CONFLICT(id) DO UPDATE SET
                version=excluded.version,
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
                input.owner_json,
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
        // F06: persist evidence rows
        tx.execute(
            "INSERT INTO evidence(entity_kind, entity_id, source, provenance, confidence, summary, observed_at)
             VALUES ('instance', ?1, 'path_pattern', 'native', 0.8, 'recognized during scan', ?2)",
            params![id, now],
        )?;
        tx.commit()?;
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
                        i.environment_id, i.trust_json, i.status
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
                    i.environment_id, i.trust_json, i.status
             FROM tool_instances i
             JOIN tool_definitions d ON d.id = i.definition_id
             WHERE d.name LIKE ?1 OR i.path LIKE ?1 OR d.id LIKE ?1
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
                    i.environment_id, i.trust_json, i.status
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
        self.db.conn.execute(
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
        self.db.conn.execute(
            "INSERT INTO environments(id, name, kind, origin_json, owner_json, labels_json)
             VALUES ('env.system','System','system','{\"system\":{}}','{\"kind\":\"user\",\"certainty\":\"known\",\"evidence\":[]}','[\"builtin\"]')
             ON CONFLICT(id) DO NOTHING",
            [],
        )?;
        self.db.conn.execute(
            "INSERT INTO environments(id, name, kind, origin_json, owner_json, labels_json)
             VALUES ('env.user','User','user','{\"user_install\":{}}','{\"kind\":\"user\",\"certainty\":\"known\",\"evidence\":[]}','[\"builtin\"]')
             ON CONFLICT(id) DO NOTHING",
            [],
        )?;
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

#[cfg(test)]
mod tests {
    use super::*;

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
