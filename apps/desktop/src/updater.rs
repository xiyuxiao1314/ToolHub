//! Native-only update UI, staged helper and whole-distribution activation.
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::{
    fs,
    path::{Path, PathBuf},
    process::{Child, Command, Stdio},
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc, Mutex,
    },
    time::{Duration, Instant, SystemTime, UNIX_EPOCH},
};
use toolhub_updater::{self as package, Offer};
type Result<T> = std::result::Result<T, String>;
#[cfg(windows)]
#[link(name = "kernel32")]
extern "system" {
    fn MoveFileExW(existing: *const u16, replacement: *const u16, flags: u32) -> i32;
    fn GetSystemDirectoryW(buffer: *mut u16, size: u32) -> u32;
}
fn now() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
}
fn root() -> Result<PathBuf> {
    if let Some(registry) = std::env::var_os("TOOLHUB_REGISTRY") {
        return Ok(PathBuf::from(registry)
            .parent()
            .ok_or("隔离目录不可用")?
            .join("desktop-updates"));
    }
    Ok(dirs::data_local_dir()
        .ok_or("用户目录不可用")?
        .join("ToolHub/updates"))
}
fn directory(path: &Path) -> Result<()> {
    fs::create_dir_all(path).map_err(|e| e.to_string())?;
    let meta = fs::symlink_metadata(path).map_err(|e| e.to_string())?;
    #[cfg(windows)]
    {
        use std::os::windows::fs::MetadataExt;
        if meta.file_attributes() & 0x400 != 0 {
            return Err("更新目录不能是目录联接".into());
        }
    }
    if !meta.is_dir() || meta.file_type().is_symlink() {
        return Err("更新目录不可用".into());
    }
    Ok(())
}
fn read_json<T: serde::de::DeserializeOwned>(path: &Path) -> Result<T> {
    let meta = fs::symlink_metadata(path).map_err(|e| e.to_string())?;
    if !meta.is_file() || meta.file_type().is_symlink() || meta.len() > 65536 {
        return Err("更新记录格式不受支持".into());
    }
    serde_json::from_slice(&fs::read(path).map_err(|e| e.to_string())?).map_err(|e| e.to_string())
}
fn write_json(path: &Path, value: &impl Serialize) -> Result<()> {
    let parent = path.parent().ok_or("更新目录不可用")?;
    directory(parent)?;
    let temp = parent.join(format!(".update-write-{}", uuid::Uuid::new_v4()));
    fs::write(&temp, serde_json::to_vec(value).map_err(|e| e.to_string())?)
        .map_err(|e| e.to_string())?;
    #[cfg(windows)]
    {
        use std::os::windows::ffi::OsStrExt;
        let from: Vec<u16> = temp.as_os_str().encode_wide().chain(Some(0)).collect();
        let to: Vec<u16> = path.as_os_str().encode_wide().chain(Some(0)).collect();
        if unsafe { MoveFileExW(from.as_ptr(), to.as_ptr(), 0x1 | 0x8) } == 0 {
            return Err(std::io::Error::last_os_error().to_string());
        }
    }
    #[cfg(not(windows))]
    fs::rename(&temp, path).map_err(|e| e.to_string())?;
    Ok(())
}
#[derive(Clone, Serialize, Deserialize)]
#[serde(default)]
struct Preferences {
    auto_check: bool,
    preview: bool,
    last_check: u64,
}
impl Default for Preferences {
    fn default() -> Self {
        Self {
            auto_check: true,
            preview: env!("CARGO_PKG_VERSION").contains('-'),
            last_check: 0,
        }
    }
}
#[derive(Clone, Serialize)]
pub struct View {
    phase: String,
    current_version: String,
    available_version: Option<String>,
    downloaded: u64,
    total: u64,
    notes: String,
    error: Option<String>,
    auto_check: bool,
    preview: bool,
    last_check: u64,
    supported: bool,
    last_result: Option<Value>,
}
struct Inner {
    view: View,
    offer: Option<Offer>,
    staged: Option<PathBuf>,
    operation: String,
    cancel: Arc<AtomicBool>,
}
#[derive(Clone)]
pub struct Updates(Arc<Mutex<Inner>>);
impl Default for Updates {
    fn default() -> Self {
        let prefs = root()
            .and_then(|r| read_json::<Preferences>(&r.join("preferences.json")))
            .unwrap_or_default();
        let offer = root()
            .and_then(|r| read_json::<Offer>(&r.join("candidate.json")))
            .ok()
            .and_then(|mut o| {
                o.manifest = package::verify(
                    o.signed_json.as_bytes(),
                    &o.signature,
                    env!("CARGO_PKG_VERSION"),
                    package::platform().ok()?,
                )
                .ok()?;
                if !prefs.preview && o.manifest.version.contains('-') {
                    None
                } else {
                    Some(o)
                }
            });
        let last = root()
            .and_then(|r| read_json::<Value>(&r.join("last-result.json")))
            .ok();
        let view = View {
            phase: if offer.is_some() { "available" } else { "idle" }.into(),
            current_version: env!("CARGO_PKG_VERSION").into(),
            available_version: offer.as_ref().map(|o| o.manifest.version.clone()),
            downloaded: 0,
            total: offer.as_ref().map_or(0, |o| o.manifest.size),
            notes: offer.as_ref().map_or(String::new(), |o| o.notes.clone()),
            error: None,
            auto_check: prefs.auto_check,
            preview: prefs.preview,
            last_check: prefs.last_check,
            supported: package::platform().is_ok(),
            last_result: last,
        };
        Self(Arc::new(Mutex::new(Inner {
            view,
            offer,
            staged: None,
            operation: String::new(),
            cancel: Arc::new(AtomicBool::new(false)),
        })))
    }
}
fn active(phase: &str) -> bool {
    matches!(
        phase,
        "checking" | "downloading" | "cancelling" | "installing"
    )
}
#[tauri::command]
pub fn updater_status(state: tauri::State<'_, Updates>) -> View {
    let mut inner = state.0.lock().unwrap();
    if let Ok(last) = root().and_then(|r| read_json::<Value>(&r.join("last-result.json"))) {
        inner.view.last_result = Some(last);
    }
    inner.view.clone()
}
#[tauri::command]
pub fn updater_preferences(
    state: tauri::State<'_, Updates>,
    auto_check: bool,
    preview: bool,
) -> Result<View> {
    let mut inner = state.0.lock().unwrap();
    if active(&inner.view.phase) {
        return Err("请等待当前更新操作完成".into());
    }
    let prefs = Preferences {
        auto_check,
        preview,
        last_check: inner.view.last_check,
    };
    write_json(&root()?.join("preferences.json"), &prefs)?;
    inner.view.auto_check = auto_check;
    inner.view.preview = preview;
    if !preview
        && inner
            .offer
            .as_ref()
            .is_some_and(|o| o.manifest.version.contains('-'))
    {
        inner.offer = None;
        inner.staged = None;
        inner.view.available_version = None;
        inner.view.phase = "idle".into();
    }
    Ok(inner.view.clone())
}
#[tauri::command]
pub async fn updater_check(state: tauri::State<'_, Updates>, automatic: bool) -> Result<View> {
    let owned = state.inner().clone();
    tauri::async_runtime::spawn_blocking(move || {
        let preview = {
            let mut inner = owned.0.lock().unwrap();
            if active(&inner.view.phase) {
                return Ok(inner.view.clone());
            }
            if automatic
                && (!inner.view.auto_check
                    || (now() >= inner.view.last_check && now() - inner.view.last_check < 21600))
            {
                return Ok(inner.view.clone());
            }
            inner.view.phase = "checking".into();
            inner.view.error = None;
            inner.view.preview
        };
        let result = package::check_latest(env!("CARGO_PKG_VERSION"), preview);
        let mut inner = owned.0.lock().unwrap();
        match result {
            Ok(offer) => {
                let prefs = Preferences {
                    auto_check: inner.view.auto_check,
                    preview,
                    last_check: now(),
                };
                if let Err(error) = write_json(&root()?.join("preferences.json"), &prefs) {
                    inner.view.phase = "error".into();
                    inner.view.error = Some(error);
                    return Ok(inner.view.clone());
                }
                inner.view.last_check = prefs.last_check;
                inner.view.available_version = offer.as_ref().map(|o| o.manifest.version.clone());
                inner.view.total = offer.as_ref().map_or(0, |o| o.manifest.size);
                inner.view.notes = offer.as_ref().map_or(String::new(), |o| o.notes.clone());
                if let Some(ref value) = offer {
                    if let Err(error) = write_json(&root()?.join("candidate.json"), value) {
                        inner.view.phase = "error".into();
                        inner.view.error = Some(error);
                        return Ok(inner.view.clone());
                    }
                } else {
                    let _ = fs::remove_file(root()?.join("candidate.json"));
                }
                inner.view.phase = if offer.is_some() {
                    "available"
                } else {
                    "current"
                }
                .into();
                inner.staged = None;
                inner.offer = offer;
            }
            Err(error) => {
                inner.view.phase = "error".into();
                inner.view.error = Some(error);
            }
        }
        Ok(inner.view.clone())
    })
    .await
    .map_err(|e| e.to_string())?
}
#[tauri::command]
pub fn updater_download(state: tauri::State<'_, Updates>) -> Result<View> {
    let owned = state.inner().clone();
    let (offer, operation, cancel, folder) = {
        let mut inner = owned.0.lock().unwrap();
        if active(&inner.view.phase) {
            return Err("已有更新操作正在进行".into());
        }
        let mut offer = inner.offer.clone().ok_or("请先检查更新")?;
        offer.manifest = package::verify(
            offer.signed_json.as_bytes(),
            &offer.signature,
            env!("CARGO_PKG_VERSION"),
            package::platform()?,
        )?;
        let operation = uuid::Uuid::new_v4().to_string();
        let folder = root()?.join(&operation);
        directory(&folder)?;
        write_json(&folder.join("offer.json"), &offer)?;
        let cancel = Arc::new(AtomicBool::new(false));
        inner.cancel = cancel.clone();
        inner.operation = operation.clone();
        inner.view.phase = "downloading".into();
        inner.view.downloaded = 0;
        inner.view.error = None;
        inner.staged = None;
        (offer, operation, cancel, folder)
    };
    let response = owned.0.lock().unwrap().view.clone();
    std::thread::spawn(move || {
        let result = package::download(
            &offer.manifest,
            &folder.join("update.zip"),
            &cancel,
            |count| {
                let mut inner = owned.0.lock().unwrap();
                if inner.operation == operation {
                    inner.view.downloaded = count;
                }
            },
        );
        let mut inner = owned.0.lock().unwrap();
        if inner.operation != operation {
            return;
        }
        match result {
            Ok(()) => {
                inner.staged = Some(folder);
                inner.view.phase = "ready".into()
            }
            Err(error) => {
                inner.view.phase = if cancel.load(Ordering::SeqCst) {
                    "cancelled"
                } else {
                    "error"
                }
                .into();
                inner.view.error = Some(error);
            }
        }
    });
    Ok(response)
}
#[tauri::command]
pub fn updater_cancel(state: tauri::State<'_, Updates>) -> Result<View> {
    let mut inner = state.0.lock().unwrap();
    if matches!(inner.view.phase.as_str(), "downloading" | "cancelling") {
        inner.cancel.store(true, Ordering::SeqCst);
        inner.view.phase = "cancelling".into();
    }
    Ok(inner.view.clone())
}
#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Plan {
    operation: String,
    source: PathBuf,
    parent_pid: u32,
    daemon_pid: u32,
}
fn sanitized_command(executable: &Path) -> Command {
    let mut command = Command::new(executable);
    command
        .env_clear()
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null());
    for key in [
        "SystemRoot",
        "SystemDrive",
        "ProgramData",
        "ALLUSERSPROFILE",
        "ProgramFiles",
        "ProgramFiles(x86)",
        "CommonProgramFiles",
        "CommonProgramFiles(x86)",
        "WINDIR",
        "USERPROFILE",
        "HOME",
        "LOCALAPPDATA",
        "APPDATA",
        "TEMP",
        "TMP",
        "PATH",
        "LANG",
    ] {
        if let Some(value) = std::env::var_os(key) {
            command.env(key, value);
        }
    }
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        command.creation_flags(0x08000000);
    }
    command
}
#[tauri::command]
pub async fn updater_install(
    app: tauri::AppHandle,
    state: tauri::State<'_, Updates>,
) -> Result<()> {
    if std::env::var_os("TOOLHUB_REGISTRY").is_some() {
        return Err("隔离实例不可更新正式安装".into());
    }
    let owned = state.inner().clone();
    tauri::async_runtime::spawn_blocking(move || {
        let folder = {
            let mut inner = owned.0.lock().unwrap();
            if inner.view.phase != "ready" {
                return Err("请先下载并验证更新".into());
            }
            inner.view.phase = "installing".into();
            inner.staged.clone().ok_or("更新缓存不存在")?
        };
        let result = (|| {
            let original = folder.clone();
            let folder = root()?.join(uuid::Uuid::new_v4().to_string());
            directory(&folder)?;
            fs::copy(original.join("update.zip"), folder.join("update.zip"))
                .map_err(|e| e.to_string())?;
            let offer: Offer = read_json(&original.join("offer.json"))?;
            write_json(&folder.join("offer.json"), &offer)?;
            let source = std::env::current_exe()
                .map_err(|e| e.to_string())?
                .canonicalize()
                .map_err(|e| e.to_string())?;
            let mut client = toolhub_ipc::RpcClient::connect_or_start(&super::daemon_bin()?)
                .map_err(|e| e.to_string())?;
            let lease = client
                .call("update.prepare", json!({}))
                .map_err(|e| e.to_string())?;
            let token = lease["token"]
                .as_str()
                .ok_or("后台未返回更新会话")?
                .to_string();
            let handoff = (|| {
                let operation = folder
                    .file_name()
                    .and_then(|v| v.to_str())
                    .ok_or("更新目录不可用")?
                    .to_string();
                uuid::Uuid::parse_str(&operation).map_err(|_| "更新标识无效")?;
                let plan = Plan {
                    operation,
                    source: source.clone(),
                    parent_pid: std::process::id(),
                    daemon_pid: lease["daemon_pid"]
                        .as_u64()
                        .and_then(|v| u32::try_from(v).ok())
                        .ok_or("后台进程无效")?,
                };
                write_json(&folder.join("plan.json"), &plan)?;
                let helper = folder.join(if cfg!(windows) {
                    "toolhub-update-helper.exe"
                } else {
                    "toolhub-update-helper"
                });
                fs::copy(&source, &helper).map_err(|e| e.to_string())?;
                #[cfg(unix)]
                {
                    use std::os::unix::fs::PermissionsExt;
                    fs::set_permissions(&helper, fs::Permissions::from_mode(0o700))
                        .map_err(|e| e.to_string())?;
                }
                if package::hash_file(&helper)? != package::hash_file(&source)? {
                    return Err("更新助手复制校验失败".into());
                }
                let mut child = sanitized_command(&helper)
                    .arg("--toolhub-apply-update")
                    .arg(&folder)
                    .spawn()
                    .map_err(|e| e.to_string())?;
                let started = Instant::now();
                loop {
                    if let Ok(value) = read_json::<Value>(&folder.join("prepared.json")) {
                        if value["ready"] == true {
                            break;
                        }
                        return Err(value["error"]
                            .as_str()
                            .unwrap_or("更新准备失败")
                            .to_string());
                    }
                    if child.try_wait().map_err(|e| e.to_string())?.is_some() {
                        return Err("更新助手未能启动，请重试".into());
                    }
                    if started.elapsed() > Duration::from_secs(60) {
                        let _ = child.kill();
                        let _ = child.wait();
                        return Err("更新准备超时，已保留当前版本".into());
                    }
                    std::thread::sleep(Duration::from_millis(100));
                }
                if let Err(error) = client.call("update.commit", json!({"token":token})) {
                    // The application stays open on failure; release only our own helper.
                    let _ = child.kill();
                    let _ = child.wait();
                    return Err(error.to_string());
                }
                app.exit(0);
                Ok(())
            })();
            if handoff.is_err() {
                let _ = client.call("update.cancel", json!({"token":token}));
            }
            handoff
        })();
        if let Err(ref error) = result {
            let mut inner = owned.0.lock().unwrap();
            inner.view.phase = "ready".into();
            inner.view.error = Some(error.clone());
        }
        result
    })
    .await
    .map_err(|e| e.to_string())?
}
#[cfg(windows)]
struct WaitProcess(windows::Win32::Foundation::HANDLE);
#[cfg(windows)]
impl Drop for WaitProcess {
    fn drop(&mut self) {
        unsafe {
            let _ = windows::Win32::Foundation::CloseHandle(self.0);
        }
    }
}
#[cfg(windows)]
impl WaitProcess {
    fn open(pid: u32, expected: &Path) -> Result<Self> {
        use windows::{core::PWSTR, Win32::System::Threading::*};
        let handle = unsafe {
            OpenProcess(
                PROCESS_QUERY_LIMITED_INFORMATION | PROCESS_SYNCHRONIZE,
                false,
                pid,
            )
        }
        .map_err(|e| e.to_string())?;
        let guard = Self(handle);
        let mut path = vec![0u16; 32768];
        let mut length = path.len() as u32;
        unsafe {
            QueryFullProcessImageNameW(
                handle,
                PROCESS_NAME_WIN32,
                PWSTR(path.as_mut_ptr()),
                &mut length,
            )
        }
        .map_err(|e| e.to_string())?;
        let actual = PathBuf::from(String::from_utf16_lossy(&path[..length as usize]))
            .canonicalize()
            .map_err(|e| e.to_string())?;
        if actual != expected.canonicalize().map_err(|e| e.to_string())? {
            return Err("更新交接进程身份不匹配".into());
        }
        Ok(guard)
    }
    fn exited(&self) -> bool {
        unsafe {
            windows::Win32::System::Threading::WaitForSingleObject(self.0, 0)
                == windows::Win32::Foundation::WAIT_OBJECT_0
        }
    }
}
fn app_bundle(executable: &Path) -> Result<PathBuf> {
    let contents = executable
        .parent()
        .and_then(Path::parent)
        .ok_or("应用位置不可用")?;
    let app = contents.parent().ok_or("应用位置不可用")?;
    if contents.file_name().is_none_or(|n| n != "Contents")
        || app.extension().is_none_or(|e| e != "app")
    {
        return Err("请从已安装的 ToolHub.app 中进行更新".into());
    }
    Ok(app.to_path_buf())
}
fn cli_probe(path: &Path, version: &str) -> Result<()> {
    let out = path
        .parent()
        .ok_or("更新程序目录不可用")?
        .join(".update-cli-version.txt");
    let file = fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&out)
        .map_err(|e| e.to_string())?;
    let mut child = sanitized_command(path)
        .arg("--version")
        .stdout(file)
        .spawn()
        .map_err(|e| e.to_string())?;
    let start = Instant::now();
    let status = loop {
        if let Some(s) = child.try_wait().map_err(|e| e.to_string())? {
            break s;
        }
        if start.elapsed() > Duration::from_secs(8) {
            let _ = child.kill();
            let _ = child.wait();
            return Err("更新程序版本检查超时".into());
        }
        std::thread::sleep(Duration::from_millis(50));
    };
    let text = fs::read_to_string(&out).map_err(|e| e.to_string())?;
    let _ = fs::remove_file(out);
    if !status.success() || text.trim() != format!("toolhub {version}") {
        return Err("更新程序真实版本与签名不一致".into());
    }
    Ok(())
}
fn stop_owned(child: &mut Option<Child>) {
    if let Some(child) = child {
        let _ = child.kill();
        let _ = child.wait();
    }
}
fn activation(folder: &Path, source: &Path, target: &Path, rollback: bool) -> Result<()> {
    #[cfg(windows)]
    {
        let script = folder.join("activate.ps1");
        if !script.exists() {
            fs::write(&script, include_bytes!("updater-activate.ps1"))
                .map_err(|e| e.to_string())?;
        }
        write_json(
            &folder.join("activation.json"),
            &json!({"source":source,"target":target,"root":dirs::data_local_dir().ok_or("用户目录不可用")?.join("ToolHub"),"stage":folder,"task_marker":crate::windows_startup_task::MARKER}),
        )?;
        let mut system = [0u16; 32768];
        let length = unsafe { GetSystemDirectoryW(system.as_mut_ptr(), system.len() as u32) };
        if length == 0 || length as usize >= system.len() {
            return Err("系统 PowerShell 路径不可用".into());
        }
        let powershell = PathBuf::from(String::from_utf16_lossy(&system[..length as usize]))
            .join("WindowsPowerShell/v1.0/powershell.exe");
        let mut command = sanitized_command(&powershell);
        command
            .args([
                "-NoLogo",
                "-NoProfile",
                "-NonInteractive",
                "-ExecutionPolicy",
                "Bypass",
                "-File",
            ])
            .arg(&script)
            .arg("-Request")
            .arg(folder.join("activation.json"));
        if rollback {
            command.arg("-Rollback");
        }
        let log = fs::File::create(folder.join(if rollback {
            "rollback.log"
        } else {
            "activation.log"
        }))
        .map_err(|e| e.to_string())?;
        let status = command
            .stdout(log.try_clone().map_err(|e| e.to_string())?)
            .stderr(log)
            .status()
            .map_err(|e| e.to_string())?;
        if !status.success() {
            return Err("更新激活未完成，已保留可恢复的旧版本；请查看设置中的更新结果".into());
        }
        Ok(())
    }
    #[cfg(not(windows))]
    {
        let _ = (folder, source, target, rollback);
        Ok(())
    }
}
fn apply(folder: &Path) -> Result<()> {
    let root = root()?;
    directory(&root)?;
    let folder = folder.canonicalize().map_err(|e| e.to_string())?;
    if folder.parent() != Some(root.canonicalize().map_err(|e| e.to_string())?.as_path()) {
        return Err("更新助手目录不匹配".into());
    }
    let plan: Plan = read_json(&folder.join("plan.json"))?;
    uuid::Uuid::parse_str(&plan.operation).map_err(|_| "更新标识无效")?;
    if folder.file_name().and_then(|p| p.to_str()) != Some(plan.operation.as_str()) {
        return Err("更新目录标识不一致".into());
    }
    let lock = fs::OpenOptions::new()
        .read(true)
        .write(true)
        .create(true)
        .open(root.join("install.lock"))
        .map_err(|e| e.to_string())?;
    lock.try_lock().map_err(|_| "另一个更新正在安装")?;
    let this = std::env::current_exe().map_err(|e| e.to_string())?;
    if this.parent().and_then(|p| p.canonicalize().ok()).as_ref() != Some(&folder)
        || package::hash_file(&this)? != package::hash_file(&plan.source)?
    {
        return Err("更新助手与当前程序不一致".into());
    }
    let offer: Offer = read_json(&folder.join("offer.json"))?;
    let manifest = package::verify(
        offer.signed_json.as_bytes(),
        &offer.signature,
        env!("CARGO_PKG_VERSION"),
        package::platform()?,
    )?;
    #[cfg(windows)]
    let parent = WaitProcess::open(plan.parent_pid, &plan.source)?;
    #[cfg(windows)]
    let daemon = WaitProcess::open(
        plan.daemon_pid,
        &plan
            .source
            .parent()
            .ok_or("旧版本目录不可用")?
            .join("toolhubd.exe"),
    )?;
    let payload = package::extract_verified(
        &folder.join("update.zip"),
        &folder.join("unpacked"),
        &manifest,
    )?;
    let target;
    let backup;
    if cfg!(windows) {
        let versions = dirs::data_local_dir()
            .ok_or("用户目录不可用")?
            .join("ToolHub/versions");
        directory(&versions)?;
        target = versions.join(format!(
            "update-{}-{}",
            manifest.version.replace('.', "-"),
            &plan.operation[..8]
        ));
        if target.exists() {
            return Err("目标更新目录已存在".into());
        }
        fs::rename(&payload, &target).map_err(|e| e.to_string())?;
        backup = None;
    } else {
        let app = app_bundle(&plan.source)?;
        let parent = app.parent().ok_or("应用目录不可用")?;
        target = parent.join(format!(".ToolHub-update-{}.app", plan.operation));
        backup = Some(parent.join(format!(".ToolHub-previous-{}.app", plan.operation)));
        // Copy to the same filesystem before any running application is replaced.
        copy_tree(&payload.join("ToolHub.app"), &target)?;
        let status = sanitized_command(Path::new("/usr/bin/codesign"))
            .args(["--verify", "--deep", "--strict"])
            .arg(&target)
            .status()
            .map_err(|e| e.to_string())?;
        if !status.success() {
            return Err("Mac 应用签名检查失败".into());
        }
    }
    let executable = if cfg!(windows) {
        target.join("toolhub-desktop.exe")
    } else {
        target.join("Contents/MacOS/toolhub-desktop")
    };
    cli_probe(
        &executable.with_file_name(if cfg!(windows) {
            "toolhub.exe"
        } else {
            "toolhub"
        }),
        &manifest.version,
    )?;
    write_json(&folder.join("prepared.json"), &json!({"ready":true}))?;
    let waiting = Instant::now();
    loop {
        #[cfg(windows)]
        let gone = parent.exited() && daemon.exited();
        #[cfg(unix)]
        let gone = unsafe {
            libc::kill(plan.parent_pid as i32, 0) != 0 && libc::kill(plan.daemon_pid as i32, 0) != 0
        };
        #[cfg(not(any(windows, unix)))]
        let gone = false;
        if gone {
            break;
        }
        if waiting.elapsed() > Duration::from_secs(90) {
            return Err("应用未完成退出，已保留当前版本".into());
        }
        std::thread::sleep(Duration::from_millis(100));
    }
    let mut running_app = None;
    let mut running_daemon = None;
    let mut committed = false;
    let mut restored = true;
    let result = (|| {
        let executable = if cfg!(windows) {
            activation(&folder, &plan.source, &target, false)?;
            executable.clone()
        } else {
            let app = app_bundle(&plan.source)?;
            let backup = backup.as_ref().unwrap();
            fs::rename(&app, backup).map_err(|e| e.to_string())?;
            if let Err(error) = fs::rename(&target, &app) {
                let _ = fs::rename(backup, &app);
                return Err(error.to_string());
            }
            app.join("Contents/MacOS/toolhub-desktop")
        };
        let daemon = executable.with_file_name(if cfg!(windows) {
            "toolhubd.exe"
        } else {
            "toolhubd"
        });
        running_daemon = Some(
            sanitized_command(&daemon)
                .arg("--listen")
                .arg("--update-probation")
                .arg(&plan.operation)
                .spawn()
                .map_err(|e| e.to_string())?,
        );
        running_app = Some(
            sanitized_command(&executable)
                .arg("--toolhub-update-health")
                .arg(&plan.operation)
                .current_dir(executable.parent().unwrap())
                .spawn()
                .map_err(|e| e.to_string())?,
        );
        let started = Instant::now();
        loop {
            if read_json::<Value>(&folder.join("boot-ready.json"))
                .ok()
                .is_some_and(|v| {
                    v["version"] == manifest.version && v["operation"] == plan.operation
                })
            {
                break;
            }
            if running_app
                .as_mut()
                .unwrap()
                .try_wait()
                .map_err(|e| e.to_string())?
                .is_some()
            {
                return Err("新版本启动失败".into());
            }
            if started.elapsed() > Duration::from_secs(45) {
                return Err("新版本启动确认超时".into());
            }
            std::thread::sleep(Duration::from_millis(100));
        }
        // Once committed, never kill the new processes: it may now accept user work.
        write_json(
            &folder.join("boot-commit.json"),
            &json!({"operation":plan.operation}),
        )?;
        committed = true;
        Ok(())
    })();
    if result.is_err() && !committed {
        stop_owned(&mut running_app);
        stop_owned(&mut running_daemon);
        if cfg!(windows) {
            restored = activation(&folder, &plan.source, &target, true).is_ok();
        } else {
            let app = app_bundle(&plan.source)?;
            let failed = target.with_extension("failed.app");
            if let Some(ref backup) = backup {
                if backup.exists() {
                    if app.exists() && fs::rename(&app, &failed).is_err() {
                        restored = false;
                    }
                    if restored && fs::rename(backup, &app).is_err() {
                        restored = false;
                    }
                }
            }
        }
        if restored {
            let _ = sanitized_command(&plan.source).spawn();
        }
    }
    let summary = json!({"status":if result.is_ok(){"updated"}else if committed||!restored{"needs_attention"}else{"rolled_back"},"version":manifest.version,"message":result.as_ref().err(),"at":now()});
    write_json(&root.join("last-result.json"), &summary)?;
    result
}
fn copy_tree(source: &Path, target: &Path) -> Result<()> {
    directory(target)?;
    for item in fs::read_dir(source).map_err(|e| e.to_string())? {
        let item = item.map_err(|e| e.to_string())?;
        let kind = item.file_type().map_err(|e| e.to_string())?;
        let out = target.join(item.file_name());
        if kind.is_symlink() {
            return Err("更新应用含不支持的链接".into());
        }
        if kind.is_dir() {
            copy_tree(&item.path(), &out)?;
        } else if kind.is_file() {
            fs::copy(item.path(), &out).map_err(|e| e.to_string())?;
        } else {
            return Err("更新应用含特殊文件".into());
        }
    }
    Ok(())
}
/// Runs before Tauri for the private updater helper. No network or URL comes from CLI input.
pub fn helper_main() -> Option<i32> {
    let args: Vec<_> = std::env::args_os().collect();
    if args.get(1).is_none_or(|v| v != "--toolhub-apply-update") {
        return None;
    }
    let Some(folder) = args.get(2).map(PathBuf::from) else {
        return Some(1);
    };
    let valid = root()
        .and_then(|r| r.canonicalize().map_err(|e| e.to_string()))
        .ok()
        .is_some_and(|r| {
            folder.canonicalize().ok().is_some_and(|f| {
                f.parent() == Some(r.as_path())
                    && f.file_name()
                        .and_then(|v| v.to_str())
                        .is_some_and(|v| uuid::Uuid::parse_str(v).is_ok())
            })
        });
    if !valid {
        return Some(1);
    }
    let result = apply(&folder);
    if let Err(error) = &result {
        let _ = write_json(
            &folder.join("prepared.json"),
            &json!({"ready":false,"error":error}),
        );
    }
    Some(if result.is_ok() { 0 } else { 1 })
}
/// The new native window and daemon must start before the helper commits the update.
pub fn boot_health() -> Result<()> {
    let args: Vec<_> = std::env::args().collect();
    let Some(index) = args.iter().position(|v| v == "--toolhub-update-health") else {
        return Ok(());
    };
    let operation = args.get(index + 1).ok_or("更新启动标识缺失")?;
    uuid::Uuid::parse_str(operation).map_err(|_| "更新启动标识无效")?;
    let folder = root()?.join(operation);
    let mut client = toolhub_ipc::RpcClient::connect_or_start(&super::daemon_bin()?)
        .map_err(|e| e.to_string())?;
    if client.call("ping", json!({})).map_err(|e| e.to_string())?["ok"] != true {
        return Err("更新服务启动失败".into());
    }
    write_json(
        &folder.join("boot-ready.json"),
        &json!({"operation":operation,"version":env!("CARGO_PKG_VERSION")}),
    )?;
    let start = Instant::now();
    while !folder.join("boot-commit.json").is_file() {
        if start.elapsed() > Duration::from_secs(15) {
            return Err("更新助手未完成交接".into());
        }
        std::thread::sleep(Duration::from_millis(100));
    }
    client
        .call("update.finish", json!({"token":operation}))
        .map_err(|e| e.to_string())?;
    Ok(())
}
