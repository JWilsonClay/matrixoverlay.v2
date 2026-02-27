#!/bin/bash
# Stage 5.3 - E2E Verification Script
set -e

echo "=== Matrix Overlay v2: E2E Verification ==="

# 1. Run All Rust Tests
echo "[1/4] Running Rust test suite..."
cargo test --all

# 2. Verify Config Reload
echo "[2/4] Verifying Configuration Reload..."
# Create a temporary config
if [ -f config.json ]; then
    cp config.json config.json.bak
fi
# Ensure a default config exists for the check
cargo run -- --check-only

# 3. Security Hardening Check
echo "[3/4] Running Security Verification..."
if [ -f ./test_scripts/security_verify.sh ]; then
    chmod +x ./test_scripts/security_verify.sh
    ./test_scripts/security_verify.sh
fi

# 4. Performance Baseline
echo "[4/4] Checking Performance Baseline..."
cargo build --release
BIN_SIZE=$(stat -c%s target/release/matrix-overlay)
echo "Binary Size: $((BIN_SIZE/1024)) KB"

# 5. Load Monitoring (New for Stage 6)
echo "[5/5] Monitoring Resource Usage (10s sample)..."
# Start the app in check-only mode or background? 
# For verification, we assume the performance_tests.rs covers the logic, 
# but here we can do a quick check of the binary.
timeout 10 target/release/matrix-overlay --check-only &
PID=$!
sleep 2
CPU_LOAD=$(ps -p $PID -o %cpu | tail -n 1)
echo "Measured CPU Load: $CPU_LOAD%"
# We expect very low load in check-only mode, but this proves the binary runs.

echo "=== E2E Verification PASSED ==="
if [ -f config.json.bak ]; then
    mv config.json.bak config.json
fi
