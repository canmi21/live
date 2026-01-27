use std::time::Duration;
use serde::Deserialize;
use validator::Validate;
use live::controller::Live;
use live::holder::Store;
use live::loader::{DynLoader, MemorySource, format::AnyFormat, PreProcess};
use live::signal::Config as WatcherConfig;

#[derive(Debug, Clone, Deserialize, Validate)]
struct AppConfig {
    #[validate(length(min = 1))]
    name: String,
    port: u16,
}

impl PreProcess for AppConfig {}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // 1. Setup Store
    let store = std::sync::Arc::new(Store::<AppConfig>::new());

    // 2. Setup Loader with an In-Memory source for this example
    // In real usage, you'd use FileSource.
    let mut source = MemorySource::new();
    source.insert("app.json", b"{\"name\": \"live-demo\", \"port\": 8080}".to_vec());

    let loader = DynLoader::builder()
        .source(source)
        .format(AnyFormat::Json)
        .build()
        .unwrap();

    // 3. Create Live Controller
    // We use "app" as base name, DynLoader will find "app.json"
    let live = Live::new(store, loader, "app");

    // 4. Initial load
    live.load().await.map_err(|e| e)?;

    if let Some(config) = live.get() {
        println!("Initial config: {:?}", config);
    }

    // 5. Start watching (using a dummy path for memory source demonstration)
    // In real life: live.watch(WatcherConfig::default(), Some(PathBuf::from("app.json")))?;
    let live = live.watch(WatcherConfig::default(), None)?;

    println!("Watching for changes... (Press Ctrl+C to stop)");
    
    // Simulate some wait
    tokio::time::sleep(Duration::from_secs(1)).await;
    
    if let Some(config) = live.get() {
        println!("Current config: {:?}", config);
    }

    Ok(())
}
