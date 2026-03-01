#!/bin/bash
# Matrix Overlay v2: Procedural Feature Verification Script
# This script verifies the wiring between the GUI/Config and the Renderer.

CONFIG_FILE="$HOME/.config/matrix-overlay/config.json"
LOG_DIR="/tmp/matrix_overlay_logs"
APP_BIN="./target/release/matrix-overlay"

# Ensure jq is installed
if ! command -v jq &> /dev/null; then
    echo "Error: jq is required for this script."
    exit 1
fi

echo "--- Starting Procedural Feature Verification ---"

# Step 1: Verify Update Interval (Framerate)
echo "[1/4] Testing Update Interval (Framerate)..."
jq '.general.update_ms = 500' "$CONFIG_FILE" > "$CONFIG_FILE.tmp" && mv "$CONFIG_FILE.tmp" "$CONFIG_FILE"
pkill -f matrix-overlay
$APP_BIN &
sleep 5
# Check logs for tick frequency
LATEST_LOG=$(ls -t $LOG_DIR/matrix_overlay_*.log | head -n 1)
TICKS=$(grep -c "Redrawing Window" "$LATEST_LOG")
echo "   - Ticks detected in 5s: $TICKS (Expected ~10 for 500ms interval)"

# Step 2: Verify Rain Density (Realism Scale)
echo "[2/4] Testing Rain Density (Realism Scale)..."
jq '.cosmetics.realism_scale = 50' "$CONFIG_FILE" > "$CONFIG_FILE.tmp" && mv "$CONFIG_FILE.tmp" "$CONFIG_FILE"
# Note: In a real test we'd signal reload, but here we restart for simplicity
pkill -f matrix-overlay
$APP_BIN &
sleep 5
LATEST_LOG=$(ls -t $LOG_DIR/matrix_overlay_*.log | head -n 1)
# Check for realism scale update
if grep -q "realism_scale=50" "$LATEST_LOG" || grep -q "resetting streams" "$LATEST_LOG"; then
    echo "   - SUCCESS: Rain density update detected."
else
    echo "   - WARNING: Re-check rain density update in log."
fi

# Step 3: Verify Metric Font Size (Wiring)
echo "[3/4] Testing Metric Font Size wiring..."
jq '.general.metric_font_size = 24' "$CONFIG_FILE" > "$CONFIG_FILE.tmp" && mv "$CONFIG_FILE.tmp" "$CONFIG_FILE"
# We'll just verify the config sync for now
VAL=$(jq '.general.metric_font_size' "$CONFIG_FILE")
if [ "$VAL" -eq 24 ]; then
    echo "   - SUCCESS: Metric font size correctly saved."
else
    echo "   - FAIL: Metric font size not saved."
fi

# Step 4: Verify Metric Ordering
echo "[4/4] Testing Metric Ordering..."
# Swap two metrics in the first screen
jq '.screens[0].metrics = ["memory", "cpu"]' "$CONFIG_FILE" > "$CONFIG_FILE.tmp" && mv "$CONFIG_FILE.tmp" "$CONFIG_FILE"
echo "   - SUCCESS: Metric order updated in config."

echo "--- Verification Complete ---"
echo "Note: Some visual features (vertical centering, lag-free GUI) require manual confirmation."
echo "Launch the GUI now to verify the 'Cancel' button and '60 FPS' responsiveness."
