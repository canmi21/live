/* tests/integration.rs */

#![cfg(feature = "full")]

use live::controller::{KeyPattern, Live, LiveDir, ScanMode};
use live::holder::{Store, UnloadPolicy};
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

#[tokio::test]
async fn test_live_dir_files_mode() -> Result<(), Box<dyn std::error::Error>> {
	let dir = tempfile::tempdir()?;
	let dir_path = dir.path();

	// Create test config files
	tokio::fs::write(dir_path.join("app.json"), b"{\"val\": 1}").await?;
	tokio::fs::write(dir_path.join("db.json"), b"{\"val\": 2}").await?;
	tokio::fs::write(dir_path.join("cache.json"), b"{\"val\": 3}").await?;

	let store = Arc::new(Store::<TestConfig>::new());
	let source = FileSource::new(dir_path);
	let loader = DynLoader::builder()
		.source(source)
		.format(AnyFormat::Json)
		.build()
		.unwrap();

	let live_dir = LiveDir::builder()
		.store(store)
		.loader(loader)
		.path(dir_path)
		.pattern(KeyPattern::Identity)
		.scan_mode(ScanMode::Files)
		.build()?;

	let result = live_dir.load().await?;

	assert_eq!(result.loaded().count(), 3);
	assert!(result.failed.is_empty());
	assert!(result.removed.is_empty());

	assert_eq!(live_dir.get("app").unwrap().val, 1);
	assert_eq!(live_dir.get("db").unwrap().val, 2);
	assert_eq!(live_dir.get("cache").unwrap().val, 3);

	let keys = live_dir.keys().await;
	assert_eq!(keys.len(), 3);
	assert!(keys.contains(&"app".to_string()));
	assert!(keys.contains(&"db".to_string()));
	assert!(keys.contains(&"cache".to_string()));

	Ok(())
}

#[tokio::test]
async fn test_live_dir_subdirs_mode() -> Result<(), Box<dyn std::error::Error>> {
	let dir = tempfile::tempdir()?;
	let dir_path = dir.path();

	// Create subdirectory structure like listener/[443]/config.json
	tokio::fs::create_dir(dir_path.join("[443]")).await?;
	tokio::fs::create_dir(dir_path.join("[80]")).await?;
	tokio::fs::create_dir(dir_path.join("[8080]")).await?;

	tokio::fs::write(
		dir_path.join("[443]").join("config.json"),
		b"{\"val\": 443}",
	)
	.await?;
	tokio::fs::write(dir_path.join("[80]").join("config.json"), b"{\"val\": 80}").await?;
	tokio::fs::write(
		dir_path.join("[8080]").join("config.json"),
		b"{\"val\": 8080}",
	)
	.await?;

	let store = Arc::new(Store::<TestConfig>::new());
	let source = FileSource::new(dir_path);
	let loader = DynLoader::builder()
		.source(source)
		.format(AnyFormat::Json)
		.build()
		.unwrap();

	let live_dir = LiveDir::builder()
		.store(store)
		.loader(loader)
		.path(dir_path)
		.pattern(KeyPattern::Bracketed)
		.scan_mode(ScanMode::Subdirs {
			config_file: "config.json".to_string(),
		})
		.build()?;

	let result = live_dir.load().await?;

	assert_eq!(result.loaded().count(), 3);

	assert_eq!(live_dir.get("443").unwrap().val, 443);
	assert_eq!(live_dir.get("80").unwrap().val, 80);
	assert_eq!(live_dir.get("8080").unwrap().val, 8080);

	Ok(())
}

#[tokio::test]
async fn test_live_dir_reload_add_remove() -> Result<(), Box<dyn std::error::Error>> {
	let dir = tempfile::tempdir()?;
	let dir_path = dir.path();

	// Initial files
	tokio::fs::write(dir_path.join("a.json"), b"{\"val\": 1}").await?;
	tokio::fs::write(dir_path.join("b.json"), b"{\"val\": 2}").await?;

	let store = Arc::new(Store::<TestConfig>::new());
	let source = FileSource::new(dir_path);
	let loader = DynLoader::builder()
		.source(source)
		.format(AnyFormat::Json)
		.build()
		.unwrap();

	let live_dir = LiveDir::builder()
		.store(store)
		.loader(loader)
		.path(dir_path)
		.build()?;

	live_dir.load().await?;
	assert_eq!(live_dir.len().await, 2);

	// Add a new file
	tokio::fs::write(dir_path.join("c.json"), b"{\"val\": 3}").await?;
	let result = live_dir.reload().await?;

	assert!(result.added.contains(&"c".to_string()));
	assert_eq!(live_dir.len().await, 3);
	assert_eq!(live_dir.get("c").unwrap().val, 3);

	// Remove a file
	tokio::fs::remove_file(dir_path.join("a.json")).await?;
	let result = live_dir.reload().await?;

	assert!(result.removed.contains(&"a".to_string()));
	assert_eq!(live_dir.len().await, 2);
	assert!(live_dir.get("a").is_none());

	Ok(())
}

