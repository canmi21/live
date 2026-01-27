use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

use tokio::sync::broadcast;
use tokio::sync::mpsc;

use super::target::CompiledTarget;
use super::{Config, Event, EventKind};

struct DebounceState {
    last_seen: Instant,
    kind: EventKind,
}

pub(crate) async fn process_events(
    mut raw_rx: mpsc::Receiver<notify::Result<notify::Event>>,
    user_tx: broadcast::Sender<Event>,
    target: CompiledTarget,
    config: Config,
    root_path: PathBuf,
) {
    let mut pending: HashMap<PathBuf, DebounceState> = HashMap::new();

    let tick_rate = if config.debounce < Duration::from_millis(50) {
        config.debounce
    } else {
        config.debounce / 5
    };

    let mut interval = tokio::time::interval(tick_rate);

    loop {
        tokio::select! {
            maybe_event = raw_rx.recv() => {
                match maybe_event {
                    Some(Ok(event)) => {
                        handle_raw_event(event, &mut pending, &target, &config, &root_path);
                    }
                    Some(Err(e)) => tracing::error!("Notify error: {:?}", e),
                    None => break,
                }
            }
            _ = interval.tick() => {
                flush_pending(&mut pending, &user_tx, &config).await;
            }
        }
    }
}

fn handle_raw_event(
    event: notify::Event,
    pending: &mut HashMap<PathBuf, DebounceState>,
    target: &CompiledTarget,
    config: &Config,
    root_path: &Path,
) {
    use notify::EventKind as NK;
    let kind = match event.kind {
        NK::Create(_) => EventKind::Create,
        NK::Modify(_) => EventKind::Modify,
        NK::Remove(_) => EventKind::Remove,
        _ => return,
    };

    for path in event.paths {
        if !target.matches(&path, config, root_path) {
            continue;
        }

        pending
            .entry(path.clone())
            .and_modify(|state| {
                let now = Instant::now();
                state.last_seen = now;

                let prev_kind = state.kind;

                if !config.coalesce {
                    state.kind = kind;
                    return;
                }

                match (prev_kind, kind) {
                    (EventKind::Create, EventKind::Modify) => { /* Keep Create */ }
                    (EventKind::Create, EventKind::Remove) => {
                        state.kind = EventKind::Remove;
                    }
                    (EventKind::Modify, EventKind::Remove) => {
                        state.kind = EventKind::Remove;
                    }
                    (EventKind::Remove, EventKind::Modify) => {
                        // Ignore noise
                    }
                    _ => {
                        state.kind = kind;
                    }
                }
            })
            .or_insert(DebounceState {
                last_seen: Instant::now(),
                kind,
            });
    }
}

async fn flush_pending(
    pending: &mut HashMap<PathBuf, DebounceState>,
    tx: &broadcast::Sender<Event>,
    config: &Config,
) {
    let now = Instant::now();
    let mut to_remove = Vec::new();

    for (path, state) in pending.iter() {
        if now.duration_since(state.last_seen) >= config.debounce {
            let kind = state.kind;
            let allowed = match &config.listen_events {
                None => true,
                Some(list) => list.contains(&kind),
            };

            if allowed {
                let event = Event {
                    paths: vec![path.clone()],
                    kind,
                };
                let _ = tx.send(event);
            }
            to_remove.push(path.clone());
        }
    }

    for path in to_remove {
        pending.remove(&path);
    }
}
