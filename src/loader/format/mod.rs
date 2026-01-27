/* src/loader/format/mod.rs */

use super::{FmtError, Format};
use serde::de::DeserializeOwned;

#[cfg(feature = "json")]
mod json;
#[cfg(feature = "json")]
pub use json::Json;

#[cfg(feature = "toml")]
mod toml;
#[cfg(feature = "toml")]
pub use toml::Toml;

#[cfg(feature = "yaml")]
mod yaml;
#[cfg(feature = "yaml")]
pub use yaml::Yaml;

#[cfg(feature = "postcard")]
mod postcard;
#[cfg(feature = "postcard")]
pub use self::postcard::Postcard;

/// An enum wrapper for all supported formats, enabling dynamic dispatch-like behavior.
#[derive(Debug, Clone, Copy)]
pub enum AnyFormat {
	#[cfg(feature = "json")]
	Json,
	#[cfg(feature = "toml")]
	Toml,
	#[cfg(feature = "yaml")]
	Yaml,
	#[cfg(feature = "postcard")]
	Postcard,
}

impl Format for AnyFormat {
	fn extensions(&self) -> &'static [&'static str] {
		match self {
			#[cfg(feature = "json")]
			Self::Json => Json.extensions(),
			#[cfg(feature = "toml")]
			Self::Toml => Toml.extensions(),
			#[cfg(feature = "yaml")]
			Self::Yaml => Yaml.extensions(),
			#[cfg(feature = "postcard")]
			Self::Postcard => Postcard.extensions(),
			#[cfg(not(any(
				feature = "json",
				feature = "toml",
				feature = "yaml",
				feature = "postcard"
			)))]
			_ => unreachable!(),
		}
	}

	fn parse<T: DeserializeOwned>(&self, _input: &[u8]) -> Result<T, FmtError> {
		match self {
			#[cfg(feature = "json")]
			Self::Json => Json.parse(_input),
			#[cfg(feature = "toml")]
			Self::Toml => Toml.parse(_input),
			#[cfg(feature = "yaml")]
			Self::Yaml => Yaml.parse(_input),
			#[cfg(feature = "postcard")]
			Self::Postcard => Postcard.parse(_input),
			#[cfg(not(any(
				feature = "json",
				feature = "toml",
				feature = "yaml",
				feature = "postcard"
			)))]
			_ => unreachable!(),
		}
	}
}
