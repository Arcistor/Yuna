use std::path::PathBuf;
use std::time::Duration;

use anyhow::{Context, Result};
use tokio::sync::mpsc;

use crate::ai::generate_note;
use crate::ascii::ascii_for_mood;
use crate::config::Config;
use crate::detector;
use crate::haunter::{drop_note, reap_notes};
use crate::mood::update_mood;
use crate::store::Store;
use crate::types::MoodState;
use crate::watcher::FsWatcher;

pub async fn run_daemon() -> Result<()> {
    let config = Config::load()?;
    let store = open_default_store()?;
    let (sender, mut receiver) = mpsc::channel(256);
    let _watcher = FsWatcher::start(&config, sender)?;

    let reaper_store = store.clone();
    let reaper_config = config.clone();
    tokio::spawn(async move {
        let mut interval = tokio::time::interval(Duration::from_secs(300));
        loop {
            interval.tick().await;
            let now = chrono::Utc::now().timestamp();
            if let Err(error) = reap_notes(&reaper_store, &reaper_config, now).await {
                eprintln!("ghost reaper error: {error:#}");
            }
        }
    });

    while let Some(event) = receiver.recv().await {
        store.insert_event(&event.path, event.kind, event.timestamp)?;
        let now = chrono::Utc::now().timestamp();
        if store.is_silenced(now)? {
            continue;
        }

        let Some(behavior) = detector::detect(&store, &config, now)? else {
            continue;
        };
        let current = store.get_mood().unwrap_or(MoodState::Calm);
        let mood = update_mood(current, &behavior);
        store.set_mood(mood)?;
        let note = generate_note(&config, mood, &behavior).await?;
        let directory = behavior
            .directory()
            .cloned()
            .or_else(dirs::home_dir)
            .context("choose note directory")?;
        drop_note(
            &directory,
            &note,
            ascii_for_mood(&mood),
            &store,
            &behavior,
            now,
        )
        .await?;
    }

    Ok(())
}

pub fn open_default_store() -> Result<Store> {
    Store::new(&default_db_path()?)
}

pub fn default_db_path() -> Result<PathBuf> {
    Ok(dirs::home_dir()
        .context("locate home directory")?
        .join(".ghost")
        .join("ghost.db"))
}
