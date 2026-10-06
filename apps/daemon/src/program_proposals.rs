//! Agents may submit metadata; only the desktop controller can select and save it.
use super::execution_jobs::{invalid, text};
use super::*;
use serde::{Deserialize, Serialize};
use toolhub_scanner::programs::{manual_candidate, ProgramCandidate};

#[derive(Serialize, Deserialize)]
struct Proposal {
    #[serde(flatten)]
    candidate: ProgramCandidate,
    args: Vec<String>,
    source: String,
    submitted_at: String,
    #[serde(default)]
    metadata: toolhub_core::program_metadata::ProgramMetadata,
}

impl DaemonService {
    pub(super) fn propose_program(&mut self, params: &Value) -> Result<Value, ProtocolError> {
        let kind = text(params, "kind")?;
        let cwd = manual_candidate(Path::new(text(params, "cwd")?), "command")
            .map_err(|e| invalid(&e))?
            .cwd;
        let mut candidate = manual_candidate(
            Path::new(if kind == "file" {
                text(params, "path")?
            } else {
                &cwd
            }),
            kind,
        )
        .map_err(|e| invalid(&e))?;
        candidate.cwd = cwd;
        candidate.name = text(params, "name")?.trim().to_string();
        if candidate.name.is_empty()
            || candidate.name.len() > 256
            || candidate.name.contains(['\0', '\r', '\n'])
        {
            return Err(invalid("请提供有效的程序名称"));
        }
        if kind == "command" {
            candidate.command = text(params, "command")?.trim().to_string();
            if candidate.command.is_empty()
                || candidate.command.len() > 8192
                || candidate.command.contains(['\0', '\r', '\n'])
            {
                return Err(invalid("请提供单行启动命令"));
            }
        }
        let args = match params.get("args") {
            None => Vec::new(),
            Some(value) => value
                .as_array()
                .filter(|items| items.len() <= 64)
                .ok_or_else(|| invalid("启动参数必须是有界字符串数组"))?
                .iter()
                .map(|arg| {
                    arg.as_str()
                        .filter(|arg| arg.len() <= 4096 && !arg.contains(['\0', '\r', '\n']))
                        .map(str::to_string)
                        .ok_or_else(|| invalid("启动参数无效"))
                })
                .collect::<Result<Vec<_>, _>>()?,
        };
        if (kind == "command" || candidate.path.to_lowercase().ends_with(".lnk"))
            && !args.is_empty()
        {
            return Err(invalid("命令参数写入 command；快捷方式保留自身参数"));
        }
        let source = params
            .get("source")
            .map(|v| {
                v.as_str()
                    .filter(|s| {
                        !s.trim().is_empty() && s.len() <= 128 && !s.contains(['\0', '\r', '\n'])
                    })
                    .ok_or_else(|| invalid("source 必须是简短的来源名称"))
            })
            .transpose()?
            .unwrap_or("Agent")
            .to_string();
        candidate.evidence = vec!["Agent 提交；用户确认前不会收录或启动".into()];
        let identity = super::programs::identity(&candidate);
        let saved: bool = self
            .registry
            .db
            .conn
            .query_row(
                "SELECT EXISTS(SELECT 1 FROM program_entries WHERE identity=?1)",
                [&identity],
                |r| r.get(0),
            )
            .map_err(sql_err)?;
        if saved {
            return Ok(json!({"status":"already_saved","requires_user_confirmation":false}));
        }
        let principal = self.peer_principal();
        let existing: Option<String> = self
            .registry
            .db
            .conn
            .query_row(
                "SELECT id FROM program_proposals WHERE identity=?1 AND principal=?2",
                rusqlite::params![identity, principal],
                |r| r.get(0),
            )
            .optional()
            .map_err(sql_err)?;
        let count: i64 = self
            .registry
            .db
            .conn
            .query_row("SELECT COUNT(*) FROM program_proposals", [], |r| r.get(0))
            .map_err(sql_err)?;
        if existing.is_none() && count >= 100 {
            return Err(invalid("待确认入口已达 100 项，请用户先处理已有条目"));
        }
        candidate.id = existing.unwrap_or_else(|| uuid::Uuid::new_v4().to_string());
        let metadata: toolhub_core::program_metadata::ProgramMetadata = serde_json::from_value(json!({"purpose":params.get("purpose").cloned().unwrap_or(json!("")),"inputs":params.get("inputs").cloned().unwrap_or(json!([])),"outputs":params.get("outputs").cloned().unwrap_or(json!([])),"dependencies":params.get("dependencies").cloned().unwrap_or(json!([])),"examples":params.get("examples").cloned().unwrap_or(json!([]))})).map_err(json_err)?;
        metadata.validate().map_err(|e| invalid(&e))?;
        let proposal = Proposal {
            metadata,
            candidate,
            args,
            source,
            submitted_at: chrono::Utc::now().to_rfc3339(),
        };
        self.registry.db.conn.execute(
            "INSERT INTO program_proposals(id,identity,principal,proposal_json,updated_at) VALUES(?1,?2,?3,?4,?5) ON CONFLICT(identity,principal) DO UPDATE SET proposal_json=excluded.proposal_json,updated_at=excluded.updated_at",
            rusqlite::params![proposal.candidate.id,identity,principal,serde_json::to_string(&proposal).map_err(json_err)?,proposal.submitted_at]).map_err(sql_err)?;
        self.event(
            "program.proposed",
            "agent submitted metadata for user review",
            json!({"proposal_id":proposal.candidate.id}),
        )?;
        Ok(
            json!({"proposal_id":proposal.candidate.id,"status":"pending","requires_user_confirmation":true}),
        )
    }

