/* src/loader/format/toml.rs */

use super::super::{FmtError, Format};
use serde::de::DeserializeOwned;

/// TOML format parser using `toml`.
pub struct Toml;

impl Format for Toml {
	fn extensions(&self) -> &'static [&'static str] {
		&["toml"]
	}

	fn parse<T: DeserializeOwned>(&self, input: &[u8]) -> Result<T, FmtError> {
		let s = std::str::from_utf8(input).map_err(|e| FmtError::ParseError(e.to_string()))?;
		toml::from_str(s).map_err(|e| FmtError::ParseError(e.to_string()))
	}
}
