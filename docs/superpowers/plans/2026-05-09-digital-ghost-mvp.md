# Digital Ghost MVP Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Build a working local-first Rust daemon and CLI for the core Digital Ghost behavior loop.

**Architecture:** Use one Cargo package with shared library modules and two binaries. Persist event, mood, note, and silence state in SQLite; use event-driven `notify` for file changes; call Ollama for generated note text with a fallback note when unavailable.

**Tech Stack:** Rust, Tokio, notify, rusqlite, reqwest, serde, toml, chrono, clap, anyhow, tempfile.

---

### Task 1: Scaffold and Core Types

**Files:**
- Create: `Cargo.toml`
- Create: `src/lib.rs`
- Create: `src/types.rs`
- Create: `src/bin/ghost.rs`
- Create: `src/bin/ghostctl.rs`

- [ ] Create the Cargo package and shared module declarations.
- [ ] Add typed `Behavior`, `MoodState`, `EventKind`, `EventRecord`, and `NoteRecord`.
- [ ] Add placeholder binaries that call library entry points.
- [ ] Run `cargo test` and confirm the crate compiles before feature modules are filled in.

### Task 2: Config and Store

**Files:**
- Create: `src/config.rs`
- Create: `src/store.rs`
- Test: `tests/config_store.rs`
- Create: `.ghostconfig.example`

- [ ] Write failing tests for config loading and SQLite CRUD.
- [ ] Implement `.ghostconfig` parsing from a supplied path plus current-dir/home fallback.
- [ ] Implement SQLite schema creation and store methods for events, mood, notes, cooldowns, and silence.
- [ ] Run `cargo test --test config_store`.

### Task 3: Mood and Detectors

**Files:**
- Create: `src/mood.rs`
- Create: `src/detector.rs`
- Test: `tests/detector_mood.rs`

- [ ] Write failing tests for mood transitions, cleaning detection, procrastinator detection, typo detection, and cooldown suppression.
- [ ] Implement the state machine and detector SQL/history logic.
- [ ] Run `cargo test --test detector_mood`.

### Task 4: Notes and AI

**Files:**
- Create: `src/ascii.rs`
- Create: `src/ai.rs`
- Create: `src/haunter.rs`
- Test: `tests/haunter_ai.rs`

- [ ] Write failing tests for prompt content, fallback note generation, note file creation, and reaping read notes.
- [ ] Implement personality profile loading, Ollama request logic, ASCII selection, note dropping, and note reaping.
- [ ] Run `cargo test --test haunter_ai`.

### Task 5: Watcher, Daemon, CLI

**Files:**
- Create: `src/watcher.rs`
- Create: `src/app.rs`
- Modify: `src/bin/ghost.rs`
- Modify: `src/bin/ghostctl.rs`

- [ ] Implement the `notify` watcher glue.
- [ ] Implement daemon startup, event loop, mood/note pipeline, and reaper task.
- [ ] Implement `ghostctl status`, `notes`, `mood`, and `silence`.
- [ ] Run `cargo fmt`, `cargo test`, and `cargo build --release`.
