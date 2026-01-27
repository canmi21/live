/* src/holder/meta.rs */

use std::path::PathBuf;
use std::time::Instant;

use super::UnloadPolicy;

/// Metadata associated with a config entry.
#[derive(Debug, Clone)]
pub struct Meta {
	/// Source file path.
	pub source: PathBuf,
	/// Timestamp when the config was loaded.
	pub loaded_at: Instant,
	/// Version number, auto-incremented on each change.
	pub version: u64,
	/// Unload policy for this entry.
	pub policy: UnloadPolicy,
}
