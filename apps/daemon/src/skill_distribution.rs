//! Controller-owned Skill collection and per-host distribution; ordinary agents only offer.
use super::execution_jobs::{invalid, text};
use super::*;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::{fs, path::PathBuf};
use toolhub_skills::distribution as pkg;
const KEY: &str = "skill_distribution_v1";
#[derive(Default, Serialize, Deserialize)]
#[serde(default)]
struct State {
    revision: u64,
    packages: Vec<Collected>,
    targets: Vec<Target>,
    deployments: Vec<Deployment>,
    history: Vec<Value>,
    offers: Vec<Offer>,
}
#[derive(Clone, Serialize, Deserialize)]
struct Collected {
    id: String,
    name: String,
    description: String,
    version: String,
    digest: String,
    source_path: String,
    source_agent: String,
    source_url: String,
    scope: String,
    host_references: Vec<String>,
    files: usize,
    bytes: usize,
    collected_at: String,
}
#[derive(Clone, Serialize, Deserialize)]
struct Target {
    id: String,
    host_id: String,
    name: String,
    root: String,
    custom: bool,
}
#[derive(Clone, Serialize, Deserialize)]
struct Deployment {
    skill_id: String,
    target_id: String,
    digest: String,
    version: String,
    synced_at: String,
}
#[derive(Clone, Serialize, Deserialize)]
struct Offer {
    id: String,
    path: String,
    source_agent: String,
    name: String,
    description: String,
    version: String,
    source_url: String,
    status: String,
    offered_at: String,
}
fn now() -> String {
    chrono::Utc::now().to_rfc3339()
}
fn short_hash(s: &str) -> String {
    format!("{:x}", Sha256::digest(s.as_bytes()))[..24].to_owned()
}
fn path_key(path: &Path) -> String {
    let s = path.to_string_lossy().replace('\\', "/");
    if cfg!(windows) {
        s.to_lowercase()
    } else {
        s
    }
}
fn target_id(host: &str, root: &Path) -> String {
    format!(
        "target.{}",
        short_hash(&format!("{host}:{}", path_key(root)))
    )
}
fn string(p: &Value, k: &str, max: usize) -> Result<String, ProtocolError> {
    let s = p[k].as_str().unwrap_or("");
    if s.len() > max || s.chars().any(char::is_control) {
        Err(invalid("字段超出大小或含控制字符"))
    } else {
        Ok(s.into())
    }
}
fn ioerr(s: String) -> ProtocolError {
    invalid(&s)
}
fn defaults(home: &Path) -> Vec<(&'static str, &'static str, PathBuf)> {
    vec![
        ("codex", "Codex", home.join(".codex/skills")),
        ("mimo", "MiMo", home.join(".config/mimocode/skills")),
        ("claude-code", "Claude Code", home.join(".claude/skills")),
        ("cursor", "Cursor", home.join(".cursor/skills")),
        ("opencode", "OpenCode", home.join(".config/opencode/skills")),
        ("gemini-cli", "Gemini CLI", home.join(".gemini/skills")),
    ]
}
impl DaemonService {
    fn distribution_state(&mut self) -> Result<State, ProtocolError> {
        self.registry
            .get_setting(KEY)
            .map_err(db_err)?
            .map(serde_json::from_value)
            .transpose()
            .map_err(json_err)
            .map(Option::unwrap_or_default)
    }
    fn save_distribution(&mut self, state: &mut State) -> Result<(), ProtocolError> {
        state.revision += 1;
        if state.history.len() > 500 {
            state.history.drain(..state.history.len() - 500);
        }
        self.registry
            .set_setting(KEY, &json!(state))
            .map_err(db_err)
    }
    fn distribution_dir(&self) -> Result<PathBuf, ProtocolError> {
        let path = Path::new(&self.registry_path)
            .parent()
            .ok_or_else(|| invalid("注册表目录不可用"))?
            .join("skill-distribution");
        Ok(path)
    }
    fn distribution_targets(&self, state: &State) -> Vec<Target> {
        let mut rows = state.targets.clone();
        if let Some(home) = dirs::home_dir() {
            for (host, name, root) in defaults(&home) {
                if let Ok(root) = pkg::checked_dir(&root) {
                    if !rows
                        .iter()
                        .any(|r| path_key(Path::new(&r.root)) == path_key(&root))
                    {
                        rows.push(Target {
                            id: target_id(host, &root),
                            host_id: host.into(),
                            name: name.into(),
                            root: root.to_string_lossy().into(),
                            custom: false,
                        });
                    }
                }
            }
        }
        rows.sort_by(|a, b| a.name.cmp(&b.name));
        rows
    }
    fn deployment_summary(&self, state: &State, p: &Collected, t: &Target) -> Value {
        if (p.scope == "host_specific"
            && (p.source_agent.is_empty() || p.source_agent != t.host_id))
            || p.host_references
                .iter()
                .any(|v| v == "skill.json 平台不适用")
        {
            return json!({"target_id":t.id,"name":t.name,"status":"incompatible","can_sync":false});
        }
        let old = state
            .deployments
            .iter()
            .find(|d| d.skill_id == p.id && d.target_id == t.id);
        json!({"target_id":t.id,"name":t.name,"path":Path::new(&t.root).join(&p.name),"status":match old {Some(d) if d.digest==p.digest=>"recorded",Some(_)=>"update_available",None=>"not_distributed"},"can_sync":old.is_none_or(|d|d.digest!=p.digest),"synced_at":old.map(|d|&d.synced_at),"loaded":null,"load_status":"unknown","verification":"打开条目或同步时重新检查目标文件"})
    }
    fn deployment_status(&self, state: &State, p: &Collected, t: &Target) -> Value {
        let mut view = json!({"target_id":t.id,"host_id":t.host_id,"name":t.name,"root":t.root,"path":Path::new(&t.root).join(&p.name),"status":"not_distributed","can_sync":true,"reason":"","loaded":null,"load_status":"unknown"});
        if p.scope == "host_specific" && (p.source_agent.is_empty() || p.source_agent != t.host_id)
        {
            view["status"] = json!("incompatible");
            view["can_sync"] = json!(false);
            view["reason"] = json!("此 Skill 依赖来源宿主，不能分发到其他智能体");
            return view;
        }
        if p.host_references
            .iter()
            .any(|v| v == "skill.json 平台不适用")
        {
            view["status"] = json!("incompatible");
            view["can_sync"] = json!(false);
            view["reason"] = json!("Skill 声明不支持当前平台");
            return view;
        }
        let root = match pkg::checked_dir(Path::new(&t.root)) {
            Ok(p) => p,
            Err(e) => {
                view["status"] = json!("target_unavailable");
                view["can_sync"] = json!(false);
                view["reason"] = json!(e);
                return view;
            }
        };
        let dest = root.join(&p.name);
        let previous = state
            .deployments
            .iter()
            .find(|d| d.skill_id == p.id && d.target_id == t.id);
        if let Some(d) = previous {
            view["synced_at"] = json!(d.synced_at);
            view["deployed_version"] = json!(d.version);
        }
        if !dest.exists() && fs::symlink_metadata(&dest).is_err() {
            if previous.is_some() {
                view["status"] = json!("missing");
            }
            return view;
        }
        match pkg::read_package(&dest) {
            Ok(actual) => {
                view["actual_digest"] = json!(actual.digest);
                if actual.digest == p.digest {
                    view["status"] = json!(if previous.is_some() {
                        "written"
                    } else {
                        "present"
                    });
                    view["can_sync"] = json!(false);
                } else if previous.is_some_and(|d| d.digest == actual.digest) {
                    view["status"] = json!("update_available");
                } else {
                    view["status"] = json!("conflict");
                    view["can_sync"] = json!(false);
                    view["reason"] =
                        json!("同名目录内容不同或已有本地修改，已保留；请手动合并后重新收录");
                }
            }
            Err(e) => {
                view["status"] = json!("conflict");
                view["can_sync"] = json!(false);
                view["reason"] = json!(format!("已有目录不可安全核对，已保留：{e}"));
            }
        }
        view
    }
    pub(super) fn dispatch_skill_distribution(
        &mut self,
        method: &str,
        params: &Value,
    ) -> Result<Value, ProtocolError> {
        let allowed: &[&str] = match method {
            "skill.offer" => &["path", "source_agent", "source_url", "version"],
            "skill.distribution.list" => &["id"],
            "skill.distribution.discover" => &["target_id"],
            "skill.distribution.preview" => &["path"],
            "skill.distribution.collect" => &[
                "revision",
                "path",
                "source_agent",
                "source_url",
                "version",
                "scope",
                "expected_digest",
                "previous_digest",
            ],
            "skill.distribution.sync" => &["revision", "id", "expected_digest", "target_ids"],
            "skill.distribution.target" => &["revision", "root", "host_id", "name"],
            "skill.distribution.target_remove" | "skill.distribution.dismiss" => {
                &["revision", "id"]
            }
            _ => return Err(invalid("unknown Skill distribution method")),
        };
        let obj = params
            .as_object()
            .ok_or_else(|| invalid("object params required"))?;
        if obj.keys().any(|k| !allowed.contains(&k.as_str())) {
            return Err(invalid("unknown Skill distribution parameter"));
        }
        for (k, v) in obj {
            if !["revision", "target_ids"].contains(&k.as_str()) && !v.is_string() {
                return Err(invalid("Skill distribution text fields must be strings"));
            }
        }

        if method == "skill.offer" {
            return self.offer_skill(params);
        }
        self.require_controller()?;
        let mut state = self.distribution_state()?;
        if method == "skill.distribution.list" {
            return self.distribution_list(&state, params["id"].as_str());
        }
        if method == "skill.distribution.discover" {
            return self.distribution_discover(&state, params);
        }
        if method == "skill.distribution.preview" {
            let path = Path::new(text(params, "path")?);
            let package = pkg::read_package(path).map_err(ioerr)?;
            return Ok(
                json!({"metadata":package.metadata,"digest":package.digest,"files":package.files.keys().collect::<Vec<_>>(),"bytes":package.bytes}),
            );
        }
        if params["revision"].as_u64() != Some(state.revision) {
            return Err(invalid("同步目录已变更，请刷新后重试"));
        }
        match method {
            "skill.distribution.target" => {
                if state.targets.len() >= 32 {
                    return Err(invalid("自定义目标最多 32 个"));
                }
                let root = pkg::checked_dir(Path::new(text(params, "root")?)).map_err(ioerr)?;
                if root.parent().is_none()
                    || root == self.distribution_dir()?
                    || root.starts_with(self.distribution_dir()?)
                    || root.join("SKILL.md").exists()
                {
                    return Err(invalid(
                        "请选择专用的 Skill 根目录，而不是磁盘根或单个 Skill",
                    ));
                }
                let home = dirs::home_dir().unwrap_or_default();
                if root == home {
                    return Err(invalid("不能将用户主目录作为 Skill 根目录"));
                }
                let host = string(params, "host_id", 64)?;
                let name = string(params, "name", 128)?;
                if !pkg::safe_name(&host) || name.trim().is_empty() {
                    return Err(invalid("需要有效的智能体标识与显示名"));
                }
                if self.distribution_targets(&state).iter().any(|t| {
                    let p = Path::new(&t.root);
                    root.starts_with(p) || p.starts_with(&root)
                }) {
                    return Err(invalid("该目录与已有目标重复或嵌套"));
                }
                let target = Target {
                    id: target_id(&host, &root),
                    host_id: host,
                    name,
                    root: root.to_string_lossy().into(),
                    custom: true,
                };
                state.history.push(json!({"at":now(),"action":"target_added","target":target.name,"target_id":target.id,"path":target.root,"status":"registered"}));
                state.targets.push(target);
                self.save_distribution(&mut state)?;
            }
            "skill.distribution.target_remove" => {
                let id = text(params, "id")?;
                let target = state
                    .targets
                    .iter()
                    .find(|t| t.id == id)
                    .cloned()
                    .ok_or_else(|| invalid("只能移除手动登记的目标"))?;
                state.targets.retain(|t| t.id != id);
                state.history.push(json!({"at":now(),"action":"target_removed","target":target.name,"target_id":id,"status":"files_preserved"}));
                self.save_distribution(&mut state)?;
            }
            "skill.distribution.dismiss" => {
                let id = text(params, "id")?;
                let o = state
                    .offers
                    .iter_mut()
                    .find(|o| o.id == id)
                    .ok_or_else(|| invalid("待同步条目不存在"))?;
                o.status = "dismissed".into();
                self.save_distribution(&mut state)?;
            }
            "skill.distribution.collect" => {
                self.collect_skill(&mut state, params)?;
            }
            "skill.distribution.sync" => {
                return self.sync_skill(&mut state, params);
            }
            _ => {
                return Err(ProtocolError::new(
                    ErrorCode::MethodNotFound,
                    "unknown skill distribution method",
                ))
            }
        }
        self.distribution_list(&state, None)
    }
    fn distribution_list(
        &self,
        state: &State,
        selected: Option<&str>,
    ) -> Result<Value, ProtocolError> {
        let targets = self.distribution_targets(state);
        let packages: Vec<Value> = state
            .packages
            .iter()
            .map(|p| {
                let mut v = json!(p);
                v["targets"] = json!(targets
                    .iter()
                    .map(|t| if selected == Some(p.id.as_str()) {
                        self.deployment_status(state, p, t)
                    } else {
                        self.deployment_summary(state, p, t)
                    })
                    .collect::<Vec<_>>());
                v
            })
            .collect();
        Ok(
            json!({"revision":state.revision,"packages":packages,"targets":targets,"offers":state.offers.iter().filter(|o|o.status=="pending").collect::<Vec<_>>(),"history":state.history.iter().rev().map(|h| {
            let mut h=h.clone();
            if h["action"]=="sync" && h.get("load_status").is_none() {
                h["loaded"]=Value::Null;h["load_status"]=json!("unknown");
            }
            h
        }).collect::<Vec<_>>(),"history_limit":500}),
        )
    }
    fn distribution_discover(&self, state: &State, params: &Value) -> Result<Value, ProtocolError> {
        let targets = self.distribution_targets(state);
        let mut rows = vec![];
        let mut errors = vec![];
        for t in targets
            .iter()
            .filter(|t| params["target_id"].as_str().is_none_or(|id| t.id == id))
        {
            let root = match pkg::checked_dir(Path::new(&t.root)) {
                Ok(p) => p,
                Err(e) => {
                    errors.push(json!({"target":t.name,"error":e}));
                    continue;
                }
            };
            match fs::read_dir(root) {
                Ok(entries) => {
                    for e in entries.take(201) {
                        let Ok(e) = e else {
                            continue;
                        };
                        if rows.len() >= 200 {
                            errors.push(json!({"target":t.name,"error":"本次最多展示 200 个候选，请按智能体筛选"}));
                            break;
                        }
                        let path = e.path();
                        if e.file_name().to_string_lossy().starts_with('.') {
                            continue;
                        }
                        if let Ok(m) = pkg::preview_directory(&path) {
                            let collected = state
                                .packages
                                .iter()
                                .any(|p| path_key(Path::new(&p.source_path)) == path_key(&path));
                            rows.push(json!({"path":path,"source_agent":t.host_id,"source_name":t.name,"name":m.name,"description":m.description,"version":m.version,"host_references":m.host_references,"collected":collected}));
                        }
                    }
                }
                Err(e) => errors.push(json!({"target":t.name,"error":e.to_string()})),
            }
        }
        Ok(json!({"candidates":rows,"errors":errors,"limit":200}))
    }
    fn offer_skill(&mut self, params: &Value) -> Result<Value, ProtocolError> {
        let path = pkg::checked_dir(Path::new(text(params, "path")?)).map_err(ioerr)?;
        let m = pkg::preview_directory(&path).map_err(ioerr)?;
        let source_agent = string(params, "source_agent", 64)?;
        if !pkg::safe_name(&source_agent) {
            return Err(invalid(
                "source_agent 使用宿主标识，例如 codex、mimo 或 dsh",
            ));
        }
        let source_url = string(params, "source_url", 2048)?;
        let version = string(params, "version", 128)?;
        let id = format!(
            "offer.{}",
            short_hash(&format!(
                "{}:{}:{}",
                path_key(&path),
                source_agent,
                format!(
                    "{:x}",
                    Sha256::digest(
                        pkg::read_bounded(&path.join("SKILL.md"), 65536).map_err(ioerr)?
                    )
                )
            ))
        );
        let mut state = self.distribution_state()?;
        let already = state.offers.iter().any(|o| o.id == id);
        if !already {
            if state.offers.len() >= 100 {
                if let Some(i) = state.offers.iter().position(|o| o.status != "pending") {
                    state.offers.remove(i);
                } else {
                    return Err(invalid("待确认队列已满，请在桌面端处理"));
                }
            }
            state.offers.push(Offer {
                id: id.clone(),
                path: path.to_string_lossy().into(),
                source_agent,
                name: m.name,
                description: m.description,
                version: if version.is_empty() {
                    m.version
                } else {
                    version
                },
                source_url,
                status: "pending".into(),
                offered_at: now(),
            });
            self.save_distribution(&mut state)?;
        }
        Ok(
            json!({"id":id,"status":if already{"already_offered"}else{"offered"},"blocking":false,"host_files_written":false,"next":"提醒用户到 ToolHub 的 Skill 库 → 已安装 / 待同步自行选择收录与同步。不要自动操作界面或选择目标，继续原任务，不等待确认。"}),
        )
    }
    fn collect_skill(&mut self, state: &mut State, params: &Value) -> Result<(), ProtocolError> {
        let path = pkg::checked_dir(Path::new(text(params, "path")?)).map_err(ioerr)?;
        let package = pkg::read_package(&path).map_err(ioerr)?;
        if params["expected_digest"] != package.digest {
            return Err(invalid("来源内容已变更，请重新预览后收录"));
        }
        let source_agent = string(params, "source_agent", 64)?;
        if !source_agent.is_empty() && !pkg::safe_name(&source_agent) {
            return Err(invalid("无效的来源宿主标识"));
        }
        let scope = text(params, "scope")?;
        if !["portable", "host_specific"].contains(&scope) {
            return Err(invalid("请选择通用或宿主专属"));
        }
        if scope == "portable" && !package.metadata.host_references.is_empty() {
            return Err(invalid("发现宿主引用或平台限制，只能按宿主专属收录"));
        }
        if scope == "host_specific" && source_agent.is_empty() {
            return Err(invalid("宿主专属 Skill 必须填写来源宿主标识"));
        }
        let id = format!("shared.{}", package.metadata.name);
        if self
            .registry
            .list_skills()
            .map_err(db_err)?
            .iter()
            .any(|row| row.0 == id)
            || self.extension_skill(&id)?.is_some()
        {
            return Err(invalid("Skill ID 已属于其他流程，保留既有内容"));
        }
        if let Some(old) = state.packages.iter().find(|p| p.id == id) {
            if old.digest == package.digest && old.scope == scope {
                return Ok(());
            }
            if params["previous_digest"] != old.digest {
                return Err(invalid("已有同名版本；请从原条目更新并保留现有来源"));
            }
            if path_key(&path) != path_key(Path::new(&old.source_path)) {
                return Err(invalid("同名 Skill 来自不同目录，请先人工合并来源"));
            }
        } else if state.packages.len() >= 200 {
            return Err(invalid("本机同步库最多 200 个 Skill"));
        }
        let base = self.distribution_dir()?.join("snapshots");
        fs::create_dir_all(&base).map_err(|e| invalid(&e.to_string()))?;
        let base = pkg::checked_dir(&base).map_err(ioerr)?;
        let snapshot = base.join(&package.digest);
        if snapshot.exists() {
            if pkg::read_package(&snapshot).map_err(ioerr)?.digest != package.digest {
                return Err(invalid("已有快照校验失败，未覆盖"));
            }
        } else {
            pkg::tree_bytes(&base, 256 * 1024 * 1024 - package.bytes as u64).map_err(ioerr)?;
            let stage = base.join(format!(".stage-{}", uuid::Uuid::new_v4()));
            pkg::write_new(&stage, &package).map_err(ioerr)?;
            fs::rename(&stage, &snapshot).map_err(|e| invalid(&e.to_string()))?;
        }
        let version = string(params, "version", 128)?;
        let source_url = string(params, "source_url", 2048)?;
        let p = Collected {
            id: id.clone(),
            name: package.metadata.name,
            description: package.metadata.description,
            version: if version.is_empty() {
                if package.metadata.version.is_empty() {
                    package.digest[..12].into()
                } else {
                    package.metadata.version
                }
            } else {
                version
            },
            digest: package.digest,
            source_path: path.to_string_lossy().into(),
            source_agent,
            source_url,
            scope: scope.into(),
            host_references: package.metadata.host_references,
            files: package.files.len(),
            bytes: package.bytes,
            collected_at: now(),
        };
        state.history.push(json!({"at":now(),"action":"collected","skill":p.name,"skill_id":p.id,"version":p.version,"digest":p.digest,"status":"saved","path":p.source_path}));
        state.packages.retain(|old| old.id != id);
        state.packages.push(p);
        for o in &mut state.offers {
            if path_key(Path::new(&o.path)) == path_key(&path) {
                o.status = "collected".into();
            }
        }
        self.save_distribution(state)
    }
    fn sync_skill(&mut self, state: &mut State, params: &Value) -> Result<Value, ProtocolError> {
        let id = text(params, "id")?;
        let p = state
            .packages
            .iter()
            .find(|p| p.id == id)
            .cloned()
            .ok_or_else(|| invalid("Skill 尚未收录"))?;
        if params["expected_digest"] != p.digest {
            return Err(invalid("Skill 已更新，请刷新后同步"));
        }
        let ids = params["target_ids"]
            .as_array()
            .filter(|v| !v.is_empty() && v.len() <= 32)
            .ok_or_else(|| invalid("请选择 1–32 个目标"))?;
        let targets = self.distribution_targets(state);
        let mut chosen = Vec::new();
        for id in ids {
            let t = targets
                .iter()
                .find(|t| Some(t.id.as_str()) == id.as_str())
                .ok_or_else(|| invalid("同步目标不存在"))?;
            if !chosen.iter().any(|x: &Target| x.id == t.id) {
                chosen.push(t.clone());
            }
        }
        let package =
            pkg::read_package(&self.distribution_dir()?.join("snapshots").join(&p.digest))
                .map_err(ioerr)?;
        if package.digest != p.digest {
            return Err(invalid("Skill 快照校验失败，停止分发"));
        }
        let mut results = vec![];
        for t in chosen {
            let status = self.deployment_status(state, &p, &t);
            if status["can_sync"] != true {
                results.push(status.clone());
                state.history.push(json!({"at":now(),"action":"sync","skill":p.name,"skill_id":p.id,"target":t.name,"target_id":t.id,"version":p.version,"digest":p.digest,"status":status["status"],"reason":status["reason"]}));
                continue;
            }
            let result = self.write_deployment(&p, &t, &package, &status);
            let (status, reason) = match result {
                Ok(()) => {
                    state
                        .deployments
                        .retain(|d| d.skill_id != p.id || d.target_id != t.id);
                    state.deployments.push(Deployment {
                        skill_id: p.id.clone(),
                        target_id: t.id.clone(),
                        digest: p.digest.clone(),
                        version: p.version.clone(),
                        synced_at: now(),
                    });
                    ("written", String::new())
                }
                Err(e) => ("failed", e),
            };
            let row = json!({"at":now(),"action":"sync","skill":p.name,"skill_id":p.id,"target":t.name,"target_id":t.id,"path":Path::new(&t.root).join(&p.name),"version":p.version,"digest":p.digest,"status":status,"reason":reason,"loaded":null,"load_status":"unknown"});
            state.history.push(row.clone());
            results.push(row);
            // Persist each result; a later target failure never erases earlier outcomes.
            self.save_distribution(state)?;
        }
        self.save_distribution(state)?;
        Ok(
            json!({"results":results,"inventory":self.distribution_list(state,Some(&p.id))?,"note":"同步结果已记录。若目标智能体尚未显示新 Skill，可刷新技能列表或新开会话；已能找到时无需操作。"}),
        )
    }
    fn write_deployment(
        &self,
        p: &Collected,
        t: &Target,
        package: &pkg::Package,
        status: &Value,
    ) -> Result<(), String> {
        let root = pkg::checked_dir(Path::new(&t.root))?;
        let dest = root.join(&p.name);
        let work = root
            .parent()
            .ok_or("目标缺少父目录")?
            .join(".toolhub-skill-staging");
        fs::create_dir_all(&work).map_err(|e| e.to_string())?;
        let work = pkg::checked_dir(&work)?;
        pkg::tree_bytes(&work, 64 * 1024 * 1024 - 2 * package.bytes as u64)?;
        let stage = work.join(format!("stage-{}", uuid::Uuid::new_v4()));
        pkg::write_new(&stage, package)?;
        if pkg::read_package(&stage)?.digest != p.digest {
            return Err("写入临时包校验失败".into());
        }
        pkg::checked_dir(&root)?;
        if status["status"] == "update_available" {
            if pkg::read_package(&dest)?.digest != status["actual_digest"].as_str().unwrap_or("") {
                return Err("目标在同步期间发生修改，已保留".into());
            }
            let backup = work.join(format!("backup-{}", uuid::Uuid::new_v4()));
            // Durable local recovery record before replacing a previously managed copy.
            let recovery = work.join(format!("recovery-{}.json", uuid::Uuid::new_v4()));
            fs::write(&recovery,json!({"target":dest,"backup":backup,"incoming":stage,"old_digest":status["actual_digest"],"new_digest":p.digest}).to_string()).map_err(|e|e.to_string())?;
            fs::rename(&dest, &backup).map_err(|e| e.to_string())?;
            if let Err(e) = fs::rename(&stage, &dest) {
                let _ = fs::rename(&backup, &dest);
                return Err(format!("替换失败，已尝试恢复原目录：{e}"));
            }
            // Backups are retained outside the host's Skill root, never permanently deleted.
        } else {
            if fs::symlink_metadata(&dest).is_ok() {
                return Err("目标目录已出现，保留已有内容".into());
            }
            fs::rename(&stage, &dest).map_err(|e| e.to_string())?;
        }
        if pkg::read_package(&dest)?.digest != p.digest {
            return Err("落盘后校验失败，请检查目标目录".into());
        }
        Ok(())
    }
}

