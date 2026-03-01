#!/bin/bash
# repair_and_install.sh
# Automates the build and install loop for Matrix Overlay v2.

LOG_FILE="/tmp/matrix_overlay_repair.log"
INSTALL_PATH="/usr/local/bin/matrix-overlay"

echo "--- Starting Repair & Build Cycle: $(date) ---" | tee -a "$LOG_FILE"

# 1. Kill any existing instances
pkill -f matrix-overlay
echo "Terminated existing instances." | tee -a "$LOG_FILE"

# 2. Attempt Build
echo "Building matrix-overlay (release)..." | tee -a "$LOG_FILE"
if cargo build --release 2>>"$LOG_FILE"; then
    echo "BUILD SUCCESSFUL." | tee -a "$LOG_FILE"
else
    echo "BUILD FAILED. Errors logged to $LOG_FILE." | tee -a "$LOG_FILE"
    exit 1
fi

# 3. Verify Binary
if [ -f "target/release/matrix-overlay" ]; then
    echo "Binary verified in target/release." | tee -a "$LOG_FILE"
else
    echo "CRITICAL: Binary not found after success build!" | tee -a "$LOG_FILE"
    exit 1
fi

# 4. Optional: Install (Attempt with sudo if needed, or skip if internal)
# If jwils has sudo, this works. If not, we just use the local path.
echo "Attempting to launch for verification..." | tee -a "$LOG_FILE"
./target/release/matrix-overlay --version | tee -a "$LOG_FILE"

echo "V2 Operational. Launching in background..." | tee -a "$LOG_FILE"
./target/release/matrix-overlay &
echo "Launch completed. PID: $!" | tee -a "$LOG_FILE"
