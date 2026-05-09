use std::fs;
use std::path::Path;

use digital_ghost::config::{BehaviorConfig, Config, GhostConfig, LimitsConfig, WatchConfig};
use digital_ghost::detector::{detect_cleaning, detect_procrastinator, detect_typo_repeater};
use digital_ghost::mood::update_mood;
use digital_ghost::store::Store;
use digital_ghost::types::{Behavior, EventKind, MoodState};
use tempfile::tempdir;

fn test_config(root: &Path) -> Config {
    Config {
        ghost: GhostConfig {
            personality: "lonely_ghost".to_string(),
            ollama_model: "mistral".to_string(),
            ollama_url: "http://127.0.0.1:9".to_string(),
        },
        watch: WatchConfig {
            paths: vec![root.to_path_buf()],
            exclude: vec![],
        },
        behavior: BehaviorConfig {
            alias_injection: false,
            note_lifetime_minutes: 60,
        },
        limits: LimitsConfig {
            max_cpu_percent: 0.5,
            cooldown_hours: 24,
        },
    }
}

#[test]
fn mood_transitions_match_behavior() {
    let cleaning = Behavior::Cleaning {
        directory: "/tmp/project".into(),
        delete_count: 12,
    };
    let typo = Behavior::TypoRepeater {
        command: "gti".to_string(),
        count: 3,
    };
    let midnight = Behavior::MidnightWorker {
        directory: "/tmp/project".into(),
        hours: 4.5,
    };

    assert_eq!(update_mood(MoodState::Calm, &cleaning), MoodState::Grateful);
    assert_eq!(update_mood(MoodState::Calm, &typo), MoodState::Amused);
    assert_eq!(
        update_mood(MoodState::Watching, &midnight),
        MoodState::Concerned
    );
}

#[test]
fn detects_cleaning_when_many_deletes_happen_quickly() {
    let dir = tempdir().unwrap();
    let store = Store::new(&dir.path().join("ghost.db")).unwrap();
    let config = test_config(dir.path());

    for index in 0..11 {
        store
            .insert_event(
                &dir.path().join(format!("old-{index}.txt")),
                EventKind::Delete,
                10_000 + index,
            )
            .unwrap();
    }

    let behavior = detect_cleaning(&store, &config, 10_599).unwrap().unwrap();
    assert_eq!(behavior.trigger_name(), "cleaning");
}

#[test]
fn detects_procrastinator_for_untouched_new_project() {
    let dir = tempdir().unwrap();
    let store = Store::new(&dir.path().join("ghost.db")).unwrap();
    let config = test_config(dir.path());
    let project = dir.path().join("New Project");

    store
        .insert_event(&project, EventKind::Create, 100)
        .unwrap();

    let behavior = detect_procrastinator(&store, &config, 100 + 3 * 24 * 60 * 60 + 1)
        .unwrap()
        .unwrap();
    assert_eq!(behavior.trigger_name(), "procrastinator");
}

#[test]
fn detects_repeated_typo_from_history_file() {
    let dir = tempdir().unwrap();
    let history = dir.path().join(".zsh_history");
    fs::write(&history, "git status\ngti status\ngti status\ngti status\n").unwrap();
    let config = test_config(dir.path());

    let behavior = detect_typo_repeater(&config, Some(&history))
        .unwrap()
        .unwrap();

    match behavior {
        Behavior::TypoRepeater { command, count } => {
            assert_eq!(command, "gti status");
            assert_eq!(count, 3);
        }
        other => panic!("unexpected behavior: {other:?}"),
    }
}
