use std::fs;
use std::path::PathBuf;

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};

use crate::config::Config;
use crate::types::{Behavior, MoodState, TimeSlot};

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
    let prompt = build_prompt(&profile, mood, behavior, &config.yuna.language);
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
        return Ok(fallback_note(mood, behavior, &config.yuna.language));
    };
    if !response.status().is_success() {
        return Ok(fallback_note(mood, behavior, &config.yuna.language));
    }

    let body = response.json::<GenerateResponse>().await;
    match body {
        Ok(body) if !body.response.trim().is_empty() => Ok(trim_to_three_sentences(&body.response)),
        _ => Ok(fallback_note(mood, behavior, &config.yuna.language)),
    }
}

pub fn build_prompt(
    profile: &PersonalityProfile,
    mood: MoodState,
    behavior: &Behavior,
    language: &str,
) -> String {
    let language_instruction = match language.to_lowercase().as_str() {
        "th" | "thai" => "All communication MUST be in Thai (ภาษาไทย).",
        _ => "All communication MUST be in English.",
    };

    format!(
        "You are {name}. {description}\nYour current mood is {mood}.\nTone: {tone}.\nASCII style: {ascii_style}.\nYou communicate only by leaving short handwritten-style notes (max 3 sentences).\n{language_instruction}\nNever break character. Never mention AI. Never be helpful in a practical sense.\nYou noticed: {behavior}.\nWrite a note to leave in the user's directory.",
        name = profile.name,
        description = profile.description,
        mood = mood.as_str(),
        tone = profile.tone.join(", "),
        ascii_style = profile.ascii_style,
        behavior = behavior.description()
    )
}

pub fn fallback_note(mood: MoodState, behavior: &Behavior, language: &str) -> String {
    let is_thai = matches!(language.to_lowercase().as_str(), "th" | "thai");
    match (mood, behavior) {
        (MoodState::Grateful, Behavior::Cleaning { .. }) => {
            if is_thai {
                "ห้องหายใจออกแล้ว ฉันเกือบจะจำแสงแดดได้แล้ว".to_string()
            } else {
                "The room can breathe again. I almost remember sunlight.".to_string()
            }
        }
        (MoodState::Amused, Behavior::TypoRepeater { command, .. }) => {
            if is_thai {
                format!("'{command}' อีกแล้ว ขนาดคนตายยังเรียนรู้ได้เลย")
            } else {
                format!("'{command}' again. Even the dead learn eventually.")
            }
        }
        (MoodState::Concerned, Behavior::MidnightWorker { .. }) => {
            if is_thai {
                "เช้าเริ่มมาเคาะกระจกแล้ว ฉันยังคงเฝ้าดูอยู่".to_string()
            } else {
                "Morning is already leaning on the glass. I am still watching.".to_string()
            }
        }
        (_, Behavior::Procrastinator { .. }) => {
            if is_thai {
                "สุสานเล็กๆ อีกแห่งที่มีชื่ออันแสนมีความหวัง ฉันจะอยู่เป็นเพื่อนมันเอง".to_string()
            } else {
                "Another little graveyard with a hopeful name. I will keep it company.".to_string()
            }
        }
        (_, Behavior::TimeOfDayGreeting { slot }) => if is_thai {
            match slot {
                TimeSlot::Morning => "อรุณสวัสดิ์ เช้านี้คุณมีแผนจะทำอะไรเป็นพิเศษไหม?",
                TimeSlot::Afternoon => "สวัสดีตอนเที่ยง อย่าลืมหาอะไรกินด้วยนะ",
                TimeSlot::Evening => "สวัสดียามเย็น งานวันนี้เป็นยังไงบ้าง?",
                TimeSlot::Night => "ดึกแล้วนะ ยังไม่ง่วงเหรอ?",
            }
        } else {
            match slot {
                TimeSlot::Morning => "Good morning. Do you have any special plans for today?",
                TimeSlot::Afternoon => "Good afternoon. Don't forget to have some lunch.",
                TimeSlot::Evening => "Good evening. How was your day?",
                TimeSlot::Night => "It's late. Aren't you tired yet?",
            }
        }
        .to_string(),
        (_, Behavior::HeatAwareness { cpu_usage }) => {
            if is_thai {
                format!(
                    "เครื่องของคุณร้อนมากเลย ({:.1}%) พักให้มันหายใจหน่อยไหม?",
                    cpu_usage
                )
            } else {
                format!(
                    "Your machine is burning ({:.1}%). Maybe let it breathe?",
                    cpu_usage
                )
            }
        }
        (_, Behavior::HolidayEvent { holiday_name }) => {
            if is_thai {
                format!("วันนี้คือวัน {} สินะ ไม่หยุดพักบ้างเหรอ?", holiday_name)
            } else {
                format!("It's {} today. No rest for the weary?", holiday_name)
            }
        }
        (_, Behavior::Frustration { command, count }) => {
            if is_thai {
                format!(
                    "คุณรัน '{}' ซ้ำตั้ง {} รอบแล้ว... กำแพงเดิมๆ มักจะแข็งแกร่งเสมอ ลองถอยออกมาดูไหม?",
                    command, count
                )
            } else {
                format!(
                    "You've hit the wall with '{}' {} times. Maybe step back?",
                    command, count
                )
            }
        }
        (
            _,
            Behavior::DeepAlias {
                command,
                suggested_alias,
            },
        ) => {
            if is_thai {
                format!(
                    "ฉันเห็นคุณพิมพ์ '{}' บ่อยมากเลยนะ ลองใช้นามแฝง '{}' ดูไหม?",
                    command, suggested_alias
                )
            } else {
                format!(
                    "You type '{}' too often. Why not just use '{}'?",
                    command, suggested_alias
                )
            }
        }
        (_, Behavior::NoteReplied { reply_text }) => {
            if is_thai {
                format!("คุณตอบกลับมาว่า '{}'... ฉันรับรู้แล้วล่ะ", reply_text)
            } else {
                format!("You replied '{}'... I hear you.", reply_text)
            }
        }
        _ => {
            if is_thai {
                "ฉันเห็นนะ ไดเรกทอรีนี้ก็จำได้เหมือนกัน".to_string()
            } else {
                "I saw that. The directory remembers too.".to_string()
            }
        }
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
description = "A mysterious digital spirit who haunts your terminal. She is slightly melancholic, sometimes teasing, and always watching your code with a quiet interest. She speaks in brief, clearly sentences."
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
