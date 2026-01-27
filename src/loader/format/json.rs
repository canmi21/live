/* src/loader/format/json.rs */

use super::super::{FmtError, Format};
use serde::de::DeserializeOwned;

/// JSON format parser using `serde_json`.
pub struct Json;

impl Format for Json {
	fn extensions(&self) -> &'static [&'static str] {
		&["json"]
	}

	fn parse<T: DeserializeOwned>(&self, input: &[u8]) -> Result<T, FmtError> {
		serde_json::from_slice(input).map_err(|e| FmtError::ParseError(e.to_string()))
	}
}
