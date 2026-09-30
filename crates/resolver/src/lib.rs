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
    pub eligibility_error: Option<EligibilityError>,
    pub rejected: Vec<RejectedCandidate>,
    pub fallback_allowed: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum EligibilityError {
    UnknownCapability,
    InvalidPreference,
    InvalidVersionConstraint,
    NoEligibleProvider,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RejectedCandidate {
    pub candidate: CandidateInstance,
    pub reasons: Vec<RejectionReason>,
}
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum RejectionReason {
    Blocked,
    UnknownTrust,
    TrustMismatch,
    ArchitectureMismatch,
    VersionMismatch,
    MissingVersion,
    MalformedVersion,
}

#[derive(Debug, Clone, Default)]
pub struct ResolvePrefs {
    pub cwd: Option<String>,
    pub prefer_environment: Option<String>,
    pub min_version: Option<String>,
    pub require_trust: Option<String>,
    pub require_arch: Option<String>,
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
                eligibility_error: Some(EligibilityError::UnknownCapability),
                rejected: vec![],
                fallback_allowed: true,
            }
        }
    };

    let invalid = prefs
        .require_trust
        .as_deref()
        .filter(|t| !["verified", "known", "user_trusted"].contains(t))
        .map(|_| "require_trust")
        .or_else(|| {
            prefs
                .require_arch
                .as_deref()
                .filter(|a| !["x86", "x86_64", "aarch64", "arm", "universal"].contains(a))
                .map(|_| "require_arch")
        })
        .or_else(|| {
            prefs
                .prefer_environment
                .as_deref()
                .filter(|e| !e.starts_with("env.") || e.chars().any(char::is_whitespace))
                .map(|_| "prefer_environment")
        });
    if let Some(field) = invalid {
        return ResolveOutcome {
            capability: raw.into(),
            canonical,
            selected: None,
            alternatives: vec![],
            explanation: format!("invalid {field} preference"),
            error: Some(format!("invalid {field} preference")),
            eligibility_error: Some(EligibilityError::InvalidPreference),
            rejected: vec![],
            fallback_allowed: true,
        };
    }
    let candidates_before = candidates.clone();
    let mut constraints = vec![];
    for value in [requirement.version.as_deref(), prefs.min_version.as_deref()]
        .into_iter()
        .flatten()
    {
        match toolhub_core::VersionConstraint::parse(value) {
            Ok(c) => constraints.push(c),
            Err(_) => {
                return ResolveOutcome {
                    capability: raw.into(),
                    canonical,
                    selected: None,
                    alternatives: vec![],
                    explanation: "invalid version constraint".into(),
                    error: Some(format!("invalid version constraint: {value}")),
                    eligibility_error: Some(EligibilityError::InvalidVersionConstraint),
                    rejected: vec![],
                    fallback_allowed: true,
                }
            }
        }
    }
    let rank = |t: &str| match t {
        "verified" => 4,
        "known" => 3,
        "user_trusted" => 2,
        _ => 0,
    };
    let mut rejected = vec![];
    candidates.retain(|c| {
        let mut reasons = vec![];
        if c.trust == "blocked" {
            reasons.push(RejectionReason::Blocked)
        } else if rank(&c.trust) == 0 {
            reasons.push(RejectionReason::UnknownTrust)
        }
        if prefs.require_arch.as_deref().is_some_and(|a| a != c.arch) {
            reasons.push(RejectionReason::ArchitectureMismatch)
        }
        if prefs
            .require_trust
            .as_deref()
            .is_some_and(|t| rank(&c.trust) < rank(t))
        {
            reasons.push(RejectionReason::TrustMismatch)
        }
        match c.version.as_deref() {
            Some(v) => {
                if toolhub_core::compare_versions(v, v).is_err() {
                    reasons.push(RejectionReason::MalformedVersion)
                } else if constraints.iter().any(|constraint| !constraint.matches(v)) {
                    reasons.push(RejectionReason::VersionMismatch)
                }
            }
            None => {
                if !constraints.is_empty() {
                    reasons.push(RejectionReason::MissingVersion)
                }
            }
        }
        if !reasons.is_empty() {
            rejected.push(RejectedCandidate {
                candidate: c.clone(),
                reasons,
            });
            false
        } else {
            true
        }
    });

    if let Some(env) = &prefs.prefer_environment {
        for c in &mut candidates {
            c.cwd_match = c.environment.as_deref() == Some(env.as_str());
        }
    }

    if let Some(cwd) = &prefs.cwd {
        for c in &mut candidates {
            if std::path::Path::new(&toolhub_core::path_norm::canonicalize_best_effort(&c.path))
                .starts_with(toolhub_core::path_norm::canonicalize_best_effort(cwd))
            {
                c.cwd_match = true;
            }
        }
    }

    candidates.sort_by(|a, b| {
        score(b)
            .partial_cmp(&score(a))
            .unwrap_or(std::cmp::Ordering::Equal)
            .then_with(|| match (&a.version, &b.version) {
                (Some(a), Some(b)) => {
                    toolhub_core::compare_versions(b, a).unwrap_or(std::cmp::Ordering::Equal)
                }
                _ => std::cmp::Ordering::Equal,
            })
            .then_with(|| a.path.cmp(&b.path))
    });

    // R2-B06: distinguish blocked-only from empty.
    let blocked_count = candidates_before
        .iter()
        .filter(|c| c.trust == "blocked")
        .count();
    let explanation = if candidates.is_empty() {
        if blocked_count > 0 {
            format!("{blocked_count} providers exist but are blocked/untrusted")
        } else {
            "no available provider instances".to_string()
        }
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
        selected: selected.clone(),
        alternatives,
        explanation,
        error: if selected.is_none() {
            Some("no eligible provider".into())
        } else {
            None
        },
        eligibility_error: if selected.is_none() {
            Some(EligibilityError::NoEligibleProvider)
        } else {
            None
        },
        rejected,
        fallback_allowed: true,
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

    #[test]
    fn malformed_trust_preference_cannot_select_provider() {
        let reg = CapabilityRegistry::with_core_taxonomy();
        let req = CapabilityRequirement {
            capability: CapabilityId::new("language.python.execute").unwrap(),
            version: None,
            optional: false,
        };
        let out = resolve(
            &reg,
            &req,
            vec![inst("a", "C:/x/python.exe", "known", Some("3.13"), None)],
            &ResolvePrefs {
                require_trust: Some("banana".into()),
                ..Default::default()
            },
        );
        assert!(out.selected.is_none());
        assert!(out.error.is_some());
    }

    #[test]
    fn newest_eligible_version_ranks_before_path_order() {
        let reg = CapabilityRegistry::with_core_taxonomy();
        let req = CapabilityRequirement {
            capability: CapabilityId::new("language.python.execute").unwrap(),
            version: None,
            optional: false,
        };
        let out = resolve(
            &reg,
            &req,
            vec![
                inst("old", "C:/a/python.exe", "known", Some("3.11"), None),
                inst("new", "C:/z/python.exe", "known", Some("3.13"), None),
            ],
            &ResolvePrefs::default(),
        );
        assert_eq!(out.selected.unwrap().instance_id, "new");
    }

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
