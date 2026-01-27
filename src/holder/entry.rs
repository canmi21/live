/* src/holder/entry.rs */

use std::sync::Arc;

use super::Meta;

/// A config entry containing value and metadata.
#[derive(Debug, Clone)]
pub struct Entry<T> {
	/// The config value wrapped in Arc for efficient sharing.
	pub value: Arc<T>,
	/// Metadata about this entry.
	pub meta: Meta,
}
