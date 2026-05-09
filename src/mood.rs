use crate::types::{Behavior, MoodState};

pub fn update_mood(current: MoodState, behavior: &Behavior) -> MoodState {
    match behavior {
        Behavior::Cleaning { .. } => MoodState::Grateful,
        Behavior::FreshStart { .. } => MoodState::Grateful,
        Behavior::TypoRepeater { .. } => MoodState::Amused,
        Behavior::AliasCandidate { .. } => MoodState::Amused,
        Behavior::RevertSpiral { .. } => MoodState::Amused,
        Behavior::Duplicator { .. } => MoodState::Amused,
        Behavior::MidnightWorker { .. } => match current {
            MoodState::Calm => MoodState::Watching,
            MoodState::Watching | MoodState::Concerned => MoodState::Concerned,
            MoodState::Amused | MoodState::Grateful => MoodState::Watching,
        },
        Behavior::NightOwl { .. } => match current {
            MoodState::Calm => MoodState::Watching,
            MoodState::Watching | MoodState::Concerned => MoodState::Concerned,
            MoodState::Amused | MoodState::Grateful => MoodState::Watching,
        },
        Behavior::WeekendWarrior { .. } => MoodState::Watching,
        Behavior::DeadlineMode { .. } => MoodState::Concerned,
        Behavior::Procrastinator { .. } => MoodState::Concerned,
        Behavior::EmptyNest { .. } => MoodState::Concerned,
        Behavior::GhostCommit { .. } => MoodState::Concerned,
        Behavior::Ghosted { .. } => MoodState::Concerned,
        Behavior::Hoarder { .. } => MoodState::Watching,
        Behavior::Archaeologist { .. } => MoodState::Watching,
    }
}
