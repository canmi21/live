/* src/holder/store/sync.rs */

use std::collections::HashSet;
use std::path::PathBuf;

use super::super::UnloadPolicy;
use super::Store;

/// Result of a sync operation.
#[derive(Debug, Clone)]
pub struct SyncResult<E> {
	/// Keys that were newly added.
	pub added: Vec<String>,
	/// Keys that failed to load.
	pub failed: Vec<(String, E)>,
	/// Keys that were removed.
	pub removed: Vec<String>,
	/// Keys that were retained due to Persistent policy.
	pub retained: Vec<String>,
}

impl<E> Default for SyncResult<E> {
	fn default() -> Self {
		Self {
			added: Vec::new(),
			failed: Vec::new(),
			removed: Vec::new(),
			retained: Vec::new(),
		}
	}
}

impl<T> Store<T>
where
	T: Clone + Send + Sync,
{
	/// Syncs the store with a set of keys from the filesystem.
	///
	/// - Calls `loader` for keys in `fs_keys` but not in store.
	/// - Removes entries in store but not in `fs_keys` (respects UnloadPolicy).
	///
	/// # Note
	///
	/// This operation is **not atomic**. Individual inserts and removes are
	/// atomic, but other threads may observe intermediate states during sync.
	/// If full atomicity is required, use [`replace_all`](Store::replace_all).
	pub fn sync_with<F, E>(&self, fs_keys: HashSet<String>, mut loader: F) -> SyncResult<E>
	where
		F: FnMut(&str) -> Result<(T, PathBuf, UnloadPolicy), E>,
	{
		let mut result = SyncResult::default();
		let current_keys: HashSet<String> = self.keys().into_iter().collect();

		// Load new keys.
		for key in fs_keys.iter() {
			if !current_keys.contains(key) {
				match loader(key) {
					Ok((value, source, policy)) => {
						self.insert(key.clone(), value, source, policy);
						result.added.push(key.clone());
					}
					Err(e) => {
						result.failed.push((key.clone(), e));
					}
				}
			}
		}

		// Remove keys not in fs_keys.
		for key in current_keys.iter() {
			if !fs_keys.contains(key) {
				match self.remove(key) {
					Ok(_) => {
						result.removed.push(key.clone());
					}
					Err(_) => {
						// Persistent policy prevented removal.
						result.retained.push(key.clone());
					}
				}
			}
		}

		result
	}
}
