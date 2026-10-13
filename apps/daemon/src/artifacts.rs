//! Explicitly registered files only. Discovery never deletes; cleanup is recycle-only.
use super::execution_jobs::{invalid, text};
use super::*;
use sha2::{Digest, Sha256};
use std::collections::BTreeSet;
use std::fs::{File, OpenOptions};
use std::io::Read;
use std::path::{Component, PathBuf};
use toolhub_core::artifact::*;

#[path = "artifact_recycle.rs"]
mod recycle;

type Result<T> = std::result::Result<T, ProtocolError>;
fn denied(reason: &str) -> ProtocolError {
    ProtocolError::denied(reason)
}
fn file_err(e: std::io::Error) -> ProtocolError {
    invalid(&format!("artifact file unavailable: {e}"))
}
fn now() -> String {
    chrono::Utc::now().to_rfc3339()
}
fn plain(path: &Path) -> String {
    let text = path.to_string_lossy();
    text.strip_prefix(r"\\?\").unwrap_or(&text).to_string()
}
fn path_key(path: &Path) -> String {
    let text = plain(path);
    if cfg!(windows) {
        text.to_lowercase().replace('/', "\\")
    } else {
        text
    }
}
fn forbidden(path: &Path) -> bool {
    path.components().any(|part| matches!(part, Component::Normal(name) if matches!(name.to_string_lossy().to_lowercase().as_str(),
        ".git"|".hg"|".svn"|"node_modules"|".venv"|"venv"|".toolhub"|".codex"|".config"|".ssh"|".aws"|"windows"|"program files"|"program files (x86)"|"appdata"|"$recycle.bin"|"system volume information"|"applications"|"system"|"library")))
}
fn is_link(meta: &std::fs::Metadata) -> bool {
    #[cfg(windows)]
    {
        use std::os::windows::fs::MetadataExt;
        if meta.file_attributes() & 0x400 != 0 {
            return true;
        }
    }
    meta.file_type().is_symlink()
}
fn checked_directory(input: &Path) -> Result<PathBuf> {
    if !input.is_absolute() || forbidden(input) {
        return Err(denied(
            "Select a project/task directory, not a system, configuration or dependency directory",
        ));
    }
    let mut current = PathBuf::new();
    for component in input.components() {
        if matches!(component, Component::ParentDir) {
            return Err(invalid("Parent traversal is not allowed"));
        }
        current.push(component);
        if matches!(component, Component::Normal(_))
            && is_link(&std::fs::symlink_metadata(&current).map_err(file_err)?)
        {
            return Err(denied("Links and junctions are not cleanup roots"));
        }
    }
    let canonical = input.canonicalize().map_err(file_err)?;
    if !canonical.is_dir()
        || canonical.parent().is_none()
        || forbidden(&canonical)
        || dirs::home_dir()
            .is_some_and(|home| home.canonicalize().ok().as_ref() == Some(&canonical))
    {
        return Err(denied("A specific existing task directory is required"));
    }
    Ok(canonical)
}
fn relative(input: &str) -> Result<PathBuf> {
    let path = Path::new(input);
    if (cfg!(windows) && input.contains(':'))
        || input.is_empty()
        || input.len() > 4096
        || path
            .components()
            .any(|c| !matches!(c, Component::Normal(_)))
        || forbidden(path)
    {
        return Err(invalid(
            "File paths must be confined relative paths without links or protected components",
        ));
    }
    Ok(path.to_owned())
}
fn checked_file(root: &Path, input: &str) -> Result<PathBuf> {
    let part = relative(input)?;
    let canonical_root = checked_directory(root)?;
    if path_key(root) != path_key(&canonical_root) {
        return Err(denied("Cleanup root changed"));
    }
    let mut path = canonical_root.clone();
    for component in part.components() {
        path.push(component);
        if is_link(&std::fs::symlink_metadata(&path).map_err(file_err)?) {
            return Err(denied(
                "Links/junctions cannot be recycled as registered files",
            ));
        }
    }
    let canonical = path.canonicalize().map_err(file_err)?;
    let root_key = path_key(&canonical_root);
    let file_key = path_key(&canonical);
    if !canonical.is_file()
        || !file_key.starts_with(&(root_key + if cfg!(windows) { "\\" } else { "/" }))
    {
        return Err(denied(
            "Only registered regular files inside the root can be recycled",
        ));
    }
    // Resolve Windows aliases before deduplication and reference checks.
    Ok(canonical)
}
fn open_read(path: &Path) -> Result<File> {
    let mut opts = OpenOptions::new();
    opts.read(true);
    // Prevent concurrent writers during fingerprint and recycle. Delete sharing permits the Shell move.
    #[cfg(windows)]
    {
        use std::os::windows::fs::OpenOptionsExt;
        opts.share_mode(1 | 4);
    }
    opts.open(path).map_err(file_err)
}
fn fingerprint(root: &Path, input: &str) -> Result<ArtifactFile> {
    let path = checked_file(root, input)?;
    let mut file = open_read(&path)?;
    let before = file.metadata().map_err(file_err)?;
    if before.len() > ARTIFACT_MAX_FILE_BYTES {
        return Err(invalid("File exceeds the 256 MiB artifact limit"));
    }
    let mut hash = Sha256::new();
    let mut buffer = [0u8; 65536];
    let mut total = 0;
    loop {
        let count = file.read(&mut buffer).map_err(file_err)?;
        if count == 0 {
            break;
        }
        total += count as u64;
        if total > ARTIFACT_MAX_FILE_BYTES {
            return Err(invalid("Artifact grew during registration"));
        }
        hash.update(&buffer[..count]);
    }
    let after = file.metadata().map_err(file_err)?;
    if before.len() != after.len() || before.modified().ok() != after.modified().ok() {
        return Err(invalid("Artifact changed during fingerprinting"));
    }
    let modified_ns = after
        .modified()
        .map_err(file_err)?
        .duration_since(std::time::UNIX_EPOCH)
        .map_err(|_| invalid("Invalid file timestamp"))?
        .as_nanos()
        .to_string();
    Ok(ArtifactFile {
        relative_path: input.to_string(),
        bytes: after.len(),
        sha256: format!("{:x}", hash.finalize()),
        modified_ns,
        status: "present".into(),
        error: None,
    })
}
fn bound(params: &Value, key: &str, max: usize) -> Result<String> {
    let value = text(params, key)?.trim().to_owned();
    if value.is_empty() || value.len() > max {
        Err(invalid(&format!("{key} must be nonempty and bounded")))
    } else {
        Ok(value)
    }
}

impl DaemonService {
    fn artifact_roots(&self) -> Result<Vec<ArtifactRoot>> {
        let mut statement = self
            .registry
            .db
            .conn
            .prepare("SELECT id,path,enabled FROM artifact_roots ORDER BY path")
            .map_err(sql_err)?;
        let rows = statement
            .query_map([], |r| {
                Ok(ArtifactRoot {
                    id: r.get(0)?,
                    path: r.get(1)?,
                    enabled: r.get(2)?,
                })
            })
            .map_err(sql_err)?;
        rows.collect::<std::result::Result<Vec<_>, _>>()
            .map_err(sql_err)
    }
    fn artifact_scope(&self, root: &Path) -> Result<bool> {
        let target = path_key(root);
        Ok(self.artifact_roots()?.iter().any(|r| {
            if !r.enabled {
                return false;
            }
            let Ok(selected) = checked_directory(Path::new(&r.path)) else {
                return false;
            };
            let prefix = path_key(&selected);
            target == prefix
                || target.starts_with(&(prefix + if cfg!(windows) { "\\" } else { "/" }))
        }))
    }
    pub(super) fn artifact_packages(&self) -> Result<Vec<ArtifactPackage>> {
        let mut statement = self
            .registry
            .db
            .conn
            .prepare(
                "SELECT package_json FROM artifact_packages ORDER BY updated_at DESC LIMIT 1000",
            )
            .map_err(sql_err)?;
        let rows = statement
            .query_map([], |r| r.get::<_, String>(0))
            .map_err(sql_err)?;
        rows.map(|r| serde_json::from_str(&r.map_err(sql_err)?).map_err(json_err))
            .collect()
    }
    fn artifact_package(&self, id: &str) -> Result<ArtifactPackage> {
        let raw = self
            .registry
            .db
            .conn
            .query_row(
                "SELECT package_json FROM artifact_packages WHERE id=?1",
                [id],
                |r| r.get::<_, String>(0),
            )
            .optional()
            .map_err(sql_err)?
            .ok_or_else(|| ProtocolError::new(ErrorCode::NotFound, "Artifact package not found"))?;
        let item: ArtifactPackage = serde_json::from_str(&raw).map_err(json_err)?;
        if !self.is_admin()
            && item.owner != self.peer_principal()
            && !self.artifact_scope(Path::new(&item.root))?
        {
            return Err(denied("Package is outside shared management scope"));
        }
        Ok(item)
    }
    pub(super) fn save_artifact(&mut self, item: &mut ArtifactPackage, kind: &str) -> Result<()> {
        item.updated_at = now();
        let encoded = serde_json::to_string(item).map_err(json_err)?;
        let tx = self.registry.db.conn.transaction().map_err(sql_err)?;
        tx.execute("INSERT INTO artifact_packages(id,owner,package_json,updated_at) VALUES(?1,?2,?3,?4) ON CONFLICT(id) DO UPDATE SET package_json=excluded.package_json,updated_at=excluded.updated_at",rusqlite::params![item.id,item.owner,encoded,item.updated_at]).map_err(sql_err)?;
        tx.execute(
            "INSERT INTO artifact_events(package_id,at,kind,data_json) VALUES(?1,?2,?3,?4)",
            rusqlite::params![
                item.id,
                item.updated_at,
                kind,
                json!({"status":item.status,"priority":item.priority,"reason":item.reason})
                    .to_string()
            ],
        )
        .map_err(sql_err)?;
        tx.commit().map_err(sql_err)?;
        self.event(
            &format!("artifact.{kind}"),
            &format!("产物包 {} · {}", item.name, item.status),
            json!({"package_id":item.id,"status":item.status}),
        )?;
        Ok(())
    }
    pub(super) fn dispatch_artifact(&mut self, method: &str, params: &Value) -> Result<Value> {
        match method {
            "artifact.roots" => Ok(json!(self.artifact_roots()?)),
            "artifact.root_add" => {
                self.require_controller()?;
                let root = checked_directory(Path::new(text(params, "path")?))?;
                let path = plain(&root);
                let roots = self.artifact_roots()?;
                if let Some(existing) = roots
                    .iter()
                    .find(|r| path_key(Path::new(&r.path)) == path_key(&root))
                {
                    self.registry
                        .db
                        .conn
                        .execute(
                            "UPDATE artifact_roots SET enabled=1 WHERE id=?1",
                            [&existing.id],
                        )
                        .map_err(sql_err)?;
                    return Ok(json!(self.artifact_roots()?));
                }
                if roots.len() >= 32 {
                    return Err(invalid("At most 32 artifact roots"));
                }
                self.registry.db.conn.execute("INSERT INTO artifact_roots(id,path,enabled) VALUES(?1,?2,1) ON CONFLICT(path) DO UPDATE SET enabled=1",rusqlite::params![uuid::Uuid::new_v4().to_string(),path]).map_err(sql_err)?;
                Ok(json!(self.artifact_roots()?))
            }
            "artifact.root_disable" => {
                self.require_controller()?;
                let changed = self
                    .registry
                    .db
                    .conn
                    .execute(
                        "UPDATE artifact_roots SET enabled=0 WHERE id=?1",
                        [text(params, "id")?],
                    )
                    .map_err(sql_err)?;
                if changed == 0 {
                    return Err(invalid("Unknown artifact root"));
                }
                Ok(json!({"disabled":true}))
            }
            "artifact.list" => {
                let query = params["query"].as_str().unwrap_or("").to_lowercase();
                let offset = params["offset"].as_u64().unwrap_or(0) as usize;
                let limit = params["limit"].as_u64().unwrap_or(20).clamp(1, 20) as usize;
                let mut skipped = 0;
                let mut rows = Vec::new();
                let scopes: Vec<_> = self
                    .artifact_roots()?
                    .into_iter()
                    .filter(|r| r.enabled)
                    .filter_map(|r| checked_directory(Path::new(&r.path)).ok())
                    .map(|p| path_key(&p))
                    .collect();
                for item in self.artifact_packages()? {
                    let key = path_key(Path::new(&item.root));
                    let scope = scopes.iter().any(|p| {
                        key == *p
                            || key
                                .starts_with(&(p.clone() + if cfg!(windows) { "\\" } else { "/" }))
                    });
                    if (self.is_admin() || item.owner == self.peer_principal() || scope)
                        && format!("{} {} {}", item.name, item.task, item.source)
                            .to_lowercase()
                            .contains(&query)
                    {
                        if skipped < offset {
                            skipped += 1;
                            continue;
                        }
                        let count = item.files.len();
                        let bytes = item.files.iter().map(|f| f.bytes).sum::<u64>();
                        let remaining = item
                            .files
                            .iter()
                            .filter(|f| f.status == "present")
                            .map(|f| f.bytes)
                            .sum::<u64>();
                        let mut value = serde_json::to_value(item).map_err(json_err)?;
                        value.as_object_mut().unwrap().remove("owner");
                        value["files"] = json!([]);
                        value["file_count"] = json!(count);
                        value["bytes"] = json!(bytes);
                        value["remaining_bytes"] = json!(remaining);
                        value["reason"] = json!(value["reason"]
                            .as_str()
                            .unwrap_or("")
                            .chars()
                            .take(256)
                            .collect::<String>());
                        value["auto_scope"] = json!(scope);
                        rows.push(value);
                        if rows.len() >= limit {
                            break;
                        }
                    }
                }
                Ok(json!(rows))
            }
            "artifact.inspect" => {
                let item = self.artifact_package(text(params, "id")?)?;
                let mut statement=self.registry.db.conn.prepare("SELECT at,kind,data_json FROM artifact_events WHERE package_id=?1 ORDER BY seq DESC LIMIT 40").map_err(sql_err)?;
                let rows=statement.query_map([&item.id],|r|Ok(json!({"at":r.get::<_,String>(0)?,"kind":r.get::<_,String>(1)?,"data":serde_json::from_str::<Value>(&r.get::<_,String>(2)?).unwrap_or(Value::Null)}))).map_err(sql_err)?;
                Ok(
                    json!({"package":item,"events":rows.collect::<std::result::Result<Vec<_>,_>>().map_err(sql_err)?}),
                )
            }
            "artifact.register" => self.register_artifact(params),
            "artifact.complete" => self.complete_artifact(params),
            "artifact.review" => self.review_artifact(params),
            "artifact.update" => self.update_artifact(params),
            "artifact.recycle" => {
                self.require_controller()?;
                if params["confirmed"] != true {
                    return Err(invalid("Explicit recycle confirmation required"));
                }
                let mut item = self.artifact_package(text(params, "id")?)?;
                item.reason = "用户手动移入回收站".into();
                self.recycle_artifact(&mut item, true)?;
                Ok(json!(item))
            }
            "artifact.forget" => {
                self.require_controller()?;
                let mut item = self.artifact_package(text(params, "id")?)?;
                if item.status == "recycling" {
                    return Err(invalid("Cleanup is in progress"));
                }
                item.status = "released".into();
                item.cleanup_after = None;
                item.retained = true;
                item.reason = "用户解除登记，保留原文件；如需管理新版本，请重新选择登记".into();
                for file in &mut item.files {
                    if file.status == "present" {
                        file.status = "released".into();
                    }
                }
                self.save_artifact(&mut item, "registration_released")?;
                Ok(json!(item))
            }
            "artifact.scan" => self.scan_artifacts(params),
            _ => Err(invalid("Unknown artifact method")),
        }
    }
    fn register_artifact(&mut self, params: &Value) -> Result<Value> {
        let root = checked_directory(Path::new(text(params, "root")?))?;
        let inputs = params["files"]
            .as_array()
            .filter(|v| !v.is_empty() && v.len() <= ARTIFACT_MAX_FILES)
            .ok_or_else(|| {
                invalid("Choose 1–128 explicit files; folders and globs are not accepted")
            })?;
        let priority: ArtifactPriority =
            serde_json::from_value(params.get("priority").cloned().unwrap_or(json!("high")))
                .map_err(json_err)?;
        let mut seen = BTreeSet::new();
        let mut files = Vec::new();
        let mut bytes = 0;
        for input in inputs {
            let path = input
                .as_str()
                .ok_or_else(|| invalid("files must contain relative strings"))?;
            let full = checked_file(&root, path)?;
            if !seen.insert(path_key(&full)) {
                return Err(invalid("Duplicate artifact file"));
            }
            self.check_artifact_reference(&full)?;
            let file = fingerprint(&root, path)?;
            bytes += file.bytes;
            if bytes > 1024 * 1024 * 1024 {
                return Err(invalid("Package exceeds 1 GiB"));
            }
            files.push(file);
        }
        let active_files: BTreeSet<_> = self
            .artifact_packages()?
            .iter()
            .flat_map(|p| {
                p.files
                    .iter()
                    .filter(|f| f.status == "present")
                    .map(|f| path_key(&Path::new(&p.root).join(&f.relative_path)))
                    .collect::<Vec<_>>()
            })
            .collect();
        if seen.iter().any(|p| active_files.contains(p)) {
            return Err(invalid(
                "File already belongs to another active artifact package",
            ));
        }
        if self.artifact_packages()?.len() >= 1000 {
            return Err(invalid("Artifact registry is full"));
        }
        let mut item = ArtifactPackage {
            id: uuid::Uuid::new_v4().to_string(),
            name: bound(params, "name", 256)?,
            task: bound(params, "task", 2048)?,
            source: params["source"]
                .as_str()
                .unwrap_or("用户手动登记")
                .chars()
                .take(128)
                .collect(),
            owner: self.peer_principal(),
            root: plain(&root),
            priority,
            retained: false,
            status: "active".into(),
            files,
            completed: false,
            no_longer_needed: false,
            cleanup_after: None,
            reason: String::new(),
            reviewer: None,
            created_at: now(),
            updated_at: now(),
        };
        self.save_artifact(&mut item, "registered")?;
        Ok(
            json!({"id":item.id,"package":item,"auto_scope":self.artifact_scope(&root)?,"note":"Registration never deletes. Outside user-selected scopes, automatic cleanup is disabled; the Agent may continue its task."}),
        )
    }
    fn complete_artifact(&mut self, params: &Value) -> Result<Value> {
        let mut item = self.artifact_package(text(params, "id")?)?;
        if !self.is_admin() && item.owner != self.peer_principal() {
            return Err(denied("Only the creator can mark its task complete"));
        }
        if matches!(
            item.status.as_str(),
            "trashed" | "recycling" | "partial" | "released"
        ) {
            return Err(invalid("Package has already entered recycling"));
        }
        if params["no_longer_needed"] != true {
            return Err(invalid("Explicit no_longer_needed=true is required"));
        }
        item.completed = true;
        item.no_longer_needed = true;
        item.reason = bound(params, "reason", 2048)?;
        item.cleanup_after = None;
        item.status = if item.retained || item.priority == ArtifactPriority::High {
            "retained"
        } else if !self.artifact_scope(Path::new(&item.root))? {
            "awaiting_scope"
        } else if item.priority == ArtifactPriority::Low {
            item.cleanup_after = Some(chrono::Utc::now().timestamp() + ARTIFACT_GRACE_SECONDS);
            "scheduled"
        } else {
            "needs_review"
        }
        .into();
        self.save_artifact(&mut item, "completed")?;
        Ok(json!(item))
    }
    fn review_artifact(&mut self, params: &Value) -> Result<Value> {
        let mut item = self.artifact_package(text(params, "id")?)?;
        if item.priority != ArtifactPriority::Medium
            || item.retained
            || !item.completed
            || !item.no_longer_needed
            || !self.artifact_scope(Path::new(&item.root))?
            || !matches!(item.status.as_str(), "needs_review" | "blocked")
        {
            return Err(denied("AI review only applies to completed, unretained medium packages in a selected scope"));
        }
        item.reason = bound(params, "reason", 2048)?;
        let decision = text(params, "decision")?;
        item.reviewer = Some(self.peer_principal());
        if decision == "retain" {
            item.status = "retained".into();
            item.cleanup_after = None;
        } else if decision == "recycle" {
            if params["unused"] != true || params["dependencies_checked"] != true {
                return Err(invalid(
                    "AI must confirm unused=true and dependencies_checked=true",
                ));
            }
            self.validate_artifact_cleanup(&item)?;
            item.status = "scheduled".into();
            item.cleanup_after = Some(chrono::Utc::now().timestamp() + ARTIFACT_GRACE_SECONDS);
        } else {
            return Err(invalid("decision must be recycle or retain"));
        }
        self.save_artifact(&mut item, "ai_reviewed")?;
        Ok(json!(item))
    }
    fn update_artifact(&mut self, params: &Value) -> Result<Value> {
        self.require_controller()?;
        let mut item = self.artifact_package(text(params, "id")?)?;
        if matches!(item.status.as_str(), "trashed" | "recycling" | "released") {
            return Err(invalid("Package is already recycled"));
        }
        if let Some(priority) = params.get("priority") {
            item.priority = serde_json::from_value(priority.clone()).map_err(json_err)?;
        }
        if let Some(retained) = params.get("retained") {
            item.retained = retained
                .as_bool()
                .ok_or_else(|| invalid("retained must be boolean"))?;
        }
        item.cleanup_after = None;
        item.reviewer = None;
        item.status = if item.retained || item.priority == ArtifactPriority::High {
            "retained"
        } else if item.completed && item.no_longer_needed {
            if !self.artifact_scope(Path::new(&item.root))? {
                "awaiting_scope"
            } else if item.priority == ArtifactPriority::Low {
                item.cleanup_after = Some(chrono::Utc::now().timestamp() + ARTIFACT_GRACE_SECONDS);
                "scheduled"
            } else {
                "needs_review"
            }
        } else {
            "active"
        }
        .into();
        self.save_artifact(&mut item, "user_updated")?;
        Ok(json!(item))
    }
    fn check_artifact_reference(&self, path: &Path) -> Result<()> {
        let key = path_key(path);
        let filename = path
            .file_name()
            .unwrap_or_default()
            .to_string_lossy()
            .to_lowercase();
        let mut statement=self.registry.db.conn.prepare("SELECT path FROM tool_instances UNION ALL SELECT path FROM skills WHERE path IS NOT NULL").map_err(sql_err)?;
        for input in statement
            .query_map([], |r| r.get::<_, String>(0))
            .map_err(sql_err)?
        {
            let reference = PathBuf::from(input.map_err(sql_err)?);
            if let Ok(reference) = reference.canonicalize() {
                if path_key(&reference) == key
                    || (reference.is_dir() && path.starts_with(reference))
                {
                    return Err(denied(
                        "Artifact is referenced by a registered tool or Skill",
                    ));
                }
            }
        }
        let mut statement = self
            .registry
            .db
            .conn
            .prepare("SELECT entry_json FROM program_entries")
            .map_err(sql_err)?;
        for input in statement
            .query_map([], |r| r.get::<_, String>(0))
            .map_err(sql_err)?
        {
            let entry: Value = serde_json::from_str(&input.map_err(sql_err)?).map_err(json_err)?;
            if entry["path"]
                .as_str()
                .is_some_and(|p| path_key(Path::new(p)) == key)
                || entry["command"]
                    .as_str()
                    .is_some_and(|command| command.to_lowercase().contains(&filename))
                || entry["args"].as_array().is_some_and(|a| {
                    a.iter()
                        .filter_map(Value::as_str)
                        .any(|a| a.to_lowercase().contains(&filename))
                })
            {
                return Err(denied("Artifact is referenced by a saved program"));
            }
        }
        Ok(())
    }
    fn validate_artifact_cleanup(&self, item: &ArtifactPackage) -> Result<()> {
        if !self.running_jobs.is_empty() || !self.running_scans.is_empty() {
            return Err(denied("ToolHub tasks are running; defer artifact cleanup"));
        }
        let root = Path::new(&item.root);
        checked_directory(root)?;
        for file in item.files.iter().filter(|f| f.status == "present") {
            let path = checked_file(root, &file.relative_path)?;
            self.check_artifact_reference(&path)?;
            let actual = fingerprint(root, &file.relative_path)?;
            if actual.sha256 != file.sha256
                || actual.bytes != file.bytes
                || actual.modified_ns != file.modified_ns
            {
                return Err(denied("Artifact changed since registration; keep it and register the new version explicitly"));
            }
        }
        Ok(())
    }
    fn recycle_artifact(&mut self, item: &mut ArtifactPackage, manual: bool) -> Result<()> {
        if matches!(item.status.as_str(), "trashed" | "recycling" | "released") {
            return Err(invalid("Package already recycled"));
        }
        if !manual
            && (item.retained
                || item.priority == ArtifactPriority::High
                || !item.completed
                || !item.no_longer_needed
                || !self.artifact_scope(Path::new(&item.root))?)
        {
            return Err(denied(
                "Package is protected or outside automatic cleanup scope",
            ));
        }
        self.validate_artifact_cleanup(item)?;
        item.status = "recycling".into();
        item.cleanup_after = None;
        self.save_artifact(item, "recycle_started")?;
        for index in 0..item.files.len() {
            if item.files[index].status != "present" {
                continue;
            }
            let file = &item.files[index];
            let root = Path::new(&item.root);
            let result = (|| -> Result<()> {
                let path = checked_file(root, &file.relative_path)?;
                self.check_artifact_reference(&path)?;
                let held = open_read(&path)?;
                let actual = fingerprint(root, &file.relative_path)?;
                if actual.sha256 != file.sha256 || actual.modified_ns != file.modified_ns {
                    return Err(denied("Artifact changed during cleanup"));
                }
                recycle::recycle(&path).map_err(|e| {
                    ProtocolError::new(
                        ErrorCode::Unavailable,
                        format!("Recycle Bin operation failed; no permanent fallback: {e}"),
                    )
                })?;
                drop(held);
                Ok(())
            })();
            match result {
                Ok(()) => item.files[index].status = "trashed".into(),
                Err(e) => {
                    item.files[index].error = Some(e.message.clone());
                    item.status = if item.files.iter().any(|f| f.status == "trashed") {
                        "partial"
                    } else {
                        "blocked"
                    }
                    .into();
                    self.save_artifact(item, "recycle_failed")?;
                    return Ok(());
                }
            }
            self.save_artifact(item, "file_recycled")?;
        }
        item.status = "trashed".into();
        self.save_artifact(item, "recycled")?;
        Ok(())
    }
    pub fn artifact_cleanup_tick(&mut self) -> Result<()> {
        if !self.module_enabled("artifacts")? {
            return Ok(());
        }
        if !self.running_jobs.is_empty() || !self.running_scans.is_empty() {
            return Ok(());
        }
        let now = chrono::Utc::now().timestamp();
        for mut item in self
            .artifact_packages()?
            .into_iter()
            .filter(|p| p.status == "scheduled" && p.cleanup_after.is_some_and(|at| at <= now))
            .take(4)
        {
            if let Err(e) = self.recycle_artifact(&mut item, false) {
                item.status = "blocked".into();
                item.cleanup_after = None;
                item.reason = format!("自动清理暂缓：{}", e.message);
                self.save_artifact(&mut item, "cleanup_blocked")?;
            }
        }
        Ok(())
    }
    pub fn artifact_recover_interrupted(&mut self) -> Result<()> {
        for mut item in self
            .artifact_packages()?
            .into_iter()
            .filter(|p| p.status == "recycling")
        {
            item.status = "blocked".into();
            item.cleanup_after = None;
            item.reason = "上次清理被中断，请检查原目录与系统回收站后人工处理".into();
            self.save_artifact(&mut item, "interrupted")?;
        }
        Ok(())
    }
    fn scan_artifacts(&mut self, params: &Value) -> Result<Value> {
        self.require_controller()?;
        let id = text(params, "root_id")?;
        let selected = self
            .artifact_roots()?
            .into_iter()
            .find(|r| r.id == id && r.enabled)
            .ok_or_else(|| invalid("Choose an enabled artifact root"))?;
        let root = checked_directory(Path::new(&selected.path))?;
        let mut stack = vec![(root.clone(), 0)];
        let mut files = Vec::new();
        let mut visited = 0;
        let mut skipped = 0;
        let mut unreadable = 0;
        let mut truncated = false;
        let existing: BTreeSet<_> = self
            .artifact_packages()?
            .iter()
            .flat_map(|p| {
                p.files
                    .iter()
                    .filter(|f| f.status == "present")
                    .map(|f| path_key(&Path::new(&p.root).join(&f.relative_path)))
                    .collect::<Vec<_>>()
            })
            .collect();
        while let Some((directory, depth)) = stack.pop() {
            let entries = match std::fs::read_dir(directory) {
                Ok(e) => e,
                Err(_) => {
                    unreadable += 1;
                    continue;
                }
            };
            for entry in entries {
                visited += 1;
                if visited > 20000 || files.len() >= 500 {
                    truncated = true;
                    break;
                }
                let entry = match entry {
                    Ok(e) => e,
                    Err(_) => {
                        unreadable += 1;
                        continue;
                    }
                };
                let path = entry.path();
                let metadata = match std::fs::symlink_metadata(&path) {
                    Ok(m) => m,
                    Err(_) => {
                        unreadable += 1;
                        continue;
                    }
                };
                if is_link(&metadata) || forbidden(&path) {
                    skipped += 1;
                    continue;
                }
                if metadata.is_dir() {
                    if depth < 6 {
                        stack.push((path, depth + 1));
                    } else {
                        skipped += 1;
                    }
                    continue;
                }
                let name = entry.file_name().to_string_lossy().to_lowercase();
                if !metadata.is_file()
                    || metadata.len() > ARTIFACT_MAX_FILE_BYTES
                    || existing.contains(&path_key(&path))
                    || self.check_artifact_reference(&path).is_err()
                {
                    skipped += 1;
                    continue;
                }
                let candidate=name.starts_with("test_") || name.starts_with("verify_") || name.starts_with("tmp_") || name.contains("backup") || [".bak",".tmp",".old",".log"].iter().any(|suffix|name.ends_with(suffix)) || path.components().any(|c|matches!(c,Component::Normal(n) if matches!(n.to_string_lossy().to_lowercase().as_str(),"backups"|"tmp"|"temp"|"test-results"|"playwright-report")));
                if candidate {
                    files.push(json!({"relative_path":path.strip_prefix(&root).unwrap().to_string_lossy(),"bytes":metadata.len(),"evidence":"名称或目录符合测试/备份/临时文件模式；不证明 Agent 来源或可删除性"}));
                }
            }
            if truncated {
                break;
            }
        }
        Ok(
            json!({"root":selected.path,"root_id":selected.id,"files":files,"visited":visited,"skipped":skipped,"unreadable":unreadable,"truncated":truncated,"limitations":"仅扫描选中目录；深度 6 / 20000 项 / 500 候选。候选需勾选收录，默认高优先级，不会自动删除。"}),
        )
    }
}

#[cfg(test)]
#[path = "artifact_tests.rs"]
mod tests;
