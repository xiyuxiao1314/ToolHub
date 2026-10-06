//! Explicit user launch entries. Metadata proposals are allowed from Agents;
//! selecting, saving and launching require an OS-verified controller.
use super::execution_jobs::{invalid, text};
use super::*;
use serde::{Deserialize, Serialize};
use std::sync::{atomic::Ordering, Mutex};
use toolhub_scanner::programs::{self, ProgramCandidate, ProgramScan};

#[derive(Default)]
pub struct ProgramRuntime {
    scan: Option<(Arc<Mutex<ProgramScan>>, Arc<AtomicBool>)>,
    pub(super) selections: BTreeMap<String, (String, ProgramCandidate)>,
}

#[derive(Clone, Serialize, Deserialize)]
pub(super) struct ProgramEntry {
    #[serde(flatten)]
    pub(super) candidate: ProgramCandidate,
    pub(super) args: Vec<String>,
    #[serde(default)]
    pub(super) metadata: toolhub_core::program_metadata::ProgramMetadata,
    favorite: bool,
    last_launched: Option<String>,
    launch_count: u64,
}

pub(super) fn identity(c: &ProgramCandidate) -> String {
    let normalize = |value: &str| {
        if cfg!(windows) {
            value.to_lowercase().replace('/', "\\")
        } else {
            value.to_owned()
        }
    };
    if c.kind == "file" {
        format!("file:{}", normalize(&c.path))
    } else {
        format!("command:{}:{}", normalize(&c.cwd), c.command)
    }
}

fn entry_available(entry: &ProgramEntry) -> bool {
    Path::new(&entry.candidate.cwd).is_dir()
        && (entry.candidate.kind == "command" || Path::new(&entry.candidate.path).is_file())
}

