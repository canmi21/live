/* src/holder/store/mod.rs */

mod read;
mod replace;
mod sync;
mod write;

pub use sync::SyncResult;

use std::collections::HashMap;
use std::sync::atomic::AtomicU64;

use arc_swap::ArcSwap;

use super::Entry;
#[cfg(feature = "events")]
use super::HoldEvent;

/// Default event channel capacity.
pub const DEFAULT_EVENT_CAPACITY: usize = 100;

/// Thread-safe config store with atomic replacement support.
///
/// Uses RCU (Read-Copy-Update) pattern for lock-free reads and atomic updates.
/// Requires `T: Clone + Send + Sync` for safe concurrent access across threads.
pub struct Store<T> {
	pub(crate) inner: ArcSwap<HashMap<String, Entry<T>>>,
	pub(crate) version: AtomicU64,
	#[cfg(feature = "events")]
	pub(crate) events: tokio::sync::broadcast::Sender<HoldEvent<T>>,
}

impl<T> Store<T>
where
	T: Clone + Send + Sync,
{
	/// Creates a new empty store with default event channel capacity.
	pub fn new() -> Self {
		Self {
			inner: ArcSwap::from_pointee(HashMap::new()),
			version: AtomicU64::new(0),
			#[cfg(feature = "events")]
			events: tokio::sync::broadcast::channel(DEFAULT_EVENT_CAPACITY).0,
		}
	}

	/// Creates a new empty store with custom event channel capacity.
	///
	/// Note: Events may be dropped if subscribers process slower than
	/// the write rate and the channel fills up.
	#[cfg(feature = "events")]
	pub fn with_event_capacity(capacity: usize) -> Self {
		Self {
			inner: ArcSwap::from_pointee(HashMap::new()),
			version: AtomicU64::new(0),
			events: tokio::sync::broadcast::channel(capacity).0,
		}
	}
}

impl<T> Default for Store<T>
where
	T: Clone + Send + Sync,
{
	fn default() -> Self {
		Self::new()
	}
}
