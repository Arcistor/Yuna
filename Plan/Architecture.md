# Digital Ghost — Architecture

## High-Level Structure

```
ghost (binary)
├── main.rs           — startup, load config, spawn tasks
├── config.rs         — parse .ghostconfig (TOML)
├── watcher.rs        — filesystem event listener (notify crate)
├── store.rs          — SQLite read/write (events, mood, notes)
├── detector.rs       — behavior classification functions
├── mood.rs           — ghost mood state machine
├── ai.rs             — Ollama HTTP client, prompt builder
├── haunter.rs        — note writer, self-delete scheduler
├── aliases.rs        — shell alias injector (opt-in)
├── ascii.rs          — ASCII art by mood
└── ghostctl/         — CLI tool (separate binary)
    └── main.rs       — start/stop/status/notes/mood commands
```

---

## Config File (`.ghostconfig`)

```toml
[ghost]
personality = "lonely_ghost"   # profile name
ollama_model = "mistral"
ollama_url = "http://localhost:11434"

[watch]
paths = ["/home/user/projects", "/home/user/code"]
exclude = ["/home/user/projects/node_modules", "/.git/"]

[behavior]
alias_injection = false        # opt-in only
note_lifetime_minutes = 60

[limits]
max_cpu_percent = 0.5
cooldown_hours = 24
```

---

## Data Model (SQLite)

### Table: `events`
```sql
CREATE TABLE events (
  id        INTEGER PRIMARY KEY,
  path      TEXT NOT NULL,
  kind      TEXT NOT NULL,       -- 'create'|'modify'|'delete'|'rename'
  timestamp INTEGER NOT NULL     -- unix epoch
);
```

### Table: `mood`
```sql
CREATE TABLE mood (
  id        INTEGER PRIMARY KEY CHECK (id = 1),
  state     TEXT NOT NULL,       -- 'calm'|'watching'|'concerned'|'amused'|'grateful'
  updated   INTEGER NOT NULL
);
```

### Table: `notes`
```sql
CREATE TABLE notes (
  id        INTEGER PRIMARY KEY,
  path      TEXT NOT NULL,       -- full path to the dropped note file
  trigger   TEXT NOT NULL,       -- behavior that caused it
  created   INTEGER NOT NULL,
  read_at   INTEGER,             -- null until atime check passes
  deleted   INTEGER DEFAULT 0
);
```

---

## Core Data Flow

```
[FS Event]
     │
     ▼
watcher.rs  ──► store.rs (write event)
     │
     ▼
detector.rs (query last N events from SQLite)
     │
     ├─ behavior detected? ──► mood.rs (update mood state)
     │                              │
     │                              ▼
     │                         ai.rs (build prompt + call Ollama)
     │                              │
     │                              ▼
     │                         haunter.rs (write note file to dir)
     │                              │
     │                              ▼
     │                         store.rs (record note, schedule delete)
     │
     └─ no behavior → idle, check atime on existing notes → delete if read
```

---

## Behavior Detectors (`detector.rs`)

```rust
pub enum Behavior {
    MidnightWorker { directory: PathBuf, hours: f32 },
    Procrastinator  { directory: PathBuf, days_idle: u32 },
    Cleaning        { directory: PathBuf, delete_count: u32 },
    TypoRepeater    { command: String, count: u32 },
}

pub fn detect(store: &Store, config: &Config) -> Option<Behavior>
```

Each detector queries `events` table with time-window SQL:
- MidnightWorker: `WHERE kind='modify' AND timestamp > midnight AND timestamp < now` group by hour
- Procrastinator: `WHERE kind='create' AND path LIKE '%/New%'` then check no follow-up events
- Cleaning: `WHERE kind='delete' AND timestamp > (now - 600)` count > 10
- TypoRepeater: reads last 50 lines of `~/.zsh_history` or `~/.bash_history`

---

## Mood State Machine (`mood.rs`)

```
calm ──► watching ──► concerned
 ▲           │             │
 └───────────┴─────────────┘
       (cooldown reset)

calm    + cleaning    → grateful
calm    + typo        → amused
watching + midnight   → concerned
concerned + procrastin→ concerned (stays)
```

Mood persists in SQLite row id=1. Loaded on startup.

---

## Ollama Prompt Builder (`ai.rs`)

System prompt structure:
```
You are [PERSONALITY_NAME]. [PERSONALITY_DESCRIPTION].
Your current mood is [MOOD].
You communicate only by leaving short handwritten-style notes (max 3 sentences).
Never break character. Never mention AI. Never be helpful in a practical sense.
You noticed: [BEHAVIOR_DESCRIPTION].
Write a note to leave in the user's directory.
```

Personality profiles loaded from `~/.ghost/profiles/<name>.toml`:
```toml
name = "lonely_ghost"
description = "A ghost who died coding alone and never shipped their project. Melancholic, dry humor, occasional jealousy."
tone = ["melancholic", "wry", "brief"]
ascii_style = "minimal"
```

---

## Note File Format

Filename: `.ghost_note` or randomized like `MESSAGE_FROM_THE_VOID.txt`

```
    ░▒▓  the ghost was here  ▓▒░

  That's your third cup of coffee...
  Rest your eyes.
  The bugs will still be there in the morning.

                              — still watching
```

---

## Self-Delete Scheduler (`haunter.rs`)

- On note creation: record `path` + `created` in `notes` table
- Every 5 minutes: check `atime` of each undeleted note file
- If `atime > created` (file was read) AND `now - atime > note_lifetime_minutes`: delete file, set `deleted=1`
- If file already missing (user deleted manually): mark deleted anyway

---

## ghostctl CLI

```
ghostctl start          # start daemon (systemd/launchd)
ghostctl stop           # stop daemon
ghostctl status         # show: running, mood, last event, notes count
ghostctl notes          # list unread note paths
ghostctl mood           # show current mood + what triggered it
ghostctl silence 24h    # suppress notes for N hours
```

---

## File Structure

```
ghost/
├── Cargo.toml
├── src/
│   ├── main.rs
│   ├── config.rs
│   ├── watcher.rs
│   ├── store.rs
│   ├── detector.rs
│   ├── mood.rs
│   ├── ai.rs
│   ├── haunter.rs
│   ├── aliases.rs
│   └── ascii.rs
├── ghostctl/
│   └── src/main.rs
├── profiles/
│   ├── lonely_ghost.toml
│   ├── obsessive_maid.toml
│   ├── dead_veteran_programmer.toml
│   └── silent_monk.toml
├── .ghostconfig.example
└── README.md
```
