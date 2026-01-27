/* src/controller/mod.rs */

//!
//! The `Live` struct is the main entry point for using this crate.
//! It binds a `Store` to a `Loader` and optionally a `Watcher`.

use serde::de::DeserializeOwned;
use std::sync::Arc;

#[cfg(feature = "signal")]
use tokio::sync::Mutex;
#[cfg(feature = "signal")]
use tokio::task::JoinHandle;

use crate::holder::{Store, UnloadPolicy};
use crate::loader::{DynLoader, LoadResult, PreProcess, ValidateConfig};

#[cfg(feature = "signal")]
use crate::signal::{Config as WatcherConfig, Target, Watcher};

#[cfg(feature = "logging")]
use log::{error, info, warn};

pub mod error;
pub use error::LiveError;

/// A controller for a live-reloading configuration value.
pub struct Live<T> {
	store: Arc<Store<T>>,
	loader: Arc<DynLoader>,
	key: String,
	#[cfg(feature = "signal")]
	watcher: Option<Arc<Watcher>>,
	#[cfg(feature = "signal")]
	task_handle: Arc<Mutex<Option<JoinHandle<()>>>>,
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
			task_handle: Arc::new(Mutex::new(None)),
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
			task_handle: Arc::new(Mutex::new(None)),
		}
	}

	/// Performs an immediate load from the source.
	pub async fn load(&self) -> Result<(), LiveError> {
		match self.loader.load::<T>(&self.key).await {
			LoadResult::Ok { value, info } => {
				#[cfg(feature = "logging")]
				info!("Loaded config '{}' from {:?}", self.key, info.path);

				self
					.store
					.insert(self.key.clone(), value, info.path, UnloadPolicy::default());
				Ok(())
			}
			LoadResult::NotFound => {
				let msg = format!("Config not found: {}", self.key);
				#[cfg(feature = "logging")]
				warn!("{}", msg);
				Err(LiveError::Load(crate::loader::FmtError::NotFound))
			}
			LoadResult::Invalid(e) => {
				let msg = format!("Invalid config: {}", e);
				#[cfg(feature = "logging")]
				error!("{}", msg);
				Err(LiveError::Load(e))
			}
		}
	}

	/// Manually reloads the configuration.
	pub async fn reload(&self) -> Result<(), LiveError> {
		self.load().await
	}

	pub fn get(&self) -> Option<Arc<T>> {
		self.store.get(&self.key)
	}

	#[cfg(feature = "events")]
	pub fn subscribe(&self) -> tokio::sync::broadcast::Receiver<crate::holder::HoldEvent<T>> {
		self.store.subscribe()
	}

	#[cfg(feature = "signal")]
	pub async fn watch(mut self, config: WatcherConfig) -> Result<Self, LiveError> {
		// Retrieve the source path from the store metadata.
		let meta = self.store.get_meta(&self.key).ok_or(LiveError::NotLoaded)?;
		let watch_path = meta.source;

		let target = Target::File(watch_path.clone());
		let watcher = Watcher::new(target, config)?;

		let mut rx = watcher.subscribe();
		let store = self.store.clone();
		let loader = self.loader.clone();
		let key = self.key.clone();
		#[cfg(feature = "logging")]
		let _source_path = watch_path.clone();

		let handle = tokio::spawn(async move {
			#[cfg(feature = "logging")]
			info!("Started watching config '{}' at {:?}", key, _source_path);

			while let Ok(_event) = rx.recv().await {
				match loader.load::<T>(&key).await {
					LoadResult::Ok { value, info } => {
						store.insert(key.clone(), value, info.path, UnloadPolicy::default());
						#[cfg(feature = "logging")]
						info!("Reloaded config '{}'", key);
					}
					LoadResult::NotFound => {
						#[cfg(feature = "logging")]
						warn!("Config '{}' not found during reload", key);
					}
					LoadResult::Invalid(e) => {
						#[cfg(feature = "logging")]
						error!("Failed to reload config '{}': {}", key, e);
					}
				}
			}
			#[cfg(feature = "logging")]
			info!("Stopped watching config '{}'", key);
		});

		*self.task_handle.lock().await = Some(handle);
		self.watcher = Some(Arc::new(watcher));
		Ok(self)
	}

	#[cfg(feature = "signal")]
	pub async fn stop_watching(&self) {
		if let Some(watcher) = &self.watcher {
			watcher.stop();
		}
		let mut lock = self.task_handle.lock().await;
		if let Some(handle) = lock.take() {
			handle.abort();
		}
	}
}