    pub(super) fn search_shared_programs(&self, params: &Value) -> Result<Value, ProtocolError> {
        let query = params["query"].as_str().unwrap_or("").to_lowercase();
        let mut statement = self
            .registry
            .db
            .conn
            .prepare("SELECT entry_json FROM program_entries ORDER BY updated_at DESC LIMIT 1000")
            .map_err(sql_err)?;
        let mut entries = vec![];
        for raw in statement
            .query_map([], |r| r.get::<_, String>(0))
            .map_err(sql_err)?
        {
            let entry: super::programs::ProgramEntry =
                serde_json::from_str(&raw.map_err(sql_err)?).map_err(json_err)?;
            if entry.metadata.agent_visible
                && (entry.candidate.name.to_lowercase().contains(&query)
                    || entry.metadata.purpose.to_lowercase().contains(&query))
            {
                entries.push(json!({"id":entry.candidate.id,"name":entry.candidate.name,"kind":entry.candidate.kind,"path":entry.candidate.path,"cwd":entry.candidate.cwd,"command":entry.candidate.command,"args":entry.args,"metadata":entry.metadata,"execution_authority":false,"launch_requires_desktop":true}));
            }
        }
        Ok(json!(entries))
    }

    pub(super) fn list_program_proposals(&self) -> Result<Value, ProtocolError> {
        let mut statement = self
            .registry
            .db
            .conn
            .prepare("SELECT proposal_json FROM program_proposals ORDER BY updated_at DESC")
            .map_err(sql_err)?;
        let rows = statement
            .query_map([], |r| r.get::<_, String>(0))
            .map_err(sql_err)?;
        let mut entries = Vec::new();
        for row in rows {
            let proposal: Proposal =
                serde_json::from_str(&row.map_err(sql_err)?).map_err(json_err)?;
            let mut value = serde_json::to_value(&proposal).map_err(json_err)?;
            value["available"] = json!(
                Path::new(&proposal.candidate.cwd).is_dir()
                    && (proposal.candidate.kind == "command"
                        || Path::new(&proposal.candidate.path).is_file())
            );
            entries.push(value);
        }
        Ok(json!(entries))
    }

