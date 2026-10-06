//! Portable instruction bundles. No host config, binaries, hooks or installation payloads.
use serde::{Deserialize, Serialize};
use toolhub_core::{skill::SkillKind, SkillManifest};

pub const MAX_BUNDLE_BYTES: usize = 131_072;
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SkillBundle {
    pub schema: String,
    pub version: String,
    pub category: String,
    pub tags: Vec<String>,
    pub author: String,
    pub license: String,
    pub manifest: SkillManifest,
    pub instructions: String,
}
impl SkillBundle {
    pub fn parse(data: &[u8]) -> Result<Self, String> {
        if data.len() > MAX_BUNDLE_BYTES {
            return Err("能力包超过 128 KiB".into());
        }
        let bundle: Self = serde_json::from_slice(data)
            .map_err(|_| "能力包需要 toolhub.bundle/v1 JSON 格式".to_string())?;
        bundle.validate()?;
        Ok(bundle)
    }
    pub fn validate(&self) -> Result<(), String> {
        let bounded = |s: &str, max| {
            !s.trim().is_empty() && s.len() <= max && !s.chars().any(char::is_control)
        };
        if self.schema != "toolhub.bundle/v1"
            || !bounded(&self.version, 48)
            || !bounded(&self.author, 128)
            || !bounded(&self.license, 64)
            || !["media", "document", "archive", "data", "general"]
                .contains(&self.category.as_str())
            || self.tags.len() > 12
            || self.tags.iter().any(|tag| !bounded(tag, 64))
        {
            return Err("能力包来源、版本、分类或标签声明有误".into());
        }
        self.manifest.validate().map_err(|e| e.to_string())?;
        if self.manifest.kind != SkillKind::Instruction
            || self.manifest.instruction_file.as_deref() != Some("SKILL.md")
            || self.manifest.requires.len() + self.manifest.optional.len() > 32
            || self.instructions.is_empty()
            || self.instructions.len() > 65536
            || self.instructions.contains('\0')
        {
            return Err("能力包仅包含通用流程声明和 SKILL.md 正文（限 64 KiB）".into());
        }
        let platform = self
            .manifest
            .portability
            .as_ref()
            .and_then(|p| p.platforms.first())
            .map(|p| serde_json::to_value(p).unwrap())
            .ok_or("能力包缺少通用性声明")?;
        let compatibility = self
            .manifest
            .library_compatibility(platform.as_str().unwrap(), Some(&self.instructions));
        if !compatibility.reusable {
            return Err(compatibility.issues.join("；"));
        }
        Ok(())
    }
    pub fn bytes(&self) -> Result<Vec<u8>, String> {
        self.validate()?;
        let data = serde_json::to_vec_pretty(self).map_err(|e| e.to_string())?;
        if data.len() > MAX_BUNDLE_BYTES {
            return Err("能力包超过 128 KiB".into());
        }
        Ok(data)
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    fn sample() -> serde_json::Value {
        json!({"schema":"toolhub.bundle/v1","version":"1.0.0","category":"general","tags":["generic"],"author":"Fixture","license":"MIT","manifest":{"schema":"toolhub.skill/v1","id":"workflow.fixture","name":"Fixture","description":"Generic instructions","kind":"instruction","instruction_file":"SKILL.md","requires":[],"portability":{"scope":"portable","platforms":["windows"],"inputs":["User input"],"outputs":["Result"],"permissions":[],"host_dependencies":[]}},"instructions":"Inspect user input and report results."})
    }
    #[test]
    fn shared_bundle_is_bounded_declarative_and_round_trips() {
        let bundle = SkillBundle::parse(&serde_json::to_vec(&sample()).unwrap()).unwrap();
        assert_eq!(
            SkillBundle::parse(&bundle.bytes().unwrap())
                .unwrap()
                .manifest
                .id
                .as_str(),
            "workflow.fixture"
        );
        for key in ["hooks", "executable", "mcp_config", "provider_token"] {
            let mut value = sample();
            value[key] = json!("not allowed");
            assert!(SkillBundle::parse(&serde_json::to_vec(&value).unwrap()).is_err());
        }
        assert!(SkillBundle::parse(&vec![b' '; MAX_BUNDLE_BYTES + 1]).is_err());
        let mut value = sample();
        value["instructions"] = json!("image_gen.imagegen");
        assert!(SkillBundle::parse(&serde_json::to_vec(&value).unwrap()).is_err());
        value = sample();
        value["manifest"]["portability"]["scope"] = json!("host_specific");
        assert!(SkillBundle::parse(&serde_json::to_vec(&value).unwrap()).is_err());
        value = sample();
        value["manifest"]["instruction_file"] = json!("../SKILL.md");
        assert!(SkillBundle::parse(&serde_json::to_vec(&value).unwrap()).is_err());
    }
}
