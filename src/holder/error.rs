/* src/holder/error.rs */

#[derive(Debug, Clone, thiserror::Error)]
pub enum HoldError {
	/// Attempted to remove a config with Persistent policy.
	#[error("cannot remove persistent config: {key}")]
	PersistentRemoval { key: String },
	/// The requested key was not found in the store.
	#[error("key not found: {key}")]
	NotFound { key: String },
	/// The key was removed by another thread between check and removal.
	///
	/// This differs from `NotFound` in that the key existed when the
	/// operation started, but was concurrently removed by another thread.
	#[error("key was concurrently removed: {key}")]
	ConcurrentlyRemoved { key: String },
}
