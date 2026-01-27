/* src/loader/loader/dyn_loader.rs */

use super::super::{
	FmtError, Format, LoadInfo, LoadResult, PreProcess, Source, ValidateConfig, format::AnyFormat,
};
use serde::de::DeserializeOwned;
use std::path::PathBuf;

pub struct DynLoader {
	source: Box<dyn Source>,
	formats: Vec<AnyFormat>,
}

pub struct DynLoaderBuilder {
	source: Option<Box<dyn Source>>,
	formats: Vec<AnyFormat>,
}

impl DynLoaderBuilder {
	pub fn new() -> Self {
		Self {
			source: None,
			formats: Vec::new(),
		}
	}

	pub fn source(mut self, source: impl Source + 'static) -> Self {
		self.source = Some(Box::new(source));
		self
	}

	pub fn format(mut self, format: AnyFormat) -> Self {
		self.formats.push(format);
		self
	}

	pub fn build(self) -> Result<DynLoader, &'static str> {
		let source = self.source.ok_or("source is required")?;
		if self.formats.is_empty() {
			return Err("at least one format is required");
		}
		Ok(DynLoader {
			source,
			formats: self.formats,
		})
	}
}

impl Default for DynLoaderBuilder {
	fn default() -> Self {
		Self::new()
	}
}

impl DynLoader {
	pub fn new(source: Box<dyn Source>, formats: Vec<AnyFormat>) -> Self {
		Self { source, formats }
	}

	pub fn builder() -> DynLoaderBuilder {
		DynLoaderBuilder::new()
	}

	/// Automatically detects and loads the configuration based on registered formats.
	pub async fn load<T>(&self, base_name: &str) -> LoadResult<T>
	where
		T: DeserializeOwned + PreProcess + ValidateConfig,
	{
		let mut found = None;

		for format in &self.formats {
			for ext in format.extensions() {
				let key = format!("{}.{}", base_name, ext);
				if self.source.exists(&key).await {
					#[cfg(feature = "logging")]
					{
						if let Some((ref first_key, _)) = found {
							log::warn!(
								"Conflict detected: multiple configuration files found for '{}'. Using '{}', ignoring '{}'.",
								base_name,
								first_key,
								key
							);
							continue;
						}
					}

					if found.is_none() {
						found = Some((key, *format));
						#[cfg(not(feature = "logging"))]
						break;
					}
				}
			}
			#[cfg(not(feature = "logging"))]
			if found.is_some() {
				break;
			}
		}

		if let Some((key, format)) = found {
			self.load_explicit(&key, &format).await
		} else {
			LoadResult::NotFound
		}
	}

	/// Directly loads a specific path, selecting parser by extension.
	pub async fn load_file<T>(&self, path: &str) -> LoadResult<T>
	where
		T: DeserializeOwned + PreProcess + ValidateConfig,
	{
		let ext = if let Some(idx) = path.rfind('.') {
			&path[idx + 1..]
		} else {
			return LoadResult::Invalid(FmtError::ParseError("missing extension".to_string()));
		};

		for format in &self.formats {
			if format.extensions().contains(&ext) {
				return self.load_explicit(path, format).await;
			}
		}
		LoadResult::NotFound
	}

	/// Dry-run mode, validates without returning data.
	#[cfg(feature = "validate")]
	pub async fn validate<T>(&self, base_name: &str) -> Result<(), FmtError>
	where
		T: DeserializeOwned + PreProcess + validator::Validate,
	{
		match self.load::<T>(base_name).await {
			LoadResult::Ok { .. } => Ok(()),
			LoadResult::Invalid(e) => Err(e),
			LoadResult::NotFound => Err(FmtError::NotFound),
		}
	}

	/// Loads the configuration using a specific key and format.
	async fn load_explicit<T>(&self, key: &str, format: &AnyFormat) -> LoadResult<T>
	where
		T: DeserializeOwned + PreProcess + ValidateConfig,
	{
		let bytes = match self.source.read(key).await {
			Ok(b) => b,
			Err(FmtError::NotFound) => return LoadResult::NotFound,
			Err(e) => return LoadResult::Invalid(e),
		};

		match format.parse::<T>(&bytes) {
			Ok(mut obj) => {
				obj.pre_process();
				obj.set_context(key);

				if let Err(e) = obj.validate_config() {
					return LoadResult::Invalid(e);
				}

				let format_name = format.extensions().first().copied().unwrap_or("unknown");

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
