use crate::types::{Behavior, MoodState};

pub fn update_mood(current: MoodState, behavior: &Behavior) -> MoodState {
    match behavior {
        Behavior::Cleaning { .. } => MoodState::Grateful,
        Behavior::TypoRepeater { .. } => MoodState::Amused,
        Behavior::MidnightWorker { .. } => match current {
            MoodState::Calm => MoodState::Watching,
            MoodState::Watching | MoodState::Concerned => MoodState::Concerned,
            MoodState::Amused | MoodState::Grateful => MoodState::Watching,
        },
        Behavior::Procrastinator { .. } => MoodState::Concerned,
    }
}
