/* src/controller/live.rs */

//!
//! Single-file configuration controller with live reloading.

use std::sync::Arc;

use atomhold::{Store, UnloadPolicy};
use fmtstruct::{DynLoader, LoadResult, PreProcess, ValidateConfig};
use serde::de::DeserializeOwned;

#[cfg(feature = "signal")]
use fsig::{Config as WatcherConfig, Target, Watcher};
#[cfg(feature = "signal")]
use tokio::task::AbortHandle;

use super::LiveError;

/// A controller for a live-reloading configuration value.
pub struct Live<T> {
	store: Arc<Store<T>>,
	loader: Arc<DynLoader>,
	key: String,
	#[cfg(feature = "signal")]
	watcher: Option<Arc<Watcher>>,
	#[cfg(feature = "signal")]
	abort_handle: Option<AbortHandle>,
}

impl<T> Clone for Live<T> {
	fn clone(&self) -> Self {
		Self {
			store: self.store.clone(),
			loader: self.loader.clone(),
			key: self.key.clone(),
			#[cfg(feature = "signal")]
			watcher: self.watcher.clone(),
			#[cfg(feature = "signal")]
			abort_handle: self.abort_handle.clone(),
		}
	}
}

#[cfg(feature = "signal")]
impl<T> Drop for Live<T> {
	fn drop(&mut self) {
		if let Some(watcher) = self.watcher.take() {
			watcher.stop();
		}
		if let Some(handle) = self.abort_handle.take() {
			handle.abort();
		}
	}
}

/// Builder for Live controller.
pub struct LiveBuilder<T> {
	store: Option<Arc<Store<T>>>,
	loader: Option<Arc<DynLoader>>,
	key: Option<String>,
}

impl<T> LiveBuilder<T>
where
	T: Clone + Send + Sync + DeserializeOwned + PreProcess + ValidateConfig + 'static,
{
	pub fn new() -> Self {
		Self {
			store: None,
			loader: None,
			key: None,
		}
	}

	pub fn store(mut self, store: Arc<Store<T>>) -> Self {
		self.store = Some(store);
		self
	}

	pub fn loader(mut self, loader: DynLoader) -> Self {
		self.loader = Some(Arc::new(loader));
		self
	}

	pub fn key(mut self, key: impl Into<String>) -> Self {
		self.key = Some(key.into());
		self
	}

	pub fn build(self) -> Result<Live<T>, LiveError> {
		let store = self
			.store
			.ok_or_else(|| LiveError::Builder("store is required".to_string()))?;
		let loader = self
			.loader
			.ok_or_else(|| LiveError::Builder("loader is required".to_string()))?;
		let key = self
			.key
			.ok_or_else(|| LiveError::Builder("key is required".to_string()))?;

		Ok(Live {
			store,
			loader,
			key,
			#[cfg(feature = "signal")]
			watcher: None,
			#[cfg(feature = "signal")]
			abort_handle: None,
		})
	}
}

impl<T> Default for LiveBuilder<T>
where
	T: Clone + Send + Sync + DeserializeOwned + PreProcess + ValidateConfig + 'static,
{
	fn default() -> Self {
		Self::new()
	}
}

impl<T> Live<T>
where
	T: Clone + Send + Sync + DeserializeOwned + PreProcess + ValidateConfig + 'static,
{
	pub fn builder() -> LiveBuilder<T> {
		LiveBuilder::new()
	}

	pub fn new(store: Arc<Store<T>>, loader: DynLoader, key: impl Into<String>) -> Self {
		Self {
			store,
			loader: Arc::new(loader),
			key: key.into(),
			#[cfg(feature = "signal")]
			watcher: None,
			#[cfg(feature = "signal")]
			abort_handle: None,
		}
	}

	/// Performs an immediate load from the source.
	pub async fn load(&self) -> Result<(), LiveError> {
		match self.loader.load::<T>(&self.key).await {
			LoadResult::Ok { value, info } => {
				let source_path = tokio::fs::canonicalize(&info.path)
					.await
					.unwrap_or(info.path);

				self.store.insert(
					self.key.clone(),
					value,
					source_path,
					UnloadPolicy::default(),
				);
				Ok(())
			}
			LoadResult::NotFound => Err(LiveError::Load(fmtstruct::FmtError::NotFound)),
			LoadResult::Invalid(e) => Err(LiveError::Load(e)),
		}
	}

	/// Manually reloads the configuration.
	pub async fn reload(&self) -> Result<(), LiveError> {
		self.load().await
	}

	/// Returns the current configuration value.
	pub fn get(&self) -> Option<Arc<T>> {
		self.store.get(&self.key)
	}

	/// Subscribes to store change events.
	#[cfg(feature = "events")]
	pub fn subscribe(&self) -> tokio::sync::broadcast::Receiver<atomhold::HoldEvent<T>> {
		self.store.subscribe()
	}

	/// Attaches a filesystem watcher for live reloading (borrowing version).
	///
	/// Must call `load()` before `start_watching()` to establish the source path.
	#[cfg(feature = "signal")]
	pub async fn start_watching(&mut self, config: WatcherConfig) -> Result<(), LiveError> {
		let meta = self.store.get_meta(&self.key).ok_or(LiveError::NotLoaded)?;
		let watch_path = meta.source;

		let target = Target::File(watch_path);
		let watcher = Watcher::new(target, config)?;

		let mut rx = watcher.subscribe();
		let store = self.store.clone();
		let loader = self.loader.clone();
		let key = self.key.clone();

		let handle = tokio::spawn(async move {
			while let Ok(_event) = rx.recv().await {
				if let LoadResult::Ok { value, info } = loader.load::<T>(&key).await {
					let source_path = tokio::fs::canonicalize(&info.path)
						.await
						.unwrap_or(info.path);
					store.insert(key.clone(), value, source_path, UnloadPolicy::default());
				}
				// NotFound and Invalid are silently ignored during watch.
				// Use events feature to observe reload failures if needed.
			}
		});

		self.abort_handle = Some(handle.abort_handle());
		self.watcher = Some(Arc::new(watcher));
		Ok(())
	}

	/// Attaches a filesystem watcher for live reloading (consuming version).
	///
	/// Must call `load()` before `watch()` to establish the source path.
	#[cfg(feature = "signal")]
	pub async fn watch(mut self, config: WatcherConfig) -> Result<Self, LiveError> {
		self.start_watching(config).await?;
		Ok(self)
	}

	/// Stops the filesystem watcher.
	#[cfg(feature = "signal")]
	pub fn stop_watching(&mut self) {
		if let Some(watcher) = self.watcher.take() {
			watcher.stop();
		}
		if let Some(handle) = self.abort_handle.take() {
			handle.abort();
		}
	}

	/// Returns true if the watcher is currently active.
	#[cfg(feature = "signal")]
	pub fn is_watching(&self) -> bool {
		self.watcher.is_some()
	}
}

impl<T> std::fmt::Debug for Live<T>
where
	T: std::fmt::Debug,
{
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		let mut s = f.debug_struct("Live");
		s.field("store", &self.store);
		s.field("loader", &self.loader);
		s.field("key", &self.key);
		#[cfg(feature = "signal")]
		s.field("watcher", &self.watcher.is_some());
		s.finish_non_exhaustive()
	}
}
