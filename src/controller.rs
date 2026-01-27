use std::sync::Arc;

use serde::de::DeserializeOwned;

use crate::holder::{Store, UnloadPolicy};
use crate::loader::{DynLoader, LoadResult, PreProcess, ValidateConfig};

#[cfg(feature = "signal")]
use crate::signal::{Config as WatcherConfig, Target, Watcher};

#[cfg(feature = "logging")]
use log::{error, info, warn};

/// A controller for a live-reloading configuration value.
pub struct Live<T> {
    store: Arc<Store<T>>,
    loader: Arc<DynLoader>,
    key: String,
    #[cfg(feature = "signal")]
    watcher: Option<Arc<Watcher>>,
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

    pub fn build(self) -> Result<Live<T>, &'static str> {
        let store = self.store.ok_or("store is required")?;
        let loader = self.loader.ok_or("loader is required")?;
        let key = self.key.ok_or("key is required")?;
        
        Ok(Live {
            store,
            loader,
            key,
            #[cfg(feature = "signal")]
            watcher: None,
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
        }
    }

    /// Performs an immediate load from the source.
    pub async fn load(&self) -> Result<(), String> {
        match self.loader.load::<T>(&self.key).await {
            LoadResult::Ok { value, info } => {
                #[cfg(feature = "logging")]
                info!("Loaded config '{}' from {:?}", self.key, info.path);
                
                self.store.insert(
                    self.key.clone(),
                    value,
                    info.path,
                    UnloadPolicy::default(),
                );
                Ok(())
            }
            LoadResult::NotFound => {
                let msg = format!("Config not found: {}", self.key);
                #[cfg(feature = "logging")]
                warn!("{}", msg);
                Err(msg)
            }
            LoadResult::Invalid(e) => {
                let msg = format!("Invalid config: {}", e);
                #[cfg(feature = "logging")]
                error!("{}", msg);
                Err(msg)
            }
        }
    }
    
    /// Manually reloads the configuration.
    pub async fn reload(&self) -> Result<(), String> {
        self.load().await
    }

    pub fn get(&self) -> Option<Arc<T>> {
        self.store.get(&self.key)
    }

    #[cfg(feature = "signal")]
    pub fn watch(mut self, config: WatcherConfig) -> Result<Self, String> {
        // Retrieve the source path from the store metadata.
        let meta = self.store.get_meta(&self.key).ok_or("Config not loaded yet. Call load() before watch().")?;
        let watch_path = meta.source;
        
        let target = Target::File(watch_path.clone());
        let watcher = Watcher::new(target, config).map_err(|e| e.to_string())?;
        
        let mut rx = watcher.subscribe();
        let store = self.store.clone();
        let loader = self.loader.clone();
        let key = self.key.clone();
        let _source_path = watch_path.clone();

        tokio::spawn(async move {
            #[cfg(feature = "logging")]
            info!("Started watching config '{}' at {:?}", key, _source_path);

            while let Ok(_event) = rx.recv().await {
                 match loader.load::<T>(&key).await {
                     LoadResult::Ok { value, info } => {
                         store.insert(
                             key.clone(), 
                             value, 
                             info.path, 
                             UnloadPolicy::default()
                         );
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

        self.watcher = Some(Arc::new(watcher));
        Ok(self)
    }
}