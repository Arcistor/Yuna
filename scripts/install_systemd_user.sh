#!/usr/bin/env bash
set -euo pipefail

if [[ "$(uname -s)" != "Linux" ]]; then
  echo "install_systemd_user.sh only runs on Linux."
  exit 1
fi

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
BIN_DIR="${HOME}/.local/bin"
UNIT_DIR="${HOME}/.config/systemd/user"
UNIT_PATH="${UNIT_DIR}/digital-ghost.service"

mkdir -p "$BIN_DIR" "$UNIT_DIR" "${HOME}/.ghost"

echo "Building release binaries..."
cargo build --manifest-path "$ROOT_DIR/Cargo.toml" --release --bins

cp "$ROOT_DIR/target/release/ghost" "$BIN_DIR/ghost"
cp "$ROOT_DIR/target/release/ghostctl" "$BIN_DIR/ghostctl"

cat > "$UNIT_PATH" <<EOF_UNIT
[Unit]
Description=Digital Ghost
After=default.target

[Service]
Type=simple
ExecStart=${BIN_DIR}/ghost
WorkingDirectory=${HOME}
Environment=HOME=${HOME}
Restart=on-failure
RestartSec=10

[Install]
WantedBy=default.target
EOF_UNIT

systemctl --user daemon-reload
systemctl --user enable --now digital-ghost.service

echo "Installed Digital Ghost systemd user service: $UNIT_PATH"
echo "Binaries installed to: $BIN_DIR"
echo "Check status with: systemctl --user status digital-ghost.service"
