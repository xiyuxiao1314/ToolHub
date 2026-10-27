//! Official service-monitor feature. Registration is observation, never execution authority.
use super::{
    execution_jobs::{invalid, text},
    *,
};
use serde::{Deserialize, Serialize};
#[cfg(windows)]
#[path = "binding_native.rs"]
mod binding_native;
#[cfg(windows)]
#[path = "service_native.rs"]
pub(crate) mod native;
#[cfg(windows)]
#[path = "proxy_native.rs"]
mod proxy_native;

#[derive(Default)]
pub struct ServiceRuntime {
    #[cfg(windows)]
    jobs: BTreeMap<String, native::Job>,
    #[cfg(windows)]
    binding_errors: BTreeMap<String, String>,
}
#[derive(Clone, Serialize, Deserialize)]
struct Record {
    id: String,
    name: String,
    kind: String,
    owner: String,
    source: String,
    project_dir: String,
    program_id: Option<String>,
    #[serde(default)]
    web_url: Option<String>,
    #[cfg(windows)]
    #[serde(default)]
    binding: Option<binding_native::Binding>,
    #[serde(default)]
    archived: bool,
    pid: u32,
    created: String,
    executable: String,
    started_at: String,
    #[serde(default)]
    ended_at: Option<String>,
    job_name: Option<String>,
    #[serde(default)]
    auto_stop_at: Option<String>,
    #[serde(default)]
    reminder_minutes: u64,
    #[serde(default)]
    stop_requested: bool,
    #[serde(default)]
    history: Vec<Value>,
}
impl Record {
    fn note(&mut self, action: &str) {
        self.history
            .push(json!({"at":chrono::Utc::now().to_rfc3339(),"action":action}));
        if self.history.len() > 30 {
            self.history.remove(0);
        }
    }
    #[cfg(windows)]
    fn identity(&self) -> native::Identity {
        native::Identity {
            pid: self.pid,
            created: self.created.clone(),
            executable: self.executable.clone(),
        }
    }
}
#[cfg(all(test, windows))]
#[path = "service_tests.rs"]
mod tests;

