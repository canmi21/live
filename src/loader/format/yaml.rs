use super::super::{FmtError, Format};
use serde::de::DeserializeOwned;

/// YAML format parser using `serde_yaml`.
pub struct Yaml;

impl Format for Yaml {
    fn extensions(&self) -> &'static [&'static str] {
        &["yaml", "yml"]
    }

    fn parse<T: DeserializeOwned>(&self, input: &[u8]) -> Result<T, FmtError> {
        serde_yaml::from_slice(input).map_err(|e| FmtError::ParseError(e.to_string()))
    }
}
