//! Error types for the Live controller.

use atomhold::HoldError;
use fmtstruct::FmtError;
use thiserror::Error;

/// Errors that can occur in the Live controller.
#[derive(Debug, Error)]
pub enum LiveError {
	#[error("Load error: {0}")]
	Load(#[from] FmtError),

	#[error("Store error: {0}")]
	Store(#[from] HoldError),

	#[cfg(feature = "signal")]
	#[error("Signal error: {0}")]
	Signal(#[from] fsig::Error),

	#[error("Config not loaded yet. Call load() before watch().")]
	NotLoaded,

	#[error("Builder error: {0}")]
	Builder(String),
}