const KEY: &str = "service_monitor_v1";
impl DaemonService {
    fn service_records(&mut self) -> Result<Vec<Record>, ProtocolError> {
        serde_json::from_value(
            self.registry
                .get_setting(KEY)
                .map_err(db_err)?
                .unwrap_or_else(|| json!([])),
        )
        .map_err(json_err)
    }
    fn save_services(&mut self, records: &[Record]) -> Result<(), ProtocolError> {
        self.registry
            .set_setting(KEY, &json!(records))
            .map_err(db_err)
    }
    pub(super) fn pause_service_timers(&mut self) -> Result<(), ProtocolError> {
        let mut records = self.service_records()?;
        let mut changed = false;
        for r in &mut records {
            if r.auto_stop_at.take().is_some() {
                r.note("自动停止已暂停，请重新设置");
                changed = true;
            }
        }
        if changed {
            self.save_services(&records)?
        }
        Ok(())
    }
    pub(super) fn service_monitor_init(&mut self) -> Result<(), ProtocolError> {
        if !cfg!(windows) {
            return Ok(());
        }
        // Versioned additive seed; never resurrect a user-uninstalled extension.
        if self
            .registry
            .get_setting("service_monitor_seeded_v1")
            .map_err(db_err)?
            .is_none()
        {
            let p: toolhub_skills::extension::ExtensionPackage = serde_json::from_str(
                include_str!("../../../extensions/official/services.toolhub-extension.json"),
            )
            .map_err(json_err)?;
            p.validate().map_err(|e| invalid(&e))?;
            use sha2::Digest;
            let bytes = serde_json::to_vec(&p).map_err(json_err)?;
            let digest = format!("{:x}", sha2::Sha256::digest(&bytes));
            let tx = self.registry.db.conn.transaction().map_err(sql_err)?;
            tx.execute("INSERT OR IGNORE INTO extension_packages(id,package_json,digest,source,enabled,installed_at,updated_at) VALUES(?1,?2,?3,'builtin',1,?4,?4)",rusqlite::params![p.id,String::from_utf8(bytes).unwrap(),digest,chrono::Utc::now().to_rfc3339()]).map_err(sql_err)?;
            tx.execute(
                "INSERT INTO settings(key,value_json) VALUES('service_monitor_seeded_v1','true')",
                [],
            )
            .map_err(sql_err)?;
            tx.commit().map_err(sql_err)?;
        }
        self.pause_service_timers()
    }
    pub(super) fn dispatch_service(
        &mut self,
        method: &str,
        params: &Value,
    ) -> Result<Value, ProtocolError> {
        let obj = params
            .as_object()
            .ok_or_else(|| invalid("参数必须为对象"))?;
        let allowed: &[&str] = match method {
            "service.page_url" => &["id"],
            "service.proxy_status" => &[],
            "service.proxy_configure" => &["ports"],
            "service.register" => &["name", "pid", "project_dir", "kind"],
            "service.list" => &["id", "offset", "limit"],
            "service.start" => &["program_id"],
            "service.stop" => &["id", "force", "expected_pid", "expected_created"],
            "service.restart" | "service.remove" => &["id"],
            "service.consolidate" => &["program_id", "keep_id"],
            "service.configure" => &[
                "id",
                "program_id",
                "binding",
                "web_url",
                "reminder_minutes",
                "auto_stop_minutes",
                "confirm_force",
            ],
            _ => return Err(invalid("unknown service method")),
        };
        if obj.keys().any(|k| !allowed.contains(&k.as_str()))
            || serde_json::to_vec(params).map_or(true, |s| s.len() > 8192)
        {
            return Err(invalid("服务参数无效或过大"));
        }
        if method != "service.register" && method != "service.list" {
            self.require_controller()?;
        }
        #[cfg(not(windows))]
        {
            return Err(ProtocolError::new(
                ErrorCode::Unavailable,
                "服务监控第一版仅支持 Windows；不会降级执行进程命令",
            ));
        }
        #[cfg(windows)]
        self.dispatch_windows_service(method, params)
    }
    #[cfg(windows)]
    fn service_job(&mut self, r: &Record) -> Result<Option<&native::Job>, ProtocolError> {
        if let Some(name) = &r.job_name {
            if !self.services.jobs.contains_key(&r.id) {
                if let Ok(job) = native::Job::open(name) {
                    self.services.jobs.insert(r.id.clone(), job);
                }
            }
            return Ok(self.services.jobs.get(&r.id));
        }
        Ok(None)
    }
    #[cfg(windows)]
    fn service_snapshot(&mut self, r: &Record) -> Value {
        let (pids, status) =
            if r.binding.is_some() && self.services.binding_errors.contains_key(&r.id) {
                (
                    vec![],
                    if r.stop_requested {
                        "stopped"
                    } else {
                        "waiting_for_process"
                    },
                )
            } else if r.job_name.is_some() {
                match self.service_job(r) {
                    Ok(Some(job)) => match job.pids() {
                        Ok(p) => {
                            let s = if p.is_empty() { "stopped" } else { "running" };
                            (p, s)
                        }
                        Err(_) => (vec![], "unavailable"),
                    },
                    _ => (
                        vec![],
                        if r.ended_at.is_some() {
                            "stopped"
                        } else {
                            "unavailable"
                        },
                    ),
                }
            } else {
                match native::exact(&r.identity(), false) {
                    Ok(_) => (vec![r.pid], "running"),
                    Err(_) => (vec![], "exited_or_changed"),
                }
            };
        let mut identities = Vec::new();
        let mut memory = 0usize;
        let mut memory_available = false;
        for pid in pids.iter().take(256) {
            if let Ok(p) = if r.job_name.is_some() {
                native::owned_process(*pid)
            } else {
                native::process(*pid, false)
            } {
                if r.job_name.is_some()
                    && !self
                        .service_job(r)
                        .ok()
                        .flatten()
                        .is_some_and(|job| job.contains(&p))
                {
                    continue;
                }
                if let Some(bytes) = p.memory_bytes {
                    memory = memory.saturating_add(bytes);
                    memory_available = true;
                }
                identities.push(p.identity);
            }
        }
        let pid_set = identities.iter().map(|p| p.pid).collect();
        let (endpoints, port_error) = match native::endpoints(&pid_set) {
            Ok(e) => (json!(e), None),
            Err(e) => (json!([]), Some(e)),
        };
        let started = chrono::DateTime::parse_from_rfc3339(&r.started_at).ok();
        let elapsed = started
            .map(|t| {
                (r.ended_at
                    .as_ref()
                    .and_then(|end| chrono::DateTime::parse_from_rfc3339(end).ok())
                    .map(|end| end.with_timezone(&chrono::Utc))
                    .unwrap_or_else(chrono::Utc::now)
                    - t.with_timezone(&chrono::Utc))
                .num_seconds()
                .max(0)
            })
            .unwrap_or(0);
        let application = identities
            .iter()
            .find(|p| {
                let name = Path::new(&p.executable)
                    .file_name()
                    .unwrap_or_default()
                    .to_string_lossy()
                    .to_lowercase();
                !name.starts_with("toolhub")
                    && !matches!(
                        name.as_str(),
                        "cmd.exe"
                            | "powershell.exe"
                            | "pwsh.exe"
                            | "conhost.exe"
                            | "openconsole.exe"
                    )
            })
            .map(|p| p.executable.clone())
            .unwrap_or_else(|| r.executable.clone());
        let protected = self.service_is_proxy(r, &endpoints);
        json!({"binding_error":self.services.binding_errors.get(&r.id),"binding":r.binding,"created":r.created,"web_url":r.web_url,"application_path":application,"persistent_proxy":protected,"id":r.id,"name":r.name,"kind":r.kind,"source":r.source,"project_dir":r.project_dir,"program_id":r.program_id,"pid":r.pid,"executable":r.executable,"started_at":r.started_at,"status":status,"stop_requested":r.stop_requested,"managed":r.job_name.is_some(),"process_count":pids.len(),"elapsed_seconds":elapsed,"memory_bytes":if memory_available{Some(memory)}else{None},"endpoints":endpoints,"port_error":port_error,"ports_truncated":pids.len()>256||endpoints.as_array().is_some_and(|e|e.len()>=256),"reminder_minutes":r.reminder_minutes,"reminder_due":status=="running"&&r.reminder_minutes>0&&elapsed>=r.reminder_minutes as i64*60,"auto_stop_at":r.auto_stop_at,"history":r.history})
    }
    #[cfg(windows)]
    fn service_is_proxy(&mut self, r: &Record, endpoints: &Value) -> bool {
        if r.kind == "proxy" {
            return true;
        }
        let configured = self
            .registry
            .get_setting("service_proxy_ports_v1")
            .ok()
            .flatten()
            .unwrap_or_else(|| json!([]));
        let system = proxy_native::settings();
        let system_ports: Vec<u16> = if system["system_proxy_enabled"] == true {
            system["server"]
                .as_str()
                .unwrap_or("")
                .split(';')
                .filter_map(|s| s.rsplit(':').next()?.parse().ok())
                .collect()
        } else {
            vec![]
        };
        endpoints.as_array().is_some_and(|rows| {
            rows.iter().any(|e| {
                e["port"].as_u64().is_some_and(|port| {
                    system_ports.contains(&(port as u16))
                        || configured
                            .as_array()
                            .is_some_and(|p| p.iter().any(|p| p["port"].as_u64() == Some(port)))
                })
            })
        })
    }
    #[cfg(windows)]
    fn proxy_status(&mut self) -> Result<Value, ProtocolError> {
        let configured = self
            .registry
            .get_setting("service_proxy_ports_v1")
            .map_err(db_err)?
            .unwrap_or_else(|| json!([]));
        let ports = configured
            .as_array()
            .map(|v| {
                v.iter()
                    .filter_map(|p| p["port"].as_u64().map(|p| p as u16))
                    .collect()
            })
            .unwrap_or_default();
        let endpoints = native::endpoints_for_ports(&ports).map_err(|e| invalid(&e))?;
        let rows=configured.as_array().into_iter().flatten().map(|c|{
            let active:Vec<_>=endpoints.iter().filter(|e|Some(e.port as u64)==c["port"].as_u64()).collect();
            let app=active.iter().find_map(|e|native::process(e.pid,false).ok().map(|p|p.identity.executable));
            json!({"name":c["name"],"port":c["port"],"listening":!active.is_empty(),"application_path":app,"read_only":true})
        }).collect::<Vec<_>>();
        let mut status = proxy_native::settings();
        status["configured_ports"] = configured;
        status["items"] = json!(rows);
        Ok(status)
    }
    #[cfg(windows)]
    fn dispatch_windows_service(
        &mut self,
        method: &str,
        params: &Value,
    ) -> Result<Value, ProtocolError> {
        if method == "service.proxy_status" {
            return self.proxy_status();
        }
        if method == "service.proxy_configure" {
            let ports = params["ports"]
                .as_array()
                .filter(|v| v.len() <= 8)
                .ok_or_else(|| invalid("最多 8 个代理端口"))?;
            let mut seen = std::collections::BTreeSet::new();
            for p in ports {
                let n = p["port"]
                    .as_u64()
                    .filter(|n| *n > 0 && *n <= 65535)
                    .ok_or_else(|| invalid("端口无效"))?;
                let name = p["name"]
                    .as_str()
                    .filter(|s| !s.is_empty() && s.len() <= 64)
                    .ok_or_else(|| invalid("代理名称无效"))?;
                if !seen.insert(n) {
                    return Err(invalid("端口重复"));
                }
                let _ = name;
            }
            self.registry
                .set_setting("service_proxy_ports_v1", &params["ports"])
                .map_err(db_err)?;
            self.pause_service_timers()?;
            return self.proxy_status();
        }
        let mut records = self.service_records()?;
        self.reconcile_bindings(&mut records)?;
        if method == "service.register" {
            let name = text(params, "name")?;
            if name.trim().is_empty() || name.len() > 128 {
                return Err(invalid("名称需为 1 至 128 字节"));
            }
            let pid = params["pid"]
                .as_u64()
                .filter(|n| *n > 4 && *n <= u32::MAX as u64)
                .ok_or_else(|| invalid("pid 无效"))? as u32;
            let p = native::process(pid, false).map_err(|e| invalid(&e))?;
            let project = text(params, "project_dir")?;
            if project.len() > 2048
                || !Path::new(project).is_absolute()
                || !Path::new(project).is_dir()
            {
                return Err(invalid("请选择实际存在的项目目录"));
            }
            let kind = params
                .get("kind")
                .and_then(Value::as_str)
                .unwrap_or("other");
            if !["web", "model", "proxy", "other"].contains(&kind) {
                return Err(invalid("服务类型无效"));
            }
            let owner = self.peer_principal();
            if let Some(r) = records
                .iter()
                .find(|r| r.pid == pid && r.created == p.identity.created)
            {
                if r.owner != owner && !self.is_admin() {
                    return Err(invalid("进程已由其他来源登记"));
                }
                return Ok(json!({"id":r.id,"registered":true,"execution_authority":false}));
            }
            if records.len() >= 200 || records.iter().filter(|r| r.owner == owner).count() >= 64 {
                return Err(invalid("服务记录已达上限，请移除已结束记录"));
            }
            let ticks = p
                .identity
                .created
                .parse::<i64>()
                .map_err(|_| invalid("创建时间无效"))?;
            let started =
                chrono::DateTime::from_timestamp((ticks / 10_000_000) - 11_644_473_600, 0)
                    .ok_or_else(|| invalid("创建时间无效"))?;
            let mut r = Record {
                id: uuid::Uuid::new_v4().to_string(),
                name: name.into(),
                kind: kind.into(),
                owner,
                source: "agent_registered".into(),
                project_dir: Path::new(project)
                    .canonicalize()
                    .map_err(|_| invalid("目录不可读取"))?
                    .to_string_lossy()
                    .into(),
                program_id: None,
                web_url: None,
                binding: None,
                archived: false,
                pid,
                created: p.identity.created,
                executable: p.identity.executable,
                started_at: started.to_rfc3339(),
                ended_at: None,
                job_name: None,
                auto_stop_at: None,
                reminder_minutes: 120,
                stop_requested: false,
                history: vec![],
            };
            r.note("登记已有进程（仅观察）");
            let id = r.id.clone();
            records.insert(0, r);
            self.save_services(&records)?;
            self.event(
                "service.registered",
                "observed an explicit same-user process; no control authority granted",
                json!({"id":id,"pid":pid}),
            )?;
            return Ok(json!({"id":id,"registered":true,"execution_authority":false}));
        }
        if method == "service.list" {
            let offset = integer(params, "offset", 0, 0, 10000)? as usize;
            let limit = integer(params, "limit", 10, 1, 50)? as usize;
            let owner = self.peer_principal();
            let admin = self.is_admin();
            let selected: Vec<_> = records
                .iter()
                .filter(|r| {
                    !r.archived
                        && (admin || r.owner == owner)
                        && params.get("id").is_none_or(|id| id.as_str() == Some(&r.id))
                })
                .cloned()
                .collect();
            return Ok(
                json!({"total":selected.len(),"items":selected.iter().skip(offset).take(limit).map(|r|{
                    let snapshot=self.service_snapshot(r);
                    if admin||params.get("id").is_some(){snapshot}else{json!({"id":r.id,"name":r.name,"pid":r.pid,"status":snapshot["status"],"elapsed_seconds":snapshot["elapsed_seconds"],"persistent_proxy":snapshot["persistent_proxy"],"endpoints":snapshot["endpoints"].as_array().map(|v|v.iter().take(8).collect::<Vec<_>>()),"execution_authority":false})}
                }).collect::<Vec<_>>(),"platform":"windows"}),
            );
        }
        if method == "service.start" {
            let (id, pid) = self.launch_monitored_program(text(params, "program_id")?)?;
            return Ok(json!({"id":id,"pid":pid,"started":true}));
        }
        if method == "service.consolidate" {
            let keep = text(params, "keep_id")?;
            let program = text(params, "program_id")?;
            let i = records
                .iter()
                .position(|r| r.id == keep && r.program_id.as_deref() == Some(program))
                .ok_or_else(|| invalid("保留的服务不属于该程序"))?;
            let old = records
                .iter()
                .filter(|r| r.id != keep && r.program_id.as_deref() == Some(program) && !r.archived)
                .cloned()
                .collect::<Vec<_>>();
            for r in &old {
                if self.service_snapshot(r)["status"] == "running" {
                    return Err(invalid("存在另一个运行实例，请先核对，不会自动结束"));
                }
            }
            for r in old {
                records[i].note(&format!("已合并旧启动记录 PID {}", r.pid));
                if records[i].web_url.is_none() {
                    records[i].web_url = r.web_url.clone();
                }
                for item in records.iter_mut().filter(|v| v.id == r.id) {
                    item.archived = true;
                    item.auto_stop_at = None;
                }
            }
            self.save_services(&records)?;
            return Ok(json!({"consolidated":true}));
        }
        let index = records
            .iter()
            .position(|r| Some(r.id.as_str()) == params["id"].as_str())
            .ok_or_else(|| invalid("服务记录不存在"))?;
        let r = &mut records[index];
        match method {
            "service.page_url" => {
                let url = r
                    .web_url
                    .as_deref()
                    .ok_or_else(|| invalid("请先设置页面地址"))?;
                Ok(
                    json!({"url":toolhub_core::service_url::normalize(url).map_err(|e|invalid(&e))?}),
                )
            }
            "service.stop" => {
                if r.binding.is_some()
                    && (params["expected_pid"].as_u64() != Some(r.pid as u64)
                        || params["expected_created"].as_str() != Some(r.created.as_str()))
                {
                    return Err(invalid("进程身份已变化，请刷新后重新确认"));
                }

                let force = match params.get("force") {
                    None => false,
                    Some(Value::Bool(v)) => *v,
                    _ => return Err(invalid("force 必须为布尔值")),
                };
                self.stop_service_record(r, force)?;
                self.save_services(&records)?;
                Ok(json!({"requested":true,"force":force,"check_status":true}))
            }
            "service.restart" => {
                // Never overlap an old live instance, even after graceful-stop request.
                if self.service_snapshot(r)["status"] == "running" {
                    return Err(invalid(
                        "请先停止服务并确认退出，再重启；不会同时启动第二份",
                    ));
                }
                let program_id = r
                    .program_id
                    .clone()
                    .ok_or_else(|| invalid("请先关联程序入口"))?;
                let (id, pid) = self.launch_monitored_program(&program_id)?;
                Ok(json!({"id":id,"pid":pid,"started":true}))
            }
            "service.configure" => {
                if let Some(binding) = params.get("binding") {
                    if binding.is_null() {
                        r.binding = None;
                    } else {
                        let binding: binding_native::Binding =
                            serde_json::from_value(binding.clone()).map_err(json_err)?;
                        binding.validate().map_err(|e| invalid(&e))?;
                        r.binding = Some(binding);
                        r.auto_stop_at = None;
                        r.note("用户设置独立进程观察规则；不授予自动停止权限");
                    }
                }

                if let Some(url) = params.get("web_url") {
                    r.web_url = if url.is_null() || url.as_str() == Some("") {
                        None
                    } else {
                        Some(
                            toolhub_core::service_url::normalize(
                                url.as_str()
                                    .ok_or_else(|| invalid("页面地址必须是字符串"))?,
                            )
                            .map_err(|e| invalid(&e))?,
                        )
                    };
                }
                if let Some(id) = params.get("program_id") {
                    let id = id.as_str().ok_or_else(|| invalid("program_id 无效"))?;
                    self.program_entry(id)?;
                    r.program_id = Some(id.into());
                }
                if params.get("reminder_minutes").is_some() {
                    r.reminder_minutes = integer(params, "reminder_minutes", 120, 0, 10080)?;
                }
                if params.get("auto_stop_minutes").is_some() {
                    let minutes = integer(params, "auto_stop_minutes", 0, 0, 10080)?;
                    if minutes > 0 {
                        if r.binding.is_some()
                            || r.job_name.is_none()
                            || self.service_snapshot(r)["persistent_proxy"] == true
                            || self.service_snapshot(r)["status"] != "running"
                        {
                            return Err(invalid("自动停止仅适用于 ToolHub 正在管理的进程组"));
                        }
                        if params["confirm_force"] != true {
                            return Err(invalid(
                                "需要确认定时结束服务及其子进程，未保存的数据可能丢失",
                            ));
                        }
                        r.auto_stop_at = Some(
                            (chrono::Utc::now() + chrono::Duration::minutes(minutes as i64))
                                .to_rfc3339(),
                        );
                    } else {
                        r.auto_stop_at = None;
                    }
                }
                r.note("用户更新提醒或运行时限");
                self.save_services(&records)?;
                Ok(json!({"saved":true}))
            }
            "service.remove" => {
                if self.service_snapshot(r)["status"] == "running" && r.auto_stop_at.is_some() {
                    return Err(invalid("请先取消自动停止，再移除监控"));
                }
                let r = records.remove(index);
                self.services.jobs.remove(&r.id);
                self.save_services(&records)?;
                Ok(json!({"removed":true,"process_unchanged":true}))
            }
            _ => Err(invalid("unknown service method")),
        }
    }
    #[cfg(windows)]
    fn stop_service_record(&mut self, r: &mut Record, force: bool) -> Result<(), ProtocolError> {
        if self.service_snapshot(r)["persistent_proxy"] == true {
            return Err(invalid("持续运行的代理仅供观察，请在代理软件中管理"));
        }
        if r.job_name.is_some() {
            let job = self
                .service_job(r)?
                .ok_or_else(|| invalid("进程组不可用，不会按 PID 猜测结束"))?;
            if force {
                job.terminate().map_err(|e| invalid(&e))?;
            } else {
                for pid in job.pids().map_err(|e| invalid(&e))? {
                    if let Ok(p) = native::owned_process(pid) {
                        if job.contains(&p) {
                            native::graceful(&p).map_err(|e| invalid(&e))?;
                        }
                    }
                }
            }
        } else {
            let p = native::exact(&r.identity(), force).map_err(|e| invalid(&e))?;
            if force {
                native::force(&p)
            } else {
                native::graceful(&p)
            }
            .map_err(|e| invalid(&e))?;
        }
        if force {
            r.ended_at = Some(chrono::Utc::now().to_rfc3339());
        }
        r.stop_requested = true;
        r.auto_stop_at = None;
        r.note(if force {
            "已请求结束进程（含受管子进程）"
        } else {
            "已请求正常关闭；无窗口服务可能需强制结束"
        });
        self.event(
            "service.stop_requested",
            "explicit controller request or preconfigured runtime deadline",
            json!({"id":r.id,"force":force}),
        )?;
        Ok(())
    }
    #[cfg(windows)]
    pub(super) fn launch_monitored_program(
        &mut self,
        program_id: &str,
    ) -> Result<(String, u32), ProtocolError> {
        self.require_controller()?;
        self.require_feature("program.launch")?;
        let mut records = self.service_records()?;
        self.reconcile_bindings(&mut records)?;
        if records.len() >= 200 {
            return Err(invalid("请先移除已结束的监控记录"));
        }
        for r in records
            .iter()
            .filter(|r| r.program_id.as_deref() == Some(program_id) && !r.archived)
        {
            if matches!(
                self.service_snapshot(r)["status"].as_str(),
                Some("running" | "unavailable")
            ) {
                return Ok((r.id.clone(), r.pid));
            }
        }
        for r in records.iter().filter(|r| {
            !r.archived && r.program_id.as_deref() == Some(program_id) && r.job_name.is_some()
        }) {
            if self
                .service_job(r)?
                .is_some_and(|job| job.pids().is_ok_and(|p| !p.is_empty()))
            {
                return Ok((r.id.clone(), r.pid));
            }
        }
        let active = records
            .iter()
            .filter(|r| r.job_name.is_some())
            .filter(|r| self.service_snapshot(r)["status"] == "running")
            .count();
        if active >= 32 {
            return Err(invalid("同时受管服务最多 32 个，请先停止不需要的服务"));
        }
        let mut entry = self.program_entry(program_id)?;
        if !Path::new(&entry.candidate.cwd).is_dir()
            || (entry.candidate.kind == "file" && !Path::new(&entry.candidate.path).is_file())
        {
            return Err(invalid("程序入口或目录已失效"));
        }
        if Path::new(&entry.candidate.path)
            .extension()
            .is_some_and(|e| e.eq_ignore_ascii_case("lnk"))
        {
            return Err(invalid(
                "快捷方式可能把进程交给已有实例，请改选实际文件或命令入口",
            ));
        }
        let job_name = format!("Local\\ToolHub.Service.{}", uuid::Uuid::new_v4());
        let job = native::Job::create(&job_name).map_err(|e| invalid(&e))?;
        native::start_keeper(&job_name).map_err(|e| invalid(&e))?;
        let pid = super::program_process::spawn_helper_job(
            &serde_json::to_string(&entry).map_err(json_err)?,
            &entry,
            false,
            Some(job.0 .0),
        )
        .map_err(|e| invalid(&e))?;
        let identity = match native::owned_process(pid) {
            Ok(p) => p.identity,
            Err(e) => {
                let _ = job.terminate();
                return Err(invalid(&format!("无法建立受管进程身份：{e}")));
            }
        };
        let previous = records
            .iter()
            .find(|r| r.program_id.as_deref() == Some(program_id) && !r.archived)
            .cloned();
        let id = previous
            .as_ref()
            .map(|r| r.id.clone())
            .unwrap_or_else(|| uuid::Uuid::new_v4().to_string());
        let mut r = Record {
            id: id.clone(),
            name: entry.candidate.name.clone(),
            kind: if ["clash", "mihomo", "nano", "v2ray", "sing-box"]
                .iter()
                .any(|name| entry.candidate.name.to_lowercase().contains(name))
            {
                "proxy"
            } else {
                "other"
            }
            .into(),
            owner: self.peer_principal(),
            source: "toolhub".into(),
            project_dir: entry.candidate.cwd.clone(),
            program_id: Some(program_id.into()),
            web_url: None,
            binding: None,
            archived: false,
            pid,
            created: identity.created,
            executable: identity.executable,
            started_at: chrono::Utc::now().to_rfc3339(),
            ended_at: None,
            job_name: Some(job_name),
            auto_stop_at: None,
            reminder_minutes: 120,
            stop_requested: false,
            history: vec![],
        };
        if let Some(old) = &previous {
            r.web_url = old.web_url.clone();
            r.binding = old.binding.clone();
            r.name = old.name.clone();
            r.history = old.history.clone();
            r.reminder_minutes = old.reminder_minutes;
            r.note(&format!("上次实例 PID {}，已提交新的启动", old.pid));
        }
        records.retain(|old| old.id != id);
        r.note("ToolHub 启动并建立受管进程组");
        records.insert(0, r);
        if let Err(e) = self.save_services(&records) {
            let _ = job.terminate();
            return Err(e);
        }
        self.services.jobs.insert(id.clone(), job);
        entry.launch_count += 1;
        entry.last_launched = Some(chrono::Utc::now().to_rfc3339());
        self.registry
            .db
            .conn
            .execute(
                "UPDATE program_entries SET entry_json=?1 WHERE id=?2",
                rusqlite::params![serde_json::to_string(&entry).map_err(json_err)?, program_id],
            )
            .map_err(sql_err)?;
        self.event(
            "service.started",
            "explicit user launch of a collected program",
            json!({"id":id,"program_id":program_id,"pid":pid}),
        )?;
        Ok((id, pid))
    }
    #[cfg(windows)]
    fn reconcile_bindings(&mut self, records: &mut [Record]) -> Result<(), ProtocolError> {
        let mut changed = false;
        for r in records.iter_mut().filter(|r| !r.archived) {
            if let Some(binding) = &r.binding {
                match binding.find() {
                    Ok(Some(p)) => {
                        self.services.binding_errors.remove(&r.id);
                        if r.pid != p.identity.pid
                            || r.created != p.identity.created
                            || r.job_name.is_some()
                        {
                            self.services.jobs.remove(&r.id);
                            r.pid = p.identity.pid;
                            r.created = p.identity.created;
                            r.executable = p.identity.executable;
                            r.job_name = None;
                            r.auto_stop_at = None;
                            r.ended_at = None;
                            r.stop_requested = false;
                            r.source = "external_binding".into();
                            let ticks = r.created.parse::<i64>().unwrap_or(0);
                            if let Some(time) = chrono::DateTime::from_timestamp(
                                ticks / 10_000_000 - 11_644_473_600,
                                0,
                            ) {
                                r.started_at = time.to_rfc3339();
                            }
                            r.note(&format!("观测到指定的独立进程 PID {}", r.pid));
                            changed = true;
                        }
                    }
                    Ok(None) => {
                        self.services.binding_errors.insert(
                            r.id.clone(),
                            "等待指定的独立进程；启动器运行不代表服务已就绪".into(),
                        );
                    }
                    Err(e) => {
                        self.services.binding_errors.insert(r.id.clone(), e);
                    }
                }
            }
        }
        if changed {
            self.save_services(records)?
        }
        Ok(())
    }
    pub(crate) fn service_monitor_tick(&mut self) -> Result<(), ProtocolError> {
        if !self.module_enabled("services")? {
            return Ok(());
        }
        #[cfg(windows)]
        {
            let mut records = self.service_records()?;
            let mut changed = false;
            for r in &mut records {
                if r.job_name.is_some()
                    && r.ended_at.is_none()
                    && self
                        .service_job(r)
                        .ok()
                        .flatten()
                        .and_then(|job| job.pids().ok())
                        .is_some_and(|pids| pids.is_empty())
                {
                    r.ended_at = Some(chrono::Utc::now().to_rfc3339());
                    changed = true;
                }

                if r.auto_stop_at
                    .as_ref()
                    .and_then(|t| chrono::DateTime::parse_from_rfc3339(t).ok())
                    .is_some_and(|t| t <= chrono::Utc::now())
                {
                    if r.job_name.is_some() {
                        match self.stop_service_record(r, true) {
                            Ok(()) => r.note("已达到用户设定的运行时限"),
                            Err(_) => {
                                r.auto_stop_at = None;
                                r.note("自动停止失败，请手动检查");
                            }
                        }
                    } else {
                        r.auto_stop_at = None;
                    }
                    changed = true;
                }
            }
            if changed {
                self.save_services(&records)?
            }
        }
        Ok(())
    }
}
fn integer(
    params: &Value,
    key: &str,
    default: u64,
    min: u64,
    max: u64,
) -> Result<u64, ProtocolError> {
    match params.get(key) {
        None => Ok(default),
        Some(v) => v
            .as_u64()
            .filter(|v| *v >= min && *v <= max)
            .ok_or_else(|| invalid(&format!("{key} 超出范围"))),
    }
}
