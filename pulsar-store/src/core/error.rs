//! Typed errors. Messages shown to users are looked up by `message_key`;
//! the `Display` text is English and meant for logs.

use crate::core::model::SourceKind;
use thiserror::Error;

pub type Result<T> = std::result::Result<T, StoreError>;

#[derive(Debug, Error)]
pub enum StoreError {
    #[error("network error: {0}")]
    Network(String),
    #[error("could not parse data: {0}")]
    Parse(String),
    #[error("i/o error: {0}")]
    Io(#[from] std::io::Error),
    #[error("package source {0:?} is not available on this system")]
    SourceUnavailable(SourceKind),
    #[error("command failed: {0}")]
    CommandFailed(String),
    #[error("not found: {0}")]
    NotFound(String),
    #[error("not supported: {0}")]
    Unsupported(String),
    #[error("operation cancelled")]
    Cancelled,
}

impl StoreError {
    /// Stable key for the translatable, user-facing message.
    pub fn message_key(&self) -> &'static str {
        match self {
            Self::Network(_) => "error.network",
            Self::Parse(_) => "error.parse",
            Self::Io(_) => "error.io",
            Self::SourceUnavailable(_) => "error.source_unavailable",
            Self::CommandFailed(_) => "error.command_failed",
            Self::NotFound(_) => "error.not_found",
            Self::Unsupported(_) => "error.unsupported",
            Self::Cancelled => "error.cancelled",
        }
    }
}

impl From<serde_json::Error> for StoreError {
    fn from(e: serde_json::Error) -> Self {
        Self::Parse(e.to_string())
    }
}
