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

    /// F09: detect distinct project/venv/conda/package/agent environments from path markers.
    pub fn environment_for_path(&self, path: &str) -> EnvironmentId {
        let p = toolhub_core::path_norm::canonicalize_best_effort(path).replace('\\', "/");
        // Project venv: .../project/.venv or .../project/venv
        if let Some(env_id) = detect_project_env(&p) {
            return env_id;
        }
        if let Some((kind, root)) = manager_root(&p) {
            return EnvironmentId::new(format!(
                "env.{kind}.{}",
                &toolhub_core::path_fingerprint(&root)[..20]
            ))
            .unwrap();
        }
        let lower = p.to_ascii_lowercase();
        if lower.contains("program files")
            || lower.contains("/usr/bin")
            || lower.contains("/usr/local/bin")
        {
            return EnvironmentId::new("env.system").unwrap();
        }
        EnvironmentId::new("env.user").unwrap()
    }

    #[allow(dead_code)]
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
                "env.homebrew" => EnvironmentKind::Homebrew,
                "env.scoop" => EnvironmentKind::Scoop,
                "env.chocolatey" => EnvironmentKind::Chocolatey,
                "env.conda" => EnvironmentKind::Conda,
                "env.agent" => EnvironmentKind::AgentSandbox,
                s if s.starts_with("env.homebrew.") => EnvironmentKind::Homebrew,
                s if s.starts_with("env.scoop.") => EnvironmentKind::Scoop,
                s if s.starts_with("env.chocolatey.") => EnvironmentKind::Chocolatey,
                s if s.starts_with("env.conda.") => EnvironmentKind::Conda,
                s if s.starts_with("env.agent.") => EnvironmentKind::AgentSandbox,
                s if s.starts_with("env.cargo.") => EnvironmentKind::Cargo,
                s if s.starts_with("env.nvm.") => EnvironmentKind::Nvm,
                s if s.starts_with("env.pyenv.") => EnvironmentKind::Pyenv,
                s if s.starts_with("env.uv.") => EnvironmentKind::Uv,
                s if s.starts_with("env.winget.") => EnvironmentKind::Winget,
                s if s.starts_with("env.project.") => EnvironmentKind::Venv,
                _ => EnvironmentKind::Unknown,
            };
            let root = detected_root(path);
            let name = std::path::Path::new(&root)
                .file_name()
                .map(|s| s.to_string_lossy().to_string())
                .unwrap_or_else(|| id.as_str().into());
            let parent_id = if id.as_str().starts_with("env.project.") {
                let parent_root = root.rsplit_once('/').map(|(p, _)| p).unwrap_or(&root);
                let parent = EnvironmentId::new(format!(
                    "env.projectroot.{}",
                    &toolhub_core::path_fingerprint(parent_root)[..20]
                ))
                .unwrap();
                self.insert(Environment {
                    id: parent.clone(),
                    name: std::path::Path::new(parent_root)
                        .file_name()
                        .map(|s| s.to_string_lossy().to_string())
                        .unwrap_or_else(|| "Project".into()),
                    kind: EnvironmentKind::ProjectLocal,
                    root_path: Some(parent_root.into()),
                    parent_id: Some(EnvironmentId::new("env.user").unwrap()),
                    origin: Origin::ProjectLocal,
                    owner: attribute_owner(path),
                    labels: vec!["root inferred from venv marker; size unknown".into()],
                });
                Some(parent)
            } else {
                Some(EnvironmentId::new("env.user").unwrap())
            };
            self.insert(Environment {
                id: id.clone(),
                name,
                kind: kind.clone(),
                root_path: Some(root),
                parent_id,
                origin: if kind == EnvironmentKind::Venv {
                    Origin::ProjectLocal
                } else {
                    Origin::Unknown
                },
                owner: attribute_owner(path),
                labels: vec!["inferred from path; size unknown".into()],
            });
        }
        id
    }
}

/// F09: project-local venvs become distinct environments keyed by project root.
fn detect_project_env(p: &str) -> Option<EnvironmentId> {
    let parts: Vec<&str> = p.split('/').collect();
    for (i, seg) in parts.iter().enumerate() {
        if ([".venv", "venv", "env"]
            .iter()
            .any(|s| seg.eq_ignore_ascii_case(s)))
            && i > 0
        {
            let root = parts[..=i].join("/");
            let id = format!(
                "env.project.{}",
                &toolhub_core::path_fingerprint(&root)[..20]
            );
            return EnvironmentId::new(id).ok();
        }
    }
    None
}

fn detected_root(path: &str) -> String {
    let p = toolhub_core::path_norm::canonicalize_best_effort(path)
        .replace('\\', "/")
        .trim_start_matches("//?/")
        .to_string();
    let parts: Vec<_> = p.split('/').collect();
    for (i, part) in parts.iter().enumerate() {
        if [".venv", "venv", "env"].contains(&part.to_ascii_lowercase().as_str()) {
            return parts[..=i].join("/");
        }
    }
    if let Some((_, root)) = manager_root(&p) {
        return root;
    }
    p.rsplit_once('/').map(|(p, _)| p.to_string()).unwrap_or(p)
}

fn manager_root(path: &str) -> Option<(&'static str, String)> {
    let parts: Vec<_> = path.trim_start_matches("//?/").split('/').collect();
    for (index, part) in parts.iter().enumerate() {
        let lower = part.to_ascii_lowercase();
        let kind = if lower == "homebrew" || lower == "cellar" {
            "homebrew"
        } else if lower == "scoop" {
            "scoop"
        } else if lower == "chocolatey" {
            "chocolatey"
        } else if lower.contains("conda") {
            "conda"
        } else if [
            ".cursor",
            "cursor",
            ".codex",
            "codex",
            "opencode",
            ".opencode",
        ]
        .contains(&lower.as_str())
        {
            "agent"
        } else if lower == ".cargo" {
            "cargo"
        } else if lower == ".nvm" || lower == "nvm" {
            "nvm"
        } else if lower == ".pyenv" {
            "pyenv"
        } else if lower == "uv" {
            "uv"
        } else if lower == "winget" {
            "winget"
        } else {
            continue;
        };
        let end = if kind == "conda"
            && parts
                .get(index + 1)
                .is_some_and(|p| p.eq_ignore_ascii_case("envs"))
            && parts.get(index + 2).is_some()
        {
            index + 2
        } else if lower == "cellar" {
            index.saturating_sub(1)
        } else {
            index
        };
        return Some((kind, parts[..=end].join("/")));
    }
    None
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
    fn same_basename_projects_have_distinct_root_identity() {
        let mut graph = EnvironmentGraph::new();
        let a = graph.ensure_detected("C:/first/same/.venv/Scripts/python.exe");
        let b = graph.ensure_detected("C:/second/same/.venv/Scripts/python.exe");
        assert_ne!(a, b);
        let env = graph.environments.iter().find(|e| e.id == a).unwrap();
        assert_eq!(env.root_path.as_deref(), Some("C:/first/same/.venv"));
        assert!(env.parent_id.is_some());
    }

    #[test]
    fn conda_environments_preserve_each_canonical_root() {
        let mut graph = EnvironmentGraph::new();
        let a = graph.ensure_detected("C:/first/miniconda3/envs/same/python.exe");
        let b = graph.ensure_detected("C:/second/miniconda3/envs/same/python.exe");
        assert_ne!(a, b);
        let env = graph.environments.iter().find(|e| e.id == a).unwrap();
        assert_eq!(
            env.root_path.as_deref(),
            Some("C:/first/miniconda3/envs/same")
        );
        assert_eq!(env.kind, EnvironmentKind::Conda);
    }

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
