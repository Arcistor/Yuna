use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};

use anyhow::Result;
use chrono::{Datelike, Local, TimeZone, Timelike};

use crate::config::Config;
use crate::store::Store;
use crate::types::{Behavior, EventKind, TimeSlot};

pub fn detect(
    store: &Store,
    config: &Config,
    now: i64,
    event_path: &Path,
    event_kind: EventKind,
) -> Result<Option<Behavior>> {
    if let Some(b) = detect_note_reply(store, config, event_path, event_kind, now)? {
        return Ok(Some(b));
    }
    if let Some(b) = detect_holiday(store, config, now)? {
        return Ok(Some(b));
    }
    if let Some(b) = detect_time_greeting(store, config, now)? {
        return Ok(Some(b));
    }
    if let Some(b) = detect_cleaning(store, config, now)? {
        return Ok(Some(b));
    }
    if let Some(b) = detect_fresh_start(store, config, now)? {
        return Ok(Some(b));
    }
    if let Some(b) = detect_yuna_missing(store, config, now)? {
        return Ok(Some(b));
    }
    if let Some(b) = detect_midnight_worker(store, config, now)? {
        return Ok(Some(b));
    }
    if let Some(b) = detect_night_owl(store, config, now)? {
        return Ok(Some(b));
    }
    if let Some(b) = detect_weekend_warrior(store, config, now)? {
        return Ok(Some(b));
    }
    if let Some(b) = detect_deadline_mode(store, config, now)? {
        return Ok(Some(b));
    }
    if let Some(b) = detect_hoarder(store, config, now)? {
        return Ok(Some(b));
    }
    if let Some(b) = detect_archaeologist(store, config, now)? {
        return Ok(Some(b));
    }
    if let Some(b) = detect_yuna_commit(store, config, now)? {
        return Ok(Some(b));
    }
    if let Some(b) = detect_revert_spiral(store, config, now)? {
        return Ok(Some(b));
    }
    if let Some(b) = detect_duplicator(store, config, now)? {
        return Ok(Some(b));
    }
    if let Some(b) = detect_empty_nest(store, config, now)? {
        return Ok(Some(b));
    }
    if let Some(b) = detect_procrastinator(store, config, now)? {
        return Ok(Some(b));
    }
    if in_cooldown(store, config, "typo_repeater", now)? {
        return Ok(None);
    }
    detect_typo_repeater(config, None)
}

