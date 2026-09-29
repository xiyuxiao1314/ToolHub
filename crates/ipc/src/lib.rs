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
