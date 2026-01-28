/* src/controller/pattern.rs */

//!
//! Key extraction patterns and scan configuration types for directory scanning.

use std::sync::Arc;

/// Result of a directory scan operation.
#[derive(Debug, Clone, Default)]
pub struct ScanResult {
	/// Keys that were newly added.
	pub added: Vec<String>,
	/// Keys that were updated (value changed).
	pub updated: Vec<String>,
	/// Keys that failed to load (kept old value if available).
	pub failed: Vec<(String, String)>,
	/// Keys that were removed (file no longer exists).
	pub removed: Vec<String>,
	/// Keys retained due to Persistent policy.
	pub retained: Vec<String>,
}

impl ScanResult {
	/// Returns an iterator over all successfully loaded keys (both added and updated).
	pub fn loaded(&self) -> impl Iterator<Item = &String> {
		self.added.iter().chain(self.updated.iter())
	}
}

/// Custom key extractor function type.
pub type KeyExtractorFn = Arc<dyn Fn(&str) -> Option<String> + Send + Sync>;

/// Pattern for extracting keys from directory/file names.
#[derive(Clone, Default)]
pub enum KeyPattern {
	/// Use the file/directory name as-is, removing the last extension.
	///
	/// Examples:
	/// - `app.json` → `app`
	/// - `config.backup.json` → `config.backup`
	#[default]
	Identity,

	/// Try known extensions in order, strip first match. Falls back to Identity.
	///
	/// Useful for compound extensions like `.tar.gz` or `.config.json`.
	///
	/// Example:
	/// ```ignore
	/// KeyPattern::Extensions(vec![".tar.gz".into(), ".json".into()])
	/// // "data.tar.gz" → "data"
	/// // "app.json" → "app"
	/// // "unknown.xyz" → "unknown" (fallback)
	/// ```
	Extensions(Vec<String>),

	/// Extract content from brackets: `[443]` → `443`.
	Bracketed,

	/// Custom prefix/suffix stripping.
	Strip { prefix: String, suffix: String },

	/// Custom extractor function for full control.
	Custom(KeyExtractorFn),
}

impl std::fmt::Debug for KeyPattern {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		match self {
			Self::Identity => write!(f, "Identity"),
			Self::Extensions(exts) => f.debug_tuple("Extensions").field(exts).finish(),
			Self::Bracketed => write!(f, "Bracketed"),
			Self::Strip { prefix, suffix } => f
				.debug_struct("Strip")
				.field("prefix", prefix)
				.field("suffix", suffix)
				.finish(),
			Self::Custom(_) => write!(f, "Custom(<fn>)"),
		}
	}
}

impl KeyPattern {
	/// Create a custom key extractor from a function.
	pub fn custom<F>(f: F) -> Self
	where
		F: Fn(&str) -> Option<String> + Send + Sync + 'static,
	{
		Self::Custom(Arc::new(f))
	}

	/// Extract a key from a name based on the pattern.
	pub fn extract(&self, name: &str) -> Option<String> {
		match self {
			KeyPattern::Identity => Self::extract_identity(name),

			KeyPattern::Extensions(exts) => {
				// Try each known extension
				for ext in exts {
					if let Some(key) = name.strip_suffix(ext.as_str())
						&& !key.is_empty()
					{
						return Some(key.to_string());
					}
				}
				// Fall back to Identity behavior
				Self::extract_identity(name)
			}

			KeyPattern::Bracketed => {
				// Match [content], reject empty brackets
				if name.starts_with('[')
					&& let Some(end) = name.find(']')
				{
					let key = &name[1..end];
					if !key.is_empty() {
						return Some(key.to_string());
					}
				}
				None
			}

			KeyPattern::Strip { prefix, suffix } => {
				let mut s = name;
				if !prefix.is_empty() {
					s = s.strip_prefix(prefix.as_str())?;
				}
				if !suffix.is_empty() {
					s = s.strip_suffix(suffix.as_str())?;
				}
				if s.is_empty() {
					None
				} else {
					Some(s.to_string())
				}
			}

			KeyPattern::Custom(f) => f(name),
		}
	}

	/// Extract key using Identity logic (remove last extension).
	fn extract_identity(name: &str) -> Option<String> {
		let key = name.rfind('.').map(|i| &name[..i]).unwrap_or(name);
		if key.is_empty() {
			None
		} else {
			Some(key.to_string())
		}
	}
}

/// Defines what to scan within the directory.
#[derive(Debug, Clone, Default)]
pub enum ScanMode {
	/// Scan only files directly in the directory.
	#[default]
	Files,
	/// Scan subdirectories, loading a specific config file from each.
	Subdirs {
		/// The config file name to load from each subdirectory (e.g., `config.json`).
		config_file: String,
	},
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn test_key_pattern_identity() {
		let pattern = KeyPattern::Identity;
		assert_eq!(pattern.extract("config.toml"), Some("config".to_string()));
		assert_eq!(pattern.extract("app.json"), Some("app".to_string()));
		assert_eq!(pattern.extract("noext"), Some("noext".to_string()));
		assert_eq!(
			pattern.extract("multi.dot.name.json"),
			Some("multi.dot.name".to_string())
		);
		// Edge case: hidden file
		assert_eq!(pattern.extract(".hidden"), None);
	}

	#[test]
	fn test_key_pattern_extensions() {
		let pattern = KeyPattern::Extensions(vec![
			".tar.gz".into(),
			".config.json".into(),
			".json".into(),
		]);

		assert_eq!(pattern.extract("data.tar.gz"), Some("data".to_string()));
		assert_eq!(pattern.extract("app.config.json"), Some("app".to_string()));
		assert_eq!(pattern.extract("simple.json"), Some("simple".to_string()));
		// Fallback to Identity
		assert_eq!(pattern.extract("unknown.xyz"), Some("unknown".to_string()));
		// Order matters: .config.json matches before .json
		assert_eq!(pattern.extract("db.config.json"), Some("db".to_string()));
	}

	#[test]
	fn test_key_pattern_bracketed() {
		let pattern = KeyPattern::Bracketed;
		assert_eq!(pattern.extract("[443]"), Some("443".to_string()));
		assert_eq!(pattern.extract("[http]"), Some("http".to_string()));
		assert_eq!(pattern.extract("nobraces"), None);
		assert_eq!(pattern.extract("[incomplete"), None);
		// Empty brackets should return None
		assert_eq!(pattern.extract("[]"), None);
	}

	#[test]
	fn test_key_pattern_strip() {
		let pattern = KeyPattern::Strip {
			prefix: "port_".to_string(),
			suffix: "_config".to_string(),
		};
		assert_eq!(pattern.extract("port_443_config"), Some("443".to_string()));
		assert_eq!(pattern.extract("port_80_config"), Some("80".to_string()));
		assert_eq!(pattern.extract("other"), None);
	}

	#[test]
	fn test_key_pattern_custom() {
		let pattern = KeyPattern::custom(|name| {
			if name.ends_with(".special") {
				Some(name.strip_suffix(".special").unwrap().to_uppercase())
			} else {
				None
			}
		});
		assert_eq!(pattern.extract("foo.special"), Some("FOO".to_string()));
		assert_eq!(pattern.extract("bar.special"), Some("BAR".to_string()));
		assert_eq!(pattern.extract("baz.other"), None);
	}
}
