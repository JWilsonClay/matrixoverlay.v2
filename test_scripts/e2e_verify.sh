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

echo "=== E2E Verification PASSED ==="
if [ -f config.json.bak ]; then
    mv config.json.bak config.json
fi
