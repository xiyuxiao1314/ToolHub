//! Capability resolution: selection separate from execution authorization.

use serde::{Deserialize, Serialize};
use toolhub_core::{CapabilityId, CapabilityRegistry, CapabilityRequirement};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CandidateInstance {
    pub instance_id: String,
    pub definition_id: String,
    pub name: String,
    pub version: Option<String>,
    pub path: String,
    pub environment: Option<String>,
    pub trust: String,
    pub arch: String,
    pub cwd_match: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResolveOutcome {
    pub capability: String,
    pub canonical: String,
    pub selected: Option<CandidateInstance>,
    pub alternatives: Vec<CandidateInstance>,
    pub explanation: String,
    pub error: Option<String>,
}

#[derive(Debug, Clone, Default)]
pub struct ResolvePrefs {
    pub cwd: Option<String>,
    pub prefer_environment: Option<String>,
    pub min_version: Option<String>,
    pub require_trust: Option<String>,
}

/// Deterministic scoring: project env > version > trust > path stability.
pub fn resolve(
    registry: &CapabilityRegistry,
    requirement: &CapabilityRequirement,
    mut candidates: Vec<CandidateInstance>,
    prefs: &ResolvePrefs,
) -> ResolveOutcome {
    let raw = requirement.capability.as_str();
    let canonical = match registry.canonical_of(raw) {
        Some(c) => c.to_string(),
        None => {
            return ResolveOutcome {
                capability: raw.to_string(),
                canonical: raw.to_string(),
                selected: None,
                alternatives: vec![],
                explanation: "capability id not in registry".into(),
                error: Some(format!("unknown capability: {raw}")),
            }
        }
    };

    // F10: eligibility before ranking
    candidates.retain(|c| c.trust != "blocked" && c.trust != "unknown");
    if let Some(want_trust) = prefs.require_trust.as_deref() {
        let rank = |t: &str| match t {
            "verified" => 4,
            "known" => 3,
            "user_trusted" => 2,
            "unknown" => 1,
            _ => 0,
        };
        let min = rank(want_trust);
        candidates.retain(|c| rank(&c.trust) >= min);
    }

    if let Some(min) = prefs
        .min_version
        .as_deref()
        .or(requirement.version.as_deref())
    {
        match toolhub_core::VersionConstraint::parse(min) {
            Ok(vc) => {
                candidates.retain(|c| match &c.version {
                    Some(v) => vc.matches(v),
                    None => false,
                });
            }
            Err(_) => {
                return ResolveOutcome {
                    capability: raw.to_string(),
                    canonical: raw.to_string(),
                    selected: None,
                    alternatives: vec![],
                    explanation: "invalid version constraint".into(),
                    error: Some(format!("invalid version constraint: {min}")),
                };
            }
        }
    }

    if let Some(env) = &prefs.prefer_environment {
        for c in &mut candidates {
            c.cwd_match = c.environment.as_deref() == Some(env.as_str());
        }
    }

    if let Some(cwd) = &prefs.cwd {
        for c in &mut candidates {
            if c.path.contains(cwd.as_str()) {
                c.cwd_match = true;
            }
        }
    }

    candidates.sort_by(|a, b| {
        score(b)
            .partial_cmp(&score(a))
            .unwrap_or(std::cmp::Ordering::Equal)
            .then_with(|| a.path.cmp(&b.path))
    });

    let explanation = if candidates.is_empty() {
        "no available provider instances".to_string()
    } else {
        format!(
            "selected {} from {} candidates using project/version/trust/path rules; selection is not execution authority",
            candidates[0].name,
            candidates.len()
        )
    };

    let selected = candidates.first().cloned();
    let alternatives = candidates.into_iter().skip(1).collect();

    ResolveOutcome {
        capability: raw.to_string(),
        canonical,
        selected,
        alternatives,
        explanation,
        error: None,
    }
}

fn score(c: &CandidateInstance) -> f64 {
    let mut s = 0.0;
    if c.cwd_match {
        s += 50.0;
    }
    match c.trust.as_str() {
        "verified" => s += 20.0,
        "known" => s += 15.0,
        "user_trusted" => s += 10.0,
        "unknown" => s += 1.0,
        "blocked" => s -= 100.0,
        _ => {}
    }
    if c.version.is_some() {
        s += 5.0;
    }
    // Prefer shorter/stable system-ish paths slightly for determinism.
    if c.path.to_ascii_lowercase().contains("program files") {
        s += 2.0;
    }
    s
}

/// Ensure we never invent providers for unknown capabilities.
pub fn resolve_id_or_err(registry: &CapabilityRegistry, raw: &str) -> Result<CapabilityId, String> {
    registry.resolve_id(raw).map_err(|e| e.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn inst(
        id: &str,
        path: &str,
        trust: &str,
        ver: Option<&str>,
        env: Option<&str>,
    ) -> CandidateInstance {
        CandidateInstance {
            instance_id: id.into(),
            definition_id: "org.python.python".into(),
            name: "Python".into(),
            version: ver.map(|s| s.to_string()),
            path: path.into(),
            environment: env.map(|s| s.to_string()),
            trust: trust.into(),
            arch: "x86_64".into(),
            cwd_match: false,
        }
    }

    #[test]
    fn deterministic_selection() {
        let reg = CapabilityRegistry::with_core_taxonomy();
        let req = CapabilityRequirement {
            capability: CapabilityId::new("language.python.execute").unwrap(),
            version: Some(">=3.11".into()),
            optional: false,
        };
        let cands = vec![
            inst("a", "C:/x/python.exe", "known", Some("3.10"), None),
            inst("b", "C:/y/python.exe", "known", Some("3.13"), None),
            inst("c", "C:/z/python.exe", "blocked", Some("3.13"), None),
        ];
        let out = resolve(&reg, &req, cands, &ResolvePrefs::default());
        assert_eq!(out.selected.unwrap().instance_id, "b");
        assert!(out.error.is_none());
    }
}
