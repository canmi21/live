/* tests/store_tests.rs */

#![cfg(feature = "holder")]

use live::holder::{Store, UnloadPolicy};
use std::path::PathBuf;

#[test]
fn test_store_insert_get() {
	let store = Store::<i32>::new();
	store.insert(
		"key".to_string(),
		42,
		PathBuf::from("test"),
		UnloadPolicy::default(),
	);

	assert_eq!(*store.get("key").unwrap(), 42);
}

#[test]
fn test_store_remove() {
	let store = Store::<i32>::new();
	store.insert(
		"key".to_string(),
		42,
		PathBuf::from("test"),
		UnloadPolicy::Removable,
	);

	store.remove("key").unwrap();
	assert!(store.get("key").is_none());
}

#[test]
fn test_store_persistent() {
	let store = Store::<i32>::new();
	store.insert(
		"key".to_string(),
		42,
		PathBuf::from("test"),
		UnloadPolicy::Persistent,
	);

	let err = store.remove("key").unwrap_err();
	match err {
		live::holder::HoldError::PersistentRemoval { .. } => (),
		_ => panic!("Expected PersistentRemoval error"),
	}
	assert_eq!(*store.get("key").unwrap(), 42);
}
