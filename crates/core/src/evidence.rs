use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::error::{CoreError, CoreResult};

/// Where a fact came from. Classification text is untrusted data.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EvidenceSource {
    PackageManager,
    PublisherMetadata,
    CodeSignature,
    BundleMetadata,
    PathPattern,
    ExecutableMetadata,
    StaticFingerprint,
    KnownProbe,
    AiClassification,
    UserDecision,
    Filesystem,
    Registry,
    EnvironmentMetadata,
}

/// Provenance tag for AI/user claims. Never elevates to execution authority.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Provenance {
    Native,
    Ai,
    User,
    Imported,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Evidence {
    pub source: EvidenceSource,
    pub provenance: Provenance,
    /// Closed unit interval; not a permission.
    pub confidence: f64,
    pub summary: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub detail: Option<String>,
    #[serde(default = "Utc::now")]
    pub observed_at: DateTime<Utc>,
}

impl Evidence {
    pub fn native(
        source: EvidenceSource,
        confidence: f64,
        summary: impl Into<String>,
    ) -> CoreResult<Self> {
        crate::validate_confidence(confidence)?;
        Ok(Self {
            source,
            provenance: Provenance::Native,
            confidence,
            summary: summary.into(),
            detail: None,
            observed_at: Utc::now(),
        })
    }

    pub fn user(summary: impl Into<String>) -> Self {
        Self {
            source: EvidenceSource::UserDecision,
            provenance: Provenance::User,
            confidence: 1.0,
            summary: summary.into(),
            detail: None,
            observed_at: Utc::now(),
        }
    }

    pub fn ai(source_summary: impl Into<String>, confidence: f64) -> CoreResult<Self> {
        crate::validate_confidence(confidence)?;
        Ok(Self {
            source: EvidenceSource::AiClassification,
            provenance: Provenance::Ai,
            confidence,
            summary: source_summary.into(),
            detail: None,
            observed_at: Utc::now(),
        })
    }

    pub fn validate(&self) -> CoreResult<()> {
        crate::validate_confidence(self.confidence)?;
        if self.summary.trim().is_empty() {
            return Err(CoreError::Validation("evidence summary empty".into()));
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn confidence_bounds_enforced() {
        assert!(Evidence::native(EvidenceSource::PathPattern, 1.2, "x").is_err());
        assert!(Evidence::ai("guess", 0.4).unwrap().provenance == Provenance::Ai);
    }
}
