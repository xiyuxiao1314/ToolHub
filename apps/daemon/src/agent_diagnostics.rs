use super::*;
impl DaemonService {
    pub(super) fn agent_inventory(&mut self) -> Result<Value, ProtocolError> {
        let mut saved: BTreeMap<String, Value> = self
            .registry
            .get_setting("agent_inventory")
            .map_err(db_err)?
            .and_then(|s| serde_json::from_value(s).ok())
            .unwrap_or_default();
        // Import prior registry registrations into display history, preserving their lack of authority.
        {
            let mut statement = self
                .registry
                .db
                .conn
                .prepare("SELECT id,name,kind FROM agents LIMIT 64")
                .map_err(sql_err)?;
            for row in statement
                .query_map([], |r| {
                    Ok((
                        r.get::<_, String>(0)?,
                        r.get::<_, String>(1)?,
                        r.get::<_, String>(2)?,
                    ))
                })
                .map_err(sql_err)?
            {
                let (id, name, kind) = row.map_err(sql_err)?;
                saved
                    .entry(id.clone())
                    .or_insert_with(|| json!({"id":id,"name":name,"kind":kind}));
            }
        }
        for entry in saved.values_mut() {
            entry["health"] = json!("not_detected");
            entry["configuration"] = json!({"configured":false,"reader_supported":toolhub_agent_bridge::hosts::profile(entry["id"].as_str().unwrap_or("")).is_some()});
        }
        let now = chrono::Utc::now().to_rfc3339();
        for agent in toolhub_agent_bridge::detect_all() {
            let entry = saved
                .entry(agent.id.clone())
                .or_insert_with(|| json!({"id":agent.id,"name":agent.name}));
            entry["name"] = json!(agent.name);
            entry["executable"] = json!(agent.executable);
            entry["kind"] = json!(agent.kind);
            entry["health"] = json!("detected");
            entry["last_detected_at"] = json!(now);
        }
        if let Some(home) = dirs::home_dir() {
            let roaming = std::env::var_os("APPDATA").map(std::path::PathBuf::from);
            for host in toolhub_agent_bridge::hosts::HOSTS {
                let (id, name) = (host.id, host.name);
                let configuration =
                    toolhub_agent_bridge::hosts::configuration(id, &home, roaming.as_deref());
                if saved.contains_key(id)
                    || configuration["configured"] == true
                    || configuration.get("error").is_some()
                {
                    let entry = saved
                        .entry(id.into())
                        .or_insert_with(|| json!({"id":id,"name":name,"health":"not_detected"}));
                    entry["configuration"] = configuration;
                }
            }
        }
        self.registry
            .set_setting("agent_inventory", &json!(saved))
            .map_err(db_err)?;
        Ok(json!(saved.into_values().collect::<Vec<_>>()))
    }
    pub(super) fn agent_observed(&mut self, params: &Value) -> Result<Value, ProtocolError> {
        let Some((id, name)) =
            toolhub_agent_bridge::hosts::observed_identity(params["name"].as_str().unwrap_or(""))
        else {
            return Ok(json!({"recorded":false}));
        };
        let field = match params["stage"].as_str() {
            Some("handshake") => "last_handshake_at",
            Some("call") => "last_call_at",
            _ => {
                return Err(ProtocolError::new(
                    ErrorCode::InvalidParams,
                    "unknown observation stage",
                ))
            }
        };
        let mut saved: BTreeMap<String, Value> = self
            .registry
            .get_setting("agent_inventory")
            .map_err(db_err)?
            .and_then(|s| serde_json::from_value(s).ok())
            .unwrap_or_default();
        if id.starts_with("mcp-client-")
            && !saved.contains_key(&id)
            && saved
                .keys()
                .filter(|key| key.starts_with("mcp-client-"))
                .count()
                >= 64
        {
            return Ok(json!({"recorded":false,"reason":"client history capacity reached"}));
        }
        let entry = saved
            .entry(id.clone())
            .or_insert_with(|| json!({"id":id,"name":name,"kind":"mcp","health":"not_detected"}));
        entry["name"] = json!(name);
        entry[field] = json!(chrono::Utc::now().to_rfc3339());
        entry["observation_source"] =
            json!("MCP client self-reported name; not an authorization identity");
        self.registry
            .set_setting("agent_inventory", &json!(saved))
            .map_err(db_err)?;
        Ok(json!({"recorded":true}))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn arbitrary_clients_persist_with_bounded_history_and_no_authority() {
        let dir = tempfile::tempdir().unwrap();
        let db = dir.path().join("registry.sqlite");
        let mut svc = DaemonService::open(&db).unwrap();
        assert_eq!(
            svc.agent_observed(&json!({"name":"ToolHub self-test","stage":"call"}))
                .unwrap()["recorded"],
            false
        );
        for n in 0..64 {
            assert_eq!(
                svc.agent_observed(
                    &json!({"name":format!("Independent Host {n}"),"stage":"handshake"})
                )
                .unwrap()["recorded"],
                true
            );
        }
        assert_eq!(
            svc.agent_observed(&json!({"name":"Overflow Host","stage":"call"}))
                .unwrap()["recorded"],
            false
        );
        assert_eq!(
            svc.agent_observed(&json!({"name":"Independent Host 0","stage":"call"}))
                .unwrap()["recorded"],
            true
        );
        assert_eq!(
            svc.agent_observed(&json!({"name":"Gemini CLI","stage":"call"}))
                .unwrap()["recorded"],
            true
        );
        assert!(svc
            .agent_observed(&json!({"name":"Independent Host 0","stage":"invalid"}))
            .is_err());
        drop(svc);
        let mut svc = DaemonService::open(&db).unwrap();
        let rows = svc.agent_inventory().unwrap();
        let rows = rows.as_array().unwrap();
        assert_eq!(
            rows.iter()
                .filter(|row| row["id"].as_str().unwrap().starts_with("mcp-client-"))
                .count(),
            64
        );
        assert!(rows
            .iter()
            .any(|row| row["name"] == "Independent Host 0" && row["last_call_at"].is_string()));
        assert!(rows
            .iter()
            .any(|row| row["id"] == "gemini-cli" && row["last_call_at"].is_string()));
        assert!(!svc.controller_verified);
        assert!(svc.approvals.is_empty());
    }
    #[test]
    fn registration_and_observations_survive_restart_without_granting_authority() {
        let dir = tempfile::tempdir().unwrap();
        let db = dir.path().join("registry.sqlite");
        {
            let mut svc = DaemonService::open(&db).unwrap();
            svc.registry
                .upsert_agent("historic", "Historic host", "cli", "detected")
                .unwrap();
            let rows = svc.agent_inventory().unwrap();
            assert!(rows
                .as_array()
                .unwrap()
                .iter()
                .any(|a| a["id"] == "historic" && a["health"] == "not_detected"));
            svc.agent_observed(&json!({"name":"Codex fixture","stage":"handshake"}))
                .unwrap();
            svc.agent_observed(&json!({"name":"Codex fixture","stage":"call"}))
                .unwrap();
        }
        let mut svc = DaemonService::open(&db).unwrap();
        let rows = svc.agent_inventory().unwrap();
        let codex = rows
            .as_array()
            .unwrap()
            .iter()
            .find(|a| a["id"] == "codex")
            .unwrap();
        assert!(codex["last_call_at"].is_string());
        assert!(!svc.controller_verified);
        assert!(svc.approvals.is_empty());
    }
    #[test]
    fn mimo_observations_are_persistent_metadata_without_controller_authority() {
        let dir = tempfile::tempdir().unwrap();
        let db = dir.path().join("registry.sqlite");
        {
            let mut svc = DaemonService::open(&db).unwrap();
            assert_eq!(
                svc.agent_observed(&json!({"name":"mimocode-verify","stage":"handshake"}))
                    .unwrap()["recorded"],
                true
            );
            assert_eq!(
                svc.agent_observed(&json!({"name":"Xiaomi MiMo","stage":"call"}))
                    .unwrap()["recorded"],
                true
            );
            assert!(!svc.controller_verified);
            assert!(svc.approvals.is_empty());
        }
        let mut svc = DaemonService::open(&db).unwrap();
        let rows = svc.agent_inventory().unwrap();
        let rows = rows.as_array().unwrap();
        assert_eq!(rows.iter().filter(|row| row["id"] == "mimo").count(), 1);
        let mimo = rows.iter().find(|row| row["id"] == "mimo").unwrap();
        assert!(mimo["last_handshake_at"].is_string());
        assert!(mimo["last_call_at"].is_string());
        assert!(!svc.controller_verified);
        assert!(svc.approvals.is_empty());
    }
}