impl DaemonService {
    pub(super) fn dispatch_program(
        &mut self,
        method: &str,
        params: &Value,
    ) -> Result<Value, ProtocolError> {
        if method == "program.propose" {
            return self.propose_program(params);
        }
        if method == "program.search" {
            return self.search_shared_programs(params);
        }
        self.require_controller()?;
        match method {
            "program.proposals" => self.list_program_proposals(),
            "program.proposal_dismiss" => {
                let id = text(params, "id")?;
                self.registry
                    .db
                    .conn
                    .execute("DELETE FROM program_proposals WHERE id=?1", [id])
                    .map_err(sql_err)?;
                Ok(json!({"dismissed":true}))
            }
            "program.list" => {
                let mut statement = self
                    .registry
                    .db
                    .conn
                    .prepare("SELECT entry_json FROM program_entries ORDER BY updated_at DESC")
                    .map_err(sql_err)?;
                let rows = statement
                    .query_map([], |r| r.get::<_, String>(0))
                    .map_err(sql_err)?;
                let mut entries = Vec::new();
                for row in rows {
                    let entry: ProgramEntry =
                        serde_json::from_str(&row.map_err(sql_err)?).map_err(json_err)?;
                    let mut value = serde_json::to_value(&entry).map_err(json_err)?;
                    value["available"] = json!(entry_available(&entry));
                    entries.push(value);
                }
                Ok(json!(entries))
            }
            "program.scan_start" => {
                if self
                    .programs
                    .scan
                    .as_ref()
                    .is_some_and(|(s, _)| s.lock().unwrap().status == "running")
                {
                    return Err(invalid("程序扫描正在进行"));
                }
                let roots = if let Some(roots) = params.get("roots") {
                    let items = roots
                        .as_array()
                        .filter(|v| !v.is_empty() && v.len() <= 64)
                        .ok_or_else(|| invalid("需要 1 至 64 个扫描目录"))?;
                    items
                        .iter()
                        .map(|v| {
                            let p = v.as_str().ok_or_else(|| invalid("扫描目录必须是字符串"))?;
                            if !Path::new(p).is_absolute() || !Path::new(p).is_dir() {
                                return Err(invalid("扫描目录不存在"));
                            }
                            Ok(p.to_string())
                        })
                        .collect::<Result<Vec<_>, _>>()?
                } else {
                    programs::local_disk_roots()
                };
                if roots.is_empty() {
                    return Err(invalid("没有可读取的本地磁盘"));
                }
                let id = uuid::Uuid::new_v4().to_string();
                let state = Arc::new(Mutex::new(ProgramScan {
                    id: id.clone(),
                    status: "running".into(),
                    roots: roots.clone(),
                    ..Default::default()
                }));
                let cancel = Arc::new(AtomicBool::new(false));
                let worker_state = state.clone();
                let worker_cancel = cancel.clone();
                std::thread::Builder::new()
                    .name("program-discovery".into())
                    .spawn(move || {
                        let result = std::panic::catch_unwind(|| {
                            programs::scan_programs(roots, worker_state.clone(), worker_cancel)
                        });
                        if result.is_err() {
                            let mut report = worker_state.lock().unwrap_or_else(|e| e.into_inner());
                            report.status = "failed".into();
                            report.limitations.push("扫描工作线程异常，请重试".into());
                        }
                    })
                    .map_err(|e| invalid(&e.to_string()))?;
                self.programs.scan = Some((state, cancel));
                Ok(json!({"id":id}))
            }
            "program.scan_status" => {
                let Some((state, _)) = &self.programs.scan else {
                    return Ok(json!({"status":"idle","candidates":[],"total":0}));
                };
                let report = state.lock().map_err(|_| invalid("扫描状态不可用"))?;
                let offset = params
                    .get("offset")
                    .map(|v| {
                        v.as_u64()
                            .filter(|n| *n <= 5000)
                            .ok_or_else(|| invalid("offset 无效"))
                    })
                    .transpose()?
                    .unwrap_or(0) as usize;
                let limit = params
                    .get("limit")
                    .map(|v| {
                        v.as_u64()
                            .filter(|n| *n > 0 && *n <= 200)
                            .ok_or_else(|| invalid("limit 无效"))
                    })
                    .transpose()?
                    .unwrap_or(100) as usize;
                Ok(
                    json!({"id":report.id,"status":report.status,"roots":report.roots,"roots_attempted":report.roots_attempted,"visited":report.visited,"skipped":report.skipped,"unreadable":report.unreadable,"current":report.current,"limitations":report.limitations,"total":report.candidates.len(),"candidates":report.candidates.iter().skip(offset).take(limit).collect::<Vec<_>>() }),
                )
            }
            "program.scan_cancel" => {
                if let Some((_, cancel)) = &self.programs.scan {
                    cancel.store(true, Ordering::Relaxed);
                }
                Ok(json!({"cancel_requested":true}))
            }
            "program.select" => {
                if let Some(ids) = params.get("proposal_ids") {
                    return self.select_program_proposals(ids);
                }
                if self.programs.selections.len() >= 1024 {
                    self.programs.selections.clear();
                }
                let candidates = if let Some(ids) = params.get("candidate_ids") {
                    let ids = ids
                        .as_array()
                        .filter(|v| !v.is_empty() && v.len() <= 100)
                        .ok_or_else(|| invalid("请先勾选 1 至 100 个候选"))?;
                    let (state, _) = self
                        .programs
                        .scan
                        .as_ref()
                        .ok_or_else(|| invalid("扫描候选已失效，请重新扫描"))?;
                    let report = state.lock().map_err(|_| invalid("扫描状态不可用"))?;
                    ids.iter()
                        .map(|id| {
                            report
                                .candidates
                                .iter()
                                .find(|c| id.as_str() == Some(&c.id))
                                .cloned()
                                .ok_or_else(|| invalid("扫描候选已失效"))
                        })
                        .collect::<Result<Vec<_>, _>>()?
                } else {
                    vec![programs::manual_candidate(
                        Path::new(text(params, "path")?),
                        text(params, "kind")?,
                    )
                    .map_err(|e| invalid(&e))?]
                };
                let owner = self.peer_principal();
                Ok(json!(candidates.into_iter().map(|c| {
                    let selection_id = uuid::Uuid::new_v4().to_string();
                    self.programs.selections.insert(selection_id.clone(), (owner.clone(), c.clone()));
                    json!({"selection_id":selection_id,"kind":c.kind,"name":c.name,"path":c.path,"cwd":c.cwd,"command":c.command,"evidence":c.evidence,"args":[],"favorite":false})
                }).collect::<Vec<_>>()))
            }
            "program.save" => self.save_programs(params),
            "program.remove" => {
                let id = text(params, "id")?;
                let changed = self
                    .registry
                    .db
                    .conn
                    .execute("DELETE FROM program_entries WHERE id=?1", [id])
                    .map_err(sql_err)?;
                if changed == 0 {
                    return Err(ProtocolError::new(ErrorCode::NotFound, "程序条目不存在"));
                }
                self.event(
                    "program.removed",
                    "user removed a launch entry; files unchanged",
                    json!({"id":id}),
                )?;
                Ok(json!({"removed":true}))
            }
            "program.launch" => {
                let id = text(params, "id")?;
                let terminal = match params.get("terminal") {
                    None => false,
                    Some(Value::Bool(value)) => *value,
                    _ => return Err(invalid("terminal 必须为布尔值")),
                };
                let mut entry = self.program_entry(id)?;
                if !entry_available(&entry) {
                    return Err(invalid("启动文件或工作目录已失效，请重新选择"));
                }
                let pid = spawn_program(&entry, terminal).map_err(|e| invalid(&e))?;
                entry.launch_count += 1;
                entry.last_launched = Some(chrono::Utc::now().to_rfc3339());
                self.registry
                    .db
                    .conn
                    .execute(
                        "UPDATE program_entries SET entry_json=?1 WHERE id=?2",
                        rusqlite::params![serde_json::to_string(&entry).map_err(json_err)?, id],
                    )
                    .map_err(sql_err)?;
                self.event(
                    "program.launched",
                    "user requested a local program launch",
                    json!({"id":id,"pid":pid,"terminal":terminal}),
                )?;
                Ok(json!({"pid":pid,"submitted":true,"terminal":terminal}))
            }
            _ => Err(ProtocolError::new(
                ErrorCode::MethodNotFound,
                "unknown program method",
            )),
        }
    }

