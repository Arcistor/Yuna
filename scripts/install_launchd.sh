#!/usr/bin/env bash
set -euo pipefail

if [[ "$(uname -s)" != "Darwin" ]]; then
  echo "install_launchd.sh only runs on macOS."
  exit 1
fi

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
BIN_DIR="${HOME}/.local/bin"
GHOST_DIR="${HOME}/.ghost"
PLIST_DIR="${HOME}/Library/LaunchAgents"
LABEL="com.digital-ghost.daemon"
PLIST_PATH="${PLIST_DIR}/${LABEL}.plist"

mkdir -p "$BIN_DIR" "$GHOST_DIR" "$PLIST_DIR"

echo "Building release binaries..."
cargo build --manifest-path "$ROOT_DIR/Cargo.toml" --release --bins

cp "$ROOT_DIR/target/release/ghost" "$BIN_DIR/ghost"
cp "$ROOT_DIR/target/release/ghostctl" "$BIN_DIR/ghostctl"

cat > "$PLIST_PATH" <<EOF_PLIST
<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
  <key>Label</key>
  <string>${LABEL}</string>
  <key>ProgramArguments</key>
  <array>
    <string>${BIN_DIR}/ghost</string>
  </array>
  <key>WorkingDirectory</key>
  <string>${HOME}</string>
  <key>EnvironmentVariables</key>
  <dict>
    <key>HOME</key>
    <string>${HOME}</string>
  </dict>
  <key>RunAtLoad</key>
  <true/>
  <key>KeepAlive</key>
  <true/>
  <key>StandardOutPath</key>
  <string>${GHOST_DIR}/ghost.out.log</string>
  <key>StandardErrorPath</key>
  <string>${GHOST_DIR}/ghost.err.log</string>
</dict>
</plist>
EOF_PLIST

launchctl unload "$PLIST_PATH" >/dev/null 2>&1 || true
launchctl load "$PLIST_PATH"

echo "Installed Digital Ghost launch agent: $PLIST_PATH"
echo "Binaries installed to: $BIN_DIR"
echo "Check status with: ghostctl status"
