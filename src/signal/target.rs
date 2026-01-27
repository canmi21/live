/* src/signal/target.rs */

use std::path::{Path, PathBuf};

use super::{Config, Result, Target};

#[derive(Debug)]
pub(crate) enum CompiledTarget {
	File(PathBuf),
	Directory(#[allow(dead_code)] PathBuf),
	Filtered {
		#[allow(dead_code)]
		path: PathBuf,
		#[cfg(feature = "match")]
		include: Option<globset::GlobSet>,
		#[cfg(feature = "match")]
		exclude: globset::GlobSet,
	},
}

impl CompiledTarget {
	pub(crate) fn new(target: Target) -> Result<Self> {
		match target {
			Target::File(p) => Ok(CompiledTarget::File(p)),
			Target::Directory(p) => Ok(CompiledTarget::Directory(p)),
			Target::Filtered {
				path,
				include,
				exclude,
			} => {
				#[cfg(feature = "match")]
				{
					let include_set = if include.is_empty() {
						None
					} else {
						let mut inc_builder = globset::GlobSetBuilder::new();
						for p in include {
							inc_builder.add(globset::Glob::new(&p)?);
						}
						Some(inc_builder.build()?)
					};

					let mut exc_builder = globset::GlobSetBuilder::new();
					for p in exclude {
						exc_builder.add(globset::Glob::new(&p)?);
					}
					let exclude_set = exc_builder.build()?;

					Ok(CompiledTarget::Filtered {
						path,
						include: include_set,
						exclude: exclude_set,
					})
				}
				#[cfg(not(feature = "match"))]
				{
					let _ = (include, exclude);
					Ok(CompiledTarget::Filtered { path })
				}
			}
		}
	}

	pub(crate) fn matches(&self, path: &Path, config: &Config, root: &Path) -> bool {
		let relative_path = path.strip_prefix(root).unwrap_or(path);

		if config.ignore_hidden {
			for component in relative_path.components() {
				if component
					.as_os_str()
					.to_str()
					.is_some_and(|s| s.starts_with('.') && s != "." && s != "..")
				{
					return false;
				}
			}
		}

		match self {
			CompiledTarget::File(target_path) => relative_path == target_path,
			CompiledTarget::Directory(_) => true,
			CompiledTarget::Filtered {
				#[cfg(feature = "match")]
				include,
				#[cfg(feature = "match")]
				exclude,
				..
			} => {
				#[cfg(feature = "match")]
				{
					if exclude.is_match(relative_path) {
						return false;
					}

					match include {
						Some(set) => set.is_match(relative_path),
						None => true,
					}
				}
				#[cfg(not(feature = "match"))]
				{
					true
				}
			}
		}
	}
}
