#!/bin/bash
# Install WhatsApp Desktop for the current user

set -e

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
INSTALL_DIR="$HOME/.local/bin"
ICON_DIR="$HOME/.local/share/icons"
APP_DIR="$HOME/.local/share/applications"

# Create directories if they don't exist
mkdir -p "$INSTALL_DIR" "$ICON_DIR" "$APP_DIR"

# Copy binary
echo "Installing binary to $INSTALL_DIR..."
cp "$SCRIPT_DIR/target/release/whatsapp-desktop" "$INSTALL_DIR/"
chmod +x "$INSTALL_DIR/whatsapp-desktop"

# Copy icon
echo "Installing icon to $ICON_DIR..."
cp "$SCRIPT_DIR/assets/icon.png" "$ICON_DIR/whatsapp.png"

# Create desktop entry with correct paths
echo "Creating desktop entry..."
sed -e "s|BINARY_PATH|$INSTALL_DIR/whatsapp-desktop|" \
    -e "s|ICON_PATH|$ICON_DIR/whatsapp.png|" \
    "$SCRIPT_DIR/whatsapp.desktop" > "$APP_DIR/whatsapp.desktop"

# Update desktop database
if command -v update-desktop-database &> /dev/null; then
    update-desktop-database "$APP_DIR" 2>/dev/null || true
fi

echo ""
echo "Installation complete!"
echo "You can now:"
echo "  1. Find 'WhatsApp' in your application menu"
echo "  2. Right-click it to pin to taskbar/dock"
echo "  3. Or run: whatsapp-desktop (if ~/.local/bin is in PATH)"
