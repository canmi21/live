/* src/loader/source/mod.rs */

mod memory;
pub use memory::MemorySource;

#[cfg(feature = "fs")]
mod file;
#[cfg(feature = "fs")]
pub use file::FileSource;
