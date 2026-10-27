//! Kernel-owned lifecycle and a least-authority frontend service bridge.
use super::execution_jobs::{invalid, text};
use super::*;
use toolhub_skills::extension::ExtensionPackage;

pub(super) fn catalog() -> Result<Vec<ExtensionPackage>, ProtocolError> {
    serde_json::from_str(include_str!("../../../extensions/official/catalog.json"))
        .map_err(json_err)
}
impl DaemonService {
    pub(super) fn ensure_workbench(&mut self) -> Result<(), ProtocolError> {
        if self
            .registry
            .get_setting("workbench_initialized")
            .map_err(db_err)?
            .is_some()
        {
            return Ok(());
        }
        let packages = catalog()?;
        let tx = self.registry.db.conn.transaction().map_err(sql_err)?;
        for p in packages {
            if p.host_module.as_deref()==Some("services") && !cfg!(windows) {continue;}
            p.validate().map_err(|e| invalid(&e))?;
            let bytes = serde_json::to_vec(&p).map_err(json_err)?;
            use sha2::Digest;
            let digest = format!("{:x}", sha2::Sha256::digest(&bytes));
            tx.execute("INSERT OR IGNORE INTO extension_packages(id,package_json,digest,source,enabled,installed_at,updated_at) VALUES(?1,?2,?3,'builtin',1,?4,?4)",rusqlite::params![p.id,String::from_utf8(bytes).map_err(|_|invalid("UTF-8"))?,digest,chrono::Utc::now().to_rfc3339()]).map_err(sql_err)?;
        }
        tx.execute(
            "INSERT INTO settings(key,value_json) VALUES('workbench_initialized','true')",
            [],
        )
        .map_err(sql_err)?;
        tx.commit().map_err(sql_err)?;
        Ok(())
    }
    pub(super) fn pause_artifact_extension(&mut self) -> Result<(), ProtocolError> {
        for mut item in self.artifact_packages()? {
            if item.status == "scheduled" {
                item.status = "blocked".into();
                item.cleanup_after = None;
                item.reason = "产物扩展已停用，清理已暂停；请重新核对后安排".into();
                self.save_artifact(&mut item, "extension_paused")?;
            }
        }
        Ok(())
    }
    pub(super) fn module_enabled(&self, module: &str) -> Result<bool, ProtocolError> {
        Ok(self
            .extension_packages(true)?
            .iter()
            .any(|(p, _, _)| p.host_module.as_deref() == Some(module)))
    }
    pub(super) fn require_feature(&self, method: &str) -> Result<(), ProtocolError> {
        let module = if method.starts_with("service.") { Some("services") } else if method.starts_with("program.") {
            Some("programs")
        } else if method.starts_with("artifact.") {
            Some("artifacts")
        } else if method.starts_with("skill.") {
            Some("skills")
        } else if method.starts_with("registry.")
            || method.starts_with("scan.")
            || method.starts_with("resolve.")
            || method == "capability.list"
            || method.starts_with("discovery.")
        {
            Some("tools")
        } else if method.starts_with("environment.") {
            Some("envs")
        } else {
            None
        };
        if let Some(module) = module {
            if !self.module_enabled(module)? {
                return Err(ProtocolError::new(
                    ErrorCode::Unavailable,
                    format!("{module} extension disabled or uninstalled; enable it in Extensions"),
                ));
            }
        }
        Ok(())
    }
    pub(super) fn check_workbench_dependencies(
        &self,
        p: &ExtensionPackage,
    ) -> Result<(), ProtocolError> {
        let installed = self.extension_packages(true)?;
        for dep in &p.dependencies {
            if !installed.iter().any(|(other, _, _)| {
                other.id == dep.id
                    && toolhub_core::compare_versions(&other.version, &dep.min_version)
                        .is_ok_and(|o| o != std::cmp::Ordering::Less)
            }) {
                return Err(invalid(&format!(
                    "enabled dependency {} >= {} required",
                    dep.id, dep.min_version
                )));
            }
        }
        // Reject cycles even when an update points to an already-installed package.
        let mut pending = p
            .dependencies
            .iter()
            .map(|d| d.id.clone())
            .collect::<Vec<_>>();
        let mut seen = std::collections::BTreeSet::new();
        while let Some(id) = pending.pop() {
            if id == p.id {
                return Err(invalid("extension dependency cycle"));
            }
            if seen.insert(id.clone()) {
                if let Some((other, _, _)) = installed.iter().find(|(q, _, _)| q.id == id) {
                    pending.extend(other.dependencies.iter().map(|d| d.id.clone()));
                }
            }
        }
        for (other, _, _) in installed {
            if other.id != p.id {
                for dep in other.dependencies.iter().filter(|d| d.id == p.id) {
                    if toolhub_core::compare_versions(&p.version, &dep.min_version)
                        .is_ok_and(|o| o == std::cmp::Ordering::Less)
                    {
                        return Err(invalid(&format!(
                            "{} requires {} >= {}",
                            other.name, p.id, dep.min_version
                        )));
                    }
                }
            }
        }
        Ok(())
    }
    pub(super) fn check_workbench_dependents(&self, id: &str) -> Result<(), ProtocolError> {
        for (p, _, _) in self.extension_packages(true)? {
            if p.dependencies.iter().any(|d| d.id == id) {
                return Err(invalid(&format!(
                    "disable dependent extension {} first",
                    p.name
                )));
            }
        }
        Ok(())
    }
    pub(super) fn dispatch_workbench(
        &mut self,
        method: &str,
        params: &Value,
    ) -> Result<Value, ProtocolError> {
        match method {
            "extension.cli_preview" => {
                self.require_controller()?;
                Ok(self.cli_request(params)?.1)
            }
            "extension.preferences" => {
                if let Some(settings) = params.get("settings") {
                    self.require_controller()?;
                    let obj = settings
                        .as_object()
                        .ok_or_else(|| invalid("settings object required"))?;
                    if obj
                        .keys()
                        .any(|k| !["theme_id", "layout", "density"].contains(&k.as_str()))
                    {
                        return Err(invalid("unknown workbench setting"));
                    }
                    if let Some(id) = settings.get("theme_id").filter(|v| !v.is_null()) {
                        let id = id.as_str().ok_or_else(|| invalid("theme ID required"))?;
                        if !self
                            .extension_packages(true)?
                            .iter()
                            .any(|(p, _, _)| p.id == id && p.theme.is_some())
                        {
                            return Err(invalid("enabled theme required"));
                        }
                    }
                    for (key, values) in [
                        ("layout", vec!["left", "right", "top"]),
                        ("density", vec!["comfortable", "compact"]),
                    ] {
                        if settings.get(key).is_some_and(|v| {
                            !v.is_null() && !v.as_str().is_some_and(|s| values.contains(&s))
                        }) {
                            return Err(invalid("invalid layout/density"));
                        }
                    }
                    self.registry
                        .set_setting("extension_workbench", settings)
                        .map_err(db_err)?;
                }
                Ok(self
                    .registry
                    .get_setting("extension_workbench")
                    .map_err(db_err)?
                    .unwrap_or_else(|| json!({})))
            }
            "extension.invoke" => {
                let id = text(params, "id")?;
                let digest = text(params, "expected_digest")?;
                let target = text(params, "method")?;
                let p = self
                    .extension_packages(true)?
                    .into_iter()
                    .find(|(p, d, _)| p.id == id && d == digest)
                    .map(|(p, _, _)| p)
                    .ok_or_else(|| invalid("extension disabled, removed or updated"))?;
                let permission = match target {
                    "registry.search"
                    | "registry.inspect_instance"
                    | "registry.inspect_tool"
                    | "resolve.capability"
                    | "capability.list" => "tools.read",
                    "program.search" => "programs.read",
                    "program.propose" => "programs.propose",
                    "skill.list" | "skill.inspect" | "skill.resolve" => "skills.read",
                    "environment.list" => "environments.read",
                    "execute.list" | "execute.status" => "tasks.read",
                    _ => {
                        return Err(ProtocolError::new(
                            ErrorCode::Denied,
                            "frontend bridge method unavailable",
                        ))
                    }
                };
                if !p.permissions.iter().any(|v| v == permission) {
                    return Err(ProtocolError::new(
                        ErrorCode::Denied,
                        "extension permission not declared",
                    ));
                }
                let request = JsonRpcRequest::new(
                    "extension-bridge",
                    target,
                    params.get("params").cloned().unwrap_or_else(|| json!({})),
                );
                // Frames never inherit the parent's verified desktop controller identity.
                let result = self.handle_authenticated(&request, &format!("extension:{id}"), false);
                if let Some(error) = result.error {
                    return Err(ProtocolError::new(
                        ErrorCode::Denied,
                        format!("{}: {}", error.code, error.message),
                    ));
                }
                Ok(result.result.unwrap_or(Value::Null))
            }
            _ => Err(invalid("unknown workbench method")),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn call(
        s: &mut DaemonService,
        method: &str,
        p: Value,
        controller: bool,
    ) -> Result<Value, toolhub_protocol::JsonRpcErrorObject> {
        let r = s.handle_authenticated(&JsonRpcRequest::new(1, method, p), "fixture", controller);
        if let Some(e) = r.error {
            Err(e)
        } else {
            Ok(r.result.unwrap())
        }
    }
    fn current(s: &mut DaemonService, id: &str) -> Value {
        call(s, "extension.list", json!({}), false)
            .unwrap()
            .as_array()
            .unwrap()
            .iter()
            .find(|r| r["package"]["id"] == id)
            .unwrap()
            .clone()
    }
    fn install(s: &mut DaemonService, p: Value, old: Option<&str>) -> Value {
        let v = call(s, "extension.preview", json!({"package":p}), false).unwrap();
        assert!(v["conflict"].is_null(), "{v}");
        let mut input = json!({"package":v["package"],"digest":v["digest"]});
        if let Some(d) = old {
            input["expected_digest"] = json!(d);
        }
        call(s, "extension.install", input, true).unwrap();
        v
    }
    fn sample() -> Value {
        serde_json::from_str(include_str!(
            "../../../extensions/examples/workspace.toolhub-extension.json"
        ))
        .unwrap()
    }
    #[test]
    fn official_lifecycle_gates_services_retains_records_and_does_not_reseed() {
        let dir = tempfile::tempdir().unwrap();
        let db = dir.path().join("r.sqlite");
        let mut s = DaemonService::open(&db).unwrap();
        assert_eq!(
            call(&mut s, "extension.list", json!({}), false)
                .unwrap()
                .as_array()
                .unwrap()
                .len(),
            if cfg!(windows) {8} else {7}
        );
        let file = dir.path().join(if cfg!(windows) {
            "start.cmd"
        } else {
            "start.command"
        });
        std::fs::write(&file, "fixture, never execute").unwrap();
        let selected = call(
            &mut s,
            "program.select",
            json!({"path":file,"kind":"file"}),
            true,
        )
        .unwrap();
        call(&mut s, "program.save", json!({"items":[selected[0]]}), true).unwrap();
        let old = current(&mut s, "toolhub.builtin.programs");
        let data = call(&mut s, "program.list", json!({}), true).unwrap();
        assert!(call(&mut s,"extension.set_enabled",json!({"id":"toolhub.builtin.programs","expected_digest":old["digest"],"enabled":false}),false).is_err());
        call(&mut s,"extension.set_enabled",json!({"id":"toolhub.builtin.programs","expected_digest":old["digest"],"enabled":false}),true).unwrap();
        assert!(call(&mut s, "program.search", json!({}), false)
            .unwrap_err()
            .message
            .contains("disabled"));
        drop(s);
        let mut s = DaemonService::open(&db).unwrap();
        assert!(!s.module_enabled("programs").unwrap());
        call(
            &mut s,
            "extension.remove",
            json!({"id":"toolhub.builtin.programs","expected_digest":old["digest"]}),
            true,
        )
        .unwrap();
        drop(s);
        let mut s = DaemonService::open(&db).unwrap();
        assert!(!s.module_enabled("programs").unwrap());
        assert!(file.is_file());
        install(&mut s, old["package"].clone(), None);
        assert_eq!(call(&mut s, "program.list", json!({}), true).unwrap(), data);
    }
    #[test]
    fn frontend_bridge_is_permission_checked_revocable_and_has_no_controller_authority() {
        let dir = tempfile::tempdir().unwrap();
        let mut s = DaemonService::open(&dir.path().join("r.sqlite")).unwrap();
        let v = install(&mut s, sample(), None);
        let invoke = |target: &str| json!({"id":"community.workspace","expected_digest":v["digest"],"method":target,"params":if target=="registry.search"{json!({"query":""})}else{json!({})}});
        assert!(call(&mut s, "extension.invoke", invoke("registry.search"), true).is_ok());
        for method in [
            "settings.set",
            "extension.remove",
            "program.select",
            "program.launch",
            "artifact.recycle",
            "skill.register",
            "program.search",
        ] {
            assert!(
                call(&mut s, "extension.invoke", invoke(method), true).is_err(),
                "{method}"
            );
        }
        call(
            &mut s,
            "extension.set_enabled",
            json!({"id":"community.workspace","expected_digest":v["digest"],"enabled":false}),
            true,
        )
        .unwrap();
        assert!(call(&mut s, "extension.invoke", invoke("registry.search"), true).is_err());
        call(
            &mut s,
            "extension.set_enabled",
            json!({"id":"community.workspace","expected_digest":v["digest"],"enabled":true}),
            true,
        )
        .unwrap();
        let mut newer = sample();
        newer["version"] = json!("1.0.1");
        install(&mut s, newer, Some(v["digest"].as_str().unwrap()));
        assert!(call(&mut s, "extension.invoke", invoke("registry.search"), true).is_err());
    }
    #[test]
    fn dependency_checks_cover_missing_disable_cycles_and_rollback() {
        let dir = tempfile::tempdir().unwrap();
        let mut s = DaemonService::open(&dir.path().join("r.sqlite")).unwrap();
        let mut first = sample();
        first["id"] = json!("community.first");
        first["views"][0]["id"] = json!("community.first.page");
        let old = install(&mut s, first.clone(), None);
        let mut second = sample();
        second["id"] = json!("community.second");
        second["views"][0]["id"] = json!("community.second.page");
        second["dependencies"] = json!([{"id":"community.first","min_version":"1.0.0"}]);
        install(&mut s, second, None);
        assert!(call(
            &mut s,
            "extension.remove",
            json!({"id":"community.first","expected_digest":old["digest"]}),
            true
        )
        .is_err());
        assert!(call(
            &mut s,
            "extension.set_enabled",
            json!({"id":"community.first","expected_digest":old["digest"],"enabled":false}),
            true
        )
        .is_err());
        first["version"] = json!("1.0.1");
        first["dependencies"] = json!([{"id":"community.second","min_version":"1.0.0"}]);
        assert!(
            call(&mut s, "extension.preview", json!({"package":first}), false).unwrap()["conflict"]
                .as_str()
                .unwrap()
                .contains("cycle")
        );
        first["dependencies"] = json!([]);
        let new = install(&mut s, first, Some(old["digest"].as_str().unwrap()));
        call(
            &mut s,
            "extension.rollback",
            json!({"id":"community.first","expected_digest":new["digest"]}),
            true,
        )
        .unwrap();
        assert_eq!(current(&mut s, "community.first")["digest"], old["digest"]);
        let mut missing = sample();
        missing["dependencies"] = json!([{"id":"community.missing","min_version":"1.0.0"}]);
        assert!(!call(
            &mut s,
            "extension.preview",
            json!({"package":missing}),
            false
        )
        .unwrap()["conflict"]
            .is_null());
    }
    #[test]
    fn themes_persist_and_reset_is_kernel_controlled() {
        let dir = tempfile::tempdir().unwrap();
        let db = dir.path().join("r.sqlite");
        let mut s = DaemonService::open(&db).unwrap();
        let p = serde_json::from_str(include_str!(
            "../../../extensions/examples/graphite.toolhub-extension.json"
        ))
        .unwrap();
        install(&mut s, p, None);
        let settings =
            json!({"theme_id":"community.graphite","layout":"right","density":"compact"});
        assert!(call(
            &mut s,
            "extension.preferences",
            json!({"settings":settings}),
            false
        )
        .is_err());
        call(
            &mut s,
            "extension.preferences",
            json!({"settings":settings}),
            true,
        )
        .unwrap();
        drop(s);
        let mut s = DaemonService::open(&db).unwrap();
        assert_eq!(
            call(&mut s, "extension.preferences", json!({}), false).unwrap(),
            settings
        );
        assert!(call(
            &mut s,
            "extension.preferences",
            json!({"settings":{"layout":"unsafe"}}),
            true
        )
        .is_err());
        call(
            &mut s,
            "extension.preferences",
            json!({"settings":{}}),
            true,
        )
        .unwrap();
        assert_eq!(
            call(&mut s, "extension.preferences", json!({}), false).unwrap(),
            json!({})
        );
    }
}
