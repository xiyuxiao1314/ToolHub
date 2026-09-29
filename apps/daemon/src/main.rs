//! toolhubd — user-level daemon owning registry, scan, resolve, policy, execution, audit.

mod service;

use std::io::{BufRead, BufReader, Write};
use std::sync::{Arc, Mutex};

use tracing_subscriber::EnvFilter;

use service::DaemonService;
use toolhub_ipc::error_response;
use toolhub_protocol::{ErrorCode, JsonRpcRequest, JsonRpcResponse, ProtocolError};

fn registry_path() -> std::path::PathBuf {
    if let Ok(p) = std::env::var("TOOLHUB_REGISTRY") {
        return std::path::PathBuf::from(p);
    }
    let base = dirs::home_dir()
        .unwrap_or_else(|| std::path::PathBuf::from("."))
        .join(".toolhub");
    base.join("registry.sqlite")
}

fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::from_default_env().add_directive("info".parse()?))
        .with_writer(std::io::stderr)
        .init();

    let path = registry_path();
    tracing::info!(path=%path.display(), "starting toolhubd");
    let service = Arc::new(Mutex::new(DaemonService::open(&path)?));

    // Stdio JSON-RPC loop (also used by CLI when spawning daemon).
    // Named-pipe/unix-socket listener is available via `toolhubd --listen`.
    if std::env::args().any(|a| a == "--listen") {
        return listen_loop(service);
    }

    let stdin = std::io::stdin();
    let mut stdout = std::io::stdout();
    let reader = BufReader::new(stdin.lock());
    for line in reader.lines() {
        let line = line?;
        if line.trim().is_empty() {
            continue;
        }
        let response = match serde_json::from_str::<JsonRpcRequest>(&line) {
            Ok(req) => {
                let mut svc = service.lock().unwrap();
                svc.handle(&req)
            }
            Err(e) => error_response(
                None,
                &ProtocolError::new(ErrorCode::ParseError, e.to_string()),
            ),
        };
        let payload = serde_json::to_string(&response)?;
        writeln!(stdout, "{payload}")?;
        stdout.flush()?;
    }
    Ok(())
}

fn listen_loop(service: Arc<Mutex<DaemonService>>) -> anyhow::Result<()> {
    // Bounded local loopback TCP is intentionally NOT used as default.
    // On Windows we use named pipes via std; on Unix, a unix socket file.
    if cfg!(windows) {
        tracing::info!(endpoint = %toolhub_ipc::daemon_socket_name(), "named pipe listen is advisory in this build; stdio remains the supported transport");
        // Keep process alive with a simple stdio fallback server for local clients.
        let stdin = std::io::stdin();
        let mut stdout = std::io::stdout();
        let reader = BufReader::new(stdin.lock());
        for line in reader.lines() {
            let line = line?;
            if line.trim().is_empty() {
                continue;
            }
            if let Ok(req) = serde_json::from_str::<JsonRpcRequest>(&line) {
                let mut svc = service.lock().unwrap();
                let resp = svc.handle(&req);
                writeln!(stdout, "{}", serde_json::to_string(&resp)?)?;
                stdout.flush()?;
            }
        }
        Ok(())
    } else {
        #[cfg(unix)]
        {
            use std::os::unix::net::UnixListener;
            let sock = toolhub_ipc::daemon_socket_name();
            let _ = std::fs::remove_file(&sock);
            let listener = UnixListener::bind(&sock)?;
            tracing::info!(endpoint=%sock, "listening");
            for stream in listener.incoming() {
                let stream = stream?;
                let service = Arc::clone(&service);
                std::thread::spawn(move || {
                    let mut reader = BufReader::new(stream.try_clone().unwrap());
                    let mut writer = stream;
                    loop {
                        match toolhub_ipc::read_request(&mut reader) {
                            Ok(req) => {
                                let mut svc = service.lock().unwrap();
                                let resp = svc.handle(&req);
                                let _ = write_frame(&mut writer, &resp);
                            }
                            Err(_) => break,
                        }
                    }
                });
            }
            Ok(())
        }
        #[cfg(not(unix))]
        {
            let _ = service;
            anyhow::bail!("unsupported platform for --listen")
        }
    }
}

/// Local peer check: agent-name strings are not authentication.
pub fn peer_is_local() -> bool {
    // stdio inherits the caller's process boundary on this machine.
    true
}

#[allow(dead_code)]
fn _resp_ok(id: Option<serde_json::Value>, result: serde_json::Value) -> JsonRpcResponse {
    JsonRpcResponse {
        jsonrpc: "2.0".into(),
        id,
        result: Some(result),
        error: None,
    }
}
