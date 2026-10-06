//! Read-only project entrypoint discovery. Evidence describes candidates, never AI authorship.
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProgramCandidate {
    pub id: String,
    pub kind: String,
    pub name: String,
    pub path: String,
    pub cwd: String,
    pub command: String,
    pub evidence: Vec<String>,
}

#[derive(Debug, Clone, Default, Serialize)]
pub struct ProgramScan {
    pub id: String,
    pub status: String,
    pub roots: Vec<String>,
    pub roots_attempted: Vec<String>,
    pub visited: usize,
    pub skipped: usize,
    pub unreadable: usize,
    pub current: String,
    pub limitations: Vec<String>,
    pub candidates: Vec<ProgramCandidate>,
}

#[derive(Clone)]
struct Project {
    root: PathBuf,
    evidence: Vec<String>,
}

pub fn supported_file(path: &Path) -> bool {
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        return path.is_file()
            && (matches!(extension(path).as_str(), "sh" | "command")
                || std::fs::metadata(path).is_ok_and(|m| m.permissions().mode() & 0o111 != 0));
    }
    #[cfg(not(unix))]
    matches!(
        extension(path).as_str(),
        "exe" | "bat" | "cmd" | "lnk" | "ps1"
    )
}

fn extension(path: &Path) -> String {
    path.extension()
        .unwrap_or_default()
        .to_string_lossy()
        .to_lowercase()
}

pub fn manual_candidate(path: &Path, kind: &str) -> Result<ProgramCandidate, String> {
    if !path.is_absolute()
        || !(if kind == "command" {
            path.is_dir()
        } else {
            path.is_file() && supported_file(path)
        })
    {
        return Err("请选择现有的启动文件或命令工作目录".into());
    }
    if !matches!(kind, "file" | "command") {
        return Err("无效的程序类型".into());
    }
    let path = std::fs::canonicalize(path).map_err(|e| e.to_string())?;
    let display = display_path(&path);
    let cwd = if kind == "command" {
        display.clone()
    } else {
        display_path(path.parent().ok_or("文件目录不可用")?)
    };
    Ok(ProgramCandidate {
        id: uuid::Uuid::new_v4().to_string(),
        kind: kind.into(),
        name: path
            .file_stem()
            .unwrap_or_default()
            .to_string_lossy()
            .into(),
        path: if kind == "command" {
            String::new()
        } else {
            display
        },
        cwd,
        command: String::new(),
        evidence: vec!["用户选择".into()],
    })
}

pub fn display_path(path: &Path) -> String {
    let text = path.to_string_lossy();
    #[cfg(windows)]
    if let Some(rest) = text.strip_prefix(r"\\?\UNC\") {
        return format!(r"\\{rest}");
    }
    #[cfg(windows)]
    if let Some(rest) = text.strip_prefix(r"\\?\") {
        return rest.into();
    }
    text.into()
}

pub fn local_disk_roots() -> Vec<String> {
    #[cfg(windows)]
    {
        use windows_sys::Win32::Storage::FileSystem::{GetDriveTypeW, GetLogicalDrives};
        let drives = unsafe { GetLogicalDrives() };
        (0..26)
            .filter_map(|i| {
                if drives & (1 << i) == 0 {
                    return None;
                }
                let root = format!("{}:\\", char::from(b'A' + i));
                let wide: Vec<u16> = root.encode_utf16().chain(Some(0)).collect();
                // Local fixed and removable volumes only; disconnected/network drives cannot block a scan.
                matches!(unsafe { GetDriveTypeW(wide.as_ptr()) }, 2 | 3).then_some(root)
            })
            .collect()
    }
    #[cfg(not(windows))]
    {
        vec![dirs::home_dir()
            .unwrap_or_else(|| PathBuf::from("/"))
            .to_string_lossy()
            .into()]
    }
}

fn installed_roots() -> Vec<String> {
    #[cfg(windows)]
    let mut roots = Vec::new();
    #[cfg(not(windows))]
    let roots = Vec::new();
    #[cfg(windows)]
    {
        use winreg::{enums::*, RegKey};
        for hive in [HKEY_LOCAL_MACHINE, HKEY_CURRENT_USER] {
            for flags in [KEY_READ | KEY_WOW64_64KEY, KEY_READ | KEY_WOW64_32KEY] {
                if let Ok(key) = RegKey::predef(hive).open_subkey_with_flags(
                    r"SOFTWARE\Microsoft\Windows\CurrentVersion\Uninstall",
                    flags,
                ) {
                    for name in key.enum_keys().flatten() {
                        if let Ok(entry) = key.open_subkey(name) {
                            if let Ok(location) = entry.get_value::<String, _>("InstallLocation") {
                                let p = PathBuf::from(location.trim().trim_matches('"'));
                                if p.is_absolute() && p.components().count() > 2 && p.is_dir() {
                                    roots.push(format!(
                                        "{}/",
                                        p.to_string_lossy()
                                            .to_lowercase()
                                            .replace('\\', "/")
                                            .trim_end_matches('/')
                                    ));
                                }
                            }
                        }
                    }
                }
            }
        }
    }
    roots
}

