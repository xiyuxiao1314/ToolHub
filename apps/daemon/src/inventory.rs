use super::*;
impl DaemonService {
    pub(super) fn search_inventory(&mut self, p: &Value) -> Result<Value, ProtocolError> {
        let query = p["query"].as_str().unwrap_or("");
        let missing = p["include_missing"].as_bool().unwrap_or(false);
        let mut rows = self
            .registry
            .search_with_missing(query, missing)
            .map_err(db_err)?;
        for hint in toolhub_core::task_search::matching(query) {
            for row in self
                .registry
                .instances_for_capability(hint.capability)
                .map_err(db_err)?
            {
                if !rows.iter().any(|r| r.id == row.id) {
                    rows.push(row)
                }
            }
        }
        rows.sort_by(|a, b| a.name.cmp(&b.name).then(a.path.cmp(&b.path)));
        let prefs = self
            .registry
            .get_setting("preferred_instances")
            .map_err(db_err)?
            .unwrap_or_else(|| json!({}));
        let health = self
            .registry
            .get_setting("instance_health")
            .map_err(db_err)?
            .unwrap_or_else(|| json!({}));
        let offset = p["offset"].as_u64().unwrap_or(0) as usize;
        let limit = p["limit"].as_u64().unwrap_or(10000) as usize;
        let mut result = vec![];
        for row in rows.into_iter().skip(offset).take(limit) {
            let caps = self
                .registry
                .capabilities_for_definition(&row.definition_id)
                .map_err(db_err)?;
            let mut value = json!(row);
            value["capabilities"] = json!(caps);
            value["preferred"] = json!(
                prefs.get(&row.definition_id).and_then(Value::as_str) == Some(row.id.as_str())
            );
            value["health"] = health.get(&row.id).cloned().unwrap_or(Value::Null);
            if p["detail"] == "summary" {
                value = json!({"id":row.id,"definition_id":row.definition_id,"name":row.name,"version":row.version,"capabilities":caps,"preferred":value["preferred"],"status":row.status});
            }
            result.push(value);
        }
        Ok(json!(result))
    }
    pub(super) fn prefer_instance(&mut self, p: &Value) -> Result<Value, ProtocolError> {
        self.require_controller()?;
        let row = self
            .registry
            .get_instance(p["id"].as_str().unwrap_or(""))
            .map_err(db_err)?
            .ok_or_else(|| ProtocolError::new(ErrorCode::NotFound, "instance not found"))?;
        if row.status != "available" || !Path::new(&row.path).is_file() {
            return Err(ProtocolError::new(
                ErrorCode::Unavailable,
                "cannot prefer an unavailable path",
            ));
        }
        let key = if p.get("project").is_some() {
            "preferred_project_instances"
        } else {
            "preferred_instances"
        };
        let mut prefs = self
            .registry
            .get_setting(key)
            .map_err(db_err)?
            .unwrap_or_else(|| json!({}));
        let project = if let Some(value) = p.get("project") {
            let root = std::fs::canonicalize(value.as_str().unwrap_or("")).map_err(|_| {
                ProtocolError::new(ErrorCode::InvalidParams, "project directory unavailable")
            })?;
            if !root.is_dir() {
                return Err(ProtocolError::new(
                    ErrorCode::InvalidParams,
                    "project must be a directory",
                ));
            }
            Some(toolhub_core::path_norm::canonicalize_best_effort(
                &root.to_string_lossy(),
            ))
        } else {
            None
        };
        let target = if let Some(project) = project {
            if prefs.get(&project).is_none() {
                prefs[&project] = json!({})
            }
            prefs.get_mut(&project).unwrap()
        } else {
            &mut prefs
        };
        if p["clear"] == true {
            if let Some(map) = target.as_object_mut() {
                map.remove(&row.definition_id);
            }
        } else {
            target[&row.definition_id] = json!(row.id);
        }
        self.registry.set_setting(key, &prefs).map_err(db_err)?;
        Ok(json!({"saved":true,"execution_authority":false}))
    }
    pub(super) fn health_instance(&mut self, p: &Value) -> Result<Value, ProtocolError> {
        let id = p["id"].as_str().unwrap_or("");
        let row = self
            .registry
            .get_instance(id)
            .map_err(db_err)?
            .ok_or_else(|| ProtocolError::new(ErrorCode::NotFound, "instance not found"))?;
        let metadata = std::fs::metadata(&row.path).ok();
        let exists = metadata.as_ref().is_some_and(|m| m.is_file());
        let hash = if metadata
            .as_ref()
            .is_some_and(|m| m.is_file() && m.len() <= 128 * 1024 * 1024)
        {
            toolhub_executor::hash_file(&row.path).ok()
        } else {
            None
        };
        let mut report = json!({"checked_at":chrono::Utc::now(),"path_exists":exists,"sha256":hash,"level":"file_check","execution_tested":false,"permission":parse_trust(&row.trust)});
        if let Some(id) = p.get("execution_id").and_then(Value::as_str) {
            self.require_controller()?;
            let task = self.task_status(&json!({"execution_id":id}))?;
            if task["instance_id"] != row.id
                || task["status"] != "success"
                || task["executable_sha256"] != report["sha256"]
            {
                return Err(ProtocolError::new(
                    ErrorCode::InvalidParams,
                    "successful execution must bind this unchanged instance",
                ));
            }
            report["execution_tested"] = json!(true);
            report["level"] = json!("successful_invocation");
            report["execution_id"] = json!(id);
        }
        let mut health = self
            .registry
            .get_setting("instance_health")
            .map_err(db_err)?
            .unwrap_or_else(|| json!({}));
        health[id] = report.clone();
        self.registry
            .set_setting("instance_health", &health)
            .map_err(db_err)?;
        Ok(report)
    }
}

pub(super) fn version_probe_args(id: &str) -> Option<Vec<&'static str>> {
    let argument = match id {
        "org.ffmpeg.ffmpeg" | "org.ffmpeg.ffprobe" | "org.openjdk.java" => "-version",
        "org.7zip.7zip" | "org.7zip.7z" => "i",
        "org.python.python"
        | "org.nodejs.node"
        | "org.rust.cargo"
        | "org.rust.rustc"
        | "org.git.git"
        | "org.curl.curl"
        | "org.tesseract.tesseract"
        | "org.pandoc.pandoc" => "--version",
        _ => return None,
    };
    Some(vec![argument])
}
