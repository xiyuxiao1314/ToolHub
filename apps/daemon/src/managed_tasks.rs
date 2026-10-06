use super::*;
pub(super) fn output_declarations(params: &Value) -> Result<Vec<String>, ProtocolError> {
    let Some(value) = params.get("outputs") else {
        return Ok(vec![]);
    };
    let items = value.as_array().filter(|a| a.len() <= 32).ok_or_else(|| {
        ProtocolError::new(
            ErrorCode::InvalidParams,
            "outputs must be bounded relative paths",
        )
    })?;
    items
        .iter()
        .map(|v| {
            let path = v.as_str().ok_or_else(|| {
                ProtocolError::new(ErrorCode::InvalidParams, "output must be string")
            })?;
            if path.is_empty()
                || path.len() > 4096
                || path.contains([':', '\0'])
                || path.starts_with(['/', '\\'])
                || path
                    .split(['/', '\\'])
                    .any(|s| s.is_empty() || s == ".." || s == ".")
            {
                return Err(ProtocolError::new(
                    ErrorCode::InvalidParams,
                    "output must stay under cwd",
                ));
            }
            Ok(path.into())
        })
        .collect()
}
pub(super) fn existing_artifacts(cwd: &str, outputs: &[String]) -> Vec<Value> {
    let Ok(root) = std::fs::canonicalize(cwd) else {
        return vec![];
    };
    outputs.iter().filter_map(|p|{
  let file=std::fs::canonicalize(root.join(p.replace('\\',"/"))).ok()?;if !file.starts_with(&root){return None}
  let metadata=file.metadata().ok()?;if !metadata.is_file(){return None}
  Some(json!({"path":file,"name":file.file_name().map(|s|s.to_string_lossy()),"bytes":metadata.len()}))
 }).collect()
}
impl DaemonService {
    pub(super) fn persist_task(
        &mut self,
        id: &str,
        owner: &str,
        view: &Value,
    ) -> Result<(), ProtocolError> {
        self.registry.db.conn.execute("INSERT INTO managed_tasks(id,owner,view_json,updated_at) VALUES(?1,?2,?3,?4) ON CONFLICT(id) DO UPDATE SET view_json=excluded.view_json,updated_at=excluded.updated_at",rusqlite::params![id,owner,view.to_string(),chrono::Utc::now().to_rfc3339()]).map_err(sql_err)?;
        Ok(())
    }
    pub(super) fn task_status(&self, p: &Value) -> Result<Value, ProtocolError> {
        let id = p["execution_id"].as_str().unwrap_or("");
        let (owner, raw) = self
            .registry
            .db
            .conn
            .query_row(
                "SELECT owner,view_json FROM managed_tasks WHERE id=?1",
                [id],
                |r| Ok((r.get::<_, String>(0)?, r.get::<_, String>(1)?)),
            )
            .optional()
            .map_err(sql_err)?
            .ok_or_else(|| ProtocolError::new(ErrorCode::NotFound, "task not found"))?;
        if owner != self.peer_principal() && !self.is_admin() {
            return Err(ProtocolError::denied("task owned by another caller"));
        }
        let mut view: Value = serde_json::from_str(&raw).map_err(json_err)?;
        if let Some(result) = self.task_results.get(id) {
            view["result"] = result.clone()
        } else {
            view["output_retained_in_memory"] = json!(false)
        }
        if let Some(start) = view["started_at"]
            .as_str()
            .and_then(|s| chrono::DateTime::parse_from_rfc3339(s).ok())
        {
            view["elapsed_ms"] = json!((chrono::Utc::now() - start.with_timezone(&chrono::Utc))
                .num_milliseconds()
                .max(0));
        }
        Ok(view)
    }
    pub(super) fn task_list(&self) -> Result<Value, ProtocolError> {
        let mut statement=self.registry.db.conn.prepare("SELECT view_json FROM managed_tasks WHERE owner=?1 OR ?2 ORDER BY updated_at DESC LIMIT 100").map_err(sql_err)?;
        let rows = statement
            .query_map(
                rusqlite::params![self.peer_principal(), self.is_admin()],
                |r| r.get::<_, String>(0),
            )
            .map_err(sql_err)?;
        let mut result = vec![];
        for row in rows {
            result.push(serde_json::from_str::<Value>(&row.map_err(sql_err)?).map_err(json_err)?)
        }
        Ok(json!(result))
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn declared_outputs_are_bounded_existing_and_confined() {
        assert!(output_declarations(&json!({"outputs":["../secret"]})).is_err());
        let t = tempfile::tempdir().unwrap();
        std::fs::write(t.path().join("result.txt"), "output").unwrap();
        assert_eq!(
            existing_artifacts(
                t.path().to_str().unwrap(),
                &["result.txt".into(), "missing.txt".into()]
            )
            .len(),
            1
        );
    }
}

#[cfg(test)]
mod persistence_tests {
    use super::*;
    #[test]
    fn tasks_are_owner_bound_and_interrupted_after_restart() {
        let dir = tempfile::tempdir().unwrap();
        let db = dir.path().join("registry.sqlite");
        {
            let mut svc = DaemonService::open(&db).unwrap();
            svc.persist_task(
                "owned",
                "alice",
                &json!({"execution_id":"owned","status":"running"}),
            )
            .unwrap();
            svc.connection_principal = Some("bob".into());
            assert!(svc.task_status(&json!({"execution_id":"owned"})).is_err());
            svc.connection_principal = Some("alice".into());
            assert_eq!(
                svc.task_status(&json!({"execution_id":"owned"})).unwrap()["status"],
                "running"
            );
        }
        let mut svc = DaemonService::open(&db).unwrap();
        svc.connection_principal = Some("alice".into());
        assert_eq!(
            svc.task_status(&json!({"execution_id":"owned"})).unwrap()["status"],
            "interrupted"
        );
    }
}

impl DaemonService {
    pub(super) fn approval_status(&self, p: &Value) -> Result<Value, ProtocolError> {
        let id = p["request_id"].as_str().unwrap_or("");
        let row = self
            .registry
            .db
            .conn
            .query_row(
                "SELECT owner,session_id,status,expires_at FROM execution_requests WHERE id=?1",
                [id],
                |r| {
                    Ok((
                        r.get::<_, String>(0)?,
                        r.get::<_, String>(1)?,
                        r.get::<_, String>(2)?,
                        r.get::<_, String>(3)?,
                    ))
                },
            )
            .optional()
            .map_err(sql_err)?
            .ok_or_else(|| ProtocolError::new(ErrorCode::NotFound, "approval request not found"))?;
        if row.0 != self.peer_principal() && !self.is_admin() {
            return Err(ProtocolError::denied("approval belongs to another caller"));
        }
        let approval = self.approvals.values().find(|a| {
            a.agent_id.as_str() == row.0
                && a.session_id == row.1
                && !a.consumed
                && a.expires_at > chrono::Utc::now()
        });
        let mut status = row.2.as_str();
        if status == "approved" && approval.is_none() {
            status = "expired"
        }
        if status == "pending"
            && (!self.pending_approvals.contains_key(id)
                || chrono::DateTime::parse_from_rfc3339(&row.3)
                    .is_ok_and(|expiry| expiry <= chrono::Utc::now()))
        {
            status = "expired"
        }
        Ok(
            json!({"request_id":id,"session_id":row.1,"status":status,"approval_id":approval.map(|a|&a.approval_id),"expires_at":row.3}),
        )
    }
}
