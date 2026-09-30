//! toolhubd: one authenticated, per-user managed service.
mod service;
use service::DaemonService;
use std::io::{BufReader, Write};
use std::sync::{Arc, Mutex};
use toolhub_ipc::{error_response, ControllerImage, PeerIdentity};
use toolhub_protocol::{ErrorCode, JsonRpcRequest, JsonRpcResponse, ProtocolError};

fn registry_path() -> std::path::PathBuf {
    std::env::var_os("TOOLHUB_REGISTRY")
        .map(Into::into)
        .unwrap_or_else(|| {
            dirs::home_dir()
                .unwrap_or_else(|| ".".into())
                .join(".toolhub/registry.sqlite")
        })
}
fn dispatch(
    service: &Arc<Mutex<DaemonService>>,
    req: &JsonRpcRequest,
    principal: &str,
    controller: bool,
) -> JsonRpcResponse {
    if let Err(error) = toolhub_protocol::validate_method_params(req) {
        return error_response(req.id.clone(), &error);
    }
    if req.method == "scan.start" {
        let prepared = service
            .lock()
            .unwrap()
            .prepare_scan(req, principal, controller);
        return match prepared {
            Ok(job) => {
                let report = job.run();
                service.lock().unwrap().finish_scan(job, report)
            }
            Err(response) => response,
        };
    }
    if req.method == "execute.tool" {
        let prepared = service
            .lock()
            .unwrap()
            .prepare_execution(req, principal, controller);
        match prepared {
            Ok(job) => {
                let result = job.run();
                service.lock().unwrap().finish_execution(job, result)
            }
            Err(response) => response,
        }
    } else {
        service
            .lock()
            .unwrap()
            .handle_authenticated(req, principal, controller)
    }
}
fn response_for_bytes(
    service: &Arc<Mutex<DaemonService>>,
    bytes: &[u8],
    principal: &str,
    controller: bool,
) -> Option<JsonRpcResponse> {
    match toolhub_protocol::parse_request(bytes) {
        Ok(req) => {
            let notification = req.id.is_none();
            let response = dispatch(service, &req, principal, controller);
            if notification {
                None
            } else {
                Some(response)
            }
        }
        Err(error) => {
            let value = serde_json::from_slice::<serde_json::Value>(bytes).ok();
            let id = value
                .as_ref()
                .and_then(|v| v.get("id"))
                .filter(|v| v.is_null() || v.is_string() || v.is_number())
                .cloned();
            Some(error_response(id, &error))
        }
    }
}
fn main() -> anyhow::Result<()> {
    if std::env::args().any(|arg| arg == "--agent-fixture") {
        let mut reader = BufReader::new(std::io::stdin());
        let bytes = toolhub_ipc::read_bounded_line(&mut reader)?
            .ok_or_else(|| anyhow::anyhow!("fixture input required"))?;
        let input: serde_json::Value = serde_json::from_slice(&bytes)?;
        std::thread::sleep(std::time::Duration::from_millis(1000));
        println!(
            "{}",
            serde_json::json!({"fixture":true,"candidate_count":input.get("candidates").and_then(|v|v.as_array()).map(|v|v.len()).unwrap_or(0)})
        );
        return Ok(());
    }
    tracing_subscriber::fmt()
        .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
        .with_writer(std::io::stderr)
        .init();
    let listen = std::env::args().any(|arg| arg == "--listen");
    // Acquire before opening SQLite or modifying an endpoint.
    let _lock = if listen {
        Some(toolhub_ipc::ServiceLock::acquire()?)
    } else {
        None
    };
    let service = Arc::new(Mutex::new(DaemonService::open(&registry_path())?));
    if listen {
        return listen_loop(service);
    }
    // Stdio is a compatibility transport with ordinary authority only.
    let principal = PeerIdentity::local_stdio()?.caller_principal()?;
    let stdin = std::io::stdin();
    let mut reader = BufReader::new(stdin.lock());
    let mut stdout = std::io::stdout();
    loop {
        let response = match toolhub_ipc::read_bounded_line(&mut reader) {
            Ok(Some(bytes)) => {
                if bytes.iter().all(|c| c.is_ascii_whitespace()) {
                    continue;
                }
                response_for_bytes(&service, &bytes, &principal, false)
            }
            Ok(None) => break,
            Err(toolhub_ipc::IpcError::Message(_)) => Some(error_response(
                None,
                &ProtocolError::new(ErrorCode::PayloadTooLarge, "request too large"),
            )),
            Err(error) => return Err(error.into()),
        };
        if let Some(response) = response {
            stdout.write_all(&toolhub_ipc::serialize_response(&response)?)?;
            stdout.write_all(b"\n")?;
            stdout.flush()?;
        }
    }
    Ok(())
}
fn controller_image() -> anyhow::Result<Option<ControllerImage>> {
    let args: Vec<String> = std::env::args().collect();
    let path = if let Some(index) = args.iter().position(|arg| arg == "--controller-image") {
        std::path::PathBuf::from(
            args.get(index + 1)
                .ok_or_else(|| anyhow::anyhow!("--controller-image requires path"))?,
        )
    } else {
        let exe = std::env::current_exe()?;
        exe.parent().unwrap().join(if cfg!(windows) {
            "toolhub-desktop.exe"
        } else {
            "toolhub-desktop"
        })
    };
    if !path.exists() {
        return Ok(None);
    }
    Ok(Some(ControllerImage::pin(&path)?))
}
fn listen_loop(service: Arc<Mutex<DaemonService>>) -> anyhow::Result<()> {
    let controller = controller_image()?;
    let listener = toolhub_ipc::LocalListener::bind()?;
    loop {
        let (mut stream, peer) = match listener.accept() {
            Ok(value) => value,
            Err(error) => {
                tracing::warn!(%error,"peer rejected");
                continue;
            }
        };
        let service = service.clone();
        let controller = controller.clone();
        std::thread::spawn(move || {
            let principal = match peer.caller_principal() {
                Ok(value) => value,
                Err(_) => return,
            };
            while let Ok(bytes) = toolhub_ipc::read_frame(&mut stream) {
                // Recheck the pinned image on each request, including substitution after connect.
                let trusted = controller
                    .as_ref()
                    .is_some_and(|pin| pin.verify(&peer).unwrap_or(false));
                if let Some(response) = response_for_bytes(&service, &bytes, &principal, trusted) {
                    if toolhub_ipc::write_response_frame(&mut stream, &response).is_err() {
                        break;
                    }
                }
            }
        });
    }
}
