/* src/controller/dir.rs */

//!
//! Directory-based configuration controller with live reloading.

use std::collections::{HashMap, HashSet};
use std::path::PathBuf;
use std::sync::Arc;

use atomhold::{Store, UnloadPolicy};
use fmtstruct::{DynLoader, LoadResult, PreProcess, ValidateConfig};
use serde::de::DeserializeOwned;
use tokio::fs;
use tokio::sync::RwLock;

#[cfg(feature = "signal")]
use fsig::{Config as WatcherConfig, Target, Watcher};

use super::LiveError;
#[cfg(feature = "signal")]
use super::WatchState;
use super::pattern::{KeyPattern, ScanMode, ScanResult};

/// A controller for live-reloading a directory of configurations.
///
/// # Clone Semantics
///
/// `LiveDir` supports efficient cloning. Cloned instances share the same underlying
/// store, loader, and filesystem watcher (if active). The watcher will only be
/// stopped and the background task aborted when the last remaining instance is dropped
/// or when `stop_watching` is called on the last instance holding the active watcher.
pub struct LiveDir<T> {
	store: Arc<Store<T>>,
	loader: Arc<DynLoader>,
	path: PathBuf,
	pattern: KeyPattern,
	scan_mode: ScanMode,
	policy: UnloadPolicy,
	max_entries: Option<usize>,
	/// Keys owned by this LiveDir instance (prevents cross-deletion with shared Store).
	owned_keys: Arc<RwLock<HashSet<String>>>,
	on_error: Option<Arc<dyn Fn(LiveError) + Send + Sync>>,
	#[cfg(feature = "signal")]
	watch_state: Option<Arc<WatchState>>,
}

impl<T> Clone for LiveDir<T> {
	fn clone(&self) -> Self {
		Self {
			store: self.store.clone(),
			loader: self.loader.clone(),
			path: self.path.clone(),
			pattern: self.pattern.clone(),
			scan_mode: self.scan_mode.clone(),
			policy: self.policy,
			max_entries: self.max_entries,
			owned_keys: self.owned_keys.clone(),
			on_error: self.on_error.clone(),
			#[cfg(feature = "signal")]
			watch_state: self.watch_state.clone(),
		}
	}
}

#[cfg(feature = "signal")]
impl<T> Drop for LiveDir<T> {
	fn drop(&mut self) {
		self.stop_watching();
	}
}

/// Builder for LiveDir controller.
pub struct LiveDirBuilder<T> {
	store: Option<Arc<Store<T>>>,
	loader: Option<Arc<DynLoader>>,
	path: Option<PathBuf>,
	pattern: KeyPattern,
	scan_mode: ScanMode,
	policy: UnloadPolicy,
	max_entries: Option<usize>,
	on_error: Option<Arc<dyn Fn(LiveError) + Send + Sync>>,
}

