//! B02-15: ToolHub product update plumbing (local verified package only).
//! Distinct from installing discovered tools. Requires explicit user action.

use serde::{Deserialize, Serialize};
use std::path::Path;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdatePackage {
    pub schema: String,
    pub product: String,
    pub version: String,
    pub min_current: String,
    pub checksum_sha256: String,
    pub payload_path: String,
}

#[derive(Debug, thiserror::Error)]
pub enum UpdateError {
    #[error("io: {0}")]
    Io(#[from] std::io::Error),
    #[error("parse: {0}")]
    Parse(#[from] serde_json::Error),
    #[error("checksum mismatch")]
    ChecksumMismatch,
    #[error("incompatible package: {0}")]
    Incompatible(String),
    #[error("cancelled")]
    Cancelled,
}

/// Validate a local update package before any apply step.
pub fn verify_package(manifest_path: &Path) -> Result<UpdatePackage, UpdateError> {
    verify_package_for_current(manifest_path, env!("CARGO_PKG_VERSION"))
}

pub fn verify_package_for_current(
    manifest_path: &Path,
    current: &str,
) -> Result<UpdatePackage, UpdateError> {
    let raw = std::fs::read_to_string(manifest_path)?;
    let mut pkg: UpdatePackage = serde_json::from_str(&raw)?;
    if pkg.schema != "toolhub.update/v1" || pkg.product != "toolhub" {
        return Err(UpdateError::Incompatible(pkg.schema));
    }
    validate_versions(&pkg, current)?;
    let payload = manifest_path
        .parent()
        .map(|d| d.join(&pkg.payload_path))
        .unwrap_or_else(|| Path::new(&pkg.payload_path).to_path_buf());
    let payload = std::fs::canonicalize(payload)?;
    let root = std::fs::canonicalize(manifest_path.parent().unwrap_or(Path::new(".")))?;
    if !payload.starts_with(&root) {
        return Err(UpdateError::Incompatible(
            "payload escapes verified package root".into(),
        ));
    }
    let data = read_payload(&payload)?;
    let hash = {
        use sha2::{Digest, Sha256};
        let mut h = Sha256::new();
        h.update(&data);
        hex::encode(h.finalize())
    };
    if !hash.eq_ignore_ascii_case(&pkg.checksum_sha256) {
        return Err(UpdateError::ChecksumMismatch);
    }
    pkg.payload_path = payload.to_string_lossy().into_owned();
    Ok(pkg)
}

fn validate_versions(pkg: &UpdatePackage, current: &str) -> Result<(), UpdateError> {
    let minimum = toolhub_core::compare_versions(current, &pkg.min_current)
        .map_err(|_| UpdateError::Incompatible("invalid current/minimum version".into()))?;
    let newer = toolhub_core::compare_versions(&pkg.version, current)
        .map_err(|_| UpdateError::Incompatible("invalid update version".into()))?;
    if minimum.is_lt() || !newer.is_gt() {
        return Err(UpdateError::Incompatible(
            "update is incompatible or not newer".into(),
        ));
    }
    Ok(())
}
fn read_payload(path: &Path) -> Result<Vec<u8>, UpdateError> {
    use std::io::Read;
    let file = std::fs::File::open(path)?;
    if file.metadata()?.len() > 64 * 1024 * 1024 {
        return Err(UpdateError::Incompatible("payload exceeds 64 MiB".into()));
    }
    let mut data = vec![];
    file.take(64 * 1024 * 1024 + 1).read_to_end(&mut data)?;
    if data.len() > 64 * 1024 * 1024 {
        return Err(UpdateError::Incompatible("payload exceeds 64 MiB".into()));
    }
    Ok(data)
}

/// Explicit apply: caller must have verified package. Records nothing to discovered tools.
pub fn apply_update(
    pkg: &UpdatePackage,
    dest: &Path,
    cancelled: &dyn Fn() -> bool,
) -> Result<(), UpdateError> {
    if cancelled() {
        return Err(UpdateError::Cancelled);
    }
    validate_versions(pkg, env!("CARGO_PKG_VERSION"))?;
    let data = read_payload(Path::new(&pkg.payload_path))?;
    use sha2::{Digest, Sha256};
    if hex::encode(Sha256::digest(&data)) != pkg.checksum_sha256.to_ascii_lowercase() {
        return Err(UpdateError::ChecksumMismatch);
    }
    let parent = dest
        .parent()
        .ok_or_else(|| UpdateError::Incompatible("destination has no parent".into()))?;
    let stage = parent.join(format!(".toolhub-update-{}.tmp", uuid::Uuid::new_v4()));
    let result = (|| {
        use std::io::Write;
        let mut file = std::fs::OpenOptions::new()
            .create_new(true)
            .write(true)
            .open(&stage)?;
        file.write_all(&data)?;
        file.sync_all()?;
        drop(file);
        if cancelled() {
            return Err(UpdateError::Cancelled);
        }
        if dest.exists() {
            std::fs::copy(dest, dest.with_extension("bak"))?;
        }
        if cancelled() {
            return Err(UpdateError::Cancelled);
        }
        std::fs::rename(&stage, dest)?;
        Ok(())
    })();
    if result.is_err() {
        let _ = std::fs::remove_file(stage);
    }
    result
}

pub fn rollback_update(dest: &Path) -> Result<(), UpdateError> {
    let data = read_payload(&dest.with_extension("bak"))?;
    let stage = dest.with_extension(format!("rollback-{}", uuid::Uuid::new_v4()));
    std::fs::write(&stage, data)?;
    std::fs::rename(&stage, dest)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    #[test]
    fn verified_relative_payload_cannot_change_before_apply() {
        use sha2::{Digest, Sha256};
        let dir = tempfile::tempdir().unwrap();
        let source = dir.path().join("payload");
        let dest = dir.path().join("installed");
        let manifest = dir.path().join("manifest.json");
        std::fs::write(&source, b"verified bytes").unwrap();
        std::fs::write(&dest, b"previous bytes").unwrap();
        let hash = hex::encode(Sha256::digest(b"verified bytes"));
        std::fs::write(&manifest,serde_json::to_vec(&serde_json::json!({"schema":"toolhub.update/v1","product":"toolhub","version":"0.3.0","min_current":"0.1.0","checksum_sha256":hash,"payload_path":"payload"})).unwrap()).unwrap();
        let pkg = verify_package(&manifest).unwrap();
        apply_update(&pkg, &dest, &|| false).unwrap();
        assert_eq!(std::fs::read(&dest).unwrap(), b"verified bytes");
        std::fs::write(&source, b"substituted bytes").unwrap();
        assert!(matches!(
            apply_update(&pkg, &dest, &|| false),
            Err(UpdateError::ChecksumMismatch)
        ));
        assert_eq!(std::fs::read(&dest).unwrap(), b"verified bytes");
    }

    #[test]
    fn checksum_mismatch_rejected() {
        let dir = std::env::temp_dir().join("toolhub-upd");
        let _ = std::fs::create_dir_all(&dir);
        let payload = dir.join("bin.dat");
        std::fs::write(&payload, b"hello").unwrap();
        let man = dir.join("manifest.json");
        let mut f = std::fs::File::create(&man).unwrap();
        write!(
            f,
            r#"{{"schema":"toolhub.update/v1","product":"toolhub","version":"0.3.0","min_current":"0.1.0","checksum_sha256":"deadbeef","payload_path":"bin.dat"}}"#
        )
        .unwrap();
        assert!(matches!(
            verify_package(&man),
            Err(UpdateError::ChecksumMismatch)
        ));
    }

    #[test]
    fn cancel_returns_cancelled() {
        let pkg = UpdatePackage {
            schema: "toolhub.update/v1".into(),
            product: "toolhub".into(),
            version: "0.3.0".into(),
            min_current: "0.1.0".into(),
            checksum_sha256: "x".into(),
            payload_path: "p".into(),
        };
        let r = apply_update(&pkg, Path::new("/tmp/x"), &|| true);
        assert!(matches!(r, Err(UpdateError::Cancelled)));
    }
}
