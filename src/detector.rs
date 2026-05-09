use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};

use anyhow::Result;

use crate::config::Config;
use crate::store::Store;
use crate::types::{Behavior, EventKind};

pub fn detect(store: &Store, config: &Config, now: i64) -> Result<Option<Behavior>> {
    if let Some(behavior) = detect_cleaning(store, config, now)? {
        return Ok(Some(behavior));
    }
    if let Some(behavior) = detect_midnight_worker(store, config, now)? {
        return Ok(Some(behavior));
    }
    if let Some(behavior) = detect_procrastinator(store, config, now)? {
        return Ok(Some(behavior));
    }
    detect_typo_repeater(config, None)
}

pub fn detect_cleaning(store: &Store, config: &Config, now: i64) -> Result<Option<Behavior>> {
    if in_cooldown(store, config, "cleaning", now)? {
        return Ok(None);
    }
    let events = store.query_events(now - 600, Some(EventKind::Delete))?;
    let mut counts: HashMap<PathBuf, u32> = HashMap::new();
    for event in events {
        let directory = event
            .path
            .parent()
            .map(Path::to_path_buf)
            .unwrap_or(event.path);
        *counts.entry(directory).or_default() += 1;
    }

    Ok(counts
        .into_iter()
        .filter(|(_, count)| *count > 10)
        .max_by_key(|(_, count)| *count)
        .map(|(directory, delete_count)| Behavior::Cleaning {
            directory,
            delete_count,
        }))
}

pub fn detect_procrastinator(store: &Store, config: &Config, now: i64) -> Result<Option<Behavior>> {
    if in_cooldown(store, config, "procrastinator", now)? {
        return Ok(None);
    }
    let cutoff = now - 3 * 24 * 60 * 60;
    let events = store.query_events(0, Some(EventKind::Create))?;

    for event in events.into_iter().filter(|event| event.timestamp <= cutoff) {
        let Some(name) = event.path.file_name().and_then(|name| name.to_str()) else {
            continue;
        };
        if !name.to_ascii_lowercase().contains("new") {
            continue;
        }

        let later = store
            .query_events(event.timestamp + 1, None)?
            .into_iter()
            .any(|candidate| candidate.path.starts_with(&event.path));
        if !later {
            return Ok(Some(Behavior::Procrastinator {
                directory: event.path,
                days_idle: ((now - event.timestamp) / (24 * 60 * 60)) as u32,
            }));
        }
    }

    Ok(None)
}

pub fn detect_midnight_worker(
    store: &Store,
    config: &Config,
    now: i64,
) -> Result<Option<Behavior>> {
    if in_cooldown(store, config, "midnight_worker", now)? {
        return Ok(None);
    }

    let day_start = now - (now % 86_400);
    let events = store.query_events(day_start, Some(EventKind::Modify))?;
    let mut first_by_dir: HashMap<PathBuf, i64> = HashMap::new();
    let mut last_by_dir: HashMap<PathBuf, i64> = HashMap::new();

    for event in events {
        if !is_code_file(&event.path) {
            continue;
        }
        let directory = event
            .path
            .parent()
            .map(Path::to_path_buf)
            .unwrap_or_else(|| PathBuf::from("."));
        first_by_dir
            .entry(directory.clone())
            .and_modify(|time| *time = (*time).min(event.timestamp))
            .or_insert(event.timestamp);
        last_by_dir
            .entry(directory)
            .and_modify(|time| *time = (*time).max(event.timestamp))
            .or_insert(event.timestamp);
    }

    for (directory, first) in first_by_dir {
        let Some(last) = last_by_dir.get(&directory) else {
            continue;
        };
        let hours = (*last - first) as f32 / 3600.0;
        if hours >= 4.0 {
            return Ok(Some(Behavior::MidnightWorker { directory, hours }));
        }
    }

    Ok(None)
}

pub fn detect_typo_repeater(
    _config: &Config,
    history_path: Option<&Path>,
) -> Result<Option<Behavior>> {
    let path = match history_path {
        Some(path) => path.to_path_buf(),
        None => default_history_path(),
    };
    let Ok(content) = fs::read_to_string(path) else {
        return Ok(None);
    };

    let lines: Vec<String> = content
        .lines()
        .rev()
        .take(50)
        .map(normalize_history_line)
        .filter(|line| !line.is_empty())
        .collect();
    let mut counts: HashMap<String, u32> = HashMap::new();
    for line in lines {
        if looks_like_typo(&line) {
            *counts.entry(line).or_default() += 1;
        }
    }

    Ok(counts
        .into_iter()
        .find(|(_, count)| *count >= 3)
        .map(|(command, count)| Behavior::TypoRepeater { command, count }))
}

fn in_cooldown(store: &Store, config: &Config, trigger: &str, now: i64) -> Result<bool> {
    let since = now - config.limits.cooldown_hours * 60 * 60;
    store.recent_note_exists(trigger, since)
}

fn is_code_file(path: &Path) -> bool {
    matches!(
        path.extension().and_then(|ext| ext.to_str()),
        Some("rs" | "py" | "js" | "ts" | "cpp" | "c" | "h" | "go")
    )
}

fn default_history_path() -> PathBuf {
    let home = dirs::home_dir().unwrap_or_else(|| PathBuf::from("."));
    let zsh = home.join(".zsh_history");
    if zsh.exists() {
        zsh
    } else {
        home.join(".bash_history")
    }
}

fn normalize_history_line(line: &str) -> String {
    if let Some((_, command)) = line.rsplit_once(';') {
        command.trim().to_string()
    } else {
        line.trim().to_string()
    }
}

fn looks_like_typo(command: &str) -> bool {
    let first = command.split_whitespace().next().unwrap_or_default();
    matches!(first, "gti" | "sl" | "pyhton" | "pnpmn" | "nmp")
}
