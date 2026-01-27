use super::super::{FmtError, Format};
use serde::de::DeserializeOwned;

/// Postcard format parser using `postcard`.
pub struct Postcard;

impl Format for Postcard {
    fn extensions(&self) -> &'static [&'static str] {
        &["bin", "post"]
    }

    fn parse<T: DeserializeOwned>(&self, input: &[u8]) -> Result<T, FmtError> {
        postcard::from_bytes(input).map_err(|e| FmtError::ParseError(e.to_string()))
    }
}
