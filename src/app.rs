use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::time::Duration;

use anyhow::{anyhow, Context, Result};
use tokio::signal::unix::{signal, SignalKind};
use tokio::sync::mpsc;

use crate::ai::generate_note;
use crate::aliases::inject_for_command;
use crate::ascii::ascii_for_mood;
use crate::config::Config;
use crate::detector;
use crate::haunter::{drop_note, reap_notes};
use crate::mood::update_mood;
use crate::store::Store;
use crate::types::{Behavior, MoodState};
use crate::watcher::FsWatcher;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DaemonState {
    Running,
    Stopped,
    Stale,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DaemonStatus {
    pub state: DaemonState,
    pub pid: Option<u32>,
}

pub async fn run_daemon() -> Result<()> {
    let config = Config::load()?;
    let store = open_default_store()?;
    let pid_path = default_pid_path()?;
    write_pid_file(&pid_path, std::process::id())?;
    let _pid_guard = PidFileGuard {
        path: pid_path.clone(),
    };
    let (sender, mut receiver) = mpsc::channel(256);
    let _watcher = FsWatcher::start(&config, sender)?;

    let mut ollama_child = if !ollama_is_running(&config).await {
        let cmd = if Path::new("/opt/homebrew/bin/ollama").exists() {
            "/opt/homebrew/bin/ollama"
        } else {
            "ollama"
        };
        match tokio::process::Command::new(cmd)
            .arg("serve")
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn() {
                Ok(child) => {
                    eprintln!("started ollama serve");
                    Some(child)
                }
                Err(e) => {
                    eprintln!("failed to start ollama: {e}");
                    None
                }
            }
    } else {
        None
    };

    let mut sigterm = signal(SignalKind::terminate())?;

    let reaper_store = store.clone();
    let reaper_config = config.clone();
    tokio::spawn(async move {
        let mut interval = tokio::time::interval(Duration::from_secs(300));
        loop {
            interval.tick().await;
            let now = chrono::Utc::now().timestamp();
            if let Err(error) = reap_notes(&reaper_store, &reaper_config, now).await {
                eprintln!("yuna reaper error: {error:#}");
            }
        }
    });

    let typo_store = store.clone();
    let typo_config = config.clone();
    tokio::spawn(async move {
        let mut interval = tokio::time::interval(Duration::from_secs(60));
        loop {
            interval.tick().await;
            let now = chrono::Utc::now().timestamp();
            if typo_store.is_silenced(now).unwrap_or(false) {
                continue;
            }

            // Global cooldown check
            let cooldown = typo_config.limits.cooldown_seconds;
            if let Ok(Some(last_time)) = typo_store.last_note_time() {
                if now - last_time < cooldown {
                    continue;
                }
            }

            let candidates = [
                detector::detect_typo_repeater(&typo_config, None),
                detector::detect_alias_candidate(&typo_config, None),
            ];

            for result in candidates {
                let Ok(Some(behavior)) = result else { continue };
                let already_noted = typo_store
                    .recent_note_exists(behavior.trigger_name(), now - cooldown)
                    .unwrap_or(true);
                if already_noted { continue; }

                let current = typo_store.get_mood().unwrap_or(MoodState::Calm);
                let mood = update_mood(current, &behavior);
                if typo_store.set_mood(mood).is_err() { continue; }

                let note = match generate_note(&typo_config, mood, &behavior).await {
                    Ok(n) => n,
                    Err(_) => continue,
                };
                let alias_note = maybe_inject_alias(&typo_config, &behavior).ok().flatten();
                let final_note = alias_note.unwrap_or(note);

                let directory = choose_default_directory(&typo_config);
                let _ = drop_note(&directory, &final_note, ascii_for_mood(&mood), &typo_store, &behavior, now).await;
                
                // Only one note per typo check cycle
                break;
            }
        }
    });

    loop {
        tokio::select! {
            event = receiver.recv() => {
                let Some(event) = event else { break; };
                store.insert_event(&event.path, event.kind, event.timestamp)?;
                let now = chrono::Utc::now().timestamp();
                if store.is_silenced(now)? {
                    continue;
                }

                // Global cooldown check
                let cooldown = config.limits.cooldown_seconds;
                if let Ok(Some(last_time)) = store.last_note_time() {
                    if now - last_time < cooldown {
                        continue;
                    }
                }

                let Some(behavior) = detector::detect(&store, &config, now)? else {
                    continue;
                };
                let current = store.get_mood().unwrap_or(MoodState::Calm);
                let mood = update_mood(current, &behavior);
                store.set_mood(mood)?;
                let mut note = generate_note(&config, mood, &behavior).await?;
                if let Some(alias_note) = maybe_inject_alias(&config, &behavior)? {
                    note = alias_note;
                }
                let directory = behavior
                    .directory()
                    .cloned()
                    .unwrap_or_else(|| choose_default_directory(&config));
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
            _ = sigterm.recv() => {
                eprintln!("received SIGTERM, shutting down...");
                if let Some(mut child) = ollama_child.take() {
                    let _ = child.kill().await;
                    eprintln!("terminated ollama serve");
                }
                break;
            }
        }
    }

    Ok(())
}

pub fn open_default_store() -> Result<Store> {
    Store::new(&default_db_path()?)
}

fn maybe_inject_alias(config: &Config, behavior: &Behavior) -> Result<Option<String>> {
    if !config.behavior.alias_injection {
        return Ok(None);
    }
    let Behavior::TypoRepeater { command, .. } = behavior else {
        return Ok(None);
    };
    Ok(inject_for_command(command)?.map(|suggestion| suggestion.note()))
}

async fn ollama_is_running(config: &Config) -> bool {
    reqwest::Client::new()
        .get(&config.yuna.ollama_url)
        .timeout(Duration::from_millis(500))
        .send()
        .await
        .is_ok()
}

fn choose_default_directory(config: &Config) -> PathBuf {
    config
        .watch
        .paths
        .first()
        .cloned()
        .unwrap_or_else(|| dirs::home_dir().unwrap_or_else(|| PathBuf::from(".")))
}

pub fn default_db_path() -> Result<PathBuf> {
    Ok(dirs::home_dir()
        .context("locate home directory")?
        .join(".yuna")
        .join("yuna.db"))
}

pub fn default_pid_path() -> Result<PathBuf> {
    Ok(dirs::home_dir()
        .context("locate home directory")?
        .join(".yuna")
        .join("yuna.pid"))
}

pub fn write_pid_file(path: &Path, pid: u32) -> Result<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)
            .with_context(|| format!("create pid directory {}", parent.display()))?;
    }
    fs::write(path, format!("{pid}\n"))
        .with_context(|| format!("write pid file {}", path.display()))
}