    fn program_entry(&self, id: &str) -> Result<ProgramEntry, ProtocolError> {
        let raw: Option<String> = self
            .registry
            .db
            .conn
            .query_row(
                "SELECT entry_json FROM program_entries WHERE id=?1",
                [id],
                |r| r.get(0),
            )
            .optional()
            .map_err(sql_err)?;
        serde_json::from_str(
            &raw.ok_or_else(|| ProtocolError::new(ErrorCode::NotFound, "程序条目不存在"))?,
        )
        .map_err(json_err)
    }

    fn selection(&self, id: &str) -> Result<ProgramCandidate, ProtocolError> {
        self.programs
            .selections
            .get(id)
            .filter(|(owner, _)| owner == &self.peer_principal())
            .map(|(_, c)| c.clone())
            .ok_or_else(|| invalid("请先选择文件、工作目录或扫描候选"))
    }

    fn save_programs(&mut self, params: &Value) -> Result<Value, ProtocolError> {
        let items = params
            .get("items")
            .and_then(Value::as_array)
            .filter(|v| !v.is_empty() && v.len() <= 100)
            .ok_or_else(|| invalid("请选择要保存的程序"))?;
        let mut entries = Vec::new();
        let mut accepted_proposals = Vec::new();
        for patch in items {
            let existing = patch
                .get("id")
                .and_then(Value::as_str)
                .map(|id| self.program_entry(id))
                .transpose()?;
            let mut candidate = if let Some(id) = patch.get("selection_id").and_then(Value::as_str)
            {
                self.selection(id)?
            } else {
                existing
                    .as_ref()
                    .map(|e| e.candidate.clone())
                    .ok_or_else(|| invalid("请先选择文件、工作目录或扫描候选"))?
            };
            if let Some(id) = patch.get("cwd_selection_id").and_then(Value::as_str) {
                let selection = self.selection(id)?;
                if selection.kind != "command" {
                    return Err(invalid("请先选择工作目录"));
                }
                candidate.cwd = selection.cwd;
            }
            let proposed: bool = self
                .registry
                .db
                .conn
                .query_row(
                    "SELECT EXISTS(SELECT 1 FROM program_proposals WHERE id=?1)",
                    [&candidate.id],
                    |r| r.get(0),
                )
                .map_err(sql_err)?;
            if proposed {
                accepted_proposals.push(candidate.id.clone());
            }
            candidate.id = existing
                .as_ref()
                .map(|e| e.candidate.id.clone())
                .unwrap_or_else(|| uuid::Uuid::new_v4().to_string());
            candidate.name = text(patch, "name")?.trim().to_string();
            if candidate.name.is_empty() || candidate.name.len() > 256 {
                return Err(invalid("程序名称不能为空且不能超过 256 字节"));
            }
            candidate.command = if candidate.kind == "command" {
                text(patch, "command")?.trim().to_string()
            } else {
                String::new()
            };
            if candidate.command.len() > 8192
                || candidate.command.contains(['\0', '\r', '\n'])
                || (candidate.kind == "command" && candidate.command.is_empty())
            {
                return Err(invalid("请填写单行启动命令（不超过 8192 字节）"));
            }
            let args = patch
                .get("args")
                .and_then(Value::as_array)
                .ok_or_else(|| invalid("参数必须是字符串数组"))?;
            if args.len() > 64 {
                return Err(invalid("启动参数过多"));
            }
            let args = args
                .iter()
                .map(|a| {
                    a.as_str()
                        .filter(|s| s.len() <= 4096 && !s.contains(['\0', '\r', '\n']))
                        .map(String::from)
                        .ok_or_else(|| invalid("启动参数无效"))
                })
                .collect::<Result<Vec<_>, _>>()?;
            if (candidate.kind == "command"
                || Path::new(&candidate.path)
                    .extension()
                    .is_some_and(|e| e.eq_ignore_ascii_case("lnk")))
                && !args.is_empty()
            {
                return Err(invalid("命令入口请将参数写入命令；快捷方式使用其自身参数"));
            }
            let favorite = patch
                .get("favorite")
                .and_then(Value::as_bool)
                .ok_or_else(|| invalid("favorite 必须为布尔值"))?;
            let metadata = match patch.get("metadata") {
                Some(value) => serde_json::from_value::<
                    toolhub_core::program_metadata::ProgramMetadata,
                >(value.clone())
                .map_err(json_err)?,
                None => existing
                    .as_ref()
                    .map(|e| e.metadata.clone())
                    .unwrap_or_default(),
            };
            metadata.validate().map_err(|e| invalid(&e))?;
            let entry = ProgramEntry {
                metadata,
                candidate,
                args,
                favorite,
                last_launched: existing.as_ref().and_then(|e| e.last_launched.clone()),
                launch_count: existing.map(|e| e.launch_count).unwrap_or(0),
            };
            if !entry_available(&entry) {
                return Err(invalid("启动文件或工作目录不存在"));
            }
            entries.push(entry);
        }
        // Validate all selected drafts before the transaction; a bad draft cannot partially add others.
        let tx = self.registry.db.conn.transaction().map_err(sql_err)?;
        for entry in &entries {
            let duplicate: Option<String> = tx
                .query_row(
                    "SELECT id FROM program_entries WHERE identity=?1 AND id<>?2",
                    rusqlite::params![identity(&entry.candidate), entry.candidate.id],
                    |r| r.get(0),
                )
                .optional()
                .map_err(sql_err)?;
            if duplicate.is_some() {
                return Err(invalid("该启动入口已添加，请编辑已有条目"));
            }
            tx.execute("INSERT INTO program_entries(id,identity,entry_json,updated_at) VALUES(?1,?2,?3,?4) ON CONFLICT(id) DO UPDATE SET identity=excluded.identity,entry_json=excluded.entry_json,updated_at=excluded.updated_at", rusqlite::params![entry.candidate.id,identity(&entry.candidate),serde_json::to_string(entry).map_err(json_err)?,chrono::Utc::now().to_rfc3339()]).map_err(sql_err)?;
        }
        for id in accepted_proposals {
            tx.execute("DELETE FROM program_proposals WHERE id=?1", [id])
                .map_err(sql_err)?;
        }
        tx.commit().map_err(sql_err)?;
        self.event(
            "program.saved",
            "user confirmed launch entries",
            json!({"count":entries.len()}),
        )?;
        Ok(json!({"saved":entries.len()}))
    }
}

