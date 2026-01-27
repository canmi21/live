/* src/loader/loader/mod.rs */

mod static_loader;
pub use static_loader::StaticLoader;

mod dyn_loader;
pub use dyn_loader::DynLoader;