pub fn read_pid_file(path: &Path) -> Result<Option<u32>> {
    if !path.exists() {
        return Ok(None);
    }
    let content =
        fs::read_to_string(path).with_context(|| format!("read pid file {}", path.display()))?;
    let pid = content
        .trim()
        .parse::<u32>()
        .with_context(|| format!("parse pid file {}", path.display()))?;
    Ok(Some(pid))
}

pub fn remove_pid_file(path: &Path) -> Result<()> {
    match fs::remove_file(path) {
        Ok(()) => Ok(()),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(error) => Err(error).with_context(|| format!("remove pid file {}", path.display())),
    }
}

pub fn daemon_status() -> Result<DaemonStatus> {
    daemon_status_from_pid_file(&default_pid_path()?)
}

pub fn daemon_status_from_pid_file(path: &Path) -> Result<DaemonStatus> {
    let pid = match read_pid_file(path) {
        Ok(Some(pid)) => pid,
        Ok(None) => {
            return Ok(DaemonStatus {
                state: DaemonState::Stopped,
                pid: None,
            })
        }
        Err(_) => {
            remove_pid_file(path)?;
            return Ok(DaemonStatus {
                state: DaemonState::Stale,
                pid: None,
            });
        }
    };

    if process_is_running(pid) {
        Ok(DaemonStatus {
            state: DaemonState::Running,
            pid: Some(pid),
        })
    } else {
        remove_pid_file(path)?;
        Ok(DaemonStatus {
            state: DaemonState::Stale,
            pid: Some(pid),
        })
    }
}

pub fn start_daemon_process() -> Result<u32> {
    let status = daemon_status()?;
    if status.state == DaemonState::Running {
        return status.pid.context("running daemon missing pid");
    }

    let yuna_path = current_yuna_binary_path()?;
    let log_path = dirs::home_dir()
        .context("locate home directory")?
        .join(".yuna")
        .join("yuna.log");
    let log_file = fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(&log_path)
        .with_context(|| format!("open log file {}", log_path.display()))?;
    let child = Command::new(&yuna_path)
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(log_file)
        .spawn()
        .with_context(|| format!("start daemon {}", yuna_path.display()))?;
    Ok(child.id())
}

pub fn stop_daemon_process() -> Result<Option<u32>> {
    let status = daemon_status()?;
    let Some(pid) = status.pid else {
        return Ok(None);
    };
    if status.state != DaemonState::Running {
        return Ok(None);
    }

    terminate_process(pid)?;
    remove_pid_file(&default_pid_path()?)?;
    Ok(Some(pid))
}

fn current_yuna_binary_path() -> Result<PathBuf> {
    let current = std::env::current_exe().context("locate current executable")?;
    let Some(directory) = current.parent() else {
        return Err(anyhow!("current executable has no parent directory"));
    };
    let candidate = directory.join("yuna");
    if candidate.exists() {
        return Ok(candidate);
    }

    #[cfg(windows)]
    {
        let candidate = directory.join("yuna.exe");
        if candidate.exists() {
            return Ok(candidate);
        }
    }

    Err(anyhow!(
        "could not find yuna binary next to {}",
        current.display()
    ))
}

fn process_is_running(pid: u32) -> bool {
    if pid == 0 {
        return false;
    }
    Command::new("kill")
        .arg("-0")
        .arg(pid.to_string())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .map(|status| status.success())
        .unwrap_or(false)
}

fn terminate_process(pid: u32) -> Result<()> {
    let status = Command::new("kill")
        .arg("-TERM")
        .arg(pid.to_string())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .with_context(|| format!("send TERM to pid {pid}"))?;
    if status.success() {
        Ok(())
    } else {
        Err(anyhow!("failed to stop daemon pid {pid}"))
    }
}

struct PidFileGuard {
    path: PathBuf,
}

impl Drop for PidFileGuard {
    fn drop(&mut self) {
        let _ = remove_pid_file(&self.path);
    }
}