fn spawn_program(entry: &ProgramEntry, terminal: bool) -> Result<u32, String> {
    #[cfg(windows)]
    {
        // Background launches create no console; explicit terminal launches get fresh handles.
        super::program_process::spawn_helper(
            &serde_json::to_string(entry).map_err(|e| e.to_string())?,
            entry,
            terminal,
        )
    }
    #[cfg(not(windows))]
    {
        if terminal {
            return Err("当前平台请使用工具详情中的打开终端，或直接后台启动程序".into());
        }
        let mut child = program_command(entry, terminal)?;
        let mut child = child.spawn().map_err(|e| format!("启动失败：{e}"))?;
        let pid = child.id();
        std::thread::spawn(move || {
            let _ = child.wait();
        });
        Ok(pid)
    }
}

#[cfg(windows)]
fn batch_quoted(value: &str) -> String {
    // CMD quoting follows Rust's batch-argument algorithm, also applied to the script path.
    // https://github.com/rust-lang/rust/blob/main/library/std/src/sys/args/windows.rs
    let mut result = String::from("\"");
    let mut slashes = 0usize;
    for ch in value.chars() {
        if ch == '\\' {
            slashes += 1;
            continue;
        }
        result.extend(std::iter::repeat_n(
            '\\',
            if ch == '"' { slashes * 2 } else { slashes },
        ));
        slashes = 0;
        match ch {
            '"' => result.push_str("\"\""),
            '%' => result.push_str("%%cd:~,%"),
            _ => result.push(ch),
        }
    }
    result.extend(std::iter::repeat_n('\\', slashes * 2));
    result.push('"');
    result
}

