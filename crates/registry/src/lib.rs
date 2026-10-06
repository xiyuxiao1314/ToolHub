//! SQLite-backed shared registry with migrations from empty database.

mod db;
mod models;
mod repo;

pub use db::{open_memory, open_path, RegistryDb};
pub use models::*;
pub use repo::Registry;

use thiserror::Error;

#[derive(Debug, Error)]
pub enum RegistryError {
    #[error("sqlite: {0}")]
    Sqlite(#[from] rusqlite::Error),
    #[error("serde: {0}")]
    Serde(#[from] serde_json::Error),
    #[error("not found: {0}")]
    NotFound(String),
    #[error("validation: {0}")]
    Validation(String),
    #[error("migration: {0}")]
    Migration(String),
}

pub type RegistryResult<T> = Result<T, RegistryError>;
