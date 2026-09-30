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
    let raw = std::fs::read_to_string(manifest_path)?;
    let pkg: UpdatePackage = serde_json::from_str(&raw)?;
    if pkg.schema != "toolhub.update/v1" || pkg.product != "toolhub" {
        return Err(UpdateError::Incompatible(pkg.schema));
    }
    let payload = manifest_path
        .parent()
        .map(|d| d.join(&pkg.payload_path))
        .unwrap_or_else(|| Path::new(&pkg.payload_path).to_path_buf());
    let data = std::fs::read(&payload)?;
    let hash = {
        use sha2::{Digest, Sha256};
        let mut h = Sha256::new();
        h.update(&data);
        hex::encode(h.finalize())
    };
    if !hash.eq_ignore_ascii_case(&pkg.checksum_sha256) {
        return Err(UpdateError::ChecksumMismatch);
    }
    Ok(pkg)
}

/// Explicit apply: caller must have verified package. Records nothing to discovered tools.
pub fn apply_update(pkg: &UpdatePackage, dest: &Path, cancelled: &dyn Fn() -> bool) -> Result<(), UpdateError> {
    if cancelled() {
        return Err(UpdateError::Cancelled);
    }
    if dest.exists() {
        // Keep rollback copy
        let bak = dest.with_extension("bak");
        std::fs::rename(dest, &bak)?;
    }
    // Payload is copied as the new artifact; no silent publish.
    std::fs::copy(&pkg.payload_path, dest)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

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
            r#"{{"schema":"toolhub.update/v1","product":"toolhub","version":"0.2.0","min_current":"0.1.0","checksum_sha256":"deadbeef","payload_path":"bin.dat"}}"#
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
            version: "0.2.0".into(),
            min_current: "0.1.0".into(),
            checksum_sha256: "x".into(),
            payload_path: "p".into(),
        };
        let r = apply_update(&pkg, Path::new("/tmp/x"), &|| true);
        assert!(matches!(r, Err(UpdateError::Cancelled)));
    }
}