fn program_command(entry: &ProgramEntry, terminal: bool) -> Result<std::process::Command, String> {
    use std::process::Command;
    let c = &entry.candidate;
    #[cfg(windows)]
    let mut child = {
        let system = std::env::var_os("SystemRoot").ok_or("SystemRoot 不可用")?;
        let system = std::path::PathBuf::from(system).join("System32");
        let powershell = system.join("WindowsPowerShell/v1.0/powershell.exe");
        if c.kind == "command" {
            use std::os::windows::process::CommandExt;
            let mut cmd = Command::new(system.join("cmd.exe"));
            // User-authored command text is intentionally shell syntax; cwd never enters this string.
            // /S removes only our outer pair, preserving quoted paths and shell operators.
            cmd.args(["/D", "/S", if terminal { "/K" } else { "/C" }])
                .raw_arg(format!("\"{}\"", c.command));
            cmd
        } else {
            let ext = Path::new(&c.path)
                .extension()
                .unwrap_or_default()
                .to_string_lossy()
                .to_lowercase();
            match ext.as_str() {
                "lnk" => {
                    let mut cmd = Command::new(powershell);
                    cmd.args(["-NoProfile", "-Command"]).arg(format!(
                        "Invoke-Item -LiteralPath '{}'",
                        c.path.replace('\'', "''")
                    ));
                    cmd
                }
                "ps1" => {
                    let mut cmd = Command::new(powershell);
                    cmd.arg("-NoProfile");
                    if terminal {
                        cmd.arg("-NoExit");
                    }
                    cmd.args(["-ExecutionPolicy", "Bypass", "-File"])
                        .arg(&c.path)
                        .args(&entry.args);
                    cmd
                }
                "bat" | "cmd" => {
                    use std::os::windows::process::CommandExt;
                    let mut cmd = Command::new(system.join("cmd.exe"));
                    let mut line = format!("\"{}", batch_quoted(&c.path));
                    for arg in &entry.args {
                        line.push(' ');
                        line.push_str(&batch_quoted(arg));
                    }
                    line.push('"');
                    cmd.args(["/e:ON", "/v:OFF", "/D", "/S", "/C"])
                        .raw_arg(line);
                    cmd
                }
                _ => {
                    // Ordinary executables use native argv quoting.
                    let mut cmd = Command::new(&c.path);
                    cmd.args(&entry.args);
                    cmd
                }
            }
        }
    };
    #[cfg(not(windows))]
    let mut child = {
        let mut cmd = if c.kind == "command" {
            let mut cmd = Command::new("sh");
            cmd.args(["-c", &c.command]);
            cmd
        } else if matches!(
            Path::new(&c.path).extension().and_then(|e| e.to_str()),
            Some("sh" | "command")
        ) {
            let mut command = Command::new("/bin/sh");
            command.arg(&c.path);
            command
        } else {
            Command::new(&c.path)
        };
        cmd.args(&entry.args);
        cmd
    };
    if !terminal {
        child
            .stdin(std::process::Stdio::null())
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null());
        #[cfg(windows)]
        {
            use std::os::windows::process::CommandExt;
            child.creation_flags(0x08000000); // CREATE_NO_WINDOW; GUI children remain visible.
        }
    }
    let env = program_environment();
    child.env_clear().envs(env.vars).current_dir(&c.cwd);
    Ok(child)
}

