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

    /// Idempotent upsert of a recognized instance. Duplicate discoveries update last_seen.
    pub fn upsert_instance(&mut self, input: &UpsertInstanceInput) -> RegistryResult<()> {
        let now = chrono::Utc::now().to_rfc3339();
        let tx = self.db.conn.transaction()?;
        tx.execute(
            "INSERT INTO tool_definitions(id, name, vendor, categories_json, description, created_at, updated_at)
             VALUES (?1, ?2, NULL, '[]', NULL, ?3, ?3)
             ON CONFLICT(id) DO UPDATE SET name=excluded.name, updated_at=excluded.updated_at",
            params![input.definition_id, input.definition_name, now],
        )?;
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
                input.id,
                input.definition_id,
                input.version,
                input.platform,
                input.arch,
                input.path,
                input.canonical_path,
                input.environment_id,
                input.origin_json,
                input.owner_json,
                input.trust_json,
                input.status,
                now
            ],
        )?;
        // If another row already owns this (definition, path) under a different id, keep both
        // but also upsert by path-match for stable identity of rediscovered copies.
        for cap in &input.capabilities {
            tx.execute(
                "INSERT INTO tool_capabilities(definition_id, capability_id) VALUES (?1, ?2)
                 ON CONFLICT DO NOTHING",
                params![input.definition_id, cap],
            )?;
        }
        tx.commit()?;
        Ok(())
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

    pub fn upsert_candidate(&mut self, c: &CandidateRow) -> RegistryResult<()> {
        self.db.conn.execute(
            "INSERT INTO scan_candidates(id, path, canonical_path, file_name, recognized, metadata_json, discovered_at)
             VALUES (?1, ?2, NULL, ?3, ?4, '{}', ?5)
             ON CONFLICT(id) DO UPDATE SET recognized=excluded.recognized, file_name=excluded.file_name",
            params![
                c.id,
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
        self.db.conn.execute(
            "INSERT INTO policy_rules(scope, subject, action, created_at) VALUES (?1,?2,?3,?4)
             ON CONFLICT(scope, subject, action) DO NOTHING",
            params![scope, subject, action, chrono::Utc::now().to_rfc3339()],
        )?;
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
}
