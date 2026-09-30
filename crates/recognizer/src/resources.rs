//! Versioned recognition/capability resources loaded independently of core/app.

use serde::{Deserialize, Serialize};
use std::path::Path;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResourcePackage {
    pub schema: String,
    pub name: String,
    pub version: String,
    pub kind: String,
    pub capabilities: Vec<String>,
    #[serde(default)]
    pub tools: Vec<serde_json::Value>,
}

#[derive(Debug, thiserror::Error)]
pub enum ResourceError {
    #[error("io: {0}")]
    Io(#[from] std::io::Error),
    #[error("parse: {0}")]
    Parse(#[from] serde_json::Error),
    #[error("incompatible schema {0}")]
    Incompatible(String),
    #[error("malformed package: {0}")]
    Malformed(String),
}

/// B02-14: validate and load a local resource package. Atomic apply is caller-owned.
pub fn load_package(path: &Path) -> Result<ResourcePackage, ResourceError> {
    let raw = std::fs::read_to_string(path)?;
    let pkg: ResourcePackage = serde_json::from_str(&raw)?;
    if pkg.schema != "toolhub.resource/v1" {
        return Err(ResourceError::Incompatible(pkg.schema));
    }
    if pkg.name.trim().is_empty() || pkg.version.trim().is_empty() {
        return Err(ResourceError::Malformed("name/version required".into()));
    }
    for c in &pkg.capabilities {
        if !c.contains('.')
            || !c
                .chars()
                .all(|ch| ch.is_ascii_lowercase() || ch == '.' || ch.is_ascii_digit())
        {
            return Err(ResourceError::Malformed(format!("bad capability {c}")));
        }
    }
    Ok(pkg)
}

/// Compare resource versions (semver-ish major.minor.patch).
pub fn is_compatible(current: &str, required: &str) -> bool {
    let cur: Vec<u64> = current.split('.').filter_map(|s| s.parse().ok()).collect();
    let req: Vec<u64> = required.split('.').filter_map(|s| s.parse().ok()).collect();
    if req.is_empty() {
        return true;
    }
    // same major required
    cur.first() == req.first()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    #[test]
    fn rejects_incompatible_schema() {
        let dir = std::env::temp_dir().join("toolhub-res");
        let _ = std::fs::create_dir_all(&dir);
        let p = dir.join("bad.json");
        let mut f = std::fs::File::create(&p).unwrap();
        writeln!(f, r#"{{"schema":"toolhub.resource/v9","name":"x","version":"1.0.0","kind":"capability","capabilities":[]}}"#).unwrap();
        assert!(matches!(
            load_package(&p),
            Err(ResourceError::Incompatible(_))
        ));
    }

    #[test]
    fn loads_valid_package() {
        let dir = std::env::temp_dir().join("toolhub-res");
        let _ = std::fs::create_dir_all(&dir);
        let p = dir.join("ok.json");
        std::fs::write(
            &p,
            r#"{"schema":"toolhub.resource/v1","name":"core-caps","version":"1.2.0","kind":"capability","capabilities":["language.python.execute"]}"#,
        )
        .unwrap();
        let pkg = load_package(&p).unwrap();
        assert_eq!(pkg.version, "1.2.0");
        assert!(is_compatible("1.2.0", "1.0.0"));
        assert!(!is_compatible("2.0.0", "1.0.0"));
    }
}
