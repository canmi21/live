use std::path::PathBuf;
use std::time::Duration;

#[cfg(feature = "signal-serde")]
use serde::{Deserialize, Serialize};

mod group;
mod target;
mod watcher;
mod worker;

pub use group::WatcherGroup;
pub use watcher::Watcher;

#[cfg(feature = "signal-stream")]
pub use watcher::EventStream;

/// Custom error type for the signal module.
#[derive(thiserror::Error, Debug)]
pub enum SignalError {
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("Notify error: {0}")]
    Notify(#[from] notify::Error),

    #[error("Invalid configuration: {0}")]
    Config(String),

    #[cfg(feature = "signal-match")]
    #[error("Glob pattern error: {0}")]
    Glob(#[from] globset::Error),
}

/// Result type alias.
pub type Result<T> = std::result::Result<T, SignalError>;

/// Defines what file system entities to monitor.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "signal-serde", derive(Serialize, Deserialize))]
pub enum Target {
    /// Monitor a single file.
    File(PathBuf),

    /// Monitor a directory recursively.
    Directory(PathBuf),

    /// Monitor a directory with glob pattern filtering.
    Filtered {
        path: PathBuf,
        /// Glob patterns to include (e.g., "*.log", "**/*.rs").
        include: Vec<String>,
        /// Glob patterns to exclude (e.g., "tmp/*", "**/.git/**").
        exclude: Vec<String>,
    },
}

/// Configuration for the watcher behavior.
#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "signal-serde", derive(Serialize, Deserialize))]
pub struct Config {
    /// Time window to debounce events.
    pub debounce: Duration,

    /// Whether to coalesce continuous events of the same file into a single event.
    pub coalesce: bool,

    /// Whether to ignore hidden files (dotfiles) automatically.
    pub ignore_hidden: bool,

    /// Specific event kinds to listen for.
    pub listen_events: Option<Vec<EventKind>>,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            debounce: Duration::from_millis(500),
            coalesce: true,
            ignore_hidden: true,
            listen_events: None,
        }
    }
}

/// The kind of filesystem event we care about.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "signal-serde", derive(Serialize, Deserialize))]
pub enum EventKind {
    /// File was created.
    Create,
    /// File content was modified.
    Modify,
    /// File was removed.
    Remove,
}

/// A simplified, high-level filesystem event.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "signal-serde", derive(Serialize, Deserialize))]
pub struct Event {
    /// The path(s) involved in the event.
    pub paths: Vec<PathBuf>,
    pub kind: EventKind,
}
