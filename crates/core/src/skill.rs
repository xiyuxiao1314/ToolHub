use serde::{Deserialize, Serialize};

use crate::capability::CapabilityRequirement;
use crate::error::{CoreError, CoreResult};
use crate::id::SkillId;

/// Shared skill model for instruction / MCP / package skills.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SkillManifest {
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
        let id = json
            .get("id")
            .and_then(|v| v.as_str())
            .ok_or_else(|| CoreError::Validation("skill.id required".into()))?;
        let name = json
            .get("name")
            .and_then(|v| v.as_str())
            .ok_or_else(|| CoreError::Validation("skill.name required".into()))?;
        let schema = json
            .get("schema")
            .and_then(|v| v.as_str())
            .unwrap_or("toolhub.skill/v1")
            .to_string();
        if !schema.starts_with("toolhub.skill/") {
            return Err(CoreError::Validation(format!(
                "unsupported skill schema: {schema}"
            )));
        }
        let mut requires = vec![];
        if let Some(arr) = json.get("requires").and_then(|v| v.as_array()) {
            for item in arr {
                requires.push(req_from_json(item)?);
            }
        }
        let mut optional = vec![];
        if let Some(arr) = json.get("optional").and_then(|v| v.as_array()) {
            for item in arr {
                optional.push(req_from_json(item)?);
            }
        }
        Ok(Self {
            schema,
            id: SkillId::new(id)?,
            name: name.to_string(),
            description: json
                .get("description")
                .and_then(|v| v.as_str())
                .unwrap_or_default()
                .to_string(),
            requires,
            optional,
            kind: SkillKind::Instruction,
            instruction_file: json
                .get("instruction_file")
                .and_then(|v| v.as_str())
                .map(|s| s.to_string()),
            mcp_config: json
                .get("mcp_config")
                .and_then(|v| v.as_str())
                .map(|s| s.to_string()),
            package_path: None,
        })
    }

    /// Skill registration must not execute hooks or installers.
    pub fn assert_safe_paths(&self) -> CoreResult<()> {
        for p in [&self.instruction_file, &self.mcp_config, &self.package_path]
            .into_iter()
            .flatten()
        {
            if p.contains("..") || p.starts_with('/') || p.starts_with('\\') || p.contains(':') {
                return Err(CoreError::Validation(format!("unsafe skill path: {p}")));
            }
        }
        Ok(())
    }
}

fn req_from_json(item: &serde_json::Value) -> CoreResult<CapabilityRequirement> {
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
    fn rejects_unsafe_paths() {
        let v = serde_json::json!({
            "schema": "toolhub.skill/v1",
            "id": "x.y",
            "name": "X",
            "instruction_file": "../etc/passwd"
        });
        let m = SkillManifest::parse_yaml_like(&v).unwrap();
        assert!(m.assert_safe_paths().is_err());
    }
}
