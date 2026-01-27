/* src/loader/source/memory.rs */

use super::super::{FmtError, Source};
use async_trait::async_trait;
use std::collections::BTreeMap;

/// A simple in-memory source useful for testing and embedded environments.
#[derive(Default)]
pub struct MemorySource {
	data: BTreeMap<String, Vec<u8>>,
}

impl MemorySource {
	/// Creates a new empty MemorySource.
	pub fn new() -> Self {
		Self::default()
	}

	/// Inserts data into the source.
	pub fn insert(&mut self, key: &str, value: Vec<u8>) {
		self.data.insert(key.to_string(), value);
	}
}

#[async_trait]
impl Source for MemorySource {
	async fn read(&self, key: &str) -> Result<Vec<u8>, FmtError> {
		self.data.get(key).cloned().ok_or(FmtError::NotFound)
	}

	async fn exists(&self, key: &str) -> bool {
		self.data.contains_key(key)
	}
}
