# Contributing To Digital Ghost

Thanks for helping with Digital Ghost. The project is meant to stay small, local-first, private, and atmospheric. Changes should preserve that feeling: no cloud services, no dashboards, no telemetry, and no surprise edits to user files.

## Project Shape

- `src/bin/ghost.rs`: daemon entry point
- `src/bin/ghostctl.rs`: CLI entry point
- `src/config.rs`: `.ghostconfig` loading and safety excludes
- `src/store.rs`: SQLite schema and persistence
- `src/watcher.rs`: event-based filesystem watching
- `src/detector.rs`: behavior detectors
- `src/mood.rs`: mood state machine
- `src/ai.rs`: profile loading, prompt building, Ollama client, fallback notes
- `src/haunter.rs`: note writing and reaping
- `src/aliases.rs`: opt-in shell alias injection
- `src/installer.rs`: launchd/systemd service template rendering
- `tests/`: integration-style tests for each major subsystem
- `scripts/smoke_test.sh`: end-to-end local smoke test with a temp `HOME`

## Local Setup

Install Rust, then run:

```bash
cargo build
cargo test
```

For generated notes through a model, run Ollama locally and set `ghost.ollama_url` in `.ghostconfig`. Tests and the smoke test do not require Ollama; they verify the fallback note path too.

## Verification Before Submitting

Run the full check set:

```bash
cargo fmt --check
cargo test
cargo clippy --all-targets -- -D warnings
cargo build --release
scripts/smoke_test.sh
```

The smoke test creates a temporary home/config/watch directory, starts `ghost` through `ghostctl start`, triggers cleanup behavior, checks that a note appears, prints `ghostctl status`, and stops the daemon. It should not touch your real `~/.ghost` data.

## Coding Guidelines

- Prefer small modules with clear boundaries.
- Keep behavior local-first. Do not add external network calls except the configured local Ollama endpoint.
- Do not add telemetry, analytics, remote logging, or cloud sync.
- Avoid `unwrap()` in production paths. Use `anyhow::Result`, `?`, and context-rich errors.
- Keep watcher behavior event-driven. Do not add polling loops for filesystem detection.
- Use deterministic tests for detector/store/config logic.
- Keep generated note text short and in-character.
- Preserve the daemon’s quiet nature: no terminal UI, popups, dashboards, or notifications.

## Safety Rules

Digital Ghost observes user files, so changes need a little extra care.

- Watch paths should be user-owned directories.
- System directories must remain excluded by default.
- Alias injection must stay opt-in through `behavior.alias_injection = true`.
- Any feature that edits user files must be idempotent and tested.
- Installer scripts must install user-level services only. Do not require root.
- Uninstall scripts should avoid deleting user data unless explicitly designed and documented.

## Testing Feature Areas

When changing config or safety excludes:

```bash
cargo test --test config_store
```

When changing behavior detection or mood:

```bash
cargo test --test detector_mood
```

When changing notes, profiles, prompts, or Ollama behavior:

```bash
cargo test --test haunter_ai
```

When changing daemon status/start/stop:

```bash
cargo test --test daemon_control
scripts/smoke_test.sh
```

When changing alias injection:

```bash
cargo test --test aliases
```

When changing service installers:

```bash
cargo test --test installer
```

## Config And Data

The daemon loads `.ghostconfig` from the current directory first, then `~/.ghostconfig`.

Local state lives under:

```text
~/.ghost/ghost.db
~/.ghost/ghost.pid
~/.ghost/profiles/
```

Avoid tests that depend on the real home directory. Prefer `tempfile` in Rust tests and temp `HOME` directories in shell scripts.

## Pull Request Checklist

- The change matches the project philosophy: ambient, private, local-first.
- Tests cover new behavior or changed behavior.
- `cargo fmt --check`, `cargo test`, `cargo clippy --all-targets -- -D warnings`, and `scripts/smoke_test.sh` pass.
- User-file mutations are opt-in, idempotent, and documented.
- README or `.ghostconfig.example` is updated when user-facing behavior changes.

## Release Notes

For user-facing changes, summarize:

- What behavior changed
- Whether config changes are needed
- Whether local data or service files are affected
- Any safety or privacy implications