pub(super) fn program_environment() -> toolhub_executor::SanitizedEnv {
    let extra_names = [
        "APPDATA",
        "LOCALAPPDATA",
        "PROGRAMDATA",
        "PROGRAMFILES",
        "PROGRAMFILES(X86)",
        "COMMONPROGRAMFILES",
        "COMMONPROGRAMFILES(X86)",
        "SYSTEMDRIVE",
        "PUBLIC",
    ];
    // Windows env names are case-insensitive; keep the original spelling for the executor's explicit allowlist.
    let extra = std::env::vars()
        .map(|(name, _)| name)
        .filter(|name| {
            extra_names
                .iter()
                .any(|allowed| allowed.eq_ignore_ascii_case(name))
        })
        .collect::<Vec<_>>();
    toolhub_executor::sanitize_env(&BTreeMap::new(), &extra)
}

#[cfg(windows)]
pub(crate) fn run_console_helper() -> bool {
    let mut args = std::env::args();
    let terminal = match args.nth(1).as_deref() {
        Some("--program-console") => true,
        Some("--program-background") => false,
        _ => return false,
    };
    let result = args
        .next()
        .ok_or_else(|| "missing program launch data".to_string())
        .and_then(|s| serde_json::from_str::<ProgramEntry>(&s).map_err(|e| e.to_string()))
        .and_then(|entry| {
            if !entry_available(&entry) {
                return Err("程序文件或工作目录已失效".into());
            }
            let status = program_command(&entry, terminal)?
                .status()
                .map_err(|e| e.to_string())?;
            if !status.success() {
                eprintln!("程序退出：{status}");
            }
            // Keep batch output visible, including a script's errors, without changing the script.
            if terminal
                && entry.candidate.kind == "file"
                && Path::new(&entry.candidate.path)
                    .extension()
                    .is_some_and(|s| s.eq_ignore_ascii_case("bat") || s.eq_ignore_ascii_case("cmd"))
            {
                let system = std::env::var_os("SystemRoot").ok_or("SystemRoot 不可用")?;
                let _ = std::process::Command::new(
                    std::path::PathBuf::from(system).join("System32/cmd.exe"),
                )
                .args(["/D", "/K"])
                .current_dir(&entry.candidate.cwd)
                .status();
            }
            Ok(())
        });
    if let Err(error) = result {
        eprintln!("启动失败：{error}");
    }
    true
}

