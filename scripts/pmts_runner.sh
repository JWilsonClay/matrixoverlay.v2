#!/bin/bash
set -e

# PMTS Runner v1.0
# Part of Diamond Hardened Substrate Verification

echo "--- [PMTS] Starting Post-Modularization Test Suite ---"

# 1. Environment Cleanup
rm -f /tmp/matrix_overlay_logs/*.log

# 2. Unit & Integration Tests
echo "[PMTS] 1/3 Running Unit Tests..."
cargo test --lib

# 3. Release Build
echo "[PMTS] 2/3 Building Release Binary..."
cargo build --release

# 4. Smoke Test
echo "[PMTS] 3/3 Performing Smoke Test (Boot Stability)..."
# Note: --test-layering is a special flag that exits after a delay if initialization succeeds
./target/release/matrix-overlay --test-layering

if [ $? -eq 0 ]; then
    echo "[PMTS] Smoke Test: SUCCESS"
else
    echo "[PMTS] Smoke Test: FAILED"
    exit 1
fi

echo "--- [PMTS] ALL TESTS PASSED (Diamond Status Verified) ---"
