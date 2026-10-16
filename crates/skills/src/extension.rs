//! Versioned workbench contributions. v1 is data-only; v2 adds isolated frontend code.
use crate::bundle::SkillBundle;
use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;
use toolhub_recognizer::resources::{ResourcePackage, ResourceTool};
pub const HOST_MODULES: &[&str] = &[
    "tools",
    "programs",
    "artifacts",
    "envs",
    "skills",
    "market",
    "tasks",
];
pub const BRIDGE_PERMISSIONS: &[&str] = &[
    "tools.read",
    "programs.read",
    "programs.propose",
    "skills.read",
    "environments.read",
    "tasks.read",
];
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Navigation {
    pub label: String,
    pub icon: String,
    pub order: u16,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ExtensionView {
    pub id: String,
    pub label: String,
    pub icon: String,
    pub order: u16,
    pub html: String,
    #[serde(default)]
    pub css: String,
    #[serde(default)]
    pub script: String,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ExtensionTheme {
    pub colors: std::collections::BTreeMap<String, String>,
    pub layout: String,
    pub density: String,
    pub sidebar_width: u16,
    pub radius: u16,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ExtensionDependency {
    pub id: String,
    pub min_version: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CliChoice {
    pub label: String,
    pub value: String,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CliParameter {
    pub id: String,
    pub label: String,
    pub kind: String,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub extensions: Vec<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub choices: Vec<CliChoice>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub default: Option<String>,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CliAction {
    pub id: String,
    pub label: String,
    pub description: String,
    pub parameters: Vec<CliParameter>,
    pub args: Vec<String>,
    pub timeout_ms: u64,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ExtensionCli {
    pub id: String,
    pub label: String,
    pub icon: String,
    pub order: u16,
    pub tool_definition: String,
    pub actions: Vec<CliAction>,
}
impl ExtensionCli {
    pub fn validate(&self, package_id: &str) -> Result<(), String> {
        let bounded =
            |s: &str, n| !s.trim().is_empty() && s.len() <= n && !s.chars().any(char::is_control);
        let key = |s: &str| {
            !s.is_empty()
                && s.len() <= 48
                && s.bytes()
                    .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == b'_')
        };
        if !self.id.starts_with(&format!("{package_id}."))
            || self.id.len() > 128
            || toolhub_core::DefinitionId::new(&self.id).is_err()
            || !bounded(&self.label, 64)
            || !bounded(&self.icon, 32)
            || self.tool_definition.len() > 128
            || toolhub_core::DefinitionId::new(&self.tool_definition).is_err()
            || self.actions.is_empty()
            || self.actions.len() > 8
        {
            return Err("invalid CLI contribution".into());
        }
        let mut ids = BTreeSet::new();
        for action in &self.actions {
            if !key(&action.id)
                || !ids.insert(&action.id)
                || !bounded(&action.label, 64)
                || !bounded(&action.description, 1024)
                || !(1..=300000).contains(&action.timeout_ms)
                || action.parameters.is_empty()
                || action.parameters.len() > 8
                || action.args.is_empty()
                || action.args.len() > 128
            {
                return Err("invalid CLI action bounds".into());
            }
            let mut parameters = BTreeSet::new();
            for p in &action.parameters {
                if !key(&p.id) || !parameters.insert(&p.id) || !bounded(&p.label, 64) {
                    return Err("invalid CLI parameter".into());
                }
                match p.kind.as_str() {
                    "input_file" | "output_file" => {
                        if p.extensions.is_empty()
                            || p.extensions.len() > 24
                            || !p.choices.is_empty()
                            || p.default.is_some()
                            || p.extensions.iter().any(|e| {
                                e.is_empty()
                                    || e.len() > 12
                                    || !e
                                        .bytes()
                                        .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit())
                            })
                        {
                            return Err(
                                "file parameters require bounded suffixes, no defaults".into()
                            );
                        }
                    }
                    "choice" => {
                        if !p.extensions.is_empty()
                            || p.choices.is_empty()
                            || p.choices.len() > 16
                            || p.choices
                                .iter()
                                .any(|c| !bounded(&c.label, 64) || !bounded(&c.value, 128))
                            || p.choices
                                .iter()
                                .map(|c| &c.value)
                                .collect::<BTreeSet<_>>()
                                .len()
                                != p.choices.len()
                            || !p
                                .default
                                .as_ref()
                                .is_some_and(|v| p.choices.iter().any(|c| &c.value == v))
                        {
                            return Err(
                                "choice parameters require finite values and a valid default"
                                    .into(),
                            );
                        }
                    }
                    _ => return Err("CLI parameters support files and finite choices only".into()),
                }
            }
            if action
                .parameters
                .iter()
                .filter(|p| p.kind == "input_file")
                .count()
                != 1
                || action
                    .parameters
                    .iter()
                    .filter(|p| p.kind == "output_file")
                    .count()
                    != 1
            {
                return Err("CLI actions require exactly one input and one output file".into());
            }
            let mut used = BTreeSet::new();
            for arg in &action.args {
                if !bounded(arg, 4096) {
                    return Err("invalid CLI argument".into());
                }
                if let Some(id) = arg.strip_prefix('{').and_then(|s| s.strip_suffix('}')) {
                    if !parameters.contains(&id.to_string()) {
                        return Err("unknown CLI placeholder".into());
                    }
                    used.insert(id);
                } else if arg.contains(['{', '}']) {
                    return Err("CLI placeholders must occupy a complete argument".into());
                }
            }
            if action
                .parameters
                .iter()
                .any(|p| !used.contains(p.id.as_str()))
            {
                return Err("unused CLI parameter".into());
            }
        }
        Ok(())
    }
}
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
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub host_module: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub navigation: Option<Navigation>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub views: Vec<ExtensionView>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub theme: Option<ExtensionTheme>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub cli: Option<ExtensionCli>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub permissions: Vec<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub dependencies: Vec<ExtensionDependency>,
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
        if !["toolhub.extension/v1", "toolhub.extension/v2"].contains(&self.schema.as_str())
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
            || (self.recognition.is_none()
                && self.skills.is_empty()
                && self.host_module.is_none()
                && self.views.is_empty()
                && self.theme.is_none()
                && self.cli.is_none())
        {
            return Err("invalid extension metadata or bounds".into());
        }
        if self.schema == "toolhub.extension/v1"
            && (self.host_module.is_some()
                || self.navigation.is_some()
                || !self.views.is_empty()
                || self.theme.is_some()
                || self.cli.is_some()
                || !self.permissions.is_empty()
                || !self.dependencies.is_empty())
        {
            return Err("v2 required for workbench contributions".into());
        }
        if let Some(module) = &self.host_module {
            if !HOST_MODULES.contains(&module.as_str())
                || self.id != format!("toolhub.builtin.{module}")
                || self.navigation.is_none()
                || !self.views.is_empty()
                || self.theme.is_some()
                || self.cli.is_some()
                || self.recognition.is_some()
                || !self.skills.is_empty()
                || !self.permissions.is_empty()
                || !self.dependencies.is_empty()
            {
                return Err("reserved native module contract".into());
            }
        } else if self.id.starts_with("toolhub.") || self.navigation.is_some() {
            return Err("reserved host namespace/navigation".into());
        }
        if let Some(nav) = &self.navigation {
            if !bounded(&nav.label, 64) || !bounded(&nav.icon, 32) {
                return Err("invalid navigation".into());
            }
        }
        if self.views.len() > 8
            || self.dependencies.len() > 16
            || self.permissions.len() > BRIDGE_PERMISSIONS.len()
            || self.permissions.iter().collect::<BTreeSet<_>>().len() != self.permissions.len()
            || self
                .permissions
                .iter()
                .any(|p| !BRIDGE_PERMISSIONS.contains(&p.as_str()))
        {
            return Err("invalid view, dependency or permission bounds".into());
        }
        if let Some(cli) = &self.cli {
            cli.validate(&self.id)?;
            if self.min_toolhub_version != "0.3.0-beta.4"
                && toolhub_core::compare_versions(&self.min_toolhub_version, "0.3.0-beta.4")
                    .is_ok_and(|v| v == std::cmp::Ordering::Less)
            {
                return Err("CLI contributions require ToolHub beta.4 or newer".into());
            }
            if self.views.iter().any(|v| v.id == cli.id) {
                return Err("duplicate navigation id".into());
            }
        }
        let mut view_ids = BTreeSet::new();
        for view in &self.views {
            if !view.id.starts_with(&format!("{}.", self.id))
                || toolhub_core::DefinitionId::new(&view.id).is_err()
                || view.id.len() > 128
                || !view_ids.insert(&view.id)
                || !bounded(&view.label, 64)
                || !bounded(&view.icon, 32)
                || view.html.trim().is_empty()
                || view.html.len() > 65536
                || view.css.len() > 32768
                || view.script.len() > 65536
            {
                return Err("invalid view identity/content bounds".into());
            }
        }
        let mut deps = BTreeSet::new();
        for dep in &self.dependencies {
            if dep.id == self.id
                || dep.id.len() > 96
                || toolhub_core::DefinitionId::new(&dep.id).is_err()
                || !deps.insert(&dep.id)
                || dep.min_version.len() > 32
                || toolhub_core::compare_versions(&dep.min_version, &dep.min_version).is_err()
            {
                return Err("invalid extension dependency".into());
            }
        }
        if let Some(theme) = &self.theme {
            const COLORS: &[&str] = &[
                "bg",
                "panel",
                "panel-soft",
                "ink",
                "ink-2",
                "muted",
                "line",
                "accent",
                "accent-soft",
                "ok",
                "ok-soft",
                "warn",
                "warn-soft",
                "danger",
                "danger-soft",
            ];
            if !["left", "right", "top"].contains(&theme.layout.as_str())
                || !["comfortable", "compact"].contains(&theme.density.as_str())
                || !(200..=340).contains(&theme.sidebar_width)
                || theme.radius > 24
                || theme.colors.is_empty()
                || theme.colors.iter().any(|(key, value)| {
                    !COLORS.contains(&key.as_str())
                        || ![7, 9].contains(&value.len())
                        || !value.starts_with('#')
                        || !value[1..].chars().all(|c| c.is_ascii_hexdigit())
                })
            {
                return Err("invalid theme tokens or layout".into());
            }
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

#[cfg(test)]
mod workbench_validation_tests {
    use super::*;
    fn sample() -> serde_json::Value {
        serde_json::from_str(include_str!(
            "../../../extensions/examples/workspace.toolhub-extension.json"
        ))
        .unwrap()
    }
    #[test]
    fn v2_is_bounded_and_reserved_native_modules_cannot_contain_third_party_code() {
        for p in [
            sample(),
            serde_json::from_str(include_str!(
                "../../../extensions/examples/graphite.toolhub-extension.json"
            ))
            .unwrap(),
        ] {
            assert!(ExtensionPackage::parse(&serde_json::to_vec(&p).unwrap()).is_ok());
        }
        for change in ["v1", "id", "permission", "bounds", "native", "theme"] {
            let mut p = sample();
            match change {
                "v1" => p["schema"] = serde_json::json!("toolhub.extension/v1"),
                "id" => p["views"][0]["id"] = serde_json::json!("settings"),
                "permission" => p["permissions"] = serde_json::json!(["desktop.controller"]),
                "bounds" => p["views"][0]["script"] = serde_json::json!("x".repeat(65537)),
                "native" => {
                    p["id"] = serde_json::json!("toolhub.builtin.programs");
                    p["host_module"] = serde_json::json!("programs");
                    p["navigation"] = serde_json::json!({"label":"程序","icon":"x","order":1});
                }
                _ => {
                    p["theme"] = serde_json::json!({"colors":{"bg":"url(file:///secret)"},"layout":"left","density":"compact","sidebar_width":224,"radius":12})
                }
            }
            assert!(
                ExtensionPackage::parse(&serde_json::to_vec(&p).unwrap()).is_err(),
                "{change}"
            );
        }
    }
    #[test]
    fn v1_serialization_does_not_add_new_fields_to_persisted_digests() {
        let p = ExtensionPackage::parse(include_bytes!(
            "../../../extensions/examples/blender.toolhub-extension.json"
        ))
        .unwrap();
        let value = serde_json::to_value(p).unwrap();
        for field in [
            "views",
            "theme",
            "host_module",
            "navigation",
            "permissions",
            "dependencies",
        ] {
            assert!(value.get(field).is_none());
        }
    }
}

#[cfg(test)]
mod cli_contract_tests {
    use super::*;
    use serde_json::{json, Value};
    fn sample() -> Value {
        serde_json::from_str(include_str!(
            "../../../extensions/examples/ffmpeg.toolhub-extension.json"
        ))
        .unwrap()
    }
    #[test]
    fn cli_sample_and_legacy_snapshots_validate_but_unsafe_templates_do_not() {
        let sample = sample();
        assert!(ExtensionPackage::parse(&serde_json::to_vec(&sample).unwrap()).is_ok());
        for (path, value) in [
            ("/schema", json!("toolhub.extension/v1")),
            ("/min_toolhub_version", json!("0.3.0-beta.3")),
            ("/cli/actions/0/args/0", json!("prefix-{input}")),
            ("/cli/actions/0/args/0", json!("{undeclared}")),
            ("/cli/actions/0/parameters/0/kind", json!("shell")),
            ("/cli/actions/0/parameters/0/extensions/0", json!("../mp4")),
            ("/cli/actions/0/parameters/2/default", json!("999")),
            ("/cli/actions/0/timeout_ms", json!(300001)),
        ] {
            let mut p = sample.clone();
            *p.pointer_mut(path).unwrap() = value;
            assert!(
                ExtensionPackage::parse(&serde_json::to_vec(&p).unwrap()).is_err(),
                "accepted {path}"
            );
        }
        let legacy = ExtensionPackage::parse(include_bytes!(
            "../../../extensions/examples/blender.toolhub-extension.json"
        ))
        .unwrap();
        assert!(!serde_json::to_value(legacy)
            .unwrap()
            .as_object()
            .unwrap()
            .contains_key("cli"));
    }
}
