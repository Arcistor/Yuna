use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use notify::{
    Config as NotifyConfig, Event, EventKind as NotifyEventKind, RecommendedWatcher, RecursiveMode,
    Watcher,
};
use tokio::sync::mpsc;

use crate::config::Config;
use crate::types::{EventKind, GhostEvent};

pub struct FsWatcher {
    _watcher: RecommendedWatcher,
}

impl FsWatcher {
    pub fn start(config: &Config, sender: mpsc::Sender<GhostEvent>) -> Result<Self> {
        let excludes = config.watch.exclude.clone();
        let mut watcher = RecommendedWatcher::new(
            move |result: notify::Result<Event>| {
                let Ok(event) = result else {
                    return;
                };
                let Some(kind) = map_event_kind(&event.kind) else {
                    return;
                };
                let now = chrono::Utc::now().timestamp();
                for path in event.paths {
                    if is_excluded(&path, &excludes) {
                        continue;
                    }
                    let _ = sender.blocking_send(GhostEvent {
                        path,
                        kind,
                        timestamp: now,
                    });
                }
            },
            NotifyConfig::default(),
        )
        .context("create filesystem watcher")?;

        for path in &config.watch.paths {
            watcher
                .watch(path, RecursiveMode::Recursive)
                .with_context(|| format!("watch {}", path.display()))?;
        }

        Ok(Self { _watcher: watcher })
    }
}

fn map_event_kind(kind: &NotifyEventKind) -> Option<EventKind> {
    match kind {
        NotifyEventKind::Create(_) => Some(EventKind::Create),
        NotifyEventKind::Modify(notify::event::ModifyKind::Name(_)) => Some(EventKind::Rename),
        NotifyEventKind::Modify(_) => Some(EventKind::Modify),
        NotifyEventKind::Remove(_) => Some(EventKind::Delete),
        _ => None,
    }
}

fn is_excluded(path: &Path, excludes: &[PathBuf]) -> bool {
    if excludes.iter().any(|exclude| path.starts_with(exclude)) {
        return true;
    }

    if let Some(filename) = path.file_name().and_then(|f| f.to_str()) {
        let ghost_patterns = [
            ".ghost_note",
            "MESSAGE_FROM_THE_VOID.txt",
            ".thankyou",
            "DO_NOT_READ_ME.txt",
        ];
        return ghost_patterns.iter().any(|p| filename.starts_with(p));
    }

    false
}