impl<T> LiveDirBuilder<T>
where
	T: Clone + Send + Sync + DeserializeOwned + PreProcess + ValidateConfig + 'static,
{
	pub fn new() -> Self {
		Self {
			store: None,
			loader: None,
			path: None,
			pattern: KeyPattern::default(),
			scan_mode: ScanMode::default(),
			policy: UnloadPolicy::default(),
			max_entries: None,
			on_error: None,
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

	pub fn path(mut self, path: impl Into<PathBuf>) -> Self {
		self.path = Some(path.into());
		self
	}

	pub fn pattern(mut self, pattern: KeyPattern) -> Self {
		self.pattern = pattern;
		self
	}

	pub fn scan_mode(mut self, mode: ScanMode) -> Self {
		self.scan_mode = mode;
		self
	}

	pub fn policy(mut self, policy: UnloadPolicy) -> Self {
		self.policy = policy;
		self
	}

	/// Set maximum number of entries to load from the directory.
	/// If exceeded, returns an error during scan.
	pub fn max_entries(mut self, max: usize) -> Self {
		self.max_entries = Some(max);
		self
	}

	pub fn on_error<F>(mut self, f: F) -> Self
	where
		F: Fn(LiveError) + Send + Sync + 'static,
	{
		self.on_error = Some(Arc::new(f));
		self
	}

	pub fn build(self) -> Result<LiveDir<T>, LiveError> {
		let store = self
			.store
			.ok_or_else(|| LiveError::Builder("store is required".to_string()))?;
		let loader = self
			.loader
			.ok_or_else(|| LiveError::Builder("loader is required".to_string()))?;
		let path = self
			.path
			.ok_or_else(|| LiveError::Builder("path is required".to_string()))?;

		Ok(LiveDir {
			store,
			loader,
			path,
			pattern: self.pattern,
			scan_mode: self.scan_mode,
			policy: self.policy,
			max_entries: self.max_entries,
			owned_keys: Arc::new(RwLock::new(HashSet::new())),
			on_error: self.on_error,
			#[cfg(feature = "signal")]
			watch_state: None,
		})
	}
}

impl<T> Default for LiveDirBuilder<T>
where
	T: Clone + Send + Sync + DeserializeOwned + PreProcess + ValidateConfig + 'static,
{
	fn default() -> Self {
		Self::new()
	}
}

impl<T> LiveDir<T> {
	/// Stops the filesystem watcher.
	///
	/// The watcher is only stopped if this `LiveDir` instance is the last one holding
	/// the active watcher. If other clones exist, they will continue watching.
	#[cfg(feature = "signal")]
	pub fn stop_watching(&mut self) {
		if let Some(state) = self.watch_state.as_ref()
			&& Arc::strong_count(state) == 1
		{
			if let Some(state) = self.watch_state.take()
				&& let Ok(state) = Arc::try_unwrap(state)
			{
				state.watcher.stop();
				state.abort_handle.abort();
			}
		}
	}

	/// Returns true if the watcher is currently active.
	#[cfg(feature = "signal")]
	pub fn is_watching(&self) -> bool {
		self.watch_state.is_some()
	}
}

impl<T> LiveDir<T>
where
	T: Clone + Send + Sync + DeserializeOwned + PreProcess + ValidateConfig + 'static,
{
	/// Creates a new LiveDir with required parameters.
	pub fn new(store: Arc<Store<T>>, loader: DynLoader, path: impl Into<PathBuf>) -> Self {
		Self {
			store,
			loader: Arc::new(loader),
			path: path.into(),
			pattern: KeyPattern::default(),
			scan_mode: ScanMode::default(),
			policy: UnloadPolicy::default(),
			max_entries: None,
			owned_keys: Arc::new(RwLock::new(HashSet::new())),
			on_error: None,
			#[cfg(feature = "signal")]
			watch_state: None,
		}
	}

	pub fn builder() -> LiveDirBuilder<T> {
		LiveDirBuilder::new()
	}

	/// Performs an initial scan and load of all configurations in the directory.
	pub async fn load(&self) -> Result<ScanResult, LiveError> {
		self.scan_directory().await
	}

	/// Manually reloads all configurations by rescanning the directory.
	pub async fn reload(&self) -> Result<ScanResult, LiveError> {
		self.scan_directory().await
	}

	/// Gets a configuration by key.
	pub fn get(&self, key: &str) -> Option<Arc<T>> {
		self.store.get(key)
	}

	/// Returns a snapshot of all configurations managed by this LiveDir.
	pub async fn snapshot(&self) -> HashMap<String, Arc<T>> {
		let owned = self.owned_keys.read().await;
		let store_snapshot = self.store.snapshot();
		store_snapshot
			.iter()
			.filter(|(k, _)| owned.contains(*k))
			.map(|(k, entry)| (k.clone(), Arc::clone(&entry.value)))
			.collect()
	}

	/// Returns all keys managed by this LiveDir.
	pub async fn keys(&self) -> Vec<String> {
		let owned = self.owned_keys.read().await;
		owned.iter().cloned().collect()
	}

	/// Returns the number of configurations managed by this LiveDir.
	pub async fn len(&self) -> usize {
		self.owned_keys.read().await.len()
	}

	/// Returns true if no configurations are loaded by this LiveDir.
	pub async fn is_empty(&self) -> bool {
		self.owned_keys.read().await.is_empty()
	}

	/// Subscribes to store change events.
	#[cfg(feature = "events")]
	pub fn subscribe(&self) -> tokio::sync::broadcast::Receiver<atomhold::HoldEvent<T>> {
		self.store.subscribe()
	}

	/// Attaches a filesystem watcher for live reloading (borrowing version).
	///
	/// Must call `load()` before `start_watching()` to ensure the directory exists.
	#[cfg(feature = "signal")]
	pub async fn start_watching(&mut self, config: WatcherConfig) -> Result<(), LiveError> {
		let watch_path = fs::canonicalize(&self.path).await.map_err(LiveError::Io)?;

		let target = Target::Directory(watch_path);
		let watcher = Watcher::new(target, config)?;

		let mut rx = watcher.subscribe();
		let store = self.store.clone();
		let loader = self.loader.clone();
		let path = self.path.clone();
		let pattern = self.pattern.clone();
		let scan_mode = self.scan_mode.clone();
		let policy = self.policy;
		let max_entries = self.max_entries;
		let owned_keys = self.owned_keys.clone();
		let on_error = self.on_error.clone();

		let handle = tokio::spawn(async move {
			while let Ok(_event) = rx.recv().await {
				// On any change, rescan the entire directory

				if let Err(e) = Self::do_scan(
					&store,
					&loader,
					&path,
					&pattern,
					&scan_mode,
					policy,
					max_entries,
					&owned_keys,
				)
				.await && let Some(ref cb) = on_error
				{
					cb(e);
				}

				// Errors during watch are silently ignored (except via callback).

				// Use events feature to observe failures if needed.
			}
		});

		self.watch_state = Some(Arc::new(WatchState {
			watcher,
			abort_handle: handle.abort_handle(),
		}));
		Ok(())
	}

	/// Attaches a filesystem watcher for live reloading (consuming version).
	///
	/// Must call `load()` before `watch()` to ensure the directory exists.
	#[cfg(feature = "signal")]
	pub async fn watch(mut self, config: WatcherConfig) -> Result<Self, LiveError> {
		self.start_watching(config).await?;
		Ok(self)
	}

	/// Internal: Scan the directory and sync with store.
	async fn scan_directory(&self) -> Result<ScanResult, LiveError> {
		Self::do_scan(
			&self.store,
			&self.loader,
			&self.path,
			&self.pattern,
			&self.scan_mode,
			self.policy,
			self.max_entries,
			&self.owned_keys,
		)
		.await
	}

	/// Static method to perform scan (used by both load and watch).
	#[allow(clippy::too_many_arguments)]
	async fn do_scan(
		store: &Arc<Store<T>>,
		loader: &Arc<DynLoader>,
		path: &std::path::Path,
		pattern: &KeyPattern,
		scan_mode: &ScanMode,
		policy: UnloadPolicy,
		max_entries: Option<usize>,
		owned_keys: &Arc<RwLock<HashSet<String>>>,
	) -> Result<ScanResult, LiveError> {
		let mut result = ScanResult::default();

		// Check if directory exists
		if !tokio::fs::try_exists(path).await.unwrap_or(false) {
			return Ok(result);
		}

		// Collect all valid entries from filesystem
		// Store (key, load_name)
		let mut fs_entries: HashMap<String, String> = HashMap::new();

		let mut entries = fs::read_dir(path).await?;
		while let Some(entry) = entries.next_entry().await? {
			// Check max_entries limit
			if let Some(max) = max_entries
				&& fs_entries.len() >= max
			{
				return Err(LiveError::LimitExceeded(format!(
					"directory contains more than {} entries",
					max
				)));
			}

			let file_type = entry.file_type().await?;
			let file_name = entry.file_name();
			let name = file_name.to_string_lossy();

			// Skip hidden files/directories
			if name.starts_with('.') {
				continue;
			}

			match scan_mode {
				ScanMode::Files => {
					if file_type.is_file()
						&& let Some(key) = pattern.extract(&name)
					{
						// Full filename with extension
						fs_entries.insert(key, name.to_string());
					}
				}
				ScanMode::Subdirs { config_file } => {
					if file_type.is_dir()
						&& let Some(key) = pattern.extract(&name)
					{
						// Base name without extension, let loader.load() probe
						let base_name = format!("{}/{}", name, config_file);
						fs_entries.insert(key, base_name);
					}
				}
			}
		}

		// Track which keys are currently valid in the filesystem
		let mut fs_keys: HashSet<String> = HashSet::new();

		// Load all configs and insert/update in store
		for (key, load_name) in &fs_entries {
			let is_new = store.get(key).is_none();

			// Files mode: load_file (exact path)
			// Subdirs mode: load (probe extensions)
			let load_result = match scan_mode {
				ScanMode::Files => loader.load_file::<T>(load_name).await,
				ScanMode::Subdirs { .. } => loader.load::<T>(load_name).await,
			};

			match load_result {
				LoadResult::Ok { value, info } => {
					let source_path = fs::canonicalize(&info.path).await.unwrap_or(info.path);
					store.insert(key.clone(), value, source_path, policy);
					fs_keys.insert(key.clone());

					if is_new {
						result.added.push(key.clone());
					} else {
						result.updated.push(key.clone());
					}
				}
				LoadResult::Invalid(e) => {
					// Keep old value if available
					if store.get(key).is_some() {
						fs_keys.insert(key.clone());
					}
					result.failed.push((key.clone(), e.to_string()));
				}
				LoadResult::NotFound => {
					// File does not exist (Subdirs mode where subdir exists but config file missing)
					// Skip
				}
			}
		}

		// Update owned_keys and remove keys that are no longer in the filesystem
		{
			let mut owned = owned_keys.write().await;
			let old_owned: HashSet<String> = owned.clone();

			let keys_to_check: Vec<_> = old_owned.difference(&fs_keys).cloned().collect();
			for key in keys_to_check {
				match store.remove(&key) {
					Ok(_) => {
						result.removed.push(key);
					}
					Err(_) => {
						// Persistent policy prevented removal
						result.retained.push(key.clone());
						fs_keys.insert(key);
					}
				}
			}

			*owned = fs_keys;
		}

		Ok(result)
	}
}

impl<T> std::fmt::Debug for LiveDir<T>
where
	T: std::fmt::Debug,
{
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		let mut s = f.debug_struct("LiveDir");
		s.field("store", &self.store);
		s.field("loader", &self.loader);
		s.field("path", &self.path);
		s.field("pattern", &self.pattern);
		s.field("scan_mode", &self.scan_mode);
		s.field("policy", &self.policy);
		s.field("max_entries", &self.max_entries);
		#[cfg(feature = "signal")]
		s.field("watching", &self.watch_state.is_some());
		s.finish_non_exhaustive()
	}
}
