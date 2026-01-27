/* src/holder/store/read.rs */

use std::collections::HashMap;
use std::sync::Arc;

#[cfg(feature = "events")]
use super::super::HoldEvent;
use super::super::{Entry, Meta};
use super::Store;

impl<T> Store<T>
where
	T: Clone + Send + Sync,
{
	/// Gets a config value by key. This is a wait-free operation.
	pub fn get(&self, key: &str) -> Option<Arc<T>> {
		let snapshot = self.inner.load();
		snapshot.get(key).map(|entry| Arc::clone(&entry.value))
	}

	/// Gets metadata for a config by key.
	pub fn get_meta(&self, key: &str) -> Option<Meta> {
		let snapshot = self.inner.load();
		snapshot.get(key).map(|entry| entry.meta.clone())
	}

	/// Gets the full entry (value + metadata) by key.
	pub fn get_entry(&self, key: &str) -> Option<Entry<T>> {
		let snapshot = self.inner.load();
		snapshot.get(key).cloned()
	}

	/// Returns an atomic snapshot of all entries.
	pub fn snapshot(&self) -> Arc<HashMap<String, Entry<T>>> {
		self.inner.load_full()
	}

	/// Returns all keys in the store.
	pub fn keys(&self) -> Vec<String> {
		let snapshot = self.inner.load();
		snapshot.keys().cloned().collect()
	}

	/// Returns the number of entries.
	pub fn len(&self) -> usize {
		let snapshot = self.inner.load();
		snapshot.len()
	}

	/// Returns true if the store is empty.
	pub fn is_empty(&self) -> bool {
		let snapshot = self.inner.load();
		snapshot.is_empty()
	}

	/// Subscribes to store change events.
	#[cfg(feature = "events")]
	pub fn subscribe(&self) -> tokio::sync::broadcast::Receiver<HoldEvent<T>> {
		self.events.subscribe()
	}
}
