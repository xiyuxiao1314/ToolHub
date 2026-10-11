//! Declarative packages contain only existing data contracts, never code payloads.
use crate::bundle::SkillBundle;
use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;
use toolhub_recognizer::resources::{ResourcePackage, ResourceTool};
pub const MAX_EXTENSION_BYTES: usize = 262144;
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ExtensionPackage {
    pub schema: String,
    pub id: String,
    pub name: String,
    pub version: String,
    pub description: String,
    pub author: String,
    pub license: String,
    pub platforms: Vec<String>,
    pub min_toolhub_version: String,
    #[serde(default)]
    pub recognition: Option<ResourcePackage>,
    #[serde(default)]
    pub skills: Vec<SkillBundle>,
}
impl ExtensionPackage {
    pub fn parse(bytes: &[u8]) -> Result<Self, String> {
        if bytes.len() > MAX_EXTENSION_BYTES {
            return Err("extension exceeds 256 KiB".into());
        }
        let package: Self = serde_json::from_slice(bytes).map_err(|e| e.to_string())?;
        package.validate()?;
        Ok(package)
    }
    pub fn validate(&self) -> Result<(), String> {
        let bounded =
            |s: &str, n| !s.trim().is_empty() && s.len() <= n && !s.chars().any(char::is_control);
        if self.schema != "toolhub.extension/v1"
            || toolhub_core::DefinitionId::new(&self.id).is_err()
            || self.id.len() > 96
            || !bounded(&self.name, 128)
            || !bounded(&self.description, 2048)
            || !bounded(&self.author, 128)
            || !bounded(&self.license, 64)
            || self.version.len() > 32
            || self.min_toolhub_version.len() > 32
            || toolhub_core::compare_versions(&self.version, &self.version).is_err()
            || toolhub_core::compare_versions(&self.min_toolhub_version, &self.min_toolhub_version)
                .is_err()
            || self.platforms.is_empty()
            || self.platforms.len() > 3
            || self
                .platforms
                .iter()
                .any(|p| !["windows", "macos", "linux"].contains(&p.as_str()))
            || self.platforms.iter().collect::<BTreeSet<_>>().len() != self.platforms.len()
            || self.skills.len() > 8
            || (self.recognition.is_none() && self.skills.is_empty())
        {
            return Err("invalid extension metadata or bounds".into());
        }
        let prefix = format!("{}.", self.id);
        if let Some(resource) = &self.recognition {
            toolhub_recognizer::resources::validate(resource).map_err(|e| e.to_string())?;
            if resource.kind != "recognition"
                || resource.version != self.version
                || resource.tools.is_empty()
                || resource.tools.len() > 32
                || resource.capabilities.len() > 64
            {
                return Err("bounded recognition rules at the package version required".into());
            }
            let mut ids = BTreeSet::new();
            let mut names = BTreeSet::new();
            for rule in self.rules() {
                if !rule.definition_id.starts_with(&prefix)
                    || !ids.insert(rule.definition_id.clone())
                    || !bounded(&rule.name, 128)
                    || !bounded(&rule.vendor, 128)
                    || rule.file_names.len() > 16
                    || rule.path_substrings.len() > 8
                    || rule.capabilities.is_empty()
                    || rule.capabilities.len() > 32
                    || rule.path_substrings.iter().any(|s| !bounded(s, 256))
                    || rule.capabilities.iter().any(|s| s.len() > 128)
                    || rule.file_names.iter().any(|s| {
                        !bounded(s, 128)
                            || s == "."
                            || s == ".."
                            || s.contains(['*', '?'])
                            || !names.insert(s.to_lowercase())
                            || toolhub_recognizer::KNOWN_TOOLS
                                .iter()
                                .any(|r| r.file_names.iter().any(|n| n.eq_ignore_ascii_case(s)))
                    })
                {
                    return Err("rule namespace, bounds or filename collision".into());
                }
            }
        }
        let mut ids = BTreeSet::new();
        for skill in &self.skills {
            skill.validate()?;
            if !skill.manifest.id.as_str().starts_with(&prefix)
                || !ids.insert(skill.manifest.id.as_str())
            {
                return Err("Skill must have a unique package-scoped ID".into());
            }
        }
        if serde_json::to_vec(self).map_err(|e| e.to_string())?.len() > MAX_EXTENSION_BYTES {
            return Err("extension exceeds 256 KiB".into());
        }
        Ok(())
    }
    pub fn rules(&self) -> Vec<ResourceTool> {
        self.recognition
            .iter()
            .flat_map(|r| &r.tools)
            .filter_map(|r| serde_json::from_value(r.clone()).ok())
            .collect()
    }
    pub fn compatible(&self, platform: &str, app: &str) -> bool {
        self.platforms.iter().any(|p| p == platform)
            && toolhub_core::compare_versions(app, &self.min_toolhub_version)
                .is_ok_and(|o| o != std::cmp::Ordering::Less)
            && self.skills.iter().all(|s| {
                s.manifest
                    .library_compatibility(platform, Some(&s.instructions))
                    .reusable
            })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn example() -> serde_json::Value {
        serde_json::from_str(include_str!(
            "../../../extensions/examples/blender.toolhub-extension.json"
        ))
        .unwrap()
    }
    #[test]
    fn example_and_bounds() {
        let p = ExtensionPackage::parse(&serde_json::to_vec(&example()).unwrap()).unwrap();
        assert!(p.compatible("windows", "0.3.0"));
        assert!(!p.compatible("linux", "0.2.0"));
        assert!(ExtensionPackage::parse(&vec![b' '; MAX_EXTENSION_BYTES + 1]).is_err());
    }
    #[test]
    fn rejects_payloads_traversal_host_skills_and_collisions() {
        for change in ["hook", "path", "host", "builtin", "namespace"] {
            let mut p = example();
            match change {
                "hook" => p["install"] = serde_json::json!("run me"),
                "path" => {
                    p["recognition"]["tools"][0]["file_names"] = serde_json::json!(["../blender"])
                }
                "host" => {
                    p["skills"][0]["instructions"] = serde_json::json!("Use image_gen.imagegen")
                }
                "builtin" => {
                    p["recognition"]["tools"][0]["file_names"] = serde_json::json!(["git.exe"])
                }
                _ => {
                    p["recognition"]["tools"][0]["definition_id"] =
                        serde_json::json!("org.blender.blender")
                }
            }
            assert!(
                ExtensionPackage::parse(&serde_json::to_vec(&p).unwrap()).is_err(),
                "{change}"
            );
        }
    }
}
