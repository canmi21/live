/* src/holder/store/replace.rs */

use std::cell::RefCell;
use std::collections::{HashMap, HashSet};
use std::path::PathBuf;
use std::sync::Arc;
use std::sync::atomic::Ordering;
use std::time::Instant;

use super::super::{Entry, Meta, UnloadPolicy};
use super::Store;

#[cfg(feature = "events")]
use super::super::{HoldError, HoldEvent};

impl<T> Store<T>
where
	T: Clone + Send + Sync,
{
	/// Atomically replaces all entries. Persistent entries are retained.
	///
	/// # Notes
	///
	/// - The global version number increments for each entry in `entries`,
	///   so replacing N entries will increase the version by N.
	/// - Events are emitted after the atomic replacement. In high concurrency
	///   scenarios, event order may interleave with other operations' events.
	///   The store state is always consistent, but event ordering is best-effort.
	pub fn replace_all(&self, entries: HashMap<String, (T, PathBuf, UnloadPolicy)>) {
		// Track provided keys to detect attempted deletions.
		let provided_keys: HashSet<String> = entries.keys().cloned().collect();

		// Build new entries.
		let mut new_entries: HashMap<String, Entry<T>> = HashMap::new();
		for (key, (value, source, policy)) in entries {
			let version = self.version.fetch_add(1, Ordering::SeqCst) + 1;
			let meta = Meta {
				source,
				loaded_at: Instant::now(),
				version,
				policy,
			};
			new_entries.insert(
				key,
				Entry {
					value: Arc::new(value),
					meta,
				},
			);
		}

		// Capture old_map inside rcu to ensure event consistency.
		let old_map: RefCell<Arc<HashMap<String, Entry<T>>>> = RefCell::new(Arc::new(HashMap::new()));

		self.inner.rcu(|current_map| {
			*old_map.borrow_mut() = Arc::clone(current_map);
			let mut result = new_entries.clone();

			// Retain entries with Persistent policy that are not in new_entries.
			for (key, entry) in current_map.iter() {
				if !result.contains_key(key) && entry.meta.policy == UnloadPolicy::Persistent {
					result.insert(key.clone(), entry.clone());
				}
			}

			result
		});

		let old_map = old_map.into_inner();

		#[cfg(feature = "events")]
		self.emit_replace_events(&old_map, &new_entries, &provided_keys);

		#[cfg(not(feature = "events"))]
		{
			let _ = (old_map, provided_keys);
		}
	}

	#[cfg(feature = "events")]
	fn emit_replace_events(
		&self,
		old_map: &HashMap<String, Entry<T>>,
		new_entries: &HashMap<String, Entry<T>>,
		provided_keys: &HashSet<String>,
	) {
		// Emit Loaded and Updated events.
		for (key, new_entry) in new_entries {
			if let Some(old_entry) = old_map.get(key) {
				if !Arc::ptr_eq(&old_entry.value, &new_entry.value) {
					let _ = self.events.send(HoldEvent::Updated {
						key: key.clone(),
						old: Arc::clone(&old_entry.value),
						new: Arc::clone(&new_entry.value),
						meta: new_entry.meta.clone(),
					});
				}
			} else {
				let _ = self.events.send(HoldEvent::Loaded {
					key: key.clone(),
					value: Arc::clone(&new_entry.value),
					meta: new_entry.meta.clone(),
				});
			}
		}

		// Emit Removed and Retained events.
		for (key, old_entry) in old_map.iter() {
			if !provided_keys.contains(key) {
				if old_entry.meta.policy == UnloadPolicy::Persistent {
					let error = HoldError::PersistentRemoval { key: key.clone() };
					let _ = self.events.send(HoldEvent::Retained {
						key: key.clone(),
						error,
					});
				} else {
					let _ = self.events.send(HoldEvent::Removed {
						key: key.clone(),
						value: Arc::clone(&old_entry.value),
					});
				}
			}
		}
	}
}
