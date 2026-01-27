/* tests/integration.rs */

use live::controller::Live;
use live::holder::Store;
use live::loader::{DynLoader, FileSource, PreProcess, format::AnyFormat};
use serde::Deserialize;
use std::sync::Arc;
use std::time::Duration;

#[derive(Debug, Clone, Deserialize, PartialEq, validator::Validate)]
struct TestConfig {
	val: i32,
}

impl PreProcess for TestConfig {}

#[tokio::test]
async fn test_live_reload() -> Result<(), Box<dyn std::error::Error>> {
	let filename = "test_integration.json";

	// Cleanup from previous runs
	if std::path::Path::new(filename).exists() {
		let _ = tokio::fs::remove_file(filename).await;
	}

	// Write initial
	tokio::fs::write(filename, b"{\"val\": 1}").await?;

	let store = Arc::new(Store::<TestConfig>::new());
	// Use current directory so Watcher and Loader agree on path
	let source = FileSource::new(".");
	let loader = DynLoader::builder()
		.source(source)
		.format(AnyFormat::Json)
		.build()
		.unwrap();

	let live = Live::new(store, loader, "test_integration");
	live.load().await?;

	assert_eq!(live.get().unwrap().val, 1);

	// Watch
	let live = live.watch(live::signal::Config::default()).await?;

	// Update file
	tokio::fs::write(filename, b"{\"val\": 2}").await?;

	// Wait for reload
	for _ in 0..50 {
		// 5 seconds max
		tokio::time::sleep(Duration::from_millis(100)).await;
		if live.get().unwrap().val == 2 {
			break;
		}
	}

	let val = live.get().unwrap().val;

	// Cleanup
	let _ = tokio::fs::remove_file(filename).await;

	assert_eq!(val, 2);

	Ok(())
}
