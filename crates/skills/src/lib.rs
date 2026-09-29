//! Skill parsing and capability availability (never installs tool dependencies).

use toolhub_core::{SkillManifest, SkillRequirement};

#[derive(Debug, Clone)]
pub struct SkillAvailability {
    pub skill: SkillManifest,
    pub requires: Vec<SkillRequirement>,
    pub optional: Vec<SkillRequirement>,
    pub status: toolhub_core::SkillStatus,
}

/// Resolve requirements against known provider names/versions. Does not install.
pub fn resolve_availability(
    skill: &SkillManifest,
    providers: &[(String, Option<String>)], // capability -> version
) -> SkillAvailability {
    let mut requires = vec![];
    let mut all_ok = true;
    for req in &skill.requires {
        let cap = req.capability.as_str();
        let found = providers.iter().find(|(c, _)| c == cap || c.ends_with(cap));
        let satisfied = match found {
            Some((_, ver)) => match (&req.version, ver) {
                (Some(constraint), Some(v)) => toolhub_core::VersionConstraint::parse(constraint)
                    .map(|c| c.matches(v))
                    .unwrap_or(false),
                (Some(_), None) => false,
                (None, _) => true,
            },
            None => false,
        };
        if !satisfied {
            all_ok = false;
        }
        requires.push(SkillRequirement {
            requirement: req.clone(),
            satisfied,
            provider: found.map(|(c, v)| match v {
                Some(v) => format!("{c} {v}"),
                None => c.clone(),
            }),
            note: if satisfied {
                None
            } else {
                Some("missing capability; ToolHub will not install it".into())
            },
        });
    }

    let mut optional = vec![];
    for req in &skill.optional {
        let cap = req.capability.as_str();
        let found = providers.iter().find(|(c, _)| c == cap);
        optional.push(SkillRequirement {
            requirement: req.clone(),
            satisfied: found.is_some(),
            provider: found.map(|(c, _)| c.clone()),
            note: None,
        });
    }

    let status = if all_ok {
        toolhub_core::SkillStatus::Available
    } else {
        toolhub_core::SkillStatus::MissingCapabilities
    };

    SkillAvailability {
        skill: skill.clone(),
        requires,
        optional,
        status,
    }
}

/// Parse a skill directory containing manifest JSON (and optional SKILL.md).
pub fn load_skill_dir(dir: &std::path::Path) -> Result<SkillManifest, String> {
    let manifest_path = dir.join("skill.json");
    if !manifest_path.exists() {
        return Err(format!("missing skill.json in {}", dir.display()));
    }
    let raw = std::fs::read_to_string(&manifest_path).map_err(|e| e.to_string())?;
    let v: serde_json::Value = serde_json::from_str(&raw).map_err(|e| e.to_string())?;
    let m = SkillManifest::parse_yaml_like(&v).map_err(|e| e.to_string())?;
    m.assert_safe_paths().map_err(|e| e.to_string())?;
    // Registration must not execute hooks/installers — we only read files.
    if dir.join("install.sh").exists() || dir.join("hooks").exists() {
        return Err("skill package contains installer/hook payload; refusing to register".into());
    }
    Ok(m)
}

#[cfg(test)]
mod tests {
    use super::*;
    use toolhub_core::{CapabilityId, CapabilityRequirement};

    #[test]
    fn missing_dependency_is_useful() {
        let skill = SkillManifest {
            schema: "toolhub.skill/v1".into(),
            id: toolhub_core::SkillId::new("x.y").unwrap(),
            name: "X".into(),
            description: String::new(),
            requires: vec![CapabilityRequirement {
                capability: CapabilityId::new("android.apk.decompile").unwrap(),
                version: None,
                optional: false,
            }],
            optional: vec![],
            kind: Default::default(),
            instruction_file: Some("SKILL.md".into()),
            mcp_config: None,
            package_path: None,
        };
        let av = resolve_availability(&skill, &[]);
        assert_eq!(av.status, toolhub_core::SkillStatus::MissingCapabilities);
        assert!(!av.requires[0].satisfied);
    }

    #[test]
    fn rejects_installer_payloads() {
        let dir = std::env::temp_dir().join("toolhub-skill-bad");
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(
            dir.join("skill.json"),
            r#"{"schema":"toolhub.skill/v1","id":"a.b","name":"A"}"#,
        )
        .unwrap();
        std::fs::write(dir.join("install.sh"), "#!/bin/sh\nrm -rf /\n").unwrap();
        assert!(load_skill_dir(&dir).is_err());
    }
}
