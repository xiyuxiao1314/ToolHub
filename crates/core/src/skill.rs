use serde::{Deserialize, Serialize};

use crate::capability::CapabilityRequirement;
use crate::error::{CoreError, CoreResult};
use crate::id::SkillId;

/// Shared skill model for instruction / MCP / package skills.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SkillManifest {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub portability: Option<crate::skill_portability::SkillPortability>,
    pub schema: String,
    pub id: SkillId,
    pub name: String,
    #[serde(default)]
    pub description: String,
    #[serde(default)]
    pub requires: Vec<CapabilityRequirement>,
    #[serde(default)]
    pub optional: Vec<CapabilityRequirement>,
    #[serde(default)]
    pub kind: SkillKind,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub instruction_file: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub mcp_config: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub package_path: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum SkillKind {
    #[default]
    Instruction,
    Mcp,
    Package,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SkillStatus {
    Available,
    MissingCapabilities,
    Blocked,
    Malformed,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SkillRequirement {
    pub requirement: CapabilityRequirement,
    pub satisfied: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub provider: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub note: Option<String>,
}

impl SkillManifest {
    pub fn parse_yaml_like(json: &serde_json::Value) -> CoreResult<Self> {
        Self::validate_json(json)
    }

    pub fn validate_json(json: &serde_json::Value) -> CoreResult<Self> {
        let obj = json
            .as_object()
            .ok_or_else(|| CoreError::Validation("skill must be an object".into()))?;
        let schema = obj
            .get("schema")
            .and_then(|v| v.as_str())
            .ok_or_else(|| CoreError::Validation("skill.schema required".into()))?;
        if schema != "toolhub.skill/v1" {
            return Err(CoreError::Validation(format!(
                "unsupported skill schema: {schema}"
            )));
        }
        for field in ["instruction_file", "mcp_config", "package_path"] {
            if obj.get(field).is_some_and(|v| !v.is_string()) {
                return Err(CoreError::Validation(format!(
                    "skill.{field} must be a string"
                )));
            }
        }
        for field in ["requires", "optional"] {
            if let Some(value) = obj.get(field) {
                let arr = value.as_array().ok_or_else(|| {
                    CoreError::Validation(format!("skill.{field} must be an array"))
                })?;
                for item in arr {
                    req_from_json(item)?;
                }
            }
        }
        let manifest: Self = serde_json::from_value(json.clone())
            .map_err(|e| CoreError::Validation(format!("invalid skill: {e}")))?;
        manifest.validate()?;
        Ok(manifest)
    }

    pub fn validate(&self) -> CoreResult<()> {
        if self.schema != "toolhub.skill/v1" || self.name.trim().is_empty() {
            return Err(CoreError::Validation(
                "unsupported schema or empty skill name".into(),
            ));
        }
        crate::validate_id(self.id.as_str(), "skill")?;
        if let Some(portability) = &self.portability {
            portability.validate()?;
        }
        for req in self.requires.iter().chain(&self.optional) {
            crate::validate_id(req.capability.as_str(), "capability")?;
            if !crate::capability::is_canonical_capability(req.capability.as_str()) {
                return Err(CoreError::Validation(
                    "skill requirement needs dotted capability identifier".into(),
                ));
            }
            if let Some(version) = &req.version {
                crate::VersionConstraint::parse(version)?;
            }
        }
        match self.kind {
            SkillKind::Instruction if self.mcp_config.is_some() || self.package_path.is_some() => {
                return Err(CoreError::Validation(
                    "instruction skill contains other kind paths".into(),
                ))
            }
            SkillKind::Mcp
                if self.mcp_config.is_none()
                    || self.instruction_file.is_some()
                    || self.package_path.is_some() =>
            {
                return Err(CoreError::Validation(
                    "mcp skill requires only mcp_config".into(),
                ))
            }
            SkillKind::Package
                if self.package_path.is_none()
                    || self.mcp_config.is_some()
                    || self.instruction_file.is_some() =>
            {
                return Err(CoreError::Validation(
                    "package skill requires only package_path".into(),
                ))
            }
            _ => {}
        }
        self.assert_safe_paths()
    }

    /// Canonicalize existing declared assets and enforce package confinement, including links.
    pub fn resolve_package_path(
        root: &std::path::Path,
        relative: &str,
    ) -> CoreResult<std::path::PathBuf> {
        validate_relative_path(relative)?;
        let root = root
            .canonicalize()
            .map_err(|e| CoreError::Validation(format!("skill root unavailable: {e}")))?;
        let target = root
            .join(relative.replace('\\', "/"))
            .canonicalize()
            .map_err(|e| CoreError::Validation(format!("skill asset unavailable: {e}")))?;
        if !target.starts_with(&root) {
            return Err(CoreError::Validation("skill asset escapes package".into()));
        }
        Ok(target)
    }
    /// Skill registration must not execute hooks or installers.
    pub fn assert_safe_paths(&self) -> CoreResult<()> {
        for p in [&self.instruction_file, &self.mcp_config, &self.package_path]
            .into_iter()
            .flatten()
        {
            if validate_relative_path(p).is_err() {
                return Err(CoreError::Validation(format!("unsafe skill path: {p}")));
            }
        }
        Ok(())
    }
}

fn validate_relative_path(p: &str) -> CoreResult<()> {
    if p.is_empty()
        || p.starts_with(['/', '\\'])
        || p.contains(':')
        || p.contains('\0')
        || p.split(['/', '\\'])
            .any(|s| s.is_empty() || s == "." || s == "..")
    {
        return Err(CoreError::Validation("unsafe skill package path".into()));
    }
    Ok(())
}

fn req_from_json(item: &serde_json::Value) -> CoreResult<CapabilityRequirement> {
    if !item.is_object() {
        return Err(CoreError::Validation(
            "requirement must be an object".into(),
        ));
    }
    if item.get("version").is_some_and(|v| !v.is_string())
        || item.get("optional").is_some_and(|v| !v.is_boolean())
    {
        return Err(CoreError::Validation(
            "invalid requirement field type".into(),
        ));
    }
    let cap = item
        .get("capability")
        .and_then(|v| v.as_str())
        .ok_or_else(|| CoreError::Validation("requirement.capability required".into()))?;
    Ok(CapabilityRequirement {
        capability: crate::id::CapabilityId::new(cap)?,
        version: item
            .get("version")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string()),
        optional: item
            .get("optional")
            .and_then(|v| v.as_bool())
            .unwrap_or(false),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_skill_manifest() {
        let v = serde_json::json!({
            "schema": "toolhub.skill/v1",
            "id": "reverse.android.apk",
            "name": "Android APK Analysis",
            "requires": [{"capability": "android.apk.decompile"}, {"capability": "language.java.runtime", "version": ">=17"}]
        });
        let m = SkillManifest::parse_yaml_like(&v).unwrap();
        assert_eq!(m.requires.len(), 2);
    }

    #[test]
    fn strict_manifest_rejects_wrong_types_and_preserves_kind() {
        let base = serde_json::json!({"schema":"toolhub.skill/v1","id":"s","name":"Skill"});
        for (field, value) in [
            ("schema", serde_json::json!("toolhub.skill/v99")),
            ("kind", serde_json::json!("invalid")),
            ("requires", serde_json::json!(true)),
            ("description", serde_json::json!(5)),
            ("instruction_file", serde_json::json!(false)),
        ] {
            let mut v = base.clone();
            v[field] = value;
            assert!(SkillManifest::parse_yaml_like(&v).is_err(), "accepted {v}");
        }
        let mut v = base;
        v["kind"] = serde_json::json!("mcp");
        v["mcp_config"] = serde_json::json!("mcp.json");
        assert_eq!(
            SkillManifest::parse_yaml_like(&v).unwrap().kind,
            SkillKind::Mcp
        );
    }

    #[test]
    fn rejects_unsafe_paths() {
        let v = serde_json::json!({
            "schema": "toolhub.skill/v1",
            "id": "x.y",
            "name": "X",
            "instruction_file": "../etc/passwd"
        });
        assert!(SkillManifest::parse_yaml_like(&v).is_err());
    }

    #[test]
    fn package_assets_are_existing_and_confined() {
        let root = std::env::temp_dir().join(format!(
            "toolhub-skill-owned-{}-{}",
            std::process::id(),
            chrono::Utc::now().timestamp_nanos_opt().unwrap()
        ));
        std::fs::create_dir(&root).unwrap();
        std::fs::write(root.join("SKILL.md"), "owned fixture").unwrap();
        assert_eq!(
            SkillManifest::resolve_package_path(&root, "SKILL.md").unwrap(),
            root.join("SKILL.md").canonicalize().unwrap()
        );
        for path in [
            "../SKILL.md",
            r"\\server\share\SKILL.md",
            r"C:\outside\SKILL.md",
            "missing.md",
            "./SKILL.md",
        ] {
            assert!(
                SkillManifest::resolve_package_path(&root, path).is_err(),
                "accepted {path}"
            );
        }
        std::fs::remove_file(root.join("SKILL.md")).unwrap();
        std::fs::remove_dir(root).unwrap();
    }
}
