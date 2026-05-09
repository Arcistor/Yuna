use std::fs;
use std::path::Path;

use digital_ghost::ai::{build_prompt, fallback_note, load_builtin_profile, PersonalityProfile};
use digital_ghost::ascii::ascii_for_mood;
use digital_ghost::config::{BehaviorConfig, Config, GhostConfig, LimitsConfig, WatchConfig};
use digital_ghost::haunter::{drop_note, reap_notes};
use digital_ghost::store::Store;
use digital_ghost::types::{Behavior, MoodState};
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
            note_lifetime_minutes: 0,
        },
        limits: LimitsConfig {
            max_cpu_percent: 0.5,
            cooldown_seconds: 86400,
        },
    }
}

#[test]
fn prompt_contains_personality_mood_and_behavior() {
    let profile = PersonalityProfile {
        name: "lonely_ghost".to_string(),
        description: "Melancholic and brief.".to_string(),
        tone: vec!["wry".to_string()],
        ascii_style: "minimal".to_string(),
    };
    let behavior = Behavior::Cleaning {
        directory: "/tmp/project".into(),
        delete_count: 12,
    };

    let prompt = build_prompt(&profile, MoodState::Grateful, &behavior);

    assert!(prompt.contains("lonely_ghost"));
    assert!(prompt.contains("grateful"));
    assert!(prompt.contains("deleted 12 files"));
    assert!(prompt.contains("Never mention AI"));
}

#[test]
fn built_in_profiles_include_distinct_voice_guidance() {
    let maid = load_builtin_profile("obsessive_maid").unwrap();
    let veteran = load_builtin_profile("dead_veteran_programmer").unwrap();
    let monk = load_builtin_profile("silent_monk").unwrap();

    assert!(maid.description.contains("tidy"));
    assert!(maid.tone.iter().any(|tone| tone.contains("fussy")));
    assert!(veteran.description.contains("C"));
    assert!(veteran.tone.iter().any(|tone| tone.contains("unimpressed")));
    assert!(monk.description.contains("silence"));
    assert_eq!(monk.ascii_style, "sparse");
}

#[test]
fn prompt_includes_profile_ascii_style_as_voice_hint() {
    let profile = load_builtin_profile("dead_veteran_programmer").unwrap();
    let behavior = Behavior::MidnightWorker {
        directory: "/tmp/project".into(),
        hours: 4.5,
    };

    let prompt = build_prompt(&profile, MoodState::Concerned, &behavior);

    assert!(prompt.contains("ASCII style: terminal"));
    assert!(prompt.contains("unimpressed"));
}

#[test]
fn fallback_note_is_short_and_in_character() {
    let behavior = Behavior::MidnightWorker {
        directory: "/tmp/project".into(),
        hours: 4.0,
    };

    let note = fallback_note(MoodState::Concerned, &behavior);

    assert!(note.split('.').count() <= 4);
    assert!(note.contains("morning") || note.contains("watching"));
}

#[tokio::test]
async fn drop_note_writes_ascii_content_and_records_note() {
    let dir = tempdir().unwrap();
    let store = Store::new(&dir.path().join("ghost.db")).unwrap();
    let behavior = Behavior::Cleaning {
        directory: dir.path().to_path_buf(),
        delete_count: 12,
    };

    let path = drop_note(
        dir.path(),
        "The dust finally moved.",
        ascii_for_mood(&MoodState::Grateful),
        &store,
        &behavior,
        1_000,
    )
    .await
    .unwrap();

    let content = fs::read_to_string(&path).unwrap();
    assert!(content.contains("The dust finally moved."));
    assert!(content.contains("still watching"));
    assert_eq!(store.list_undeleted_notes().unwrap().len(), 1);
}

#[tokio::test]
async fn reap_notes_deletes_read_expired_notes() {
    let dir = tempdir().unwrap();
    let store = Store::new(&dir.path().join("ghost.db")).unwrap();
    let config = test_config(dir.path());
    let note_path = dir.path().join(".ghost_note");
    fs::write(&note_path, "boo").unwrap();
    let id = store.insert_note(&note_path, "cleaning", 100).unwrap();
    store.mark_note_read(id, 100).unwrap();

    reap_notes(&store, &config, 101).await.unwrap();

    assert!(!note_path.exists());
    assert!(store.list_undeleted_notes().unwrap().is_empty());
}
