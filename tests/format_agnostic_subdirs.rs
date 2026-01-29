/* tests/format_agnostic_subdirs.rs */

#![cfg(feature = "full")]

use live::controller::{KeyPattern, LiveDir, ScanMode};
use live::holder::Store;
use live::loader::{DynLoader, FileSource, PreProcess, format::AnyFormat};
use serde::Deserialize;
use std::sync::Arc;

#[derive(Debug, Clone, Deserialize, PartialEq, validator::Validate)]
struct TestConfig {
	val: i32,
}

impl PreProcess for TestConfig {}

#[tokio::test]
async fn test_subdirs_format_agnostic() -> Result<(), Box<dyn std::error::Error>> {
	let dir = tempfile::tempdir()?;
	let dir_path = dir.path();

	// Create subdirectory structure
	// [tcp]/config.json
	// [udp]/config.toml
	tokio::fs::create_dir(dir_path.join("[tcp]")).await?;
	tokio::fs::create_dir(dir_path.join("[udp]")).await?;

	tokio::fs::write(dir_path.join("[tcp]").join("config.json"), b"{\"val\": 1}").await?;

	tokio::fs::write(dir_path.join("[udp]").join("config.toml"), b"val = 2").await?;

	let store = Arc::new(Store::<TestConfig>::new());
	let source = FileSource::new(dir_path);
	let loader = DynLoader::builder()
		.source(source)
		.format(AnyFormat::Json)
		.format(AnyFormat::Toml)
		.build()
		.unwrap();

	// Use "config" without extension
	let live_dir = LiveDir::builder()
		.store(store)
		.loader(loader)
		.path(dir_path)
		.pattern(KeyPattern::Bracketed)
		.scan_mode(ScanMode::Subdirs {
			config_file: "config".to_string(),
		})
		.build()?;

	let result = live_dir.load().await?;

	assert_eq!(result.loaded().count(), 2);
	assert!(result.failed.is_empty());

	assert_eq!(live_dir.get("tcp").unwrap().val, 1);
	assert_eq!(live_dir.get("udp").unwrap().val, 2);

	Ok(())
}
