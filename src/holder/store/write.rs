/* src/holder/store/write.rs */

use std::cell::RefCell;
use std::path::PathBuf;
use std::sync::Arc;
use std::sync::atomic::Ordering;
use std::time::Instant;

#[cfg(feature = "events")]
use super::super::HoldEvent;
use super::super::{Entry, HoldError, Meta, UnloadPolicy};
use super::Store;

impl<T> Store<T>
where
	T: Clone + Send + Sync,
{
	/// Inserts or updates a config entry.
	pub fn insert(&self, key: String, value: T, source: PathBuf, policy: UnloadPolicy) -> Arc<T> {
		let value = Arc::new(value);
		let version = self.version.fetch_add(1, Ordering::SeqCst) + 1;
		let meta = Meta {
			source,
			loaded_at: Instant::now(),
			version,
			policy,
		};

		let new_entry = Entry {
			value: Arc::clone(&value),
			meta: meta.clone(),
		};

		// Capture old_entry inside rcu to ensure event consistency.
		let old_entry: RefCell<Option<Entry<T>>> = RefCell::new(None);

		self.inner.rcu(|map| {
			*old_entry.borrow_mut() = map.get(&key).cloned();
			let mut new_map = (**map).clone();
			new_map.insert(key.clone(), new_entry.clone());
			new_map
		});

		let old_entry = old_entry.into_inner();

		#[cfg(feature = "events")]
		{
			let event = if let Some(old) = old_entry {
				HoldEvent::Updated {
					key,
					old: old.value,
					new: Arc::clone(&value),
					meta,
				}
			} else {
				HoldEvent::Loaded {
					key,
					value: Arc::clone(&value),
					meta,
				}
			};
			let _ = self.events.send(event);
		}

		#[cfg(not(feature = "events"))]
		{
			let _ = old_entry;
		}

		value
	}

	/// Removes a config entry by key.
	pub fn remove(&self, key: &str) -> Result<Arc<T>, HoldError> {
		// Pre-check to avoid unnecessary clone in rcu.
		let snapshot = self.inner.load();
		let entry = match snapshot.get(key) {
			None => {
				return Err(HoldError::NotFound {
					key: key.to_string(),
				});
			}
			Some(e) => e,
		};

		if entry.meta.policy == UnloadPolicy::Persistent {
			let error = HoldError::PersistentRemoval {
				key: key.to_string(),
			};
			#[cfg(feature = "events")]
			{
				let _ = self.events.send(HoldEvent::Retained {
					key: key.to_string(),
					error: error.clone(),
				});
			}
			return Err(error);
		}

		// Now perform the actual removal atomically.
		let removed: RefCell<Option<Arc<T>>> = RefCell::new(None);

		self.inner.rcu(|map| {
			let mut new_map = (**map).clone();
			if let Some(entry) = new_map.remove(key) {
				*removed.borrow_mut() = Some(entry.value);
			}
			new_map
		});

		match removed.into_inner() {
			Some(value) => {
				#[cfg(feature = "events")]
				{
					let _ = self.events.send(HoldEvent::Removed {
						key: key.to_string(),
						value: Arc::clone(&value),
					});
				}
				Ok(value)
			}
			// Entry was removed by another thread between pre-check and rcu.
			None => Err(HoldError::ConcurrentlyRemoved {
				key: key.to_string(),
			}),
		}
	}
}
