use notify::{RecommendedWatcher, RecursiveMode, Watcher as NotifyWatcher};
use tokio::sync::{broadcast, mpsc};
use tokio::task::JoinHandle;

use super::{Config, Event, Result, SignalError, Target};
use super::target::CompiledTarget;
use super::worker::process_events;

/// The main watcher struct.
pub struct Watcher {
    _internal_watcher: RecommendedWatcher,
    task_handle: JoinHandle<()>,
    event_tx: broadcast::Sender<Event>,
}

impl Watcher {
    /// Creates a new Watcher and starts monitoring immediately.
    #[must_use = "Watcher must be kept alive"]
    pub fn new(target: Target, config: Config) -> Result<Self> {
        let (raw_tx, raw_rx) = mpsc::channel(100);

        let mut internal_watcher =
            notify::recommended_watcher(move |res: notify::Result<notify::Event>| {
                let _ = raw_tx.blocking_send(res);
            })?;

        let (watch_path, mode, root_path) = match &target {
            Target::File(p) => {
                if let Some(parent) = p.parent() {
                    (
                        parent.to_path_buf(),
                        RecursiveMode::NonRecursive,
                        parent.to_path_buf(),
                    )
                } else {
                    (p.clone(), RecursiveMode::NonRecursive, p.clone())
                }
            }
            Target::Directory(p) | Target::Filtered { path: p, .. } => {
                (p.clone(), RecursiveMode::Recursive, p.clone())
            }
        };

        if !watch_path.exists() {
            return Err(SignalError::Config(format!(
                "Path does not exist: {:?}",
                watch_path
            )));
        }

        let compiled_target = CompiledTarget::new(target)?;

        internal_watcher.watch(&watch_path, mode)?;

        let (user_tx, _) = broadcast::channel(100);
        let tx_clone = user_tx.clone();
        
        let task_handle = tokio::spawn(async move {
            process_events(raw_rx, tx_clone, compiled_target, config, root_path).await;
        });

        Ok(Self {
            _internal_watcher: internal_watcher,
            task_handle,
            event_tx: user_tx,
        })
    }

    pub fn subscribe(&self) -> broadcast::Receiver<Event> {
        self.event_tx.subscribe()
    }

    pub fn stop(&self) {
        self.task_handle.abort();
    }
}

impl Drop for Watcher {
    fn drop(&mut self) {
        self.task_handle.abort();
    }
}

#[cfg(feature = "signal-stream")]
pub struct EventStream {
    inner: tokio_stream::wrappers::BroadcastStream<Event>,
}

#[cfg(feature = "signal-stream")]
impl futures_util::Stream for EventStream {
    type Item = std::result::Result<Event, tokio_stream::wrappers::errors::BroadcastStreamRecvError>;

    fn poll_next(
        mut self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Option<Self::Item>> {
        std::pin::Pin::new(&mut self.inner).poll_next(cx)
    }
}

#[cfg(feature = "signal-stream")]
impl Watcher {
    pub fn stream(&self) -> EventStream {
        EventStream {
            inner: tokio_stream::wrappers::BroadcastStream::new(self.subscribe()),
        }
    }
}
