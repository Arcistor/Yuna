#!/usr/bin/env bash
set -euo pipefail

if [[ "$(uname -s)" != "Darwin" ]]; then
  echo "uninstall_launchd.sh only runs on macOS."
  exit 1
fi

LABEL="com.yuna.daemon"
PLIST_PATH="${HOME}/Library/LaunchAgents/${LABEL}.plist"

launchctl unload "$PLIST_PATH" >/dev/null 2>&1 || true
rm -f "$PLIST_PATH"

echo "Removed Yuna launch agent: $PLIST_PATH"
echo "Binaries and local data were left in place:"
echo "  ${HOME}/.local/bin/yuna"
echo "  ${HOME}/.local/bin/yunactl"
echo "  ${HOME}/.yuna"
