//! Authenticated local IPC and shared daemon client.
use sha2::{Digest, Sha256};
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use toolhub_protocol::{JsonRpcRequest, JsonRpcResponse, ProtocolError};
mod client;
mod platform;
pub use client::RpcClient;
pub use platform::*;

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
        Err(e) if e.kind() == std::io::ErrorKind::UnexpectedEof => return Err(IpcError::Closed),
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
    Ok(serde_json::from_slice(&read_frame(r)?)?)
}
pub fn read_response(r: &mut impl Read) -> IpcResult<JsonRpcResponse> {
    Ok(serde_json::from_slice(&read_frame(r)?)?)
}

/// Every server transport shares the encoded response cap, including JSON escaping overhead.
pub fn serialize_response(response: &JsonRpcResponse) -> IpcResult<Vec<u8>> {
    let bytes = serde_json::to_vec(response)?;
    if bytes.len() <= toolhub_protocol::limits::MAX_REQUEST_BYTES {
        return Ok(bytes);
    }
    Ok(serde_json::to_vec(&error_response(
        response.id.clone(),
        &ProtocolError::new(
            toolhub_protocol::ErrorCode::PayloadTooLarge,
            "encoded response exceeds byte limit",
        ),
    ))?)
}
pub fn write_response_frame(writer: &mut impl Write, response: &JsonRpcResponse) -> IpcResult<()> {
    let bytes = serialize_response(response)?;
    writer.write_all(&(bytes.len() as u32).to_be_bytes())?;
    writer.write_all(&bytes)?;
    writer.flush()?;
    Ok(())
}

/// Reads one line without allocating beyond the wire limit. Oversized lines are drained.
pub fn read_bounded_line(r: &mut impl std::io::BufRead) -> IpcResult<Option<Vec<u8>>> {
    let mut line = Vec::new();
    let mut exceeded = false;
    loop {
        let available = r.fill_buf()?;
        if available.is_empty() {
            if line.is_empty() && !exceeded {
                return Ok(None);
            }
            break;
        }
        let end = available
            .iter()
            .position(|b| *b == b'\n')
            .map(|i| i + 1)
            .unwrap_or(available.len());
        if !exceeded && line.len() + end <= toolhub_protocol::limits::MAX_REQUEST_BYTES {
            line.extend_from_slice(&available[..end]);
        } else {
            exceeded = true;
        }
        let finished = available[end - 1] == b'\n';
        r.consume(end);
        if finished {
            break;
        }
    }
    if exceeded {
        return Err(IpcError::Message("payload_too_large".into()));
    }
    Ok(Some(line))
}

#[derive(Debug, Clone)]
pub struct PeerIdentity {
    pub principal: String,
    pub image: PathBuf,
}
impl PeerIdentity {
    pub fn local_stdio() -> IpcResult<Self> {
        Ok(Self {
            principal: current_principal()?,
            image: std::env::current_exe()?,
        })
    }
    pub fn caller_principal(&self) -> IpcResult<String> {
        let image = self.image.canonicalize()?;
        Ok(format!(
            "{}:image:{}:{}",
            self.principal,
            hex::encode(Sha256::digest(image.to_string_lossy().as_bytes())),
            hex::encode(image_hash(&image)?)
        ))
    }
}

/// Only the authenticated same-user process at the pinned canonical path and digest is a controller.
#[derive(Clone)]
pub struct ControllerImage {
    path: PathBuf,
    digest: [u8; 32],
}
impl ControllerImage {
    pub fn pin(path: &Path) -> IpcResult<Self> {
        Ok(Self {
            path: path.canonicalize()?,
            digest: image_hash(path)?,
        })
    }
    pub fn verify(&self, peer: &PeerIdentity) -> IpcResult<bool> {
        Ok(peer.principal == current_principal()?
            && peer.image.canonicalize()? == self.path
            && image_hash(&peer.image)? == self.digest
            && image_hash(&self.path)? == self.digest)
    }
}
pub(crate) fn image_hash(path: &Path) -> IpcResult<[u8; 32]> {
    let mut file = std::fs::File::open(path)?;
    let mut hash = Sha256::new();
    let mut buf = [0u8; 65536];
    loop {
        let n = file.read(&mut buf)?;
        if n == 0 {
            break;
        }
        hash.update(&buf[..n]);
    }
    Ok(hash.finalize().into())
}
pub(crate) fn namespace() -> String {
    let registry = std::env::var_os("TOOLHUB_REGISTRY").map(PathBuf::from);
    let key = registry
        .map(|p| {
            if p.is_absolute() {
                p
            } else {
                std::env::current_dir().unwrap_or_default().join(p)
            }
            .to_string_lossy()
            .into_owned()
        })
        .unwrap_or_else(|| "default".into());
    #[cfg(windows)]
    let key = key.replace('/', "\\").to_lowercase();
    hex::encode(Sha256::digest(key.as_bytes()))[..16].to_string()
}
pub fn error_response(id: Option<serde_json::Value>, err: &ProtocolError) -> JsonRpcResponse {
    JsonRpcResponse {
        jsonrpc: "2.0".into(),
        id,
        result: None,
        error: Some(toolhub_protocol::JsonRpcErrorObject {
            code: err.code.rpc_code(),
            message: err.message.clone(),
            data: Some(
                serde_json::json!({"error_code":err.code.as_str(),"fallback_allowed":err.fallback_allowed}),
            ),
        }),
    }
}
