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
    Hoarder {
        directory: PathBuf,
        filename: String,
        modify_count: u32,
    },
    Archaeologist {
        directory: PathBuf,
        filename: String,
        months_dormant: u32,
    },
    EmptyNest {
        directory: PathBuf,
        days_empty: u32,
    },
    Duplicator {
        directory: PathBuf,
        base_name: String,
        count: u32,
    },
    YunaCommit {
        directory: PathBuf,
        days_uncommitted: u32,
    },
    RevertSpiral {
        directory: PathBuf,
        filename: String,
        revert_count: u32,
    },
    AliasCandidate {
        command: String,
        count: u32,
    },
    NightOwl {
        directory: PathBuf,
        hour: u32,
    },
    WeekendWarrior {
        directory: PathBuf,
        hours: f32,
    },
    DeadlineMode {
        directory: PathBuf,
        multiplier: f32,
    },
    YunaMissing {
        days_absent: u32,
    },
    FreshStart {
        days_absent: u32,
    },
}

impl Behavior {
    pub fn trigger_name(&self) -> &'static str {
        match self {
            Self::MidnightWorker { .. } => "midnight_worker",
            Self::Procrastinator { .. } => "procrastinator",
            Self::Cleaning { .. } => "cleaning",
            Self::TypoRepeater { .. } => "typo_repeater",
            Self::Hoarder { .. } => "hoarder",
            Self::Archaeologist { .. } => "archaeologist",
            Self::EmptyNest { .. } => "empty_nest",
            Self::Duplicator { .. } => "duplicator",
            Self::YunaCommit { .. } => "yuna_commit",
            Self::RevertSpiral { .. } => "revert_spiral",
            Self::AliasCandidate { .. } => "alias_candidate",
            Self::NightOwl { .. } => "night_owl",
            Self::WeekendWarrior { .. } => "weekend_warrior",
            Self::DeadlineMode { .. } => "deadline_mode",
            Self::YunaMissing { .. } => "yuna_missing",
            Self::FreshStart { .. } => "fresh_start",
        }
    }

    pub fn directory(&self) -> Option<&PathBuf> {
        match self {
            Self::MidnightWorker { directory, .. }
            | Self::Procrastinator { directory, .. }
            | Self::Cleaning { directory, .. }
            | Self::EmptyNest { directory, .. }
            | Self::YunaCommit { directory, .. }
            | Self::NightOwl { directory, .. }
            | Self::WeekendWarrior { directory, .. }
            | Self::DeadlineMode { directory, .. }
            | Self::Duplicator { directory, .. }
            | Self::Hoarder { directory, .. }
            | Self::Archaeologist { directory, .. }
            | Self::RevertSpiral { directory, .. } => Some(directory),
            Self::TypoRepeater { .. }
            | Self::AliasCandidate { .. }
            | Self::YunaMissing { .. }
            | Self::FreshStart { .. } => None,
        }
    }

    pub fn description(&self) -> String {
        match self {
            Self::MidnightWorker { hours, .. } => {
                format!("the user has been editing code for {hours:.1} hours after midnight")
            }
            Self::Procrastinator { directory, days_idle } => format!(
                "the project folder '{}' has sat untouched for {days_idle} days",
                directory.display()
            ),
            Self::Cleaning { delete_count, .. } => {
                format!("the user deleted {delete_count} files in a short burst")
            }
            Self::TypoRepeater { command, count } => {
                format!("the user typed '{command}' incorrectly {count} times")
            }
            Self::Hoarder { filename, modify_count, .. } => format!(
                "the user modified '{filename}' {modify_count} times today without committing"
            ),
            Self::Archaeologist { filename, months_dormant, .. } => format!(
                "the user opened '{filename}' which had not been touched in {months_dormant} months"
            ),
            Self::EmptyNest { directory, days_empty } => format!(
                "the folder '{}' has been empty for {days_empty} days",
                directory.display()
            ),
            Self::Duplicator { directory, base_name, count } => format!(
                "the user has {count} near-duplicate files named like '{base_name}' in '{}'",
                directory.display()
            ),
            Self::YunaCommit { directory, days_uncommitted } => format!(
                "files in '{}' have been modified for {days_uncommitted} days with no git commit",
                directory.display()
            ),
            Self::RevertSpiral { filename, revert_count, .. } => format!(
                "the user has reverted '{filename}' {revert_count} times in the last hour"
            ),
            Self::AliasCandidate { command, count } => format!(
                "the user typed '{command}' {count} times — they should make an alias"
            ),
            Self::NightOwl { hour, .. } => format!(
                "the user is still working at {hour}:00 in the morning"
            ),
            Self::WeekendWarrior { hours, .. } => format!(
                "the user has been coding for {hours:.1} hours on a weekend"
            ),
            Self::DeadlineMode { multiplier, .. } => format!(
                "file activity is {multiplier:.1}x higher than the weekly average — crunch detected"
            ),
            Self::YunaMissing { days_absent } => format!(
                "the user has been absent for {days_absent} days"
            ),
            Self::FreshStart { days_absent } => format!(
                "the user returned after {days_absent} days away"
            ),
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
pub struct YunaEvent {
    pub path: PathBuf,
    pub kind: EventKind,
    pub timestamp: i64,
}
