#!/bin/bash
# debug_cycle.sh - Automated Repair Workflow for Matrix Overlay v2

LOG_DIR="/tmp/matrix_overlay_logs"
mkdir -p "$LOG_DIR"
BUILD_LOG="$LOG_DIR/build.log"
APP_LOG="$LOG_DIR/matrix_overlay.log"
STATE_LOG="$LOG_DIR/state.log"

echo "=== Stage 1: Clean & Kill ==="
pkill matrix-overlay
sleep 1
rm -f "$APP_LOG" "$STATE_LOG"

echo "=== Stage 2: Build (Logged) ==="
cargo run -- debug-build
if [ $? -ne 0 ]; then
    echo "ERROR: Build failed. Check $BUILD_LOG"
    exit 1
fi

echo "=== Stage 3: Launch ==="
# Force enable logging in config.json if not already enabled (simple sed)
# sed -i 's/"enabled": false/"enabled": true/' ~/.config/matrix-overlay/config.json

cargo run --release &
APP_PID=$!
echo "Launched matrix-overlay (PID: $APP_PID)"

echo "=== Stage 4: Monitor & Capture (30s) ==="
sleep 30

if ps -p $APP_PID > /dev/null; then
    echo "App is still running. Capturing state..."
    if [ -f "$STATE_LOG" ]; then
        echo "State log captured: $(tail -n 5 "$STATE_LOG")"
    else
        echo "WARNING: No state log found yet."
    fi
else
    echo "ERROR: App crashed during monitoring. Check $APP_LOG"
fi

echo "=== Stage 5: Summary ==="
echo "Logs available in $LOG_DIR"
echo "Build Log Path: $BUILD_LOG"
echo "App Log Path: $APP_LOG"

echo "Iteration complete."