fn excluded(path: &Path, installed: &[String]) -> bool {
    let normalized = format!(
        "{}/",
        path.to_string_lossy()
            .to_lowercase()
            .replace('\\', "/")
            .trim_end_matches('/')
    );
    let name = path
        .file_name()
        .unwrap_or_default()
        .to_string_lossy()
        .to_lowercase();
    let parent = path
        .parent()
        .and_then(Path::file_name)
        .unwrap_or_default()
        .to_string_lossy()
        .to_lowercase();
    matches!(
        name.as_str(),
        "windows"
            | "program files"
            | "program files (x86)"
            | "programdata"
            | "appdata"
            | "$recycle.bin"
            | "system volume information"
            | "recovery"
            | "windows.old"
            | "node_modules"
            | ".git"
            | ".venv"
            | "venv"
            | "site-packages"
            | "__pycache__"
            | ".cache"
            | ".cargo"
            | ".rustup"
            | ".npm"
            | ".pnpm-store"
            | "vendor"
            | ".next"
            | ".nuxt"
            | "coverage"
            | "venvs"
    ) || (matches!(parent.as_str(), "debug" | "release")
        && matches!(
            name.as_str(),
            "deps" | "incremental" | ".fingerprint" | "build" | "examples"
        ))
        || (name == "lib" && parent == "build")
        || installed.iter().any(|root| normalized.starts_with(root))
}

fn is_link(meta: &std::fs::Metadata) -> bool {
    #[cfg(windows)]
    {
        use std::os::windows::fs::MetadataExt;
        meta.file_attributes() & 0x400 != 0
    }
    #[cfg(not(windows))]
    {
        meta.file_type().is_symlink()
    }
}

fn bounded_json(path: &Path) -> Option<serde_json::Value> {
    if std::fs::metadata(path).ok()?.len() > 512 * 1024 {
        return None;
    }
    serde_json::from_slice(&std::fs::read(path).ok()?).ok()
}

