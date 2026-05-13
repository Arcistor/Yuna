use std::env;
use std::fs;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Config {
    pub yuna: YunaConfig,
    pub watch: WatchConfig,
    pub behavior: BehaviorConfig,
    pub limits: LimitsConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct YunaConfig {
    pub personality: String,
    pub language: String,
    pub ollama_model: String,
    pub ollama_url: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct WatchConfig {
    pub paths: Vec<PathBuf>,
    pub exclude: Vec<PathBuf>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct BehaviorConfig {
    pub alias_injection: bool,
    pub note_lifetime_minutes: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct LimitsConfig {
    pub max_cpu_percent: f32,
    pub cooldown_seconds: i64,
}

impl Config {
    pub fn load() -> Result<Self> {
        let cwd_config = env::current_dir()
            .context("read current directory")?
            .join(".yunaconfig");
        if cwd_config.exists() {
            return Self::load_from_path(&cwd_config);
        }

        let home_config = dirs::home_dir()
            .context("locate home directory")?
            .join(".yunaconfig");
        Self::load_from_path(&home_config)
    }

    pub fn load_from_path(path: &Path) -> Result<Self> {
        let content =
            fs::read_to_string(path).with_context(|| format!("read config {}", path.display()))?;
        toml::from_str(&content).with_context(|| format!("parse config {}", path.display()))
    }

    pub fn effective_excludes(&self) -> Vec<PathBuf> {
        let mut excludes = default_system_excludes();
        for path in &self.watch.exclude {
            if !excludes.iter().any(|existing| existing == path) {
                excludes.push(path.clone());
            }
        }
        excludes
    }
}

pub fn default_system_excludes() -> Vec<PathBuf> {
    [
        "/proc",
        "/sys",
        "/dev",
        "/etc",
        "/bin",
        "/sbin",
        "/System",
        "/Library",
        "/private/var/db",
    ]
    .into_iter()
    .map(PathBuf::from)
    .collect()
}
