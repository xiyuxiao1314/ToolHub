use serde::{Deserialize, Serialize};

use crate::evidence::{Evidence, EvidenceSource, Provenance};

/// How sure we are that an owner attribution is correct.
/// Separate from instance trust and from recognition confidence.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum OwnerCertainty {
    Known,
    Probable,
    Unknown,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum OwnerKind {
    User,
    System,
    Application,
    PackageManager,
    Agent,
    Unknown,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Owner {
    pub kind: OwnerKind,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub id: Option<String>,
    pub certainty: OwnerCertainty,
    #[serde(default)]
    pub evidence: Vec<Evidence>,
}

impl Owner {
    pub fn unknown() -> Self {
        Self {
            kind: OwnerKind::Unknown,
            id: None,
            certainty: OwnerCertainty::Unknown,
            evidence: vec![],
        }
    }

    /// Directory names alone must not produce `Known` ownership.
    pub fn from_directory_hint(dir: &str, app_hint: &str) -> Self {
        let mut evidence = vec![Evidence {
            source: EvidenceSource::EnvironmentMetadata,
            provenance: Provenance::Native,
            confidence: 0.55,
            summary: format!("parent directory resembles {app_hint}: {dir}"),
            detail: Some("directory name is not proof of ownership".into()),
            observed_at: chrono::Utc::now(),
        }];
        evidence.retain(|_| true);
        Self {
            kind: OwnerKind::Application,
            id: Some(app_hint.to_string()),
            certainty: OwnerCertainty::Probable,
            evidence,
        }
    }

    pub fn user() -> Self {
        Self {
            kind: OwnerKind::User,
            id: None,
            certainty: OwnerCertainty::Known,
            evidence: vec![Evidence::user("user-owned")],
        }
    }
}

/// How the installation was acquired (separate from who manages it).
impl Default for Owner {
    fn default() -> Self {
        Self::unknown()
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum Origin {
    System,
    UserInstall,
    PackageManager {
        manager: String,
    },
    ApplicationBundle {
        application: String,
    },
    AgentSandbox {
        agent: String,
    },
    ProjectLocal,
    #[default]
    Unknown,
}

impl Origin {
    pub fn label(&self) -> String {
        match self {
            Origin::System => "system".into(),
            Origin::UserInstall => "user".into(),
            Origin::PackageManager { manager } => manager.clone(),
            Origin::ApplicationBundle { application } => application.clone(),
            Origin::AgentSandbox { agent } => format!("agent:{agent}"),
            Origin::ProjectLocal => "project".into(),
            Origin::Unknown => "unknown".into(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn directory_hint_is_probable_not_known() {
        let o = Owner::from_directory_hint("C:/Users/x/AppData/Local/Programs/cursor", "cursor");
        assert_eq!(o.certainty, OwnerCertainty::Probable);
    }
}
