use super::install_context::{InstallContext, StartGate};
use super::*;
use std::sync::Mutex;
use std::time::{Duration, Instant};

// Desktop pages connect concurrently; serialize readiness so they share one startup.
static SERVICE_STARTUP: Mutex<()> = Mutex::new(());
#[cfg(windows)]
type StartedChild = super::background_child::BackgroundChild;
#[cfg(not(windows))]
type StartedChild = std::process::Child;
fn launch(daemon: &Path) -> IpcResult<StartedChild> {
    #[cfg(windows)]
    {
        Ok(super::background_child::BackgroundChild::spawn(daemon)?)
    }
    #[cfg(not(windows))]
    {
        Ok(std::process::Command::new(daemon)
            .arg("--listen")
            .stdin(std::process::Stdio::null())
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null())
            .spawn()?)
    }
}
/// One connection to the managed service. A failed call is never replayed; the next call reconnects.
pub struct RpcClient {
    daemon: PathBuf,
    install: InstallContext,
    stream: Option<LocalStream>,
    next_id: u64,
}
impl RpcClient {
    pub fn connect_or_start(daemon: &Path) -> IpcResult<Self> {
        let install = InstallContext::current();
        let requested = daemon
            .canonicalize()
            .unwrap_or_else(|_| daemon.to_path_buf());
        let stream = Self::connect_ready(&requested, &install)?;
        Ok(Self {
            daemon: requested,
            install,
            stream: Some(stream),
            next_id: 1,
        })
    }
    fn connect_ready(daemon: &Path, install: &InstallContext) -> IpcResult<LocalStream> {
        Self::connect_ready_with(daemon, install, Duration::from_secs(5), launch)
    }
    fn connect_ready_with<F>(
        daemon: &Path,
        install: &InstallContext,
        timeout: Duration,
        mut start: F,
    ) -> IpcResult<LocalStream>
    where
        F: FnMut(&Path) -> IpcResult<StartedChild>,
    {
        let _startup = SERVICE_STARTUP.lock().unwrap_or_else(|e| e.into_inner());
        let deadline = Instant::now() + timeout;
        let mut child: Option<StartedChild> = None;
        let mut _start_gate = None;
        let mut updating = false;
        loop {
            // A long-lived MCP client may reconnect after current.json has changed.
            let selected = install.daemon(daemon)?;
            if let Ok(mut stream) = LocalStream::connect(&selected) {
                let ready = if child.is_none() {
                    true
                } else {
                    let remaining = deadline.saturating_duration_since(Instant::now());
                    (|| -> IpcResult<bool> {
                        stream.set_timeout(remaining.min(Duration::from_secs(2)))?;
                        write_frame(
                            &mut stream,
                            &JsonRpcRequest::new(0, "ping", serde_json::json!({})),
                        )?;
                        let response = read_response(&mut stream)?;
                        Ok(response.jsonrpc == "2.0"
                            && response.id == Some(serde_json::json!(0))
                            && response.result.is_some()
                            && response.error.is_none())
                    })()
                    .unwrap_or(false)
                };
                if ready {
                    if let Some(mut child) = child {
                        std::thread::spawn(move || {
                            let _ = child.wait();
                        });
                    }
                    return Ok(stream);
                }
            }
            if child.is_none() {
                match install.start_gate()? {
                    StartGate::Updating => {
                        updating = true;
                    }
                    StartGate::Allowed(guard) => {
                        _start_gate = guard;
                        // Re-read activation while holding shared startup exclusion.
                        let selected = install.daemon(daemon)?;
                        if let Ok(stream) = LocalStream::connect(&selected) {
                            return Ok(stream);
                        }
                        child = Some(start(&selected)?);
                    }
                }
            }
            if Instant::now() >= deadline {
                if let Some(mut child) = child {
                    let _ = child.kill();
                    let _ = child.wait();
                }
                return Err(IpcError::Message(
                    if updating {
                        "unavailable: update handoff in progress; reconnect shortly"
                    } else {
                        "unavailable: managed daemon readiness deadline"
                    }
                    .into(),
                ));
            }
            if let Some(child) = child.as_mut() {
                let _ = child.try_wait();
            }
            std::thread::sleep(Duration::from_millis(25));
        }
    }
    pub fn call(
        &mut self,
        method: &str,
        params: serde_json::Value,
    ) -> IpcResult<serde_json::Value> {
        self.call_with_timeout(
            method,
            params,
            Duration::from_millis(toolhub_protocol::limits::MAX_TIMEOUT_MS + 5_000),
        )
    }
    pub fn call_with_timeout(
        &mut self,
        method: &str,
        params: serde_json::Value,
        timeout: Duration,
    ) -> IpcResult<serde_json::Value> {
        if self.stream.is_none() {
            self.stream = Some(Self::connect_ready(&self.daemon, &self.install)?);
        }
        let id = self.next_id;
        self.next_id += 1;
        let request = JsonRpcRequest::new(id, method, params);
        let result = (|| {
            let stream = self.stream.as_mut().unwrap();
            stream.set_timeout(timeout)?;
            write_frame(stream, &request)?;
            let response = read_response(stream)?;
            if response.jsonrpc != "2.0"
                || response.id != request.id
                || response.result.is_some() == response.error.is_some()
            {
                return Err(IpcError::Message("invalid daemon response".into()));
            }
            if let Some(error) = response.error {
                return Err(IpcError::Message(format!(
                    "{}: {}",
                    error
                        .data
                        .as_ref()
                        .and_then(|d| d.get("error_code"))
                        .and_then(|v| v.as_str())
                        .unwrap_or("daemon_error"),
                    error.message
                )));
            }
            Ok(response.result.unwrap_or(serde_json::Value::Null))
        })();
        if matches!(
            &result,
            Err(IpcError::Io(_) | IpcError::Closed | IpcError::Protocol(_))
        ) {
            self.stream = None;
        }
        result
    }
}

