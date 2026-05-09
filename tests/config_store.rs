use std::fs;
use std::path::Path;

use digital_ghost::config::Config;
use digital_ghost::store::Store;
use digital_ghost::types::{EventKind, MoodState};
use tempfile::tempdir;

fn write_config(path: &Path) {
    fs::write(
        path,
        r#"
[ghost]
personality = "lonely_ghost"
ollama_model = "mistral"
ollama_url = "http://localhost:11434"

[watch]
paths = ["/tmp/ghost-watch"]
exclude = ["/tmp/ghost-watch/target", "/tmp/ghost-watch/.git"]

[behavior]
alias_injection = false
note_lifetime_minutes = 60

[limits]
max_cpu_percent = 0.5
cooldown_hours = 24
"#,
    )
    .unwrap();
}

#[test]
fn loads_config_from_explicit_path() {
    let dir = tempdir().unwrap();
    let path = dir.path().join(".ghostconfig");
    write_config(&path);

    let config = Config::load_from_path(&path).unwrap();

    assert_eq!(config.ghost.personality, "lonely_ghost");
    assert_eq!(config.watch.paths.len(), 1);
    assert_eq!(config.behavior.note_lifetime_minutes, 60);
    assert_eq!(config.limits.cooldown_hours, 24);
}

#[test]
fn store_creates_schema_and_persists_core_state() {
    let dir = tempdir().unwrap();
    let db_path = dir.path().join("ghost.db");
    let store = Store::new(&db_path).unwrap();

    store
        .insert_event(Path::new("/tmp/project/main.rs"), EventKind::Modify, 123)
        .unwrap();
    let events = store.query_events(0, Some(EventKind::Modify)).unwrap();
    assert_eq!(events.len(), 1);
    assert_eq!(events[0].path, Path::new("/tmp/project/main.rs"));

    assert_eq!(store.get_mood().unwrap(), MoodState::Calm);
    store.set_mood(MoodState::Concerned).unwrap();
    assert_eq!(store.get_mood().unwrap(), MoodState::Concerned);

    let note_id = store
        .insert_note(Path::new("/tmp/project/.ghost_note"), "cleaning", 200)
        .unwrap();
    store.mark_note_read(note_id, 250).unwrap();
    let notes = store.list_undeleted_notes().unwrap();
    assert_eq!(notes.len(), 1);
    assert_eq!(notes[0].read_at, Some(250));

    assert!(store.recent_note_exists("cleaning", 199).unwrap());
    store.set_silenced_until(500).unwrap();
    assert!(store.is_silenced(499).unwrap());
    assert!(!store.is_silenced(501).unwrap());
}
