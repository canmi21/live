/// Core error type for the loader module.
#[derive(Debug, thiserror::Error)]
pub enum FmtError {
    /// Parsing error from format implementation.
    #[error("parse error: {0}")]
    ParseError(String),

    /// Resource not found.
    #[error("not found")]
    NotFound,

    /// Generic static error message.
    #[error("custom error: {0}")]
    Custom(&'static str),

    /// IO error from source.
    #[error("io error: {0}")]
    Io(#[from] std::io::Error),

    /// Sandbox violation in file system source.
    #[cfg(feature = "fs")]
    #[error("sandbox violation")]
    SandboxViolation,

    /// Validation error from validator crate.
    #[cfg(feature = "validate")]
    #[error("validation failed: {0}")]
    Validation(#[from] validator::ValidationErrors),
}