pub fn scan_programs(roots: Vec<String>, state: Arc<Mutex<ProgramScan>>, cancel: Arc<AtomicBool>) {
    let installed = installed_roots();
    let mut stack: Vec<_> = roots
        .iter()
        .map(|p| (PathBuf::from(p), None::<Project>, 0usize))
        .collect();
    let mut report = state.lock().unwrap().clone();
    let mut last_publish = std::time::Instant::now();
    while let Some((dir, inherited, depth)) = stack.pop() {
        if cancel.load(Ordering::Relaxed) {
            report.status = "cancelled".into();
            break;
        }
        if report.visited >= 2_000_000 || report.candidates.len() >= 5000 {
            report
                .limitations
                .push("已达到本次扫描上限（200 万项 / 5000 个候选），可选择目录继续扫描".into());
            break;
        }
        if depth == 0 {
            report.roots_attempted.push(display_path(&dir));
        }
        if depth > 48 {
            report.skipped += 1;
            if !report.limitations.iter().any(|s| s.contains("48")) {
                report.limitations.push("部分路径超过 48 层深度限制".into());
            }
            continue;
        }
        if excluded(&dir, &installed) {
            report.skipped += 1;
            continue;
        }
        let entries = match std::fs::read_dir(&dir) {
            Ok(v) => v,
            Err(_) => {
                report.unreadable += 1;
                continue;
            }
        };
        let mut files = Vec::new();
        let mut has_source = false;
        let mut dirs = Vec::new();
        let mut evidence = Vec::new();
        for entry in entries {
            if cancel.load(Ordering::Relaxed) {
                break;
            }
            let entry = match entry {
                Ok(e) => e,
                Err(_) => {
                    report.unreadable += 1;
                    continue;
                }
            };
            report.visited += 1;
            if report.visited >= 2_000_000 {
                break;
            }
            let path = entry.path();
            let meta = match std::fs::symlink_metadata(&path) {
                Ok(m) => m,
                Err(_) => {
                    report.unreadable += 1;
                    continue;
                }
            };
            if is_link(&meta) {
                report.skipped += 1;
                continue;
            }
            let name = entry.file_name().to_string_lossy().to_lowercase();
            if matches!(
                name.as_str(),
                "agents.md" | "claude.md" | ".codex" | ".mimo" | ".cursor"
            ) {
                evidence.push(format!("含 Agent 项目标记：{name}"));
            }
            if matches!(
                name.as_str(),
                "package.json"
                    | "cargo.toml"
                    | "pyproject.toml"
                    | "requirements.txt"
                    | "go.mod"
                    | ".git"
            ) {
                evidence.push(format!("含项目文件：{name}"));
            }
            if meta.is_dir() {
                dirs.push(path);
            } else if meta.is_file() {
                has_source |= matches!(
                    extension(&path).as_str(),
                    "py" | "js" | "ts" | "cs" | "rs" | "go"
                );
                if supported_file(&path) {
                    files.push(path);
                }
            }
        }
        if evidence.is_empty() && has_source {
            evidence.push("同目录含源码文件".into());
        }
        let project = if evidence.is_empty() {
            inherited
        } else {
            Some(Project {
                root: dir.clone(),
                evidence,
            })
        };
        if let Some(project) = &project {
            for path in &files {
                if report.candidates.len() >= 5000 {
                    break;
                }
                let ext = extension(path);
                if !supported_file(path) {
                    continue;
                }
                let stem = path
                    .file_stem()
                    .unwrap_or_default()
                    .to_string_lossy()
                    .to_lowercase();
                if [
                    "unins",
                    "uninstall",
                    "setup",
                    "installer",
                    "update",
                    "crash",
                ]
                .iter()
                .any(|n| stem.contains(n))
                {
                    continue;
                }
                let entry_name = [
                    "start", "launch", "run", "serve", "dev", "preview", "open", "启动", "打开",
                ]
                .iter()
                .any(|n| stem.starts_with(n) || stem.ends_with(n));
                let packaged = path.components().any(|c| {
                    matches!(
                        c.as_os_str().to_string_lossy().to_lowercase().as_str(),
                        "dist" | "build" | "debug" | "release" | "publish" | "out"
                    )
                });
                let native_binary = if cfg!(windows) {
                    ext == "exe"
                } else {
                    !matches!(ext.as_str(), "sh" | "command")
                };
                if native_binary {
                    if [
                        "node", "python", "pythonw", "git", "cargo", "rustc", "ffmpeg", "7z",
                        "pip", "uv",
                    ]
                    .contains(&stem.as_str())
                    {
                        continue;
                    }
                    if !packaged && dir != project.root && !entry_name {
                        continue;
                    }
                } else if ext != "lnk" && !entry_name {
                    continue;
                }
                let mut candidate = match manual_candidate(path, "file") {
                    Ok(c) => c,
                    Err(_) => continue,
                };
                candidate.evidence = project.evidence.clone();
                candidate.evidence.push(
                    if native_binary {
                        "项目中的可执行入口"
                    } else {
                        "项目中的启动入口"
                    }
                    .into(),
                );
                report.candidates.push(candidate);
            }
            if dir == project.root {
                if let Some(pkg) = bounded_json(&dir.join("package.json")) {
                    if let Some(scripts) = pkg.get("scripts").and_then(serde_json::Value::as_object)
                    {
                        let manager = if dir.join("pnpm-lock.yaml").exists() {
                            "pnpm"
                        } else if dir.join("yarn.lock").exists() {
                            "yarn"
                        } else {
                            "npm"
                        };
                        for key in ["start", "dev", "serve", "preview"] {
                            if scripts.get(key).is_some_and(serde_json::Value::is_string)
                                && report.candidates.len() < 5000
                            {
                                if let Ok(mut c) = manual_candidate(&dir, "command") {
                                    let name = pkg
                                        .get("name")
                                        .and_then(serde_json::Value::as_str)
                                        .unwrap_or(&c.name)
                                        .chars()
                                        .take(96)
                                        .collect::<String>();
                                    c.name = format!("{name} · {key}");
                                    c.command = format!("{manager} run {key}");
                                    c.evidence =
                                        vec![format!("package.json 明确声明 scripts.{key}")];
                                    report.candidates.push(c);
                                }
                            }
                        }
                    }
                }
                for entry in ["app.py", "main.py"] {
                    if dir.join(entry).is_file() && report.candidates.len() < 5000 {
                        if let Ok(mut c) = manual_candidate(&dir, "command") {
                            c.name = format!("{} · {entry}", c.name);
                            c.command = format!("python {entry}");
                            c.evidence =
                                vec![format!("含 Python 常见入口 {entry}，需自行确认启动方式")];
                            report.candidates.push(c);
                        }
                    }
                }
            }
        }
        for path in dirs {
            stack.push((path, project.clone(), depth + 1));
        }
        report.current = display_path(&dir);
        if last_publish.elapsed().as_millis() >= 200 {
            *state.lock().unwrap() = report.clone();
            last_publish = std::time::Instant::now();
        }
    }
    if (report.visited >= 2_000_000 || report.candidates.len() >= 5000)
        && report.limitations.is_empty()
    {
        report
            .limitations
            .push("已达到本次扫描上限（200 万项 / 5000 个候选），可选择目录继续扫描".into());
    }
    if cancel.load(Ordering::Relaxed) {
        report.status = "cancelled".into();
    }
    if report.status == "running" {
        report.status = if report.unreadable > 0 || !report.limitations.is_empty() {
            "partial"
        } else {
            "completed"
        }
        .into();
    }
    let unvisited: Vec<_> = roots
        .iter()
        .filter(|root| !report.roots_attempted.contains(root))
        .cloned()
        .collect();
    if !unvisited.is_empty() {
        report
            .limitations
            .push(format!("尚未扫描这些根目录：{}", unvisited.join(" · ")));
    }
    if report.unreadable > 0 {
        report
            .limitations
            .push(format!("{} 个目录或文件无法读取", report.unreadable));
    }
    report.current.clear();
    *state.lock().unwrap() = report;
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn project_launchers_and_commands_without_dependency_noise() {
        let tmp = tempfile::tempdir().unwrap();
        let root = tmp.path().join("agent-project");
        std::fs::create_dir_all(root.join("node_modules/noise")).unwrap();
        std::fs::create_dir_all(root.join("target/debug/deps")).unwrap();
        std::fs::write(root.join("AGENTS.md"), "agent instructions").unwrap();
        std::fs::write(
            root.join("package.json"),
            r#"{"name":"example","scripts":{"dev":"vite","test":"test"}}"#,
        )
        .unwrap();
        let fixtures = if cfg!(windows) {
            vec![
                "start.cmd",
                "build.bat",
                "target/debug/app.exe",
                "target/debug/deps/noise.exe",
                "node_modules/noise/run.cmd",
            ]
        } else {
            vec![
                "start.command",
                "build.sh",
                "target/debug/app",
                "target/debug/deps/noise",
                "node_modules/noise/run.command",
            ]
        };
        for p in fixtures {
            std::fs::write(root.join(p), "fixture").unwrap();
            #[cfg(unix)]
            {
                use std::os::unix::fs::PermissionsExt;
                std::fs::set_permissions(root.join(p), std::fs::Permissions::from_mode(0o755))
                    .unwrap();
            }
        }
        let state = Arc::new(Mutex::new(ProgramScan {
            status: "running".into(),
            ..Default::default()
        }));
        scan_programs(
            vec![display_path(tmp.path())],
            state.clone(),
            Arc::new(AtomicBool::new(false)),
        );
        let scan = state.lock().unwrap();
        assert_eq!(scan.status, "completed");
        assert_eq!(scan.candidates.len(), 3);
        assert!(scan.candidates.iter().any(|c| c.command == "npm run dev"));
        assert!(!scan.candidates.iter().any(|c| c.path.contains("noise")
            || c.path.ends_with("build.bat")
            || c.path.ends_with("build.sh")));
    }
    #[test]
    fn cancel_and_manual_validation() {
        let state = Arc::new(Mutex::new(ProgramScan {
            status: "running".into(),
            ..Default::default()
        }));
        scan_programs(
            vec!["/".into()],
            state.clone(),
            Arc::new(AtomicBool::new(true)),
        );
        assert_eq!(state.lock().unwrap().status, "cancelled");
        assert!(manual_candidate(Path::new("relative.exe"), "file").is_err());
    }
    #[test]
    fn installed_path_boundaries_and_large_manifest_names() {
        assert!(excluded(
            Path::new("D:/Apps/registered/subfolder"),
            &["d:/apps/registered/".into()]
        ));
        assert!(!excluded(
            Path::new("D:/Apps/registered-other"),
            &["d:/apps/registered/".into()]
        ));
        let tmp = tempfile::tempdir().unwrap();
        std::fs::write(
            tmp.path().join("package.json"),
            serde_json::json!({"name":"x".repeat(100_000),"scripts":{"dev":"vite"}}).to_string(),
        )
        .unwrap();
        let state = Arc::new(Mutex::new(ProgramScan {
            status: "running".into(),
            ..Default::default()
        }));
        let root = display_path(tmp.path());
        scan_programs(
            vec![root.clone()],
            state.clone(),
            Arc::new(AtomicBool::new(false)),
        );
        let scan = state.lock().unwrap();
        assert_eq!(scan.roots_attempted, vec![root]);
        assert_eq!(scan.candidates.len(), 1);
        assert!(scan.candidates[0].name.len() < 128);
    }
}
