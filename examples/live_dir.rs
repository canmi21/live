/* examples/live_dir.rs */

//! Example: Directory-based configuration with LiveDir
//!
//! This example demonstrates:
//! - Scanning a directory for configuration files
//! - Pattern-based key extraction (bracketed ports like [443])
//! - Live reloading when files change
//!
//! Run with: cargo run --example live_dir --features full

use live::controller::{KeyPattern, LiveDir, ScanMode};
use live::holder::Store;
use live::loader::{DynLoader, FileSource, PreProcess, format::AnyFormat};
use live::signal::Config as WatcherConfig;
use serde::Deserialize;
use std::fs;
use std::sync::Arc;
use std::time::Duration;
use validator::Validate;

#[derive(Debug, Clone, Deserialize, Validate)]
struct ListenerConfig {
	#[validate(length(min = 1))]
	protocol: String,
	bind: String,
	#[serde(default)]
	tls: bool,
}

impl PreProcess for ListenerConfig {}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
	// 0. Prepare test directory structure
	let listener_dir = "example_listeners";

	// Cleanup from previous runs
	if std::path::Path::new(listener_dir).exists() {
		fs::remove_dir_all(listener_dir)?;
	}

	// Create listener directories with bracketed port names
	fs::create_dir_all(format!("{}/[443]", listener_dir))?;
	fs::create_dir_all(format!("{}/[80]", listener_dir))?;
	fs::create_dir_all(format!("{}/[8080]", listener_dir))?;

	// Write config files
	fs::write(
		format!("{}/[443]/config.json", listener_dir),
		r#"{"protocol": "https", "bind": "0.0.0.0:443", "tls": true}"#,
	)?;
	fs::write(
		format!("{}/[80]/config.json", listener_dir),
		r#"{"protocol": "http", "bind": "0.0.0.0:80", "tls": false}"#,
	)?;
	fs::write(
		format!("{}/[8080]/config.json", listener_dir),
		r#"{"protocol": "http", "bind": "127.0.0.1:8080", "tls": false}"#,
	)?;

	println!("Created listener configs in {}/", listener_dir);

	// 1. Setup Store
	let store = Arc::new(Store::<ListenerConfig>::new());

	// 2. Setup Loader with FileSource rooted at listener_dir
	let source = FileSource::new(listener_dir);
	let loader = DynLoader::builder()
		.source(source)
		.format(AnyFormat::Json)
		.build()
		.unwrap();

	// 3. Create LiveDir Controller
	let live_dir = LiveDir::builder()
		.store(store)
		.loader(loader)
		.path(listener_dir)
		.pattern(KeyPattern::Bracketed) // Extract port from [443] -> "443"
		.scan_mode(ScanMode::Subdirs {
			config_file: "config.json".to_string(),
		})
		.build()?;

	// 4. Initial load
	let result = live_dir.load().await?;
	println!("\nLoaded {} listeners:", result.loaded().count());

	for key in live_dir.keys().await {
		if let Some(config) = live_dir.get(&key) {
			println!(
				"  Port {}: {} @ {} (TLS: {})",
				key, config.protocol, config.bind, config.tls
			);
		}
	}

	// 5. Start watching
	let mut live_dir = live_dir.watch(WatcherConfig::default()).await?;

	println!(
		"\nWatching for changes... (Edit files in {}/ to see updates)",
		listener_dir
	);
	println!("Waiting 20 seconds...\n");

	// Loop to display config changes
	for i in 0..10 {
		tokio::time::sleep(Duration::from_secs(2)).await;
		println!("--- Check {} ---", i + 1);
		for key in live_dir.keys().await {
			if let Some(config) = live_dir.get(&key) {
				println!("  Port {}: {} @ {}", key, config.protocol, config.bind);
			}
		}
	}

	// Cleanup
	live_dir.stop_watching();
	fs::remove_dir_all(listener_dir)?;
	println!("\nDone. Cleaned up {}.", listener_dir);

	Ok(())
}