#[tokio::test]
async fn test_live_dir_invalid_keeps_old() -> Result<(), Box<dyn std::error::Error>> {
	let dir = tempfile::tempdir()?;
	let dir_path = dir.path();

	// Create valid config
	tokio::fs::write(dir_path.join("app.json"), b"{\"val\": 42}").await?;

	let store = Arc::new(Store::<TestConfig>::new());
	let source = FileSource::new(dir_path);
	let loader = DynLoader::builder()
		.source(source)
		.format(AnyFormat::Json)
		.build()
		.unwrap();

	let live_dir = LiveDir::builder()
		.store(store)
		.loader(loader)
		.path(dir_path)
		.build()?;

	live_dir.load().await?;
	assert_eq!(live_dir.get("app").unwrap().val, 42);

	// Write invalid JSON
	tokio::fs::write(dir_path.join("app.json"), b"invalid json").await?;
	let result = live_dir.reload().await?;

	// Should fail but keep old value
	assert_eq!(result.failed.len(), 1);
	assert_eq!(result.failed[0].0, "app");
	assert_eq!(live_dir.get("app").unwrap().val, 42);

	Ok(())
}

#[tokio::test]
async fn test_live_dir_watch() -> Result<(), Box<dyn std::error::Error>> {
	let dir = tempfile::tempdir()?;
	let dir_path = dir.path().to_path_buf();

	// Create initial config
	tokio::fs::write(dir_path.join("test.json"), b"{\"val\": 1}").await?;

	let store = Arc::new(Store::<TestConfig>::new());
	let source = FileSource::new(&dir_path);
	let loader = DynLoader::builder()
		.source(source)
		.format(AnyFormat::Json)
		.build()
		.unwrap();

	let live_dir = LiveDir::builder()
		.store(store)
		.loader(loader)
		.path(&dir_path)
		.build()?;

	live_dir.load().await?;
	assert_eq!(live_dir.get("test").unwrap().val, 1);

	// Start watching
	let mut live_dir = live_dir.watch(live::signal::Config::default()).await?;

	// Modify the file
	tokio::fs::write(dir_path.join("test.json"), b"{\"val\": 100}").await?;

	// Wait for reload
	for _ in 0..50 {
		tokio::time::sleep(Duration::from_millis(100)).await;
		if live_dir.get("test").unwrap().val == 100 {
			break;
		}
	}

	assert_eq!(live_dir.get("test").unwrap().val, 100);

	// Add a new file while watching
	tokio::fs::write(dir_path.join("new.json"), b"{\"val\": 999}").await?;

	// Wait for detection
	for _ in 0..50 {
		tokio::time::sleep(Duration::from_millis(100)).await;
		if live_dir.get("new").is_some() {
			break;
		}
	}

	assert_eq!(live_dir.get("new").unwrap().val, 999);

	live_dir.stop_watching();

	Ok(())
}

#[tokio::test]
async fn test_live_dir_persistent_policy() -> Result<(), Box<dyn std::error::Error>> {
	let dir = tempfile::tempdir()?;
	let dir_path = dir.path();

	// Create test files
	tokio::fs::write(dir_path.join("persistent.json"), b"{\"val\": 1}").await?;
	tokio::fs::write(dir_path.join("removable.json"), b"{\"val\": 2}").await?;

	let store = Arc::new(Store::<TestConfig>::new());
	let source = FileSource::new(dir_path);
	let loader = DynLoader::builder()
		.source(source)
		.format(AnyFormat::Json)
		.build()
		.unwrap();

	// Create with Persistent policy
	let live_dir = LiveDir::builder()
		.store(store)
		.loader(loader)
		.path(dir_path)
		.policy(UnloadPolicy::Persistent)
		.build()?;

	live_dir.load().await?;
	assert_eq!(live_dir.len().await, 2);

	// Delete both files
	tokio::fs::remove_file(dir_path.join("persistent.json")).await?;
	tokio::fs::remove_file(dir_path.join("removable.json")).await?;

	let result = live_dir.reload().await?;

	// Both should be retained due to Persistent policy
	assert_eq!(result.retained.len(), 2);
	assert!(result.removed.is_empty());

	// Values should still be accessible
	assert_eq!(live_dir.get("persistent").unwrap().val, 1);
	assert_eq!(live_dir.get("removable").unwrap().val, 2);
	assert_eq!(live_dir.len().await, 2);

	Ok(())
}
