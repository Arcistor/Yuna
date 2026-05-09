use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::time::Duration;

use anyhow::{anyhow, Context, Result};
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
        let mut note = generate_note(&config, mood, &behavior).await?;
        if let Some(alias_note) = maybe_inject_alias(&config, &behavior)? {
            note = alias_note;
        }
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

fn maybe_inject_alias(config: &Config, behavior: &Behavior) -> Result<Option<String>> {
    if !config.behavior.alias_injection {
        return Ok(None);
    }
    let Behavior::TypoRepeater { command, .. } = behavior else {
        return Ok(None);
    };
    Ok(inject_for_command(command)?.map(|suggestion| suggestion.note()))
}

pub fn default_db_path() -> Result<PathBuf> {
    Ok(dirs::home_dir()
        .context("locate home directory")?
        .join(".ghost")
        .join("ghost.db"))
}

pub fn default_pid_path() -> Result<PathBuf> {
    Ok(dirs::home_dir()
        .context("locate home directory")?
        .join(".ghost")
        .join("ghost.pid"))
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

    let ghost_path = current_ghost_binary_path()?;
    let child = Command::new(&ghost_path)
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .with_context(|| format!("start daemon {}", ghost_path.display()))?;
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

fn current_ghost_binary_path() -> Result<PathBuf> {
    let current = std::env::current_exe().context("locate current executable")?;
    let Some(directory) = current.parent() else {
        return Err(anyhow!("current executable has no parent directory"));
    };
    let candidate = directory.join("ghost");
    if candidate.exists() {
        return Ok(candidate);
    }

    #[cfg(windows)]
    {
        let candidate = directory.join("ghost.exe");
        if candidate.exists() {
            return Ok(candidate);
        }
    }

    Err(anyhow!(
        "could not find ghost binary next to {}",
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
