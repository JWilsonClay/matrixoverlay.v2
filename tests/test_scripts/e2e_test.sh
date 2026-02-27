#!/bin/bash
# End-to-End Test Suite for ASD Compliance & Functionality
# Usage: ./test_scripts/e2e_test.sh

APP_BIN="./target/release/matrix-overlay"
AUTOSTART_FILE="$HOME/.config/autostart/matrix-overlay.desktop"
REPORT_FILE="test_report.md"
LOG_FILE="e2e_app.log"

# --- Test Report Template ---
echo "# Test Report: $(date)" > $REPORT_FILE
echo "## Environment" >> $REPORT_FILE
echo "- Host: $(hostname)" >> $REPORT_FILE
echo "- Display: $DISPLAY" >> $REPORT_FILE
echo "" >> $REPORT_FILE
echo "## Results" >> $REPORT_FILE

function log_pass() {
    echo "✅ PASS: $1" | tee -a $REPORT_FILE
}

function log_fail() {
    echo "❌ FAIL: $1" | tee -a $REPORT_FILE
    # Don't exit immediately, try to finish other tests
}

function log_info() {
    echo "ℹ️ INFO: $1" | tee -a $REPORT_FILE
}

echo "=== Starting E2E Tests ==="

# 1. Build Application
echo "Building release binary..."
cargo build --release > /dev/null 2>&1
if [ $? -eq 0 ]; then
    log_pass "Compilation successful"
else
    log_fail "Compilation failed"
    exit 1
fi

# 2. Autostart Validation
# Run app briefly (timeout) to trigger autostart generation
timeout 2s $APP_BIN > /dev/null 2>&1

if [ -f "$AUTOSTART_FILE" ]; then
    if grep -q "Exec=" "$AUTOSTART_FILE"; then
        log_pass "Autostart .desktop file created and valid"
    else
        log_fail "Autostart file exists but content is invalid"
    fi
else
    log_fail "Autostart .desktop file not created"
fi

# 3. Runtime Tests (Requires X11)
if [ -z "$DISPLAY" ]; then
    log_info "Skipping interactive X11 tests (headless environment)"
else
    # Start App in Background
    $APP_BIN > $LOG_FILE 2>&1 &
    APP_PID=$!
    sleep 3 # Wait for init

    # Check if running
    if ps -p $APP_PID > /dev/null; then
        log_pass "Application started (PID $APP_PID)"
    else
        log_fail "Application crashed on startup"
        echo "=== Application Log ($LOG_FILE) ==="
        cat $LOG_FILE
        exit 1
    fi

    # 4. Hotkey Test (Ctrl+Alt+W)
    if command -v xdotool &> /dev/null; then
        log_info "Sending Hotkey Ctrl+Alt+W..."
        xdotool key ctrl+alt+w
        sleep 1
        # We can't easily verify visibility programmatically without image analysis,
        # but we verify the process didn't crash.
        if ps -p $APP_PID > /dev/null; then
            log_pass "Application survived hotkey toggle"
        else
            log_fail "Application crashed after hotkey"
        fi
    else
        log_info "xdotool not found, skipping hotkey injection"
    fi

    # 5. Dual-Monitor Flow & Uniqueness
    # Grep logs for window creation on multiple CRTCs
    WINDOW_COUNT=$(grep -c "Created overlay window" $LOG_FILE)
    if [ "$WINDOW_COUNT" -ge 2 ]; then
        log_pass "Multi-monitor detected ($WINDOW_COUNT windows created)"
        # Check for uniqueness warning in logs
        if grep -q "low content uniqueness" $LOG_FILE; then
            log_info "Uniqueness check triggered (Warning found in logs)"
        else
            log_pass "Per-monitor content uniqueness satisfied (No warnings)"
        fi
    else
        log_info "Single monitor detected ($WINDOW_COUNT window)"
    fi

    # 6. Stability Recording (ASD Compliance)
    if command -v ffmpeg &> /dev/null; then
        log_info "Recording 30s stability video for ASD verification..."
        ffmpeg -y -f x11grab -draw_mouse 0 -framerate 10 -video_size 1920x1080 -i $DISPLAY -t 30 stability_test.mp4 > /dev/null 2>&1
        if [ -f "stability_test.mp4" ]; then
            log_pass "Stability video recorded: stability_test.mp4"
        else
            log_fail "Video recording failed"
        fi
    else
        log_info "ffmpeg not found, skipping video recording"
    fi

    # Cleanup
    kill $APP_PID
    wait $APP_PID 2>/dev/null
    log_info "Application stopped"
fi

echo "=== Tests Complete. See $REPORT_FILE ==="
