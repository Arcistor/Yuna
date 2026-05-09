use std::fs;
use std::path::{Path, PathBuf};
use std::time::UNIX_EPOCH;

use anyhow::{Context, Result};
use rand::seq::SliceRandom;

use crate::config::Config;
use crate::store::Store;
use crate::types::Behavior;

pub async fn drop_note(
    directory: &Path,
    content: &str,
    ascii: &str,
    store: &Store,
    behavior: &Behavior,
    created: i64,
) -> Result<PathBuf> {
    tokio::fs::create_dir_all(directory)
        .await
        .with_context(|| format!("create note directory {}", directory.display()))?;

    let existing = store.list_undeleted_notes()?;
    let mut dir_notes: Vec<_> = existing.iter()
        .filter(|n| !n.deleted && n.path.parent() == Some(directory))
        .collect();
    if dir_notes.len() >= 3 {
        dir_notes.sort_by_key(|n| n.created);
        for old in &dir_notes[..dir_notes.len() - 2] {
            tokio::fs::remove_file(&old.path).await.ok();
            store.mark_note_deleted(old.id)?;
        }
    }

    let path = choose_note_path(directory);
    let note = format!("{ascii}\n\n{content}\n\n                              - still watching\n");
    tokio::fs::write(&path, note)
        .await
        .with_context(|| format!("write note {}", path.display()))?;
    store.insert_note(&path, behavior.trigger_name(), created)?;
    Ok(path)
}

pub async fn reap_notes(store: &Store, config: &Config, now: i64) -> Result<()> {
    let lifetime = (config.behavior.note_lifetime_minutes as i64) * 60;
    for note in store.list_undeleted_notes()? {
        if !note.path.exists() {
            store.mark_note_deleted(note.id)?;
            continue;
        }

        let read_at = match note.read_at {
            Some(read_at) => Some(read_at),
            None => accessed_at(&note.path).filter(|accessed| *accessed > note.created),
        };

        if let Some(read_at) = read_at {
            if note.read_at.is_none() {
                store.mark_note_read(note.id, read_at)?;
            }
            if now - read_at >= lifetime {
                tokio::fs::remove_file(&note.path).await.ok();
                store.mark_note_deleted(note.id)?;
            }
        } else if now - note.created >= lifetime * 2 {
            tokio::fs::remove_file(&note.path).await.ok();
            store.mark_note_deleted(note.id)?;
        }
    }
    Ok(())
}

fn choose_note_path(directory: &Path) -> PathBuf {
    let mut rng = rand::thread_rng();
    let names = [
        ".ghost_note",
        "MESSAGE_FROM_THE_VOID.txt",
        ".thankyou",
        "DO_NOT_READ_ME.txt",
    ];
    let name = if rand::random::<f32>() < 0.7 {
        ".ghost_note"
    } else {
        names.choose(&mut rng).copied().unwrap_or(".ghost_note")
    };
    let mut path = directory.join(name);
    if !path.exists() {
        return path;
    }
    for index in 1..100 {
        path = directory.join(format!("{name}.{index}"));
        if !path.exists() {
            break;
        }
    }
    path
}

fn accessed_at(path: &Path) -> Option<i64> {
    fs::metadata(path)
        .ok()?
        .accessed()
        .ok()?
        .duration_since(UNIX_EPOCH)
        .ok()
        .map(|duration| duration.as_secs() as i64)
}
