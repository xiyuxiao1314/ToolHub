//! Default-profile startup follows the activated installation and respects updater exclusion.
use super::*;
use std::fs::{self, File, OpenOptions};
use std::io::Read;

#[derive(Clone)]
pub(super) struct InstallContext {
    pub(super) root: Option<PathBuf>,
    pub(super) redirect: bool,
}
pub(super) enum StartGate {
    Allowed(Option<File>),
    Updating,
}
impl InstallContext {
    pub(super) fn current() -> Self {
        let isolated = std::env::var_os("TOOLHUB_REGISTRY").is_some()
            || std::env::var_os("TOOLHUBD_BIN").is_some();
        Self {
            root: if isolated {
                None
            } else {
                dirs::data_local_dir().map(|p| p.join("ToolHub"))
            },
            redirect: cfg!(windows) && !isolated,
        }
    }
    pub(super) fn daemon(&self, requested: &Path) -> IpcResult<PathBuf> {
        if self.redirect {
            if let Some(root) = &self.root {
                let selected = (|| -> Option<PathBuf> {
                    let mut bytes = Vec::new();
                    File::open(root.join("current.json"))
                        .ok()?
                        .take(8193)
                        .read_to_end(&mut bytes)
                        .ok()?;
                    if bytes.len() > 8192 {
                        return None;
                    }
                    let current: serde_json::Value = serde_json::from_slice(&bytes).ok()?;
                    toolhub_core::install_redirect::current_service(
                        root,
                        &current,
                        requested.file_name()?.to_str()?,
                        env!("CARGO_PKG_VERSION"),
                    )
                })();
                if let Some(target) = selected {
                    return Ok(target);
                }
            }
        }
        Ok(requested.canonicalize()?)
    }
    pub(super) fn start_gate(&self) -> IpcResult<StartGate> {
        let Some(root) = &self.root else {
            return Ok(StartGate::Allowed(None));
        };
        let folder = root.join("updates");
        fs::create_dir_all(&folder)?;
        let metadata = fs::symlink_metadata(&folder)?;
        if !metadata.is_dir() || metadata.file_type().is_symlink() {
            return Err(IpcError::Message("unsafe update state directory".into()));
        }
        #[cfg(windows)]
        {
            use std::os::windows::fs::MetadataExt;
            if metadata.file_attributes() & 0x400 != 0 {
                return Err(IpcError::Message("unsafe update directory junction".into()));
            }
        }
        let path = folder.join("install.lock");
        if let Ok(metadata) = fs::symlink_metadata(&path) {
            if !metadata.is_file() || metadata.file_type().is_symlink() {
                return Err(IpcError::Message("unsafe update lock".into()));
            }
            #[cfg(windows)]
            {
                use std::os::windows::fs::MetadataExt;
                if metadata.file_attributes() & 0x400 != 0 {
                    return Err(IpcError::Message("unsafe update lock junction".into()));
                }
            }
        }
        let file = OpenOptions::new()
            .read(true)
            .write(true)
            .create(true)
            .open(path)?;
        if file.try_lock_shared().is_err() {
            return Ok(StartGate::Updating);
        }
        Ok(StartGate::Allowed(Some(file)))
    }
}