#[cfg(test)]
mod update_tests {
    use super::*;
    use std::sync::atomic::{AtomicUsize, Ordering};
    fn installation(root: &Path, name: &str) -> (PathBuf, serde_json::Value) {
        let folder = root.join("versions").join(name);
        std::fs::create_dir_all(&folder).unwrap();
        let suffix = if cfg!(windows) { ".exe" } else { "" };
        for binary in ["toolhub", "toolhubd", "toolhub-desktop"] {
            std::fs::write(folder.join(format!("{binary}{suffix}")), b"owned fixture").unwrap();
        }
        std::fs::write(folder.join("package.json"),serde_json::to_vec(&serde_json::json!({"schema":"toolhub.package/v1","version":env!("CARGO_PKG_VERSION")})).unwrap()).unwrap();
        let current = serde_json::json!({"path":folder,"cli":folder.join(format!("toolhub{suffix}")),"desktop":folder.join(format!("toolhub-desktop{suffix}"))});
        (folder.join(format!("toolhubd{suffix}")), current)
    }
    #[test]
    fn updater_lock_blocks_respawn_and_reconnect_selects_new_activation() {
        let d = tempfile::tempdir().unwrap();
        let root = d.path().join("ToolHub");
        let (old, a) = installation(&root, "old");
        let (new, b) = installation(&root, "new");
        std::fs::write(root.join("current.json"), serde_json::to_vec(&a).unwrap()).unwrap();
        let context = InstallContext {
            root: Some(root.clone()),
            redirect: true,
        };
        assert_eq!(context.daemon(&old).unwrap(), old.canonicalize().unwrap());
        let token = d.path().join("isolated.sqlite");
        struct EnvRestore(Option<std::ffi::OsString>);
        impl Drop for EnvRestore {
            fn drop(&mut self) {
                if let Some(v) = self.0.take() {
                    std::env::set_var("TOOLHUB_REGISTRY", v)
                } else {
                    std::env::remove_var("TOOLHUB_REGISTRY")
                }
            }
        }
        let _restore = EnvRestore(std::env::var_os("TOOLHUB_REGISTRY"));
        std::env::set_var("TOOLHUB_REGISTRY", &token);
        std::fs::create_dir_all(root.join("updates")).unwrap();
        let exclusive = std::fs::OpenOptions::new()
            .read(true)
            .write(true)
            .create(true)
            .open(root.join("updates/install.lock"))
            .unwrap();
        exclusive.try_lock().unwrap();
        let starts = AtomicUsize::new(0);
        let result =
            RpcClient::connect_ready_with(&old, &context, Duration::from_millis(120), |_| {
                starts.fetch_add(1, Ordering::SeqCst);
                Err(IpcError::Message("unexpected launch".into()))
            });
        assert!(matches!(result,Err(IpcError::Message(ref e)) if e.contains("update handoff")));
        assert_eq!(
            starts.load(Ordering::SeqCst),
            0,
            "No retained daemon may start under updater exclusion"
        );
        std::fs::write(root.join("current.json"), serde_json::to_vec(&b).unwrap()).unwrap();
        drop(exclusive);
        let result =
            RpcClient::connect_ready_with(&old, &context, Duration::from_millis(120), |target| {
                assert_eq!(target, new.canonicalize().unwrap());
                starts.fetch_add(1, Ordering::SeqCst);
                Err(IpcError::Message("launch intercepted".into()))
            });
        assert!(matches!(result,Err(IpcError::Message(ref e)) if e=="launch intercepted"));
        assert_eq!(starts.load(Ordering::SeqCst), 1);
        let probe = std::fs::OpenOptions::new()
            .read(true)
            .write(true)
            .open(root.join("updates/install.lock"))
            .unwrap();
        probe.try_lock().unwrap();
        // The active installation must never downgrade a newer portable client.
        let folder = new.parent().unwrap();
        std::fs::write(
            folder.join("package.json"),
            br#"{"schema":"toolhub.package/v1","version":"0.0.1"}"#,
        )
        .unwrap();
        assert_eq!(context.daemon(&old).unwrap(), old.canonicalize().unwrap());
    }
}