pub fn detect_cleaning(store: &Store, config: &Config, now: i64) -> Result<Option<Behavior>> {
    if in_cooldown(store, config, "cleaning", now)? {
        return Ok(None);
    }
    let delete_events = store.query_events(now - 600, Some(EventKind::Delete))?;
    let rename_events = store.query_events(now - 600, Some(EventKind::Rename))?;
    let mut counts: HashMap<PathBuf, u32> = HashMap::new();
    for event in delete_events {
        let directory = event
            .path
            .parent()
            .map(Path::to_path_buf)
            .unwrap_or(event.path);
        *counts.entry(directory).or_default() += 1;
    }
    let mut seen_renames: std::collections::HashSet<PathBuf> = std::collections::HashSet::new();
    for event in rename_events {
        if !event.path.exists() && seen_renames.insert(event.path.clone()) {
            let directory = event
                .path
                .parent()
                .map(Path::to_path_buf)
                .unwrap_or(event.path);
            *counts.entry(directory).or_default() += 1;
        }
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

    let day_start = local_midnight_timestamp(now);
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
    let content = match fs::read(&path) {
        Ok(bytes) => String::from_utf8_lossy(&bytes).into_owned(),
        Err(_) => return Ok(None),
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
    let since = now - config.limits.cooldown_seconds;
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
    if first.is_empty() {
        return false;
    }

    // If the command already exists, it's not a typo
    if is_valid_command(first) {
        return false;
    }

    if matches!(first, "gti" | "sl" | "pyhton" | "pnpmn" | "nmp") {
        return true;
    }

    common_commands()
        .iter()
        .any(|valid| first != *valid && edit_distance_at_most_one(first, valid))
}

fn is_valid_command(command: &str) -> bool {
    std::process::Command::new("which")
        .arg(command)
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .status()
        .map(|s| s.success())
        .unwrap_or(false)
}

fn common_commands() -> &'static [&'static str] {
    &[
        "git", "npm", "pnpm", "yarn", "python", "python3", "cargo", "make", "docker", "kubectl",
        "node",
    ]
}

fn edit_distance_at_most_one(left: &str, right: &str) -> bool {
    let left_chars: Vec<char> = left.chars().collect();
    let right_chars: Vec<char> = right.chars().collect();
    let len_diff = left_chars.len().abs_diff(right_chars.len());
    if len_diff > 1 {
        return false;
    }

    if left_chars.len() == right_chars.len() {
        return left_chars
            .iter()
            .zip(right_chars.iter())
            .filter(|(left, right)| left != right)
            .count()
            <= 1;
    }

    let (shorter, longer) = if left_chars.len() < right_chars.len() {
        (&left_chars, &right_chars)
    } else {
        (&right_chars, &left_chars)
    };

    let mut short_index = 0;
    let mut long_index = 0;
    let mut edits = 0;
    while short_index < shorter.len() && long_index < longer.len() {
        if shorter[short_index] == longer[long_index] {
            short_index += 1;
            long_index += 1;
        } else {
            edits += 1;
            long_index += 1;
            if edits > 1 {
                return false;
            }
        }
    }
    true
}

fn local_midnight_timestamp(now: i64) -> i64 {
    let Some(now_local) = Local.timestamp_opt(now, 0).single() else {
        return now - (now % 86_400);
    };
    let Some(midnight) = now_local.date_naive().and_hms_opt(0, 0, 0) else {
        return now - (now % 86_400);
    };
    midnight
        .and_local_timezone(Local)
        .single()
        .map(|value| value.timestamp())
        .unwrap_or_else(|| now - (now % 86_400))
}

pub fn detect_hoarder(store: &Store, config: &Config, now: i64) -> Result<Option<Behavior>> {
    if in_cooldown(store, config, "hoarder", now)? {
        return Ok(None);
    }
    let day_start = local_midnight_timestamp(now);
    let events = store.query_events(day_start, Some(EventKind::Modify))?;
    let mut counts: HashMap<PathBuf, u32> = HashMap::new();
    for event in events {
        *counts.entry(event.path).or_default() += 1;
    }
    let best = counts
        .into_iter()
        .filter(|(_, c)| *c > 200)
        .max_by_key(|(_, c)| *c);
    let Some((path, modify_count)) = best else {
        return Ok(None);
    };
    let directory = path
        .parent()
        .map(Path::to_path_buf)
        .unwrap_or_else(|| PathBuf::from("."));
    let filename = path
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("unknown")
        .to_string();
    if is_in_git_repo(&directory) && has_uncommitted_changes(&directory) {
        return Ok(Some(Behavior::Hoarder {
            directory,
            filename,
            modify_count,
        }));
    }
    Ok(None)
}

pub fn detect_archaeologist(store: &Store, config: &Config, now: i64) -> Result<Option<Behavior>> {
    if in_cooldown(store, config, "archaeologist", now)? {
        return Ok(None);
    }
    let six_months = 6 * 30 * 24 * 3600_i64;
    let recent_events = store.query_events(now - 3600, Some(EventKind::Modify))?;
    for event in recent_events {
        let older = store
            .query_events(0, Some(EventKind::Modify))?
            .into_iter()
            .filter(|e| e.path == event.path && e.timestamp < now - six_months)
            .max_by_key(|e| e.timestamp);
        let Some(old_event) = older else { continue };
        let months_dormant = ((now - old_event.timestamp) / (30 * 24 * 3600)) as u32;
        let directory = event
            .path
            .parent()
            .map(Path::to_path_buf)
            .unwrap_or_else(|| PathBuf::from("."));
        let filename = event
            .path
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("unknown")
            .to_string();
        return Ok(Some(Behavior::Archaeologist {
            directory,
            filename,
            months_dormant,
        }));
    }
    Ok(None)
}

pub fn detect_empty_nest(store: &Store, config: &Config, now: i64) -> Result<Option<Behavior>> {
    if in_cooldown(store, config, "empty_nest", now)? {
        return Ok(None);
    }
    let seven_days = 7 * 24 * 3600_i64;
    let create_events = store.query_events(0, Some(EventKind::Create))?;
    for event in create_events {
        if event.timestamp > now - seven_days {
            continue;
        }
        if !event.path.is_dir() {
            continue;
        }
        let is_empty = event
            .path
            .read_dir()
            .map(|mut d| d.next().is_none())
            .unwrap_or(false);
        if !is_empty {
            continue;
        }
        let has_later = store
            .query_events(event.timestamp + 1, None)?
            .into_iter()
            .any(|e| e.path.starts_with(&event.path));
        if !has_later {
            let days_empty = ((now - event.timestamp) / (24 * 3600)) as u32;
            return Ok(Some(Behavior::EmptyNest {
                directory: event.path,
                days_empty,
            }));
        }
    }
    Ok(None)
}

pub fn detect_duplicator(store: &Store, config: &Config, now: i64) -> Result<Option<Behavior>> {
    if in_cooldown(store, config, "duplicator", now)? {
        return Ok(None);
    }
    let events = store.query_events(now - 600, Some(EventKind::Create))?;
    let mut by_dir: HashMap<PathBuf, Vec<String>> = HashMap::new();
    for event in events {
        let Some(parent) = event.path.parent().map(Path::to_path_buf) else {
            continue;
        };
        let Some(name) = event.path.file_stem().and_then(|n| n.to_str()) else {
            continue;
        };
        by_dir.entry(parent).or_default().push(name.to_string());
    }
    for (directory, names) in by_dir {
        let mut base_counts: HashMap<String, u32> = HashMap::new();
        for name in &names {
            let base = name
                .trim_end_matches(|c: char| c.is_numeric() || c == '_' || c == '-')
                .to_string();
            if !base.is_empty() {
                *base_counts.entry(base).or_default() += 1;
            }
        }
        if let Some((base_name, count)) = base_counts
            .into_iter()
            .filter(|(_, c)| *c >= 3)
            .max_by_key(|(_, c)| *c)
        {
            return Ok(Some(Behavior::Duplicator {
                directory,
                base_name,
                count,
            }));
        }
    }
    Ok(None)
}

pub fn detect_yuna_commit(store: &Store, config: &Config, now: i64) -> Result<Option<Behavior>> {
    if in_cooldown(store, config, "yuna_commit", now)? {
        return Ok(None);
    }
    let five_days = 5 * 24 * 3600_i64;
    let events = store.query_events(now - five_days, Some(EventKind::Modify))?;
    let mut dirs: std::collections::HashSet<PathBuf> = std::collections::HashSet::new();
    for event in events {
        if let Some(parent) = event.path.parent().map(Path::to_path_buf) {
            dirs.insert(parent);
        }
    }
    for directory in dirs {
        if !is_in_git_repo(&directory) {
            continue;
        }
        let last_commit_age = git_last_commit_age_seconds(&directory);
        if last_commit_age >= five_days {
            let days_uncommitted = (last_commit_age / (24 * 3600)) as u32;
            return Ok(Some(Behavior::YunaCommit {
                directory,
                days_uncommitted,
            }));
        }
    }
    Ok(None)
}

pub fn detect_revert_spiral(store: &Store, config: &Config, now: i64) -> Result<Option<Behavior>> {
    if in_cooldown(store, config, "revert_spiral", now)? {
        return Ok(None);
    }
    let hour_ago = now - 3600;
    let events = store.query_events(hour_ago, Some(EventKind::Modify))?;
    let mut counts: HashMap<PathBuf, u32> = HashMap::new();
    for event in &events {
        *counts.entry(event.path.clone()).or_default() += 1;
    }
    let best = counts
        .into_iter()
        .filter(|(_, c)| *c >= 20)
        .max_by_key(|(_, c)| *c);
    let Some((path, revert_count)) = best else {
        return Ok(None);
    };
    let directory = path
        .parent()
        .map(Path::to_path_buf)
        .unwrap_or_else(|| PathBuf::from("."));
    let filename = path
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("unknown")
        .to_string();
    Ok(Some(Behavior::RevertSpiral {
        directory,
        filename,
        revert_count,
    }))
}

pub fn detect_alias_candidate(
    _config: &Config,
    history_path: Option<&Path>,
) -> Result<Option<Behavior>> {
    let path = match history_path {
        Some(p) => p.to_path_buf(),
        None => default_history_path(),
    };
    let content = match fs::read(&path) {
        Ok(bytes) => String::from_utf8_lossy(&bytes).into_owned(),
        Err(_) => return Ok(None),
    };
    let lines: Vec<String> = content
        .lines()
        .rev()
        .take(200)
        .map(normalize_history_line)
        .filter(|l| l.len() > 30 && !l.is_empty())
        .collect();
    let mut counts: HashMap<String, u32> = HashMap::new();
    for line in lines {
        if !looks_like_typo(&line) {
            *counts.entry(line).or_default() += 1;
        }
    }
    Ok(counts
        .into_iter()
        .filter(|(_, c)| *c >= 5)
        .max_by_key(|(_, c)| *c)
        .map(|(command, count)| Behavior::AliasCandidate { command, count }))
}

pub fn detect_night_owl(store: &Store, config: &Config, now: i64) -> Result<Option<Behavior>> {
    if in_cooldown(store, config, "night_owl", now)? {
        return Ok(None);
    }
    let now_local = match Local.timestamp_opt(now, 0).single() {
        Some(t) => t,
        None => return Ok(None),
    };
    let hour = now_local
        .format("%H")
        .to_string()
        .parse::<u32>()
        .unwrap_or(12);
    if !(2..=5).contains(&hour) {
        return Ok(None);
    }
    let events = store.query_events(now - 900, Some(EventKind::Modify))?;
    let dirs: std::collections::HashSet<PathBuf> = events
        .into_iter()
        .filter_map(|e| e.path.parent().map(Path::to_path_buf))
        .collect();
    if let Some(directory) = dirs.into_iter().next() {
        return Ok(Some(Behavior::NightOwl { directory, hour }));
    }
    Ok(None)
}

pub fn detect_weekend_warrior(
    store: &Store,
    config: &Config,
    now: i64,
) -> Result<Option<Behavior>> {
    if in_cooldown(store, config, "weekend_warrior", now)? {
        return Ok(None);
    }
    let now_local = match Local.timestamp_opt(now, 0).single() {
        Some(t) => t,
        None => return Ok(None),
    };
    let weekday = now_local
        .format("%u")
        .to_string()
        .parse::<u32>()
        .unwrap_or(1);
    if weekday < 6 {
        return Ok(None);
    }
    let day_start = local_midnight_timestamp(now);
    let events = store.query_events(day_start, Some(EventKind::Modify))?;
    let mut first_by_dir: HashMap<PathBuf, i64> = HashMap::new();
    let mut last_by_dir: HashMap<PathBuf, i64> = HashMap::new();
    for event in events {
        if !is_code_file(&event.path) {
            continue;
        }
        let dir = event
            .path
            .parent()
            .map(Path::to_path_buf)
            .unwrap_or_else(|| PathBuf::from("."));
        first_by_dir
            .entry(dir.clone())
            .and_modify(|t| *t = (*t).min(event.timestamp))
            .or_insert(event.timestamp);
        last_by_dir
            .entry(dir)
            .and_modify(|t| *t = (*t).max(event.timestamp))
            .or_insert(event.timestamp);
    }
    for (directory, first) in first_by_dir {
        let Some(last) = last_by_dir.get(&directory) else {
            continue;
        };
        let hours = (*last - first) as f32 / 3600.0;
        if hours >= 3.0 {
            return Ok(Some(Behavior::WeekendWarrior { directory, hours }));
        }
    }
    Ok(None)
}

pub fn detect_deadline_mode(store: &Store, config: &Config, now: i64) -> Result<Option<Behavior>> {
    if in_cooldown(store, config, "deadline_mode", now)? {
        return Ok(None);
    }
    let today_events = store.query_events(now - 86400, None)?.len() as f32;
    let week_events = store.query_events(now - 7 * 86400, None)?.len() as f32;
    let daily_avg = week_events / 7.0;
    if daily_avg < 50.0 {
        return Ok(None);
    }
    let multiplier = today_events / daily_avg;
    if multiplier < 3.0 {
        return Ok(None);
    }
    let recent = store.query_events(now - 3600, Some(EventKind::Modify))?;
    let dirs: std::collections::HashSet<PathBuf> = recent
        .into_iter()
        .filter_map(|e| e.path.parent().map(Path::to_path_buf))
        .collect();
    if let Some(directory) = dirs.into_iter().next() {
        return Ok(Some(Behavior::DeadlineMode {
            directory,
            multiplier,
        }));
    }
    Ok(None)
}

pub fn detect_yuna_missing(store: &Store, config: &Config, now: i64) -> Result<Option<Behavior>> {
    if in_cooldown(store, config, "yuna_missing", now)? {
        return Ok(None);
    }
    let last = store.last_event_time()?;
    let Some(last_ts) = last else { return Ok(None) };
    let days_absent = ((now - last_ts) / (24 * 3600)) as u32;
    if days_absent >= 3 {
        return Ok(Some(Behavior::YunaMissing { days_absent }));
    }
    Ok(None)
}

pub fn detect_fresh_start(store: &Store, config: &Config, now: i64) -> Result<Option<Behavior>> {
    if in_cooldown(store, config, "fresh_start", now)? {
        return Ok(None);
    }
    let recent = store.query_events(now - 300, None)?;
    if recent.is_empty() {
        return Ok(None);
    }
    let prev = store.query_events(0, None)?;
    let before_gap: Vec<_> = prev
        .iter()
        .filter(|e| e.timestamp < now - 3 * 24 * 3600)
        .collect();
    let Some(last_before) = before_gap.iter().max_by_key(|e| e.timestamp) else {
        return Ok(None);
    };
    let days_absent = ((now - last_before.timestamp) / (24 * 3600)) as u32;
    if days_absent >= 3 {
        return Ok(Some(Behavior::FreshStart { days_absent }));
    }
    Ok(None)
}

fn is_in_git_repo(directory: &Path) -> bool {
    let mut path = directory.to_path_buf();
    loop {
        if path.join(".git").exists() {
            return true;
        }
        if !path.pop() {
            return false;
        }
    }
}

fn has_uncommitted_changes(directory: &Path) -> bool {
    std::process::Command::new("git")
        .args(["status", "--porcelain"])
        .current_dir(directory)
        .output()
        .map(|o| !o.stdout.is_empty())
        .unwrap_or(false)
}

fn git_last_commit_age_seconds(directory: &Path) -> i64 {
    let out = std::process::Command::new("git")
        .args(["log", "-1", "--format=%ct"])
        .current_dir(directory)
        .output();
    let Ok(out) = out else { return i64::MAX };
    let ts: i64 = String::from_utf8_lossy(&out.stdout)
        .trim()
        .parse()
        .unwrap_or(0);
    if ts == 0 {
        return i64::MAX;
    }
    chrono::Utc::now().timestamp() - ts
}

pub fn detect_time_greeting(store: &Store, _config: &Config, now: i64) -> Result<Option<Behavior>> {
    let now_local = match Local.timestamp_opt(now, 0).single() {
        Some(t) => t,
        None => return Ok(None),
    };
    let hour = now_local.hour();
    let slot = if (5..12).contains(&hour) {
        TimeSlot::Morning
    } else if (12..17).contains(&hour) {
        TimeSlot::Afternoon
    } else if (17..21).contains(&hour) {
        TimeSlot::Evening
    } else {
        TimeSlot::Night
    };

    let trigger = match slot {
        TimeSlot::Morning => "time_greeting_morning",
        TimeSlot::Afternoon => "time_greeting_afternoon",
        TimeSlot::Evening => "time_greeting_evening",
        TimeSlot::Night => "time_greeting_night",
    };

    let day_start = local_midnight_timestamp(now);
    if store.recent_note_exists(trigger, day_start)? {
        return Ok(None);
    }

    Ok(Some(Behavior::TimeOfDayGreeting { slot }))
}

pub fn detect_holiday(store: &Store, _config: &Config, now: i64) -> Result<Option<Behavior>> {
    let now_local = match Local.timestamp_opt(now, 0).single() {
        Some(t) => t,
        None => return Ok(None),
    };

    let month = now_local.month();
    let day = now_local.day();

    let holiday_name = match (month, day) {
        (1, 1) => "New Year's Day",
        (2, 14) => "Valentine's Day",
        (4, 13) | (4, 14) | (4, 15) => "Songkran Festival",
        (10, 31) => "Halloween",
        (12, 25) => "Christmas",
        (12, 31) => "New Year's Eve",
        _ => return Ok(None),
    };

    let trigger = format!("holiday_event_{}_{}", month, day);
    let day_start = local_midnight_timestamp(now);
    if store.recent_note_exists(&trigger, day_start)? {
        return Ok(None);
    }

    Ok(Some(Behavior::HolidayEvent {
        holiday_name: holiday_name.to_string(),
    }))
}

pub fn detect_frustration(
    store: &Store,
    _config: &Config,
    history_path: Option<&Path>,
) -> Result<Option<Behavior>> {
    let path = match history_path {
        Some(path) => path.to_path_buf(),
        None => default_history_path(),
    };
    let content = match fs::read(&path) {
        Ok(bytes) => String::from_utf8_lossy(&bytes).into_owned(),
        Err(_) => return Ok(None),
    };

    let lines: Vec<String> = content
        .lines()
        .rev()
        .take(15)
        .map(normalize_history_line)
        .filter(|line| !line.is_empty())
        .collect();

    if lines.len() < 5 {
        return Ok(None);
    }

    let mut counts: HashMap<String, u32> = HashMap::new();
    for line in &lines {
        *counts.entry(line.clone()).or_default() += 1;
    }

    if let Some((command, count)) = counts.into_iter().find(|(_, c)| *c >= 5) {
        let now = chrono::Utc::now().timestamp();
        if store.recent_note_exists("frustration", now - 3600)? {
            return Ok(None);
        }
        return Ok(Some(Behavior::Frustration { command, count }));
    }

    Ok(None)
}

pub fn detect_deep_alias(
    store: &Store,
    _config: &Config,
    history_path: Option<&Path>,
) -> Result<Option<Behavior>> {
    let path = match history_path {
        Some(p) => p.to_path_buf(),
        None => default_history_path(),
    };
    let content = match fs::read(&path) {
        Ok(bytes) => String::from_utf8_lossy(&bytes).into_owned(),
        Err(_) => return Ok(None),
    };
    let lines: Vec<String> = content
        .lines()
        .rev()
        .take(500)
        .map(normalize_history_line)
        .filter(|l| {
            l.len() > 20
                && (l.contains('|') || l.contains("&&") || l.split_whitespace().count() > 4)
        })
        .collect();

    let mut counts: HashMap<String, u32> = HashMap::new();
    for line in lines {
        if !looks_like_typo(&line) {
            *counts.entry(line).or_default() += 1;
        }
    }

    if let Some((command, _count)) = counts
        .into_iter()
        .filter(|(_, c)| *c >= 8)
        .max_by_key(|(_, c)| *c)
    {
        let now = chrono::Utc::now().timestamp();
        if store.recent_note_exists("deep_alias", now - 86400)? {
            // Once a day max
            return Ok(None);
        }
        return Ok(Some(Behavior::DeepAlias {
            command,
            suggested_alias: "custom_alias".to_string(),
        }));
    }
    Ok(None)
}

pub fn detect_note_reply(
    store: &Store,
    _config: &Config,
    event_path: &Path,
    event_kind: EventKind,
    now: i64,
) -> Result<Option<Behavior>> {
    if event_kind != EventKind::Modify {
        return Ok(None);
    }
    let Some(ext) = event_path.extension() else {
        return Ok(None);
    };
    if ext != "yuna" {
        return Ok(None);
    }

    let content = match fs::read_to_string(event_path) {
        Ok(c) => c,
        Err(_) => return Ok(None),
    };

    let signature = "- still watching";
    let Some(sig_idx) = content.rfind(signature) else {
        return Ok(None);
    };

    let reply_start = sig_idx + signature.len();
    let reply_text = content[reply_start..].trim();

    if reply_text.is_empty() || reply_text.len() < 2 {
        return Ok(None);
    }

    let filename = event_path.file_name().unwrap_or_default().to_string_lossy();
    let trigger = format!("note_replied_{}", filename);
    if store.recent_note_exists(&trigger, now - 3600)? {
        return Ok(None);
    }

    Ok(Some(Behavior::NoteReplied {
        reply_text: reply_text.to_string(),
    }))
}
