/* src/lib.rs */

//!
//! This crate integrates three components:
//!
//! - **holder** (`atomhold`): Thread-safe, atomic configuration storage.
//! - **loader** (`fmtstruct`): Format-agnostic loading from various sources.
//! - **signal** (`fsig`): Filesystem monitoring for live reloading.
//! - **controller**: Unified interface integrating the above (`Live`).
//!
//! ## Feature Flags
//!
//! - `full`: Enables all features.
//! - `holder`: Enables the `holder` module (re-exports `atomhold`).
//! - `loader`: Enables the `loader` module (re-exports `fmtstruct`).
//! - `signal`: Enables the `signal` module (re-exports `fsig`).
//! - `controller`: Enables the `Live` controller (requires `holder` + `loader`).
//! - `events`: Enables event broadcasting for `Store`.
//! - `fs`, `json`, `toml`, `yaml`, `postcard`: Loader format/source features.
//! - `validate`, `regex`: Validation features.
//! - `match`, `stream`: Signal features.
//!
//! ## Basic Usage
//!
//! See `examples/basic.rs` for a complete example.

#[cfg(feature = "holder")]
pub use atomhold as holder;

#[cfg(feature = "loader")]
pub use fmtstruct as loader;

#[cfg(feature = "signal")]
pub use fsig as signal;

#[cfg(feature = "controller")]
pub mod controller;
