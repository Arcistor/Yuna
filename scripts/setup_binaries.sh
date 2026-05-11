#!/bin/bash
# scripts/setup_binaries.sh

PROJECT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
BIN_DEST="${HOME}/.local/bin"

echo "🚀 Starting Yūna Binary Setup..."

# 1. Ensure bin destination exists
mkdir -p "$BIN_DEST"

# 2. Ensure binaries are built
if [[ ! -f "$PROJECT_DIR/target/release/yuna" ]]; then
    echo "⚠️ Binaries not found in target/release. Building now..."
    cargo build --release --manifest-path "$PROJECT_DIR/Cargo.toml"
fi

# 3. Copy to ~/.local/bin
echo "📦 Copying binaries to $BIN_DEST..."
cp "$PROJECT_DIR/target/release/yuna" "$BIN_DEST/yuna"
cp "$PROJECT_DIR/target/release/yunactl" "$BIN_DEST/yunactl"
chmod +x "$BIN_DEST/yuna" "$BIN_DEST/yunactl"

# 4. Add Alias to .zshrc
echo "🔗 Adding alias 'yn' to ~/.zshrc..."
if ! grep -q "alias yn=" ~/.zshrc; then
    echo -e "\n# Yuna Alias\nalias yn='yunactl'" >> ~/.zshrc
    echo "✅ Alias 'yn' added."
else
    echo "ℹ️ Alias 'yn' already exists in .zshrc."
fi

echo "✨ Setup complete! Please run 'source ~/.zshrc' or restart your terminal."
