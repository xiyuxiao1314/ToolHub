//! Skill parsing and capability availability (never installs tool dependencies).

use toolhub_core::{SkillManifest, SkillRequirement};

#[derive(Debug, Clone, serde::Serialize)]
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
        let found = providers.iter().find(|(c, version)| {
            c == cap && version_matches(req.version.as_deref(), version.as_deref())
        });
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
        let found = providers.iter().find(|(c, version)| {
            c == cap && version_matches(req.version.as_deref(), version.as_deref())
        });
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
    for path in [&m.instruction_file, &m.mcp_config, &m.package_path]
        .into_iter()
        .flatten()
    {
        SkillManifest::resolve_package_path(dir, path).map_err(|e| e.to_string())?;
    }
    // Registration must not execute hooks/installers — we only read files.
    if dir.join("install.sh").exists() || dir.join("hooks").exists() {
        return Err("skill package contains installer/hook payload; refusing to register".into());
    }
    Ok(m)
}

fn version_matches(constraint: Option<&str>, version: Option<&str>) -> bool {
    match constraint {
        None => true,
        Some(c) => toolhub_core::VersionConstraint::parse(c)
            .map(|c| version.is_some_and(|v| c.matches(v)))
            .unwrap_or(false),
    }
}

/// Skills use the same eligibility and ranking as direct capability requests.
pub fn resolve_availability_with_resolver(
    skill: &SkillManifest,
    registry: &toolhub_core::CapabilityRegistry,
    providers: &std::collections::BTreeMap<String, Vec<toolhub_resolver::CandidateInstance>>,
    prefs: &toolhub_resolver::ResolvePrefs,
) -> SkillAvailability {
    let resolve_requirement = |req: &toolhub_core::CapabilityRequirement| {
        let canonical = registry
            .canonical_of(req.capability.as_str())
            .unwrap_or(req.capability.as_str());
        let outcome = toolhub_resolver::resolve(
            registry,
            req,
            providers.get(canonical).cloned().unwrap_or_default(),
            prefs,
        );
        SkillRequirement {
            requirement: req.clone(),
            satisfied: outcome.selected.is_some(),
            provider: outcome.selected.map(|c| c.instance_id),
            note: if outcome.error.is_some() {
                Some(outcome.explanation)
            } else {
                None
            },
        }
    };
    let requires: Vec<_> = skill.requires.iter().map(resolve_requirement).collect();
    let optional = skill.optional.iter().map(resolve_requirement).collect();
    let status = if requires.iter().all(|r| r.satisfied) {
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

#[cfg(test)]
mod tests {
    use super::*;
    use toolhub_core::{CapabilityId, CapabilityRequirement};

    #[test]
    fn optional_requirement_checks_version_and_exact_capability() {
        let mut skill = SkillManifest::validate_json(
            &serde_json::json!({"schema":"toolhub.skill/v1","id":"x.y","name":"X"}),
        )
        .unwrap();
        skill.optional.push(CapabilityRequirement {
            capability: CapabilityId::new("language.python.execute").unwrap(),
            version: Some(">=999".into()),
            optional: true,
        });
        let result = resolve_availability(
            &skill,
            &[("language.python.execute".into(), Some("3.13".into()))],
        );
        assert!(!result.optional[0].satisfied);
        skill.requires = skill.optional.clone();
        let result = resolve_availability(
            &skill,
            &[("bogus.language.python.execute".into(), Some("999".into()))],
        );
        assert!(!result.requires[0].satisfied);
    }

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
