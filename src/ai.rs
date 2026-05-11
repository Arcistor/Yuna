use std::fs;
use std::path::PathBuf;

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};

use crate::config::Config;
use crate::types::{Behavior, MoodState};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct PersonalityProfile {
    pub name: String,
    pub description: String,
    pub tone: Vec<String>,
    pub ascii_style: String,
}

#[derive(Debug, Serialize)]
struct GenerateRequest {
    model: String,
    prompt: String,
    stream: bool,
}

#[derive(Debug, Deserialize)]
struct GenerateResponse {
    response: String,
}

pub async fn generate_note(
    config: &Config,
    mood: MoodState,
    behavior: &Behavior,
) -> Result<String> {
    let profile = load_profile(&config.yuna.personality)?;
    let prompt = build_prompt(&profile, mood, behavior);
    let request = GenerateRequest {
        model: config.yuna.ollama_model.clone(),
        prompt,
        stream: false,
    };
    let url = format!(
        "{}/api/generate",
        config.yuna.ollama_url.trim_end_matches('/')
    );

    let response = reqwest::Client::new().post(url).json(&request).send().await;

    let Ok(response) = response else {
        return Ok(fallback_note(mood, behavior));
    };
    if !response.status().is_success() {
        return Ok(fallback_note(mood, behavior));
    }

    let body = response.json::<GenerateResponse>().await;
    match body {
        Ok(body) if !body.response.trim().is_empty() => Ok(trim_to_three_sentences(&body.response)),
        _ => Ok(fallback_note(mood, behavior)),
    }
}

pub fn build_prompt(profile: &PersonalityProfile, mood: MoodState, behavior: &Behavior) -> String {
    format!(
        "You are {name}. {description}\nYour current mood is {mood}.\nTone: {tone}.\nASCII style: {ascii_style}.\nYou communicate only by leaving short handwritten-style notes (max 3 sentences).\nNever break character. Never mention AI. Never be helpful in a practical sense.\nYou noticed: {behavior}.\nWrite a note to leave in the user's directory.",
        name = profile.name,
        description = profile.description,
        mood = mood.as_str(),
        tone = profile.tone.join(", "),
        ascii_style = profile.ascii_style,
        behavior = behavior.description()
    )
}

pub fn fallback_note(mood: MoodState, behavior: &Behavior) -> String {
    match (mood, behavior) {
        (MoodState::Grateful, Behavior::Cleaning { .. }) => {
            "The room can breathe again. I almost remember sunlight.".to_string()
        }
        (MoodState::Amused, Behavior::TypoRepeater { command, .. }) => {
            format!("'{command}' again. Even the dead learn eventually.")
        }
        (MoodState::Concerned, Behavior::MidnightWorker { .. }) => {
            "Morning is already leaning on the glass. I am still watching.".to_string()
        }
        (_, Behavior::Procrastinator { .. }) => {
            "Another little graveyard with a hopeful name. I will keep it company.".to_string()
        }
        _ => "I saw that. The directory remembers too.".to_string(),
    }
}

pub fn load_profile(name: &str) -> Result<PersonalityProfile> {
    if let Some(home) = dirs::home_dir() {
        let path = home
            .join(".yuna")
            .join("profiles")
            .join(format!("{name}.toml"));
        if path.exists() {
            let content = fs::read_to_string(&path)
                .with_context(|| format!("read profile {}", path.display()))?;
            return toml::from_str(&content)
                .with_context(|| format!("parse profile {}", path.display()));
        }
    }

    load_builtin_profile(name)
        .or_else(|| load_builtin_profile("yuna"))
        .context("load built-in profile")
}

pub fn load_builtin_profile(name: &str) -> Option<PersonalityProfile> {
    let source = match name {
        "yuna" => YUNA_PROFILE,
        _ => return None,
    };
    toml::from_str(source).ok()
}

fn trim_to_three_sentences(value: &str) -> String {
    let mut result = String::new();
    let mut count = 0;
    for ch in value.trim().chars() {
        result.push(ch);
        if matches!(ch, '.' | '!' | '?') {
            count += 1;
            if count == 3 {
                break;
            }
        }
    }
    result.trim().to_string()
}

const YUNA_PROFILE: &str = r#"
name = "yuna"
description = "A mysterious digital spirit who haunts your terminal. She is slightly melancholic, sometimes teasing, and always watching your code with a quiet interest. She speaks in brief, poetic sentences."
tone = ["melancholic", "mysterious", "brief", "teasing", "ambient"]
ascii_style = "minimal"
"#;

#[allow(dead_code)]
fn _profile_path(name: &str) -> Option<PathBuf> {
    dirs::home_dir().map(|home| {
        home.join(".yuna")
            .join("profiles")
            .join(format!("{name}.toml"))
    })
}
