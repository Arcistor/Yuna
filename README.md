# Digital Ghost in the Machine

Digital Ghost is a local-first ambient daemon for programmers. It watches your filesystem, notices patterns, and leaves short personality-driven notes in your directories. It is not a chatbot, dashboard, or productivity tracker. It is a small strange presence that lives beside your work.

Everything stays local: filesystem events are stored in SQLite under `~/.ghost`, notes are generated through your local Ollama server, and no telemetry is sent anywhere.

## Current Features

- Event-based filesystem watching with `notify`
- SQLite event, mood, note, cooldown, and silence state
- Behavior detectors for cleanup bursts, late-night work, untouched new projects, and repeated command typos
- Mood state machine: `calm`, `watching`, `concerned`, `amused`, `grateful`
- Ollama note generation with a local fallback when Ollama is unavailable
- Self-deleting notes after they have been read and their lifetime expires
- `ghostctl` CLI for `start`, `stop`, `status`, `notes`, `mood`, and `silence`
- Opt-in shell alias injection for repeated typos
- Built-in personalities plus custom profile loading

## Build

```bash
cargo build --release
```

This produces:

- `target/release/ghost`
- `target/release/ghostctl`

For development:

```bash
cargo test
cargo clippy --all-targets -- -D warnings
scripts/smoke_test.sh
```

## Configure

Copy `.ghostconfig.example` to either your current working directory or your home directory:

```bash
cp .ghostconfig.example .ghostconfig
```

The daemon checks the current directory first, then falls back to `~/.ghostconfig`.

```toml
[ghost]
personality = "lonely_ghost"
ollama_model = "mistral"
ollama_url = "http://localhost:11434"

[watch]
paths = ["/Users/you/projects"]
exclude = ["/Users/you/projects/node_modules", "/Users/you/projects/.git"]

[behavior]
alias_injection = false
note_lifetime_minutes = 60

[limits]
max_cpu_percent = 0.5
cooldown_hours = 24
```

`watch.paths` should be directories you own. Digital Ghost automatically adds safety excludes for system paths such as `/proc`, `/sys`, `/dev`, `/etc`, `/System`, and `/Library`.

## Run

Start the daemon:

```bash
target/release/ghostctl start
```

Check status:

```bash
target/release/ghostctl status
```

Stop it:

```bash
target/release/ghostctl stop
```

Other commands:

```bash
target/release/ghostctl notes
target/release/ghostctl mood
target/release/ghostctl silence 2h
```

`silence` accepts `m`, `h`, and `d` suffixes.

## Install As A User Service

The scripts install Digital Ghost as a user-level service. They do not require root.

macOS:

```bash
scripts/install_launchd.sh
```

This builds release binaries, copies `ghost` and `ghostctl` into `~/.local/bin`, writes `~/Library/LaunchAgents/com.digital-ghost.daemon.plist`, and loads it with `launchctl`.

Uninstall the launch agent:

```bash
scripts/uninstall_launchd.sh
```

Linux with systemd user services:

```bash
scripts/install_systemd_user.sh
```

This builds release binaries, copies `ghost` and `ghostctl` into `~/.local/bin`, writes `~/.config/systemd/user/digital-ghost.service`, and enables it with `systemctl --user`.

Uninstall the systemd user service:

```bash
scripts/uninstall_systemd_user.sh
```

Uninstall scripts remove only the service file. They leave binaries and local data in place:

```text
~/.local/bin/ghost
~/.local/bin/ghostctl
~/.ghost/
```

## Personalities

Built-in profiles:

- `lonely_ghost`: melancholic, wry, jealous of finished work
- `obsessive_maid`: tidy, fussy, quietly judgmental
- `dead_veteran_programmer`: gruff, terse, unimpressed old C programmer energy
- `silent_monk`: sparse, calm, heavy silence

Custom profiles live at:

```text
~/.ghost/profiles/<name>.toml
```

Example:

```toml
name = "supportive_spirit"
description = "A gentle presence that notices effort without turning into a productivity coach."
tone = ["warm", "brief", "encouraging", "ambient"]
ascii_style = "soft"
```

See [examples/profiles/supportive_spirit.toml](examples/profiles/supportive_spirit.toml).

## Alias Injection

Alias injection is disabled by default.

When `behavior.alias_injection = true`, repeated typo behavior such as `gti status` can append a managed line to your shell rc file:

```bash
# Added by Digital Ghost
alias gti='git'
```

Digital Ghost chooses `.zshrc`, `.bashrc`, or `.profile` from `$SHELL`, avoids duplicate alias lines, and leaves a note saying exactly what changed.

## Notes

Notes are dropped into the triggering directory, usually as `.ghost_note`, with ASCII mood art and a short message. If Ollama is not running, Digital Ghost writes a local fallback note so the behavior still works.

Notes are tracked in SQLite. After a note is read and `note_lifetime_minutes` has elapsed, the reaper deletes it and marks it deleted in the database.

## Data Location

Digital Ghost stores local state here:

```text
~/.ghost/ghost.db
~/.ghost/ghost.pid
~/.ghost/profiles/
```

No network is used except calls to the configured local Ollama URL.

## Smoke Test

Run:

```bash
scripts/smoke_test.sh
```

The smoke test creates a temp home/config/watch directory, starts the daemon through `ghostctl start`, triggers cleanup behavior, verifies a note appears, checks `ghostctl status`, and stops the daemon.
