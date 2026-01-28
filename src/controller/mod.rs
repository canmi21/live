/* src/controller/mod.rs */

//!
//! Controllers for live-reloading configurations.
//!
//! - [`Live`] - Single file controller
//! - [`LiveDir`] - Directory-based controller

mod dir;
mod error;
mod live;
mod pattern;

pub use dir::{LiveDir, LiveDirBuilder};
pub use error::LiveError;
pub use live::{Live, LiveBuilder};
pub use pattern::{KeyExtractorFn, KeyPattern, ScanMode, ScanResult};
