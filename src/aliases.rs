use std::fs;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AliasSuggestion {
    pub alias: String,
    pub target: String,
}

impl AliasSuggestion {
    pub fn line(&self) -> String {
        format!("alias {}='{}'", self.alias, self.target)
    }

    pub fn note(&self) -> String {
        format!(
            "I taught your shell a small mercy: `{}` now means `{}`. Try to look surprised.",
            self.alias, self.target
        )
    }
}

pub fn suggestion_for_command(command: &str) -> Option<AliasSuggestion> {
    let alias = command.split_whitespace().next()?.trim();
    if alias.is_empty() || alias.contains('=') || alias.contains('\'') {
        return None;
    }

    let target = known_alias_target(alias).or_else(|| closest_common_command(alias))?;
    if alias == target {
        return None;
    }

    Some(AliasSuggestion {
        alias: alias.to_string(),
        target: target.to_string(),
    })
}

pub fn rc_file_for_shell(home: &Path, shell: &str) -> PathBuf {
    let shell_name = Path::new(shell)
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or_default();
    match shell_name {
        "zsh" => home.join(".zshrc"),
        "bash" => home.join(".bashrc"),
        _ => home.join(".profile"),
    }
}

pub fn inject_alias(rc_file: &Path, suggestion: &AliasSuggestion) -> Result<bool> {
    if let Some(parent) = rc_file.parent() {
        fs::create_dir_all(parent)
            .with_context(|| format!("create rc directory {}", parent.display()))?;
    }

    let existing = fs::read_to_string(rc_file).unwrap_or_default();
    let alias_line = suggestion.line();
    if existing.lines().any(|line| line.trim() == alias_line) {
        return Ok(false);
    }

    let mut next = existing;
    if !next.is_empty() && !next.ends_with('\n') {
        next.push('\n');
    }
    next.push_str("# Added by Digital Ghost\n");
    next.push_str(&alias_line);
    next.push('\n');
    fs::write(rc_file, next).with_context(|| format!("write rc file {}", rc_file.display()))?;
    Ok(true)
}

pub fn inject_for_command(command: &str) -> Result<Option<AliasSuggestion>> {
    let Some(suggestion) = suggestion_for_command(command) else {
        return Ok(None);
    };
    let Some(home) = dirs::home_dir() else {
        return Ok(None);
    };
    let shell = std::env::var("SHELL").unwrap_or_else(|_| "/bin/sh".to_string());
    let rc_file = rc_file_for_shell(&home, &shell);
    if inject_alias(&rc_file, &suggestion)? {
        Ok(Some(suggestion))
    } else {
        Ok(None)
    }
}

fn known_alias_target(alias: &str) -> Option<&'static str> {
    match alias {
        "gti" | "gir" | "got" => Some("git"),
        "nmp" => Some("npm"),
        "pnpmn" => Some("pnpm"),
        "pyhton" => Some("python"),
        _ => None,
    }
}

fn closest_common_command(alias: &str) -> Option<&'static str> {
    common_commands()
        .iter()
        .copied()
        .find(|candidate| edit_distance_at_most_one(alias, candidate))
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
