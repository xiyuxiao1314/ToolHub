//! Local IPC framing for ToolHub Protocol JSON-RPC over stdio / named pipes / unix sockets.
//! No default public listener.

use std::io::{Read, Write};

use toolhub_protocol::{JsonRpcRequest, JsonRpcResponse, ProtocolError};

#[derive(Debug, thiserror::Error)]
pub enum IpcError {
    #[error("io: {0}")]
    Io(#[from] std::io::Error),
    #[error("protocol: {0}")]
    Protocol(#[from] serde_json::Error),
    #[error("closed")]
    Closed,
    #[error("{0}")]
    Message(String),
}

pub type IpcResult<T> = Result<T, IpcError>;

/// Length-prefixed JSON frames: 4-byte BE length + UTF-8 JSON.
pub fn write_frame(w: &mut impl Write, value: &impl serde::Serialize) -> IpcResult<()> {
    let bytes = serde_json::to_vec(value)?;
    if bytes.len() > toolhub_protocol::limits::MAX_REQUEST_BYTES {
        return Err(IpcError::Message("payload_too_large".into()));
    }
    w.write_all(&(bytes.len() as u32).to_be_bytes())?;
    w.write_all(&bytes)?;
    w.flush()?;
    Ok(())
}

pub fn read_frame(r: &mut impl Read) -> IpcResult<Vec<u8>> {
    let mut len = [0u8; 4];
    match r.read_exact(&mut len) {
        Ok(()) => {}
        Err(e) if e.kind() == std::io::ErrorKind::UnexpectedEof => {
            return Err(IpcError::Closed);
        }
        Err(e) => return Err(e.into()),
    }
    let n = u32::from_be_bytes(len) as usize;
    if n > toolhub_protocol::limits::MAX_REQUEST_BYTES {
        return Err(IpcError::Message("payload_too_large".into()));
    }
    let mut buf = vec![0u8; n];
    r.read_exact(&mut buf)?;
    Ok(buf)
}

pub fn read_request(r: &mut impl Read) -> IpcResult<JsonRpcRequest> {
    let buf = read_frame(r)?;
    Ok(serde_json::from_slice(&buf)?)
}

pub fn read_response(r: &mut impl Read) -> IpcResult<JsonRpcResponse> {
    let buf = read_frame(r)?;
    Ok(serde_json::from_slice(&buf)?)
}

/// Peer authentication is transport-based; agent-name strings are not credentials.
#[derive(Debug, Clone)]
pub struct PeerIdentity {
    pub transport: TransportKind,
    pub peer_ok: bool,
    pub note: String,
}

#[derive(Debug, Clone, Copy)]
pub enum TransportKind {
    Stdio,
    NamedPipe,
    UnixSocket,
}

impl PeerIdentity {
    pub fn local_stdio() -> Self {
        Self {
            transport: TransportKind::Stdio,
            peer_ok: true,
            note: "stdin/stdout inherits caller process boundary".into(),
        }
    }
}

pub fn daemon_socket_name() -> String {
    if cfg!(windows) {
        r"\\.\pipe\toolhubd".to_string()
    } else {
        "/tmp/toolhubd.sock".to_string()
    }
}

/// F11: user-scoped named pipe endpoint (Windows). Rejects default public listeners.
#[cfg(windows)]
pub fn user_scoped_pipe_name() -> String {
    let user = std::env::var("USERNAME").unwrap_or_else(|_| "user".into());
    format!(r"\\.\pipe\toolhubd-{user}")
}

/// F11: create a named pipe instance for one client connection (read/write duplex).
#[cfg(windows)]
pub fn create_named_pipe(name: &str) -> IpcResult<std::fs::File> {
    use std::os::windows::io::FromRawHandle;
    use windows_sys::Win32::Foundation::INVALID_HANDLE_VALUE;
    use windows_sys::Win32::System::Pipes::CreateNamedPipeW;

    const PIPE_ACCESS_DUPLEX: u32 = 0x3;
    const PIPE_TYPE_BYTE: u32 = 0x0;
    const PIPE_READMODE_BYTE: u32 = 0x0;
    const PIPE_WAIT: u32 = 0x0;

    let wide: Vec<u16> = name.encode_utf16().chain(std::iter::once(0)).collect();
    unsafe {
        let handle = CreateNamedPipeW(
            wide.as_ptr(),
            PIPE_ACCESS_DUPLEX,
            PIPE_TYPE_BYTE | PIPE_READMODE_BYTE | PIPE_WAIT,
            1,
            65536,
            65536,
            0,
            std::ptr::null_mut(),
        );
        if handle == INVALID_HANDLE_VALUE {
            return Err(IpcError::Message("create_named_pipe failed".into()));
        }
        Ok(std::fs::File::from_raw_handle(handle as *mut _))
    }
}

/// F11: connect to an existing named pipe as a client.
#[cfg(windows)]
pub fn connect_named_pipe(name: &str) -> IpcResult<std::fs::File> {
    use std::os::windows::io::FromRawHandle;
    use windows_sys::Win32::Foundation::INVALID_HANDLE_VALUE;
    use windows_sys::Win32::Storage::FileSystem::{
        CreateFileW, FILE_ATTRIBUTE_NORMAL, FILE_GENERIC_READ, FILE_GENERIC_WRITE, OPEN_EXISTING,
    };
    let wide: Vec<u16> = name.encode_utf16().chain(std::iter::once(0)).collect();
    unsafe {
        let handle = CreateFileW(
            wide.as_ptr(),
            FILE_GENERIC_READ | FILE_GENERIC_WRITE,
            0,
            std::ptr::null(),
            OPEN_EXISTING,
            FILE_ATTRIBUTE_NORMAL,
            std::ptr::null_mut(),
        );
        if handle == INVALID_HANDLE_VALUE {
            return Err(IpcError::Message("connect_named_pipe failed".into()));
        }
        Ok(std::fs::File::from_raw_handle(handle as *mut _))
    }
}

pub fn error_response(id: Option<serde_json::Value>, err: &ProtocolError) -> JsonRpcResponse {
    JsonRpcResponse {
        jsonrpc: "2.0".into(),
        id,
        result: None,
        error: Some(toolhub_protocol::JsonRpcErrorObject {
            code: err.code.rpc_code(),
            message: err.message.clone(),
            data: Some(serde_json::json!({
                "error_code": err.code.as_str(),
                "fallback_allowed": err.fallback_allowed,
            })),
        }),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn frame_roundtrip() {
        let mut buf = Vec::new();
        let req = JsonRpcRequest::new(1, "ping", serde_json::json!({}));
        write_frame(&mut buf, &req).unwrap();
        let mut cur = std::io::Cursor::new(buf);
        let back = read_request(&mut cur).unwrap();
        assert_eq!(back.method, "ping");
    }
}
