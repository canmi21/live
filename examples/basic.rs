/* examples/basic.rs */

use live::controller::Live;
use live::holder::Store;
use live::loader::{DynLoader, FileSource, PreProcess, format::AnyFormat};
use live::signal::Config as WatcherConfig;
use serde::Deserialize;
use std::fs;
use std::sync::Arc;
use std::time::Duration;
use validator::Validate;

#[derive(Debug, Clone, Deserialize, Validate)]
struct AppConfig {
	#[validate(length(min = 1))]
	name: String,
	port: u16,
}

impl PreProcess for AppConfig {}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
	// 0. Prepare a real file
	let config_path = "example_config.json";
	// Ensure we start fresh
	if std::path::Path::new(config_path).exists() {
		fs::remove_file(config_path)?;
	}
	fs::write(config_path, b"{\"name\": \"live-demo\", \"port\": 8080}")?;
	println!("Created {}", config_path);

	// 1. Setup Store
	let store = Arc::new(Store::<AppConfig>::new());

	// 2. Setup Loader with FileSource
	let source = FileSource::new("."); // Root is current dir
	let loader = DynLoader::builder()
		.source(source)
		.format(AnyFormat::Json)
		.build()
		.unwrap();

	// 3. Create Live Controller
	// Base name "example_config". DynLoader will find "example_config.json"
	let live = Live::new(store, loader, "example_config");

	// 4. Initial load
	live.load().await?;

	if let Some(config) = live.get() {
		println!("Initial config: {:?} (Port: {})", config, config.port);
	}

	// 5. Start watching
	let live = live.watch(WatcherConfig::default()).await?;

	println!(
		"Watching for changes on {}... (Edit the file to see updates)",
		config_path
	);
	println!("Waiting 20 seconds...");

	// Loop to display config
	for _ in 0..10 {
		tokio::time::sleep(Duration::from_secs(2)).await;
		if let Some(config) = live.get() {
			println!("Current config: {:?} (Port: {})", config, config.port);
		}
	}

	// Cleanup
	fs::remove_file(config_path)?;
	println!("Done.");
	Ok(())
}
