# Yuna — Project Plan

## Phase 1: Observer (Eye)

Goals: daemon runs, watches filesystem, logs events.

- [ ] Project scaffold — Rust with Cargo, single binary
- [ ] Read `.yunaconfig` file (watched paths, excluded paths, personality setting)
- [ ] Filesystem watcher using `notify` crate (event-based, not polling)
- [ ] Event types to track:
  - File created / modified / deleted
  - Session timing (first event timestamp, last event timestamp per directory)
  - Terminal alias file changes (`.bashrc`/`.zshrc`)
- [ ] SQLite store via `rusqlite` — persist event log with timestamps
- [ ] Background daemon mode (no terminal UI, silent)

Deliverable: daemon starts, watches configured paths, writes events to SQLite silently.

---

## Phase 2: Memory (Soul State)

Goals: yuna builds behavioral model from event history.

- [ ] Behavior detector functions:
  - `is_midnight_worker()` — 4+ hours editing code files past midnight
  - `is_procrastinator(dir)` — project folder created, no changes for 3 days
  - `is_cleaning()` — 10+ deletes in short window
  - `is_typo_repeater()` — same mistyped command 3+ times (read shell history)
- [ ] Ghost mood state machine: `calm | watching | concerned | amused | grateful`
  - Mood shifts based on detected behaviors
  - Mood stored in SQLite, persists across restarts
- [ ] Cooldown system — same behavior doesn't trigger note twice within 24 hours

Deliverable: yuna correctly classifies behaviors and tracks mood without writing any notes yet.

---

## Phase 3: Voice (Haunting)

Goals: yuna writes notes, manages their lifecycle.

- [ ] Ollama integration via HTTP (`reqwest` crate) — call local LLM
- [ ] System prompt builder — loads personality from `.yunaconfig`, injects current mood + behavior context
- [ ] Note writer — drops `.yuna_note` or `MESSAGE_FROM_YUNA.txt` in triggering directory
- [ ] ASCII art generator — small mood-matched ASCII embedded in note header
- [ ] Self-delete scheduler — track note `atime`; delete 1 hour after read
- [ ] Alias injector (opt-in) — appends alias to `.bashrc`/`.zshrc`, leaves note about it

Deliverable: yuna writes personality-driven notes in response to real behaviors.

---

## Phase 4: Personality Profiles

Goals: swappable yuna personalities.

- [ ] Built-in profiles: `yuna`, `obsessive_maid`, `dead_veteran_programmer`, `silent_monk`
- [ ] Each profile: name, tone descriptors, vocabulary hints, ASCII art style
- [ ] Profile loaded into Ollama system prompt at startup
- [ ] User can write custom profiles as `.toml` files in `~/.yuna/profiles/`

---

## Phase 5: Polish & Safety

- [ ] Resource guard — max 0.5% CPU average, event-based only (no polling loops)
- [ ] Path exclusion — never watch system dirs (`/sys`, `/proc`, `/etc`, etc.)
- [ ] `yunactl` CLI: `start`, `stop`, `status`, `notes` (list unread), `mood` (show current state)
- [ ] Install script — systemd service (Linux) or launchd plist (macOS)
- [ ] README with setup, config reference, example notes

---

## Tech Stack

| Concern | Choice | Reason |
|---|---|---|
| Language | Rust | Zero-cost background, memory safe, single binary |
| FS watching | `notify` crate | Event-based, cross-platform, no polling |
| LLM | Ollama (`mistral` or `tinyllama`) | Local, private, fast enough for notes |
| Database | SQLite via `rusqlite` | Lightweight, no server, persists history |
| HTTP client | `reqwest` | Ollama API calls |
| Config | TOML via `toml` crate | Human-writable `.yunaconfig` |
| Scheduling | `tokio` async runtime | Non-blocking timers, atime checks |
