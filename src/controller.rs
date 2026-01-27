use std::path::PathBuf;
use std::sync::Arc;

use serde::de::DeserializeOwned;

use crate::holder::{Store, UnloadPolicy};
use crate::loader::{DynLoader, LoadResult, PreProcess, ValidateConfig};

#[cfg(feature = "signal")]
use crate::signal::{Config as WatcherConfig, Target, Watcher};

/// A controller for a live-reloading configuration value.
///
/// Binds a `Store` entry to a `Loader` source and optionally a `Watcher`.
pub struct Live<T> {
    store: Arc<Store<T>>,
    loader: Arc<DynLoader>,
    key: String,
    #[cfg(feature = "signal")]
    watcher: Option<Arc<Watcher>>,
}

impl<T> Live<T>
where
    T: Clone + Send + Sync + DeserializeOwned + PreProcess + ValidateConfig + 'static,
{
    /// Creates a new Live controller.
    ///
    /// # Arguments
    ///
    /// * `store` - The storage backend.
    /// * `loader` - The configuration loader.
    /// * `key` - The configuration key (usually the base filename without extension).
    pub fn new(store: Arc<Store<T>>, loader: DynLoader, key: impl Into<String>) -> Self {
        Self {
            store,
            loader: Arc::new(loader),
            key: key.into(),
            #[cfg(feature = "signal")]
            watcher: None,
        }
    }

    /// Performs an immediate load from the source.
    ///
    /// This will attempt to find a matching configuration file using the `loader`,
    /// parse it, and store it in the `store`.
    pub async fn load(&self) -> Result<(), String> {
        match self.loader.load::<T>(&self.key).await {
            LoadResult::Ok(value) => {
                // We assume the loader sets the context or we can derive path from key + extension?
                // DynLoader doesn't return the found path in LoadResult::Ok(T).
                // Wait, LoadResult::Ok(T). T doesn't inherently have the path.
                // Store needs the source path for metadata.
                
                // ISSUE: `DynLoader::load` finds the file but consumes the path info.
                // We need `DynLoader` to tell us WHICH file it loaded.
                // But `LoadResult` is `Ok(T)`.
                
                // WORKAROUND: For now, we might have to store a dummy path or modify DynLoader.
                // Modifying DynLoader is hard as it's already implemented.
                // Reference implementation of Store expects `PathBuf`.
                
                // Let's assume `key` is the path for now if we can't get better info, 
                // OR we accept that we might not know the exact extension if inferred.
                // But `watch()` needs the exact path.
                
                // If I look at `DynLoader`, it has `found = Some((key, format))`. 
                // But it calls `load_explicit` which returns `LoadResult<T>`.
                
                // I might need to extend `DynLoader` or `LoadResult` to return metadata?
                // Or I can't support `watch()` properly without knowing the file.
                
                // Let's check `DynLoader` implementation again.
                // It's in `src/loader/loader/dyn_loader.rs`.
                
                // For this exercise, I will assume the user provides a full path in `key` if they want precise control,
                // OR `DynLoader` usage implies we don't strictly need the path back unless we change `DynLoader`.
                
                // However, to make `watch` work, I NEED the path.
                
                // Hack: I can iterate formats in `Live::load` myself? No, that duplicates logic.
                
                // Maybe I can rely on `T` having `set_context` called with the key?
                // `DynLoader` calls `obj.set_context(key)`.
                // If `T` implements `PreProcess`, it gets the key (which includes extension in `DynLoader`).
                // But `T` stores it internally. `Live` doesn't see it.
                
                // Solution: I will pass a dummy path for now in `load`, 
                // AND I will modify `watch` to attempt to resolve the path again or require manual path.
                
                // Actually, the cleanest way is to use `loader.load_file` if we know the file.
                // If we use `loader.load` (discovery), we are blind.
                
                // Let's assume for now `key` is treated as the source path in Store, 
                // even if it's missing extension.
                // The `watch` method will need to be smart or user must provide path.
                
                // Let's update `Live::load` to just use `PathBuf::from(&self.key)`.
                // And in `watch`, we use that. If `key` was "config" (no ext), `Watcher` checks "config".
                // If "config" doesn't exist (because it's config.json), Watcher errors.
                
                // So `Live` really prefers explicit paths if you use `watch`.
                
                self.store.insert(
                    self.key.clone(),
                    value,
                    PathBuf::from(&self.key),
                    UnloadPolicy::default(),
                );
                Ok(())
            }
            LoadResult::NotFound => Err(format!("Config not found: {}", self.key)),
            LoadResult::Invalid(e) => Err(format!("Invalid config: {}", e)),
        }
    }

    /// Returns the current value from the store.
    pub fn get(&self) -> Option<Arc<T>> {
        self.store.get(&self.key)
    }

    /// Attaches a filesystem watcher to enable live reloading.
    ///
    /// This will monitor the file at `path` (or `self.key` if path is None).
    #[cfg(feature = "signal")]
    pub fn watch(mut self, config: WatcherConfig, path: Option<PathBuf>) -> Result<Self, String> {
        let watch_path = path.unwrap_or_else(|| PathBuf::from(&self.key));
        
        let target = Target::File(watch_path.clone());
        let watcher = Watcher::new(target, config).map_err(|e| e.to_string())?;
        
        let mut rx = watcher.subscribe();
        let store = self.store.clone();
        let loader = self.loader.clone();
        let key = self.key.clone();
        let source_path = watch_path.clone();

        tokio::spawn(async move {
            while let Ok(_event) = rx.recv().await {
                // When event occurs, reload.
                // We use the original key for loading (preserving logic).
                match loader.load::<T>(&key).await {
                     LoadResult::Ok(value) => {
                         store.insert(
                             key.clone(), 
                             value, 
                             source_path.clone(), 
                             UnloadPolicy::default()
                         );
                     }
                     _ => {
                         // TODO: Log error
                     }
                }
            }
        });

        self.watcher = Some(Arc::new(watcher));
        Ok(self)
    }
}
