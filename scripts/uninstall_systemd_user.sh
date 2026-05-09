#!/usr/bin/env bash
set -euo pipefail

if [[ "$(uname -s)" != "Linux" ]]; then
  echo "uninstall_systemd_user.sh only runs on Linux."
  exit 1
fi

UNIT_PATH="${HOME}/.config/systemd/user/digital-ghost.service"

systemctl --user disable --now digital-ghost.service >/dev/null 2>&1 || true
rm -f "$UNIT_PATH"
systemctl --user daemon-reload

echo "Removed Digital Ghost systemd user service: $UNIT_PATH"
echo "Binaries and local data were left in place:"
echo "  ${HOME}/.local/bin/ghost"
echo "  ${HOME}/.local/bin/ghostctl"
echo "  ${HOME}/.ghost"
