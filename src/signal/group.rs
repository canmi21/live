/* src/signal/group.rs */

use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::Mutex;
use tokio::sync::broadcast;

use super::{Config, Result, Target, Watcher};

#[cfg(feature = "stream")]
use super::watcher::EventStream;

/// Manages multiple named watchers.
#[derive(Default, Clone)]
pub struct WatcherGroup {
	watchers: Arc<Mutex<HashMap<String, Arc<Watcher>>>>,
}

impl WatcherGroup {
	/// Creates a new empty WatcherGroup.
	pub fn new() -> Self {
		Self {
			watchers: Arc::new(Mutex::new(HashMap::new())),
		}
	}

	/// Adds a new watcher to the group.
	#[must_use = "Watcher add result must be handled"]
	pub async fn add(&self, name: impl Into<String>, target: Target, config: Config) -> Result<()> {
		let watcher = Arc::new(Watcher::new(target, config)?);
		let mut lock = self.watchers.lock().await;
		lock.insert(name.into(), watcher);
		Ok(())
	}

	/// Removes and stops a watcher by name.
	pub async fn remove(&self, name: &str) -> bool {
		let mut lock = self.watchers.lock().await;
		lock.remove(name).is_some()
	}

	/// Subscribes to the event channel of a specific watcher.
	pub async fn subscribe(&self, name: &str) -> Option<broadcast::Receiver<super::Event>> {
		let watcher = {
			let lock = self.watchers.lock().await;
			lock.get(name).cloned()
		};

		watcher.map(|watcher| watcher.subscribe())
	}

	/// Subscribes to the event stream of a specific watcher.
	#[cfg(feature = "stream")]
	pub async fn stream(&self, name: &str) -> Option<EventStream> {
		let lock = self.watchers.lock().await;
		lock.get(name).map(|watcher| watcher.stream())
	}

	/// Returns a list of all active watcher names.
	pub async fn list(&self) -> Vec<String> {
		let lock = self.watchers.lock().await;
		lock.keys().cloned().collect()
	}
}
