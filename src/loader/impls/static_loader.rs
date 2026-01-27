/* src/loader/loader/static_loader.rs */

use super::super::{FmtError, Format, LoadInfo, LoadResult, PreProcess, Source, ValidateConfig};
use serde::de::DeserializeOwned;
use std::path::PathBuf;

/// A zero-cost loader that combines a specific Source and Format at compile time.
pub struct StaticLoader<S, F> {
	pub source: S,
	pub format: F,
}

impl<S, F> StaticLoader<S, F>
where
	S: Source,
	F: Format,
{
	/// Creates a new StaticLoader.
	pub const fn new(source: S, format: F) -> Self {
		Self { source, format }
	}

	/// Loads and parses the configuration.
	pub async fn load<T>(&self, key: &str) -> LoadResult<T>
	where
		T: DeserializeOwned + PreProcess + ValidateConfig,
	{
		let bytes = match self.source.read(key).await {
			Ok(b) => b,
			Err(FmtError::NotFound) => return LoadResult::NotFound,
			Err(e) => return LoadResult::Invalid(e),
		};

		match self.format.parse::<T>(&bytes) {
			Ok(mut obj) => {
				obj.pre_process();
				obj.set_context(key);

				if let Err(e) = obj.validate_config() {
					return LoadResult::Invalid(e);
				}

				let format_name = self
					.format
					.extensions()
					.first()
					.copied()
					.unwrap_or("unknown");

				LoadResult::Ok {
					value: obj,
					info: LoadInfo {
						path: PathBuf::from(key),
						format: format_name,
					},
				}
			}
			Err(e) => LoadResult::Invalid(e),
		}
	}
}
