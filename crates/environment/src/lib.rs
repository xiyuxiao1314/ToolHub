//! Environment graph, ownership attribution, and duplicate analysis.

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};
use toolhub_core::{
    Environment, EnvironmentId, EnvironmentKind, InstanceId, Origin, Owner, OwnerCertainty,
    OwnerKind,
};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EnvironmentGraph {
    pub environments: Vec<Environment>,
    /// instance_id -> environment_id
    pub placements: BTreeMap<String, String>,
}

impl Default for EnvironmentGraph {
    fn default() -> Self {
        Self::new()
    }
}

impl EnvironmentGraph {
    pub fn new() -> Self {
        Self {
            environments: vec![Environment::system(), Environment::user()],
            placements: BTreeMap::new(),
        }
    }

    pub fn insert(&mut self, env: Environment) {
        if !self.environments.iter().any(|e| e.id == env.id) {
            self.environments.push(env);
        }
    }

    pub fn place(&mut self, instance: InstanceId, environment: EnvironmentId) {
        self.placements.insert(
            instance.as_str().to_string(),
            environment.as_str().to_string(),
        );
    }

    pub fn children_of(&self, parent: &EnvironmentId) -> Vec<&Environment> {
        self.environments
            .iter()
            .filter(|e| e.parent_id.as_ref() == Some(parent))
            .collect()
    }

    pub fn environment_for_path(&self, path: &str) -> EnvironmentId {
        let p = path.to_ascii_lowercase();
        if p.contains("homebrew") || p.contains("cellar") {
            return self
                .find_kind(EnvironmentKind::Homebrew)
                .unwrap_or_else(|| EnvironmentId::new("env.system").unwrap());
        }
        if p.contains("scoop") {
            return self
                .find_kind(EnvironmentKind::Scoop)
                .unwrap_or_else(|| EnvironmentId::new("env.user").unwrap());
        }
        if p.contains("conda") {
            return self
                .find_kind(EnvironmentKind::Conda)
                .unwrap_or_else(|| EnvironmentId::new("env.user").unwrap());
        }
        if p.contains("venv") {
            return self
                .find_kind(EnvironmentKind::Venv)
                .unwrap_or_else(|| EnvironmentId::new("env.user").unwrap());
        }
        if p.contains("cursor") || p.contains("opencode") || p.contains("codex") {
            return self
                .find_kind(EnvironmentKind::AgentSandbox)
                .unwrap_or_else(|| EnvironmentId::new("env.user").unwrap());
        }
        if p.contains("program files") || p.contains("/usr/bin") {
            return EnvironmentId::new("env.system").unwrap();
        }
        EnvironmentId::new("env.user").unwrap()
    }

    fn find_kind(&self, kind: EnvironmentKind) -> Option<EnvironmentId> {
        self.environments
            .iter()
            .find(|e| e.kind == kind)
            .map(|e| e.id.clone())
    }

    pub fn ensure_detected(&mut self, path: &str) -> EnvironmentId {
        let id = self.environment_for_path(path);
        if !self.environments.iter().any(|e| e.id == id) {
            let kind = match id.as_str() {
                "env.system" => EnvironmentKind::System,
                "env.user" => EnvironmentKind::User,
                _ => EnvironmentKind::Unknown,
            };
            self.insert(Environment {
                id: id.clone(),
                name: id.as_str().to_string(),
                kind,
                root_path: None,
                parent_id: None,
                origin: Origin::Unknown,
                owner: Owner::unknown(),
                labels: vec![],
            });
        }
        id
    }
}

/// Ownership attribution with explicit evidence; directory names never prove Known.
pub fn attribute_owner(path: &str) -> Owner {
    let p = path.to_ascii_lowercase();
    if p.contains("scoop") {
        return Owner {
            kind: OwnerKind::PackageManager,
            id: Some("scoop".into()),
            certainty: OwnerCertainty::Probable,
            evidence: vec![toolhub_core::Evidence::native(
                toolhub_core::EvidenceSource::PathPattern,
                0.7,
                "path under scoop",
            )
            .unwrap()],
        };
    }
    if p.contains("homebrew") || p.contains("cellar") {
        return Owner {
            kind: OwnerKind::PackageManager,
            id: Some("homebrew".into()),
            certainty: OwnerCertainty::Probable,
            evidence: vec![toolhub_core::Evidence::native(
                toolhub_core::EvidenceSource::PathPattern,
                0.75,
                "path under homebrew",
            )
            .unwrap()],
        };
    }
    if p.contains("cursor") {
        return Owner::from_directory_hint(path, "cursor");
    }
    if p.contains("opencode") {
        return Owner::from_directory_hint(path, "opencode");
    }
    if p.contains("codex") {
        return Owner::from_directory_hint(path, "codex");
    }
    Owner::unknown()
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DuplicateReport {
    pub name: String,
    pub count: usize,
    pub instances: Vec<DuplicateInstance>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DuplicateInstance {
    pub instance_id: String,
    pub version: Option<String>,
    pub path: String,
    pub environment: Option<String>,
    pub arch: String,
}

/// Report duplicates without merging or deleting copies.
pub fn analyze_duplicates(
    rows: &[(String, String, Option<String>, String, String)],
) -> Vec<DuplicateReport> {
    // (name, instance_id, version, path, arch)
    let mut by_name: BTreeMap<String, Vec<DuplicateInstance>> = BTreeMap::new();
    for (name, iid, ver, path, arch) in rows {
        by_name
            .entry(name.clone())
            .or_default()
            .push(DuplicateInstance {
                instance_id: iid.clone(),
                version: ver.clone(),
                path: path.clone(),
                environment: None,
                arch: arch.clone(),
            });
    }
    by_name
        .into_iter()
        .filter(|(_, v)| v.len() > 1)
        .map(|(name, instances)| {
            let count = instances.len();
            DuplicateReport {
                name,
                count,
                instances,
            }
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ownership_probable_for_agent_dirs() {
        let o = attribute_owner("C:/Users/x/.cursor/extensions/bin/tool.exe");
        assert_eq!(o.certainty, OwnerCertainty::Probable);
    }

    #[test]
    fn duplicates_do_not_merge() {
        let rows = vec![
            (
                "Python".into(),
                "i1".into(),
                Some("3.12".into()),
                "a".into(),
                "x64".into(),
            ),
            (
                "Python".into(),
                "i2".into(),
                Some("3.13".into()),
                "b".into(),
                "x64".into(),
            ),
        ];
        let d = analyze_duplicates(&rows);
        assert_eq!(d.len(), 1);
        assert_eq!(d[0].count, 2);
    }
}
