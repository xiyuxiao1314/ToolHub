//! Versioned recognition/capability resources loaded independently of core/app.

use serde::{Deserialize, Serialize};
use std::path::Path;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
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
    validate(&pkg)?;
    Ok(pkg)
}

fn validate(pkg: &ResourcePackage) -> Result<(), ResourceError> {
    if pkg.schema != "toolhub.resource/v1" {
        return Err(ResourceError::Incompatible(pkg.schema.clone()));
    }
    if pkg.name.trim().is_empty()
        || toolhub_core::compare_versions(&pkg.version, &pkg.version).is_err()
        || !["recognition", "capability"].contains(&pkg.kind.as_str())
    {
        return Err(ResourceError::Malformed("name/version required".into()));
    }
    for c in &pkg.capabilities {
        if !toolhub_core::capability::is_canonical_capability(c) {
            return Err(ResourceError::Malformed(format!("bad capability {c}")));
        }
    }
    if pkg.kind == "capability" && !pkg.tools.is_empty() {
        return Err(ResourceError::Malformed(
            "capability package cannot contain recognition rules".into(),
        ));
    }
    for value in &pkg.tools {
        let rule: ResourceTool = serde_json::from_value(value.clone())?;
        if toolhub_core::DefinitionId::new(&rule.definition_id).is_err()
            || rule.name.trim().is_empty()
            || rule.vendor.trim().is_empty()
            || rule.file_names.is_empty()
            || rule
                .file_names
                .iter()
                .any(|n| n.contains(['/', '\\', ':']) || n.is_empty())
            || rule
                .capabilities
                .iter()
                .any(|c| !pkg.capabilities.contains(c))
        {
            return Err(ResourceError::Malformed(
                "invalid recognition rule or undeclared capability".into(),
            ));
        }
    }
    Ok(())
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ResourceTool {
    pub definition_id: String,
    pub name: String,
    pub vendor: String,
    pub file_names: Vec<String>,
    #[serde(default)]
    pub path_substrings: Vec<String>,
    pub capabilities: Vec<String>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
struct ResourceState {
    active: std::collections::BTreeMap<String, ResourcePackage>,
    previous: std::collections::BTreeMap<String, ResourcePackage>,
    last_kind: Option<String>,
}
pub struct ResourceStore {
    directory: std::path::PathBuf,
    state: ResourceState,
}
impl ResourceStore {
    pub fn open(directory: &Path) -> Result<Self, ResourceError> {
        std::fs::create_dir_all(directory)?;
        let path = directory.join("state.json");
        let state: ResourceState = if path.exists() {
            serde_json::from_slice(&std::fs::read(path)?)?
        } else {
            Default::default()
        };
        for pkg in state.active.values().chain(state.previous.values()) {
            validate(pkg)?;
        }
        Ok(Self {
            directory: directory.into(),
            state,
        })
    }
    pub fn list(&self) -> Vec<ResourcePackage> {
        self.state.active.values().cloned().collect()
    }
    pub fn activate(&mut self, path: &Path) -> Result<ResourcePackage, ResourceError> {
        let pkg = load_package(path)?;
        if let Some(current) = self.state.active.get(&pkg.kind) {
            if !is_compatible(&current.version, &pkg.version) {
                return Err(ResourceError::Incompatible(
                    "resource major version changes require a new schema".into(),
                ));
            }
        }
        let mut next = self.state.clone();
        if let Some(previous) = next.active.insert(pkg.kind.clone(), pkg.clone()) {
            next.previous.insert(pkg.kind.clone(), previous);
        }
        next.last_kind = Some(pkg.kind.clone());
        self.persist(&next)?;
        self.state = next;
        Ok(pkg)
    }
    pub fn rollback(&mut self) -> Result<ResourcePackage, ResourceError> {
        let kind = self
            .state
            .last_kind
            .clone()
            .ok_or_else(|| ResourceError::Malformed("no activation to roll back".into()))?;
        let mut next = self.state.clone();
        let previous = next
            .previous
            .remove(&kind)
            .ok_or_else(|| ResourceError::Malformed("no previous compatible resource".into()))?;
        if let Some(active) = next.active.insert(kind.clone(), previous.clone()) {
            next.previous.insert(kind, active);
        }
        self.persist(&next)?;
        self.state = next;
        Ok(previous)
    }
    fn persist(&self, state: &ResourceState) -> Result<(), ResourceError> {
        use std::io::Write;
        let stage = self
            .directory
            .join(format!("state-{}.tmp", uuid::Uuid::new_v4()));
        let mut file = std::fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&stage)?;
        let result = (|| {
            file.write_all(&serde_json::to_vec(state)?)?;
            file.sync_all()?;
            drop(file);
            std::fs::rename(&stage, self.directory.join("state.json"))?;
            Ok(())
        })();
        if result.is_err() {
            let _ = std::fs::remove_file(stage);
        }
        result
    }
    pub fn capabilities(&self) -> toolhub_core::CapabilityRegistry {
        let mut registry = toolhub_core::CapabilityRegistry::with_core_taxonomy();
        for pkg in self.state.active.values() {
            for cap in &pkg.capabilities {
                let _ = registry.register(cap, &format!("resource {} {}", pkg.name, pkg.version));
            }
        }
        registry
    }
    pub fn rules(&self) -> Vec<ResourceTool> {
        self.state
            .active
            .get("recognition")
            .map(|p| {
                p.tools
                    .iter()
                    .filter_map(|v| serde_json::from_value(v.clone()).ok())
                    .collect()
            })
            .unwrap_or_default()
    }
}

/// Compare resource versions (semver-ish major.minor.patch).
pub fn is_compatible(current: &str, required: &str) -> bool {
    if toolhub_core::compare_versions(current, current).is_err()
        || toolhub_core::compare_versions(required, required).is_err()
    {
        return false;
    }
    // same major required
    current.split('.').next() == required.split('.').next()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    #[test]
    fn rejects_malformed_kind_version_and_tools() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("resource.json");
        for value in [
            serde_json::json!({"schema":"toolhub.resource/v1","name":"x","version":"banana","kind":"recognition","capabilities":[],"tools":[]}),
            serde_json::json!({"schema":"toolhub.resource/v1","name":"x","version":"1.0.0","kind":"arbitrary","capabilities":[],"tools":[]}),
            serde_json::json!({"schema":"toolhub.resource/v1","name":"x","version":"1.0.0","kind":"recognition","capabilities":[],"tools":[{}]}),
        ] {
            std::fs::write(&path, serde_json::to_vec(&value).unwrap()).unwrap();
            assert!(load_package(&path).is_err());
        }
    }

    #[test]
    fn active_resource_changes_recognition_then_rolls_back_after_restart() {
        let dir = tempfile::tempdir().unwrap();
        let package = dir.path().join("package.json");
        let binary = dir.path().join("example.exe");
        std::fs::write(&binary, b"read-only unknown fixture").unwrap();
        let candidate = toolhub_core::ScanCandidate::from_path(binary.to_string_lossy());
        let mut store = ResourceStore::open(&dir.path().join("store")).unwrap();
        assert!(!crate::recognize_with_resources(&candidate, &store).recognized);
        let value = |version: &str, name: &str| serde_json::json!({"schema":"toolhub.resource/v1","name":"fixture-rules","version":version,"kind":"recognition","capabilities":["fixture.example.inspect"],"tools":[{"definition_id":"fixture.example","name":name,"vendor":"Fixture","file_names":["example.exe"],"capabilities":["fixture.example.inspect"]}]});
        std::fs::write(
            &package,
            serde_json::to_vec(&value("1.0.0", "ExampleOne")).unwrap(),
        )
        .unwrap();
        store.activate(&package).unwrap();
        let result = crate::recognize_with_resources(&candidate, &store);
        assert_eq!(result.definition.unwrap().name, "ExampleOne");
        assert_eq!(
            result.instance.unwrap().trust.level,
            toolhub_core::TrustLevel::Unknown
        );
        assert!(store.capabilities().contains("fixture.example.inspect"));
        std::fs::write(
            &package,
            serde_json::to_vec(&value("1.1.0", "ExampleTwo")).unwrap(),
        )
        .unwrap();
        store.activate(&package).unwrap();
        drop(store);
        let mut store = ResourceStore::open(&dir.path().join("store")).unwrap();
        assert_eq!(
            crate::recognize_with_resources(&candidate, &store)
                .definition
                .unwrap()
                .name,
            "ExampleTwo"
        );
        store.rollback().unwrap();
        assert_eq!(
            crate::recognize_with_resources(&candidate, &store)
                .definition
                .unwrap()
                .name,
            "ExampleOne"
        );
        std::fs::write(&package, b"{}").unwrap();
        assert!(store.activate(&package).is_err());
        assert_eq!(
            crate::recognize_with_resources(&candidate, &store)
                .definition
                .unwrap()
                .name,
            "ExampleOne"
        );
    }

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
