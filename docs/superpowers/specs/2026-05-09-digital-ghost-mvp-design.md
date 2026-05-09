# Digital Ghost MVP Design

## Scope

Build the Phase 1-3 MVP from the project plan: a Rust daemon named `ghost`, a CLI named `ghostctl`, local config loading, SQLite persistence, event-based filesystem watching, behavior detection, mood updates, Ollama-backed note generation, note writing, and note reaping.

Alias injection, service installation, and full start/stop daemon management are out of scope for this slice. `ghostctl` will expose status, notes, mood, and silence commands against the shared SQLite database.

## Architecture

The project is a single Cargo package with shared library modules in `src/` and two binaries in `src/bin/`. The daemon loads config, opens `~/.ghost/ghost.db`, starts a `notify` watcher, receives file events through a Tokio channel, records them, detects behaviors, updates mood, asks Ollama for a note, and writes the resulting note into the triggering directory.

SQLite owns persistent state for events, mood, notes, and silence windows. The implementation keeps module boundaries small: config parsing, store access, detector queries, mood transitions, AI prompt/client logic, note lifecycle, watcher glue, and CLI presentation each live in separate files.

## Data Flow

`watcher` normalizes filesystem events into `GhostEvent` values. `main` writes each event to `Store`, then calls `detector::detect`. When a `Behavior` is found and the store is not silenced, `mood::update_mood` chooses the new state, `ai::generate_note` builds and sends the Ollama request, and `haunter::drop_note` writes the note plus ASCII mood art and records it in SQLite.

The reaper runs on a Tokio interval. It asks the store for undeleted notes, checks filesystem access time when available, and deletes notes whose read time exceeds `note_lifetime_minutes`.

## Error Handling

Production paths return `anyhow::Result` and propagate recoverable errors with context. The daemon logs failures with `eprintln!` and continues processing later events where possible. Ollama failures fall back to a short in-character local note so the ghost can still haunt without network availability.

## Testing

Tests cover config loading, SQLite schema and CRUD behavior, cooldown/silence logic, mood transitions, detector behavior for cleaning/procrastination/typos, note writing, and note reaping. Watcher integration is kept thin because `notify` behavior varies by platform; the watcher module is compiled and exercised indirectly through type-level construction and daemon build verification.