impl DaemonService {
    pub(super) fn distributed_inventory(&mut self) -> Result<Vec<Value>, ProtocolError> {
        let state = self.distribution_state()?;
        state
            .packages
            .iter()
            .map(|p| self.distributed_view(p, false))
            .collect()
    }
    pub(super) fn inspect_distributed(&mut self, id: &str) -> Result<Option<Value>, ProtocolError> {
        let state = self.distribution_state()?;
        state
            .packages
            .iter()
            .find(|p| p.id == id)
            .map(|p| self.distributed_view(p, true))
            .transpose()
    }
    fn distributed_view(&mut self, p: &Collected, body: bool) -> Result<Value, ProtocolError> {
        let reusable = p.scope == "portable" && p.host_references.is_empty();
        let mut view = json!({"id":p.id,"name":p.name,"schema":"toolhub.skill/v1","version":p.version,
            "manifest":{"id":p.id,"name":p.name,"description":p.description,"requires":[],"optional":[]},
            "compatibility":{"reusable":reusable,"status":p.scope,"issues":p.host_references},
            "status":"workflow_only","dependencies_declared":false,"instruction_authority":false});
        if body && reusable {
            let package =
                pkg::read_package(&self.distribution_dir()?.join("snapshots").join(&p.digest))
                    .map_err(ioerr)?;
            if package.digest != p.digest {
                return Err(invalid("Skill snapshot integrity check failed"));
            }
            if let Some(bytes) = package.files.get("skill.json") {
                let manifest: Value = serde_json::from_slice(bytes).map_err(json_err)?;
                let skill = toolhub_core::SkillManifest::validate_json(&manifest)
                    .map_err(|e| invalid(&e.to_string()))?;
                // Reuse the existing capability resolver; retain the distribution identity.
                if let Ok(mut resolved)=self.preview_skill(&json!({"manifest":manifest,"instructions":String::from_utf8_lossy(&package.files["SKILL.md"])})) {
                    resolved["id"]=json!(p.id);resolved["name"]=json!(p.name);resolved["dependencies_declared"]=json!(true);view=resolved;
                }
                let _ = skill;
            }
            view["instructions"] = json!(String::from_utf8_lossy(&package.files["SKILL.md"]));
            view["supporting_files"] = json!(package
                .files
                .keys()
                .filter(|k| k.as_str() != "SKILL.md")
                .collect::<Vec<_>>());
            view["package_path"] =
                json!(self.distribution_dir()?.join("snapshots").join(&p.digest));
        }
        Ok(view)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    // macOS exposes TMPDIR through /var -> /private/var. Fixtures must use
    // the physical parent; production continues rejecting every symlink ancestor.
    fn fixture_dir() -> tempfile::TempDir {
        let parent = std::env::temp_dir().canonicalize().unwrap();
        tempfile::tempdir_in(parent).unwrap()
    }
    fn source(path: &Path, name: &str, body: &str) {
        fs::create_dir_all(path.join("references")).unwrap();
        fs::write(
            path.join("SKILL.md"),
            format!("---\nname: {name}\ndescription: Fixture workflow\n---\n{body}"),
        )
        .unwrap();
        fs::write(path.join("references/readme.md"), "support data").unwrap();
    }
    fn service(path: &Path) -> DaemonService {
        let mut s = DaemonService::open(&path.join("registry.sqlite")).unwrap();
        s.controller_verified = true;
        s.connection_principal = Some("desktop.fixture".into());
        s
    }
    fn revision(s: &mut DaemonService) -> u64 {
        s.distribution_state().unwrap().revision
    }
    fn collect(s: &mut DaemonService, source: &Path, scope: &str, previous: &str) -> Value {
        let p = pkg::read_package(source).unwrap();
        let r = revision(s);
        s.dispatch_skill_distribution("skill.distribution.collect",&json!({"revision":r,"path":source,"source_agent":"codex","scope":scope,"expected_digest":p.digest,"previous_digest":previous})).unwrap()
    }
    fn target(s: &mut DaemonService, path: &Path, host: &str) -> String {
        fs::create_dir_all(path).unwrap();
        let r = revision(s);
        s.dispatch_skill_distribution(
            "skill.distribution.target",
            &json!({"revision":r,"root":path,"host_id":host,"name":host}),
        )
        .unwrap();
        s.distribution_state()
            .unwrap()
            .targets
            .last()
            .unwrap()
            .id
            .clone()
    }
    fn sync(s: &mut DaemonService, id: &str, ids: &[String]) -> Value {
        let st = s.distribution_state().unwrap();
        let p = st.packages.iter().find(|p| p.id == id).unwrap();
        s.dispatch_skill_distribution(
            "skill.distribution.sync",
            &json!({"revision":st.revision,"id":id,"expected_digest":p.digest,"target_ids":ids}),
        )
        .unwrap()
    }
    #[test]
    fn installed_offer_never_copies_or_blocks_and_controller_gate() {
        let d = fixture_dir();
        let src = d.path().join("source");
        source(&src, "test-flow", "Read reference");
        let mut s = service(d.path());
        s.controller_verified = false;
        let out = s
            .dispatch_skill_distribution("skill.offer", &json!({"path":src,"source_agent":"codex"}))
            .unwrap();
        assert_eq!(out["blocking"], false);
        assert_eq!(out["host_files_written"], false);
        assert!(s.distribution_state().unwrap().packages.is_empty());
        let rev = revision(&mut s);
        assert!(s
            .dispatch_skill_distribution(
                "skill.distribution.collect",
                &json!({"revision":rev,"path":src,"scope":"portable"})
            )
            .is_err());
        assert!(s
            .dispatch_skill_distribution("skill.distribution.list", &json!({}))
            .is_err());
        assert_eq!(
            s.dispatch_skill_distribution(
                "skill.offer",
                &json!({"path":src,"source_agent":"codex"})
            )
            .unwrap()["status"],
            "already_offered"
        );
        assert!(s
            .dispatch_skill_distribution(
                "skill.offer",
                &json!({"path":src,"source_agent":"codex","force":true})
            )
            .is_err());
        assert!(!d.path().join("skill-distribution").exists());
    }
    #[test]
    fn legacy_load_flag_is_unknown_without_rewriting_saved_history() {
        let d = fixture_dir();
        let mut s = service(d.path());
        let mut state = s.distribution_state().unwrap();
        state.history.push(
            json!({"action":"sync","status":"written","loaded":false,"at":"2026-10-24T00:00:00Z"}),
        );
        s.save_distribution(&mut state).unwrap();
        let before = s.registry.get_setting(KEY).unwrap();
        let view = s
            .dispatch_skill_distribution("skill.distribution.list", &json!({}))
            .unwrap();
        assert!(view["history"][0]["loaded"].is_null());
        assert_eq!(view["history"][0]["load_status"], "unknown");
        assert_eq!(view["history"][0]["status"], "written");
        assert_eq!(s.registry.get_setting(KEY).unwrap(), before);
    }
    #[test]
    fn full_package_multi_host_reopen_and_new_host_backfill() {
        let d = fixture_dir();
        let src = d.path().join("source");
        source(&src, "test-flow", "Read reference");
        let mut s = service(d.path());
        collect(&mut s, &src, "portable", "");
        let a = target(&mut s, &d.path().join("codex-skills"), "codex");
        let b = target(&mut s, &d.path().join("mimo-skills"), "mimo");
        let out = sync(&mut s, "shared.test-flow", &[a, b]);
        assert_eq!(out["results"][0]["status"], "written");
        assert!(out["results"][0]["loaded"].is_null());
        assert_eq!(out["results"][0]["load_status"], "unknown");
        assert_eq!(out["results"][1]["status"], "written");
        assert_eq!(
            fs::read(d.path().join("mimo-skills/test-flow/references/readme.md")).unwrap(),
            b"support data"
        );
        drop(s);
        let mut s = service(d.path());
        assert_eq!(s.distribution_state().unwrap().deployments.len(), 2);
        let c = target(&mut s, &d.path().join("tomorrow-dsh"), "dsh");
        let out = sync(&mut s, "shared.test-flow", &[c]);
        assert_eq!(out["results"][0]["status"], "written");
        assert_eq!(s.distribution_state().unwrap().deployments.len(), 3);
        let view = s.inspect_distributed("shared.test-flow").unwrap().unwrap();
        assert_eq!(view["compatibility"]["reusable"], true);
        assert!(view["instructions"]
            .as_str()
            .unwrap()
            .contains("Read reference"));
        assert_eq!(view["dependencies_declared"], false);
    }
    #[test]
    fn stale_revision_source_guard_and_local_edits_survive() {
        let d = fixture_dir();
        let src = d.path().join("source");
        source(&src, "test-flow", "v1");
        let mut s = service(d.path());
        collect(&mut s, &src, "portable", "");
        let a = target(&mut s, &d.path().join("a"), "mimo");
        sync(&mut s, "shared.test-flow", &[a.clone()]);
        fs::write(
            d.path().join("a/test-flow/references/readme.md"),
            "user edit",
        )
        .unwrap();
        let old = s.distribution_state().unwrap().packages[0].digest.clone();
        source(&src, "test-flow", "v2");
        collect(&mut s, &src, "portable", &old);
        let out = sync(&mut s, "shared.test-flow", &[a]);
        assert_eq!(out["results"][0]["status"], "conflict");
        assert_eq!(
            fs::read_to_string(d.path().join("a/test-flow/references/readme.md")).unwrap(),
            "user edit"
        );
        let p = pkg::read_package(&src).unwrap();
        let rev = revision(&mut s);
        assert!(s
            .dispatch_skill_distribution(
                "skill.distribution.collect",
                &json!({"revision":rev-1,"path":src,"scope":"portable","expected_digest":p.digest})
            )
            .is_err());
        source(&src, "test-flow", "v3");
        assert!(s
            .dispatch_skill_distribution(
                "skill.distribution.collect",
                &json!({"revision":rev,"path":src,"scope":"portable","expected_digest":p.digest})
            )
            .is_err());
    }
    #[test]
    fn managed_update_keeps_backup_and_duplicate_is_idempotent() {
        let d = fixture_dir();
        let src = d.path().join("source");
        source(&src, "test-flow", "v1");
        let mut s = service(d.path());
        collect(&mut s, &src, "portable", "");
        let a = target(&mut s, &d.path().join("targets"), "mimo");
        sync(&mut s, "shared.test-flow", &[a.clone()]);
        let out = sync(&mut s, "shared.test-flow", &[a.clone()]);
        assert_eq!(out["results"][0]["status"], "written");
        let old = s.distribution_state().unwrap().packages[0].digest.clone();
        source(&src, "test-flow", "v2");
        collect(&mut s, &src, "portable", &old);
        assert_eq!(
            sync(&mut s, "shared.test-flow", &[a])["results"][0]["status"],
            "written"
        );
        assert!(
            fs::read_to_string(d.path().join("targets/test-flow/SKILL.md"))
                .unwrap()
                .contains("v2")
        );
        assert!(fs::read_dir(d.path().join(".toolhub-skill-staging"))
            .unwrap()
            .flatten()
            .any(|e| e.file_name().to_string_lossy().starts_with("backup-")));
    }
    #[test]
    fn host_specific_and_unmanaged_conflicts_never_overwrite() {
        let d = fixture_dir();
        let src = d.path().join("source");
        source(&src, "test-flow", "Call image_gen");
        let mut s = service(d.path());
        let p = pkg::read_package(&src).unwrap();
        let rev = revision(&mut s);
        assert!(s.dispatch_skill_distribution("skill.distribution.collect",&json!({"revision":rev,"path":src,"source_agent":"codex","scope":"portable","expected_digest":p.digest})).is_err());
        collect(&mut s, &src, "host_specific", "");
        let a = target(&mut s, &d.path().join("mimo"), "mimo");
        assert_eq!(
            sync(&mut s, "shared.test-flow", &[a])["results"][0]["status"],
            "incompatible"
        );
        assert!(!d.path().join("mimo/test-flow").exists());
        let a = target(&mut s, &d.path().join("codex"), "codex");
        source(&d.path().join("codex/test-flow"), "test-flow", "user owned");
        assert_eq!(
            sync(&mut s, "shared.test-flow", &[a])["results"][0]["status"],
            "conflict"
        );
        assert!(
            !s.distributed_inventory().unwrap()[0]["compatibility"]["reusable"]
                .as_bool()
                .unwrap()
        );
    }
    #[test]
    fn target_unavailable_tampered_snapshot_and_bounded_history() {
        let d = fixture_dir();
        let src = d.path().join("source");
        source(&src, "test-flow", "v1");
        let mut s = service(d.path());
        collect(&mut s, &src, "portable", "");
        let a = target(&mut s, &d.path().join("a"), "dsh");
        fs::remove_dir(d.path().join("a")).unwrap();
        assert_eq!(
            sync(&mut s, "shared.test-flow", &[a.clone()])["results"][0]["status"],
            "target_unavailable"
        );
        fs::create_dir(d.path().join("a")).unwrap();
        let p = &s.distribution_state().unwrap().packages[0];
        fs::write(
            s.distribution_dir()
                .unwrap()
                .join("snapshots")
                .join(&p.digest)
                .join("SKILL.md"),
            "tampered",
        )
        .unwrap();
        let rev = revision(&mut s);
        assert!(s.dispatch_skill_distribution("skill.distribution.sync",&json!({"revision":rev,"id":"shared.test-flow","expected_digest":p.digest,"target_ids":[a]})).is_err());
        assert!(!d.path().join("a/test-flow").exists());
        let mut st = s.distribution_state().unwrap();
        st.history = vec![json!({"status":"fixture"}); 550];
        s.save_distribution(&mut st).unwrap();
        assert_eq!(s.distribution_state().unwrap().history.len(), 500);
    }
    #[cfg(windows)]
    #[test]
    fn directory_junction_is_rejected_at_source_and_target() {
        // Symbolic links require a privileged test environment; existing link metadata is handled
        // by both FILE_ATTRIBUTE_REPARSE_POINT and is_symlink, not followed by the walker.
        let d = fixture_dir();
        source(&d.path().join("source"), "test-flow", "body");
        if std::os::windows::fs::symlink_dir(d.path().join("source"), d.path().join("link")).is_ok()
        {
            assert!(pkg::read_package(&d.path().join("link")).is_err());
            assert!(pkg::checked_dir(&d.path().join("link")).is_err());
        }
    }
}
