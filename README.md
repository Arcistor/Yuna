# Yūna (幽奈) 👻🌸

Yūna is a local-first ambient digital spirit for programmers. She haunts your filesystem, notices your patterns, and leaves short, personality-driven notes in your directories. She is not a chatbot, dashboard, or productivity tracker. She is a mysterious presence that lives beside your work.

Everything stays local: filesystem events are stored in SQLite under `~/.yuna`, notes are generated through your local Ollama server, and no telemetry is sent anywhere.

## Current Features

- Event-based filesystem watching with `notify`
- SQLite event, mood, note, cooldown, and silence state
- Behavior detectors for cleanup bursts, late-night work, untouched new projects, and repeated command typos
- Mood state machine: `calm`, `watching`, `concerned`, `amused`, `grateful`
- Ollama note generation with a local fallback when Ollama is unavailable
- Self-deleting notes after they have been read and their lifetime expires
- `yunactl` CLI for `start`, `stop`, `status`, `notes`, `mood`, and `silence`
- Opt-in shell alias injection for repeated typos
- Built-in personalities (including the default `yuna`) plus custom profile loading

## Build

```bash
cargo build --release
```

This produces:

- `target/release/yuna`
- `target/release/yunactl`

## Configure

Copy `.yunaconfig.example` to either your current working directory or your home directory:

```bash
cp .yunaconfig.example .yunaconfig
```

The daemon checks the current directory first, then falls back to `~/.yunaconfig`.

```toml
[yuna]
personality = "yuna"
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
cooldown_seconds = 60
```

## Run

Start the daemon:

```bash
target/release/yunactl start
```

Check status:

```bash
target/release/yunactl status
```

Stop it:

```bash
target/release/yunactl stop
```

Other commands:

```bash
target/release/yunactl notes
target/release/yunactl mood
target/release/yunactl log
target/release/yunactl silence 2h
```

## Personalities

Built-in profiles:

- `yuna`: Mysterious, slightly melancholic, and poetic. The default spirit.

Custom profiles live at:

```text
~/.yuna/profiles/<name>.toml
```

## Alias Injection

When `behavior.alias_injection = true`, repeated typo behavior such as `gti status` can trigger Yūna to rearrange your shell's memories by adding an alias to your shell rc file:

```bash
# Added by Yūna
alias gti='git'
```

## Notes

Notes are dropped into the triggering directory, usually as `.yuna_note`, with ASCII mood art and a short message.

## Data Location

Yūna stores local state here:

```text
~/.yuna/yuna.db
~/.yuna/yuna.pid
~/.yuna/profiles/
```
