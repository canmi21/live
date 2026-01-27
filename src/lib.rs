/* src/lib.rs */

//!
//! This crate provides a complete solution for managing configuration in Rust applications:
//!
//! - **Holder**: Thread-safe, atomic configuration storage (`Store`).
//! - **Loader**: Format-agnostic loading from various sources (`Loader`, `FileSource`, `MemorySource`).
//! - **Signal**: Filesystem monitoring for live reloading (`Watcher`).
//! - **Controller**: Unified interface integrating the above (`Live`).
//!
//! ## Feature Flags
//!
//! - `full`: Enables all features.
//! - `holder`: Enables the `holder` module.
//! - `loader`: Enables the `loader` module.
//! - `signal`: Enables the `signal` module.
//! - `controller`: Enables the `Live` controller (requires `holder` + `loader`).
//! - `events`: Enables event broadcasting for `Store`.
//!
//! ## Basic Usage
//!
//! See `examples/basic.rs` for a complete example.

#[cfg(feature = "holder")]
pub mod holder;

#[cfg(feature = "loader")]
pub mod loader;

#[cfg(feature = "signal")]
pub mod signal;

#[cfg(all(feature = "controller"))]
pub mod controller;