#[cfg(test)]
mod tests {
    use super::*;
    fn call(
        service: &mut DaemonService,
        method: &str,
        params: Value,
        controller: bool,
    ) -> JsonRpcResponse {
        service.handle_authenticated(
            &JsonRpcRequest {
                jsonrpc: "2.0".into(),
                id: Some(json!(1)),
                method: method.into(),
                params,
            },
            "fixture.controller",
            controller,
        )
    }
    #[test]
    fn selection_required_controller_only_persistence_and_atomic_save() {
        let tmp = tempfile::tempdir().unwrap();
        let db = tmp.path().join("registry.sqlite");
        let file = tmp.path().join(if cfg!(windows) {
            "start.cmd"
        } else {
            "start.command"
        });
        std::fs::write(&file, "@echo off").unwrap();
        let mut svc = DaemonService::open(&db).unwrap();
        assert!(call(&mut svc, "program.list", json!({}), false)
            .error
            .is_some());
        let fake =
            json!({"name":"Example","path":file,"cwd":tmp.path(),"args":[],"favorite":false});
        assert!(
            call(&mut svc, "program.save", json!({"items":[fake]}), true)
                .error
                .is_some()
        );
        let selected = call(
            &mut svc,
            "program.select",
            json!({"path":file,"kind":"file"}),
            true,
        )
        .result
        .unwrap();
        let item = selected[0].clone();
        let other = svc.handle_authenticated(
            &JsonRpcRequest::new(2, "program.save", json!({"items":[selected[0]]})),
            "fixture.other-controller",
            true,
        );
        assert!(
            other.error.is_some(),
            "selection is bound to its controller principal"
        );
        let mut invalid_item = item.clone();
        invalid_item["name"] = json!("");
        assert!(call(
            &mut svc,
            "program.save",
            json!({"items":[item,invalid_item]}),
            true
        )
        .error
        .is_some());
        assert_eq!(
            call(&mut svc, "program.list", json!({}), true)
                .result
                .unwrap()
                .as_array()
                .unwrap()
                .len(),
            0
        );
        assert!(
            call(&mut svc, "program.save", json!({"items":[item]}), true)
                .error
                .is_none()
        );
        assert!(
            call(&mut svc, "program.save", json!({"items":[item]}), true)
                .error
                .is_some()
        );
        drop(svc);
        let mut svc = DaemonService::open(&db).unwrap();
        let rows = call(&mut svc, "program.list", json!({}), true)
            .result
            .unwrap();
        assert_eq!(rows[0]["available"], true);
        std::fs::remove_file(&file).unwrap();
        assert_eq!(
            call(&mut svc, "program.list", json!({}), true)
                .result
                .unwrap()[0]["available"],
            false
        );
        assert!(call(
            &mut svc,
            "program.launch",
            json!({"id":rows[0]["id"]}),
            true
        )
        .error
        .is_some());
        assert!(call(
            &mut svc,
            "program.remove",
            json!({"id":rows[0]["id"]}),
            true
        )
        .error
        .is_none());
    }
    #[cfg(windows)]
    #[test]
    fn launch_environment_preserves_windows_system_and_app_directories() {
        let env = program_environment();
        for name in [
            "SystemDrive",
            "ProgramData",
            "ProgramFiles",
            "CommonProgramFiles",
            "APPDATA",
            "LOCALAPPDATA",
        ] {
            if let Some((actual, value)) =
                std::env::vars().find(|(key, _)| key.eq_ignore_ascii_case(name))
            {
                assert_eq!(
                    env.vars.get(&actual),
                    Some(&value),
                    "missing Windows launch variable {actual}"
                );
            }
        }
        assert!(!env
            .vars
            .keys()
            .any(|name| toolhub_executor::is_secret_name(name)));
    }
}