    pub(super) fn select_program_proposals(&mut self, ids: &Value) -> Result<Value, ProtocolError> {
        let ids = ids
            .as_array()
            .filter(|ids| !ids.is_empty() && ids.len() <= 100)
            .ok_or_else(|| invalid("请先勾选待确认入口"))?;
        let mut proposals = Vec::new();
        for id in ids {
            let id = id.as_str().ok_or_else(|| invalid("入口 ID 必须是字符串"))?;
            let raw: Option<String> = self
                .registry
                .db
                .conn
                .query_row(
                    "SELECT proposal_json FROM program_proposals WHERE id=?1",
                    [id],
                    |r| r.get(0),
                )
                .optional()
                .map_err(sql_err)?;
            let proposal: Proposal =
                serde_json::from_str(&raw.ok_or_else(|| invalid("待确认入口已失效，请刷新"))?)
                    .map_err(json_err)?;
            if !Path::new(&proposal.candidate.cwd).is_dir()
                || (proposal.candidate.kind == "file"
                    && !Path::new(&proposal.candidate.path).is_file())
            {
                return Err(invalid("入口文件或工作目录已失效，请重新选择"));
            }
            proposals.push(proposal);
        }
        if self.programs.selections.len() + proposals.len() > 1024 {
            self.programs.selections.clear();
        }
        let owner = self.peer_principal();
        Ok(json!(proposals
            .into_iter()
            .map(|proposal| {
                let selection_id = uuid::Uuid::new_v4().to_string();
                self.programs.selections.insert(
                    selection_id.clone(),
                    (owner.clone(), proposal.candidate.clone()),
                );
                let mut value = serde_json::to_value(&proposal).expect("serializable proposal");
                value.as_object_mut().unwrap().remove("id");
                value["selection_id"] = json!(selection_id);
                value["favorite"] = json!(false);
                value
            })
            .collect::<Vec<_>>()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn call(
        svc: &mut DaemonService,
        method: &str,
        params: Value,
        controller: bool,
    ) -> JsonRpcResponse {
        svc.handle_authenticated(
            &JsonRpcRequest::new(1, method, params),
            "fixture.agent",
            controller,
        )
    }
    #[test]
    fn proposal_is_persistent_metadata_until_controller_selects_and_saves() {
        let tmp = tempfile::tempdir().unwrap();
        let db = tmp.path().join("registry.sqlite");
        let file = tmp.path().join("start.cmd");
        let marker = tmp.path().join("EXECUTED");
        std::fs::write(
            &file,
            format!(
                "@echo off\r\necho should-not-run > {}\r\n",
                marker.display()
            ),
        )
        .unwrap();
        let params = json!({"kind":"file","name":"Agent app","path":file,"cwd":tmp.path(),"args":["literal & argument"],"source":"fixture","purpose":"Owned application","inputs":["input.txt"],"outputs":["output.txt"],"dependencies":["archive.extract"],"examples":["Use a local input"]});
        let mut svc = DaemonService::open(&db).unwrap();
        let proposed = call(&mut svc, "program.propose", params.clone(), false)
            .result
            .unwrap();
        assert_eq!(proposed["status"], "pending");
        assert_eq!(
            call(&mut svc, "program.propose", params, false)
                .result
                .unwrap()["proposal_id"],
            proposed["proposal_id"]
        );
        assert_eq!(
            call(&mut svc, "program.list", json!({}), true)
                .result
                .unwrap(),
            json!([])
        );
        assert!(call(&mut svc, "program.proposals", json!({}), false)
            .error
            .is_some());
        assert!(call(
            &mut svc,
            "program.select",
            json!({"proposal_ids":[proposed["proposal_id"]]}),
            false
        )
        .error
        .is_some());
        assert!(call(
            &mut svc,
            "program.launch",
            json!({"id":proposed["proposal_id"]}),
            false
        )
        .error
        .is_some());
        assert!(call(
            &mut svc,
            "program.proposal_dismiss",
            json!({"id":proposed["proposal_id"]}),
            false
        )
        .error
        .is_some());
        assert!(call(
            &mut svc,
            "program.save",
            json!({"items":[{"selection_id":proposed["proposal_id"],"name":"bypass"}]}),
            false
        )
        .error
        .is_some());
        assert!(!marker.exists());
        drop(svc);
        let mut svc = DaemonService::open(&db).unwrap();
        assert_eq!(
            call(&mut svc, "program.proposals", json!({}), true)
                .result
                .unwrap()
                .as_array()
                .unwrap()
                .len(),
            1
        );
        let drafts = call(
            &mut svc,
            "program.select",
            json!({"proposal_ids":[proposed["proposal_id"]]}),
            true,
        )
        .result
        .unwrap();
        assert_eq!(drafts[0]["args"], json!(["literal & argument"]));
        assert_eq!(drafts[0]["metadata"]["purpose"], "Owned application");
        assert!(drafts[0].get("id").is_none());
        let mut bad = drafts[0].clone();
        bad["name"] = json!("");
        assert!(call(
            &mut svc,
            "program.save",
            json!({"items":[drafts[0],bad]}),
            true
        )
        .error
        .is_some());
        assert_eq!(
            call(&mut svc, "program.proposals", json!({}), true)
                .result
                .unwrap()
                .as_array()
                .unwrap()
                .len(),
            1
        );
        assert!(
            call(&mut svc, "program.save", json!({"items":[drafts[0]]}), true)
                .error
                .is_none()
        );
        assert_eq!(
            call(&mut svc, "program.proposals", json!({}), true)
                .result
                .unwrap(),
            json!([])
        );
        assert_eq!(
            call(&mut svc, "program.list", json!({}), true)
                .result
                .unwrap()
                .as_array()
                .unwrap()
                .len(),
            1
        );
        let saved = call(&mut svc, "program.list", json!({}), true)
            .result
            .unwrap();
        assert_eq!(saved[0]["metadata"]["purpose"], "Owned application");
        assert_eq!(
            call(&mut svc, "program.search", json!({}), false)
                .result
                .unwrap(),
            json!([])
        );
        let mut shared = saved[0].clone();
        shared["metadata"]["agent_visible"] = json!(true);
        assert!(
            call(&mut svc, "program.save", json!({"items":[shared]}), true)
                .error
                .is_none()
        );
        let searchable = call(&mut svc, "program.search", json!({"query":"Owned"}), false)
            .result
            .unwrap();
        assert_eq!(searchable.as_array().unwrap().len(), 1);
        assert_eq!(searchable[0]["execution_authority"], false);
        assert!(!marker.exists());
    }

    #[test]
    fn rejects_invalid_proposals_and_dismisses_only_metadata() {
        let tmp = tempfile::tempdir().unwrap();
        let mut svc = DaemonService::open(&tmp.path().join("registry.sqlite")).unwrap();
        let params =
            json!({"kind":"command","name":"app","cwd":tmp.path(),"command":"npm run start"});
        for (key, value) in [
            ("cwd", json!("relative")),
            ("kind", json!("execute")),
            ("command", json!("echo ok\r\nrun-other")),
            ("args", json!(["separate command arg"])),
        ] {
            let mut bad = params.clone();
            bad[key] = value;
            assert!(call(&mut svc, "program.propose", bad, false)
                .error
                .is_some());
        }
        let id = call(&mut svc, "program.propose", params, false)
            .result
            .unwrap()["proposal_id"]
            .clone();
        assert!(
            call(&mut svc, "program.proposal_dismiss", json!({"id":id}), true)
                .error
                .is_none()
        );
        assert_eq!(
            call(&mut svc, "program.proposals", json!({}), true)
                .result
                .unwrap(),
            json!([])
        );
        assert!(tmp.path().is_dir());
    }
}
