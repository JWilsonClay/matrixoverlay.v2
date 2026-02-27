#!/bin/bash
# install_prereqs.sh
# Installs dependencies and builds the matrix-overlay project.

set -e

echo "=== Matrix Overlay Setup ==="

echo "1. Installing system dependencies..."
sudo apt update && sudo apt install -y \
    libxcb1-dev \
    libcairo2-dev \
    libpango1.0-dev \
    libayatana-appindicator3-dev \
    lm-sensors \
    fonts-dejavu-core \
    x11-xserver-utils \
    cargo \
    libssl-dev \
    pkg-config \
    libxdo-dev

echo "2. Installing project via Cargo..."
cargo install --path .

echo "=== Setup Complete ==="
echo "Note: If you have an NVIDIA GPU, ensure proprietary drivers are installed for GPU stats."
echo "Suggestion: The application runs 'xsetroot -solid \"#000000\"' on startup via main.rs."
echo "Ensure your window manager supports this for the best visual effect."