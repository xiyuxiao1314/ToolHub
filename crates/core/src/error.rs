#[derive(Debug, thiserror::Error)]
pub enum CoreError {
    #[error("invalid identifier: {0}")]
    InvalidId(String),
    #[error("invalid confidence: {0}")]
    InvalidConfidence(f64),
    #[error("invalid version constraint: {0}")]
    InvalidVersion(String),
    #[error("invalid capability id: {0}")]
    InvalidCapability(String),
    #[error("alias cycle or conflict: {0}")]
    AliasConflict(String),
    #[error("unknown capability: {0}")]
    UnknownCapability(String),
    #[error("validation failed: {0}")]
    Validation(String),
    #[error("not found: {0}")]
    NotFound(String),
    #[error("state error: {0}")]
    State(String),
}

pub type CoreResult<T> = Result<T, CoreError>;

impl From<CoreError> for String {
    fn from(e: CoreError) -> Self {
        e.to_string()
    }
}
