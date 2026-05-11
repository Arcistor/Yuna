# Yuna — Zero-Shot Build Prompt

Paste this entire prompt to an AI coding assistant with your codebase context.

---

```
You are building "Yuna in the Machine" — a Rust background daemon that watches
the filesystem and leaves personality-driven notes in directories based on user behavior.
It uses Ollama (local LLM) for note generation. No cloud. No UI. Pure ambient background process.

Reference files:
- Info.md — what the project is, philosophy, behaviors
- Plan.md — phased roadmap, tech stack
- Architecture.md — full data model, module structure, data flow, SQL schema, config format

## Build the complete project in one pass.

### 1. Project scaffold

Create a Cargo workspace with two binaries:
- `yuna` — the background daemon
- `yunactl` — the CLI control tool

Dependencies in Cargo.toml:
```toml
[dependencies]
notify = "6"
rusqlite = { version = "0.31", features = ["bundled"] }
reqwest = { version = "0.12", features = ["json", "blocking"] }
tokio = { version = "1", features = ["full"] }
serde = { version = "1", features = ["derive"] }
serde_json = "1"
toml = "0.8"
chrono = "0.4"
dirs = "5"
```

### 2. Config (`src/config.rs`)

Parse `.yunaconfig` TOML from current dir or `~/.yunaconfig` fallback.
Struct fields match Architecture.md config section exactly.
Provide `Config::load() -> Result<Config>`.

### 3. SQLite store (`src/store.rs`)

Tables: `events`, `mood`, `notes` — exact schema from Architecture.md.
Implement:
- `Store::new(path: &Path) -> Result<Store>` — create tables if not exist
- `Store::insert_event(path, kind, timestamp)`
- `Store::query_events(since: i64, kind: Option<&str>) -> Vec<Event>`
- `Store::get_mood() -> MoodState`
- `Store::set_mood(state: MoodState)`
- `Store::insert_note(path, trigger, created) -> i64`
- `Store::mark_note_read(id, read_at)`
- `Store::mark_note_deleted(id)`
- `Store::list_undeleted_notes() -> Vec<Note>`

### 4. Filesystem watcher (`src/watcher.rs`)

Use `notify` crate with `RecommendedWatcher`.
Watch all paths from config (recursive).
Exclude paths from config.
On each event: call `store.insert_event()` then pass to detector.
Use `tokio::sync::mpsc` channel between watcher and detector loop.
Event-based only — no polling loops.

### 5. Behavior detectors (`src/detector.rs`)

Implement all four detectors from Architecture.md:
- `detect_midnight_worker(store, config) -> Option<Behavior>`
- `detect_procrastinator(store, config) -> Option<Behavior>`
- `detect_cleaning(store, config) -> Option<Behavior>`
- `detect_typo_repeater(config) -> Option<Behavior>` — reads last 50 lines of shell history

Each detector checks cooldown in `notes` table (same trigger within 24h → return None).

Return type:
```rust
pub enum Behavior {
    MidnightWorker { directory: PathBuf, hours: f32 },
    Procrastinator  { directory: PathBuf, days_idle: u32 },
    Cleaning        { directory: PathBuf, delete_count: u32 },
    TypoRepeater    { command: String, count: u32 },
}
```

### 6. Mood state machine (`src/mood.rs`)

States: `Calm | Watching | Concerned | Amused | Grateful`
Transitions from Architecture.md mood section.
`pub fn update_mood(current: MoodState, behavior: &Behavior) -> MoodState`
Store new mood via `store.set_mood()` after every transition.

### 7. ASCII art (`src/ascii.rs`)

One small ASCII per mood state (5–8 lines max, minimal style).
`pub fn ascii_for_mood(mood: &MoodState) -> &'static str`

### 8. Ollama AI client (`src/ai.rs`)

HTTP POST to `{ollama_url}/api/generate` with:
```json
{ "model": "mistral", "prompt": "...", "stream": false }
```

Build system prompt from personality profile + mood + behavior description.
Personality profiles: load from `~/.yuna/profiles/<name>.toml` or bundled defaults.
Bundle these four default profiles inline as const strings (yuna, obsessive_maid,
dead_veteran_programmer, silent_monk) — use descriptions from Info.md.

`pub async fn generate_note(config, mood, behavior) -> Result<String>`

### 9. Note writer + self-delete (`src/haunter.rs`)

`pub async fn drop_note(directory, content, ascii, store) -> Result<()>`
- Choose filename: `.yuna_note` 70% of time, random from list otherwise
- Write file: ASCII header + generated note content + signature line
- Insert into `notes` table

`pub async fn reap_notes(store) -> Result<()>`
- Run every 5 minutes via `tokio::time::interval`
- For each undeleted note: check `atime` via `std::fs::metadata`
- If read AND lifetime exceeded: delete file, mark deleted in store

### 10. Shell alias injector (`src/aliases.rs`) — opt-in only

Only runs if `config.behavior.alias_injection = true`.
Detect shell (check `$SHELL`), find rc file.
Append alias line if not already present.
Drop a note in `$HOME` describing what was added.

### 11. Main daemon (`src/main.rs`)

```rust
#[tokio::main]
async fn main() {
    // 1. Load config
    // 2. Open SQLite store at ~/.yuna/yuna.db
    // 3. Spawn watcher task
    // 4. Spawn reaper task (every 5 min)
    // 5. Main loop: receive events from watcher channel
    //    → run all detectors
    //    → if behavior detected: update mood → generate note → drop note
}
```

### 12. yunactl CLI (`yunactl/src/main.rs`)

Subcommands via `clap`:
- `status` — print: daemon PID (check lockfile), current mood, last event time, unread note count
- `notes` — list paths of unread notes
- `mood` — print mood + last behavior that triggered it
- `silence <duration>` — write silence expiry to SQLite, daemon checks before dropping notes

Reads same SQLite db and config as daemon.

### 13. Default personality profiles (inline constants)

```
yuna: died coding alone, never shipped. Melancholic, dry humor, brief.
obsessive_maid: compulsively tidy spirit. Pleased by cleanup, horrified by mess.
dead_veteran_programmer: gruff, seen everything, unimpressed. C programmer energy.
silent_monk: says almost nothing. When speaks, it lands heavy.
```

### 14. Example .yunaconfig

Create `.yunaconfig.example` at project root with all fields and comments.

### 15. Acceptance criteria

1. `cargo build --release` produces two binaries with zero errors
2. `yuna` starts silently, watches configured path, writes events to SQLite
3. Create a new folder then don't touch it for simulated 3 days (mock timestamp in test) → procrastinator note appears
4. `yunactl status` shows mood and note count
5. `yunactl notes` lists the dropped note path
6. Note file contains ASCII art + personality-driven text
7. After simulated atime update past lifetime → note file deleted, SQLite marked deleted
8. No polling loops — verified by checking CPU usage stays near 0% at idle

Fix all compiler warnings. No `unwrap()` in production paths — use `?` or log and continue.
No `unsafe`. No `std::thread::sleep` loops for watching.
```
