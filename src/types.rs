use std::fmt;
use std::path::PathBuf;
use std::str::FromStr;

use anyhow::{anyhow, Result};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EventKind {
    Create,
    Modify,
    Delete,
    Rename,
}

impl EventKind {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Create => "create",
            Self::Modify => "modify",
            Self::Delete => "delete",
            Self::Rename => "rename",
        }
    }

    fn parse(value: &str) -> Result<Self> {
        match value {
            "create" => Ok(Self::Create),
            "modify" => Ok(Self::Modify),
            "delete" => Ok(Self::Delete),
            "rename" => Ok(Self::Rename),
            other => Err(anyhow!("unknown event kind: {other}")),
        }
    }
}

impl FromStr for EventKind {
    type Err = anyhow::Error;

    fn from_str(value: &str) -> Result<Self> {
        Self::parse(value)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MoodState {
    Calm,
    Watching,
    Concerned,
    Amused,
    Grateful,
}

impl MoodState {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Calm => "calm",
            Self::Watching => "watching",
            Self::Concerned => "concerned",
            Self::Amused => "amused",
            Self::Grateful => "grateful",
        }
    }

    fn parse(value: &str) -> Result<Self> {
        match value {
            "calm" => Ok(Self::Calm),
            "watching" => Ok(Self::Watching),
            "concerned" => Ok(Self::Concerned),
            "amused" => Ok(Self::Amused),
            "grateful" => Ok(Self::Grateful),
            other => Err(anyhow!("unknown mood state: {other}")),
        }
    }
}

impl FromStr for MoodState {
    type Err = anyhow::Error;

    fn from_str(value: &str) -> Result<Self> {
        Self::parse(value)
    }
}

impl fmt::Display for MoodState {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum Behavior {
    MidnightWorker {
        directory: PathBuf,
        hours: f32,
    },
    Procrastinator {
        directory: PathBuf,
        days_idle: u32,
    },
    Cleaning {
        directory: PathBuf,
        delete_count: u32,
    },
    TypoRepeater {
        command: String,
        count: u32,
    },
}

impl Behavior {
    pub fn trigger_name(&self) -> &'static str {
        match self {
            Self::MidnightWorker { .. } => "midnight_worker",
            Self::Procrastinator { .. } => "procrastinator",
            Self::Cleaning { .. } => "cleaning",
            Self::TypoRepeater { .. } => "typo_repeater",
        }
    }

    pub fn directory(&self) -> Option<&PathBuf> {
        match self {
            Self::MidnightWorker { directory, .. }
            | Self::Procrastinator { directory, .. }
            | Self::Cleaning { directory, .. } => Some(directory),
            Self::TypoRepeater { .. } => None,
        }
    }

    pub fn description(&self) -> String {
        match self {
            Self::MidnightWorker { hours, .. } => {
                format!("the user has been editing code for {hours:.1} hours after midnight")
            }
            Self::Procrastinator {
                directory,
                days_idle,
            } => format!(
                "the project folder '{}' has sat untouched for {days_idle} days",
                directory.display()
            ),
            Self::Cleaning { delete_count, .. } => {
                format!("the user deleted {delete_count} files in a short burst")
            }
            Self::TypoRepeater { command, count } => {
                format!("the user typed '{command}' incorrectly {count} times")
            }
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EventRecord {
    pub id: i64,
    pub path: PathBuf,
    pub kind: EventKind,
    pub timestamp: i64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NoteRecord {
    pub id: i64,
    pub path: PathBuf,
    pub trigger: String,
    pub created: i64,
    pub read_at: Option<i64>,
    pub deleted: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GhostEvent {
    pub path: PathBuf,
    pub kind: EventKind,
    pub timestamp: i64,
}
