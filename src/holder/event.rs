/* src/holder/event.rs */

use std::sync::Arc;

use super::{HoldError, Meta};

/// Events emitted by the store on config changes.
#[derive(Debug, Clone)]
pub enum HoldEvent<T> {
	/// A new config was loaded.
	Loaded {
		key: String,
		value: Arc<T>,
		meta: Meta,
	},
	/// An existing config was updated.
	Updated {
		key: String,
		old: Arc<T>,
		new: Arc<T>,
		meta: Meta,
	},
	/// A config was removed.
	Removed { key: String, value: Arc<T> },
	/// A removal was rejected due to Persistent policy.
	Retained { key: String, error: HoldError },
}
