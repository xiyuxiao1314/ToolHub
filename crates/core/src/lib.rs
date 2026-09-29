//! ToolHub core domain model.
//!
//! Separates tool definitions, installed instances, interfaces, capabilities,
//! environments, ownership evidence, trust, and execution identity.
//! Numeric confidence is never execution authority.

pub mod capability;
pub mod env;
pub mod error;
pub mod evidence;
pub mod execution;
pub mod id;
pub mod instance;
pub mod owner;
pub mod path_norm;
pub mod skill;
pub mod trust;
pub mod version;

pub use capability::{CapabilityRegistry, CapabilityRequirement};
pub use env::{Environment, EnvironmentKind};
pub use error::{CoreError, CoreResult};
pub use evidence::{Evidence, EvidenceSource, Provenance};
pub use execution::{
    digest_map, digest_strings, hash_bytes, ExecutionApproval, ExecutionRequest, ExecutionResult,
    ExecutionStatus,
};
pub use id::{
    AgentId, CandidateId, CapabilityId, DefinitionId, EnvironmentId, InstanceId, InterfaceId,
    SkillId,
};
pub use instance::{
    InstanceStatus, Interface, InterfaceKind, ScanCandidate, ToolDefinition, ToolInstance,
};
pub use owner::{Origin, Owner, OwnerCertainty, OwnerKind};
pub use path_norm::{normalize_path, path_fingerprint};
pub use skill::{SkillManifest, SkillRequirement, SkillStatus};
pub use trust::{TrustLevel, TrustRecord};
pub use version::{VersionConstraint, VersionReqOp};

/// Domain validation helper: identifiers must be non-empty and charset-safe.
pub fn validate_id(raw: &str, kind: &str) -> CoreResult<()> {
    if raw.is_empty() {
        return Err(CoreError::InvalidId(format!("{kind} id is empty")));
    }
    if raw.len() > 256 {
        return Err(CoreError::InvalidId(format!("{kind} id exceeds 256 bytes")));
    }
    let ok = raw
        .chars()
        .all(|c| c.is_ascii_alphanumeric() || matches!(c, '.' | '_' | '-' | ':' | '/' | '+'));
    if !ok {
        return Err(CoreError::InvalidId(format!(
            "{kind} id contains unsupported characters: {raw}"
        )));
    }
    Ok(())
}

/// Confidence is stored as a closed unit interval and is not a permission.
pub fn validate_confidence(v: f64) -> CoreResult<f64> {
    if !(0.0..=1.0).contains(&v) || !v.is_finite() {
        return Err(CoreError::InvalidConfidence(v));
    }
    Ok(v)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rejects_bad_ids() {
        assert!(validate_id("", "tool").is_err());
        assert!(validate_id("bad id", "tool").is_err());
        assert!(validate_id("org.ffmpeg.ffmpeg", "tool").is_ok());
    }

    #[test]
    fn confidence_is_unit_interval() {
        assert!(validate_confidence(0.5).is_ok());
        assert!(validate_confidence(-0.1).is_err());
        assert!(validate_confidence(f64::NAN).is_err());
    }
}
