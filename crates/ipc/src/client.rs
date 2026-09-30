use super::*;
use std::time::{Duration, Instant};
/// One connection to the managed service. A failed call is never replayed; the next call reconnects.
pub struct RpcClient {
    daemon: PathBuf,
    stream: Option<LocalStream>,
    next_id: u64,
}
impl RpcClient {
    pub fn connect_or_start(daemon: &Path) -> IpcResult<Self> {
        let daemon = daemon.canonicalize()?;
        let stream = Self::connect_ready(&daemon)?;
        Ok(Self {
            daemon,
            stream: Some(stream),
            next_id: 1,
        })
    }
    fn connect_ready(daemon: &Path) -> IpcResult<LocalStream> {
        if let Ok(stream) = LocalStream::connect(daemon) {
            return Ok(stream);
        }
        let mut command = std::process::Command::new(daemon);
        command
            .arg("--listen")
            .stdin(std::process::Stdio::null())
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null());
        #[cfg(windows)]
        {
            use std::os::windows::process::CommandExt;
            command.creation_flags(0x08000000);
        }
        let mut child = command.spawn()?;
        let deadline = Instant::now() + Duration::from_secs(5);
        loop {
            if let Ok(mut stream) = LocalStream::connect(daemon) {
                stream.set_timeout(Duration::from_secs(2))?;
                write_frame(
                    &mut stream,
                    &JsonRpcRequest::new(0, "ping", serde_json::json!({})),
                )?;
                let response = read_response(&mut stream)?;
                if response.id == Some(serde_json::json!(0)) && response.error.is_none() {
                    // Reap the managed child only after it eventually exits, never on client drop.
                    std::thread::spawn(move || {
                        let _ = child.wait();
                    });
                    return Ok(stream);
                }
            }
            if Instant::now() >= deadline {
                let _ = child.kill();
                let _ = child.wait();
                return Err(IpcError::Message(
                    "unavailable: managed daemon readiness deadline".into(),
                ));
            }
            // A competing startup may exit because the service lock is held; keep checking readiness.
            let _ = child.try_wait();
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
            self.stream = Some(Self::connect_ready(&self.daemon)?);
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
