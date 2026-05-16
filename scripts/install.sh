#!/bin/bash
# scripts/install.sh
# Sovereign Installation Substrate for Matrix Overlay v2.

set -e

echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
echo "Initializing Matrix Overlay Production Deployment..."
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"

# 1. Compile Release Binary
echo "[1/4] Compiling high-performance release substrate..."
cargo build --release

# 2. Deploy Binary to Local Path
echo "[2/4] Installing binary to ~/.local/bin/..."
mkdir -p ~/.local/bin
cp target/release/matrix-overlay ~/.local/bin/matrix-overlay
chmod +x ~/.local/bin/matrix-overlay

# 3. Setup Configuration Substrate
echo "[3/4] Initializing configuration directory..."
mkdir -p ~/.config/matrix-overlay
if [ ! -f ~/.config/matrix-overlay/config.json ]; then
    cp config.example.json ~/.config/matrix-overlay/config.json
fi

# 4. Configure Autostart
echo "[4/4] Establishing persistence (Autostart)..."
mkdir -p ~/.config/autostart
cat <<EOF > ~/.config/autostart/matrix-overlay.desktop
[Desktop Entry]
Type=Application
Name=Matrix Overlay
Comment=Sovereign HUD Substrate
Exec=$HOME/.local/bin/matrix-overlay
Terminal=false
Icon=system-run
Categories=Utility;
X-GNOME-Autostart-enabled=true
EOF

echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
echo "INSTALLATION COMPLETE"
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
echo "The HUD is now installed as a system utility."
echo "Location: ~/.local/bin/matrix-overlay"
echo "Autostart: Enabled"
echo ""
echo "You can now run 'matrix-overlay' from anywhere."
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
