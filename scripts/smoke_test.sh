#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
RUN_DIR="$(mktemp -d "${TMPDIR:-/tmp}/digital-ghost-smoke.XXXXXX")"
HOME_DIR="$RUN_DIR/home"
WATCH_DIR="$RUN_DIR/watch"
CONFIG_DIR="$RUN_DIR/config"
LOG_FILE="$RUN_DIR/ghost.log"

cleanup() {
  (cd "$CONFIG_DIR" && HOME="$HOME_DIR" "$ROOT_DIR/target/debug/ghostctl" stop >/dev/null 2>&1) || true
  rm -rf "$RUN_DIR"
}
trap cleanup EXIT

mkdir -p "$HOME_DIR" "$WATCH_DIR" "$CONFIG_DIR"

cat > "$CONFIG_DIR/.ghostconfig" <<EOF_CONFIG
[ghost]
personality = "lonely_ghost"
ollama_model = "mistral"
ollama_url = "http://127.0.0.1:9"

[watch]
paths = ["$WATCH_DIR"]
exclude = []

[behavior]
alias_injection = false
note_lifetime_minutes = 60

[limits]
max_cpu_percent = 0.5
cooldown_hours = 0
EOF_CONFIG

echo "Building debug binaries..."
cargo build --manifest-path "$ROOT_DIR/Cargo.toml" --bins >/dev/null

echo "Starting ghost with temp HOME at $HOME_DIR"
(
  cd "$CONFIG_DIR"
  HOME="$HOME_DIR" "$ROOT_DIR/target/debug/ghostctl" start
) >"$LOG_FILE" 2>&1

sleep 2

echo "Triggering cleaning behavior..."
for index in $(seq 1 12); do
  touch "$WATCH_DIR/junk-$index.tmp"
done
sleep 1
rm "$WATCH_DIR"/junk-*.tmp

NOTE_PATH=""
for _ in $(seq 1 30); do
  NOTE_PATH="$(find "$WATCH_DIR" -maxdepth 1 -type f \( -name '.ghost_note*' -o -name 'MESSAGE_FROM_THE_VOID*' -o -name '.thankyou*' -o -name 'DO_NOT_READ_ME*' \) | head -n 1)"
  if [[ -n "$NOTE_PATH" ]]; then
    break
  fi
  sleep 1
done

if [[ -z "$NOTE_PATH" ]]; then
  echo "Smoke test failed: no note appeared in $WATCH_DIR"
  echo "--- ghost log ---"
  cat "$LOG_FILE"
  exit 1
fi

echo "Note appeared: $NOTE_PATH"
echo "--- note preview ---"
sed -n '1,12p' "$NOTE_PATH"

echo "--- ghostctl status ---"
(cd "$CONFIG_DIR" && HOME="$HOME_DIR" "$ROOT_DIR/target/debug/ghostctl" status)

echo "Smoke test passed."
