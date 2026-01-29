/* src/controller/mod.rs */

//!
//! Controllers for live-reloading configurations.
//!
//! - [`Live`] - Single file controller
//! - [`LiveDir`] - Directory-based controller

#[cfg(feature = "signal")]
use fsig::Watcher;
#[cfg(feature = "signal")]
use tokio::task::AbortHandle;

#[cfg(feature = "signal")]
pub(crate) struct WatchState {
	pub watcher: Watcher,
	pub abort_handle: AbortHandle,
}

mod dir;
mod error;
mod live;
mod pattern;

pub use dir::{LiveDir, LiveDirBuilder};
pub use error::LiveError;
pub use live::{Live, LiveBuilder};
pub use pattern::{KeyExtractorFn, KeyPattern, ScanMode, ScanResult};
