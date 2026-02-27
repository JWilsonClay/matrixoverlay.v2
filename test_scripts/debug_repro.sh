#!/bin/bash
# test_scripts/debug_repro.sh

echo "[DEBUG REPRO] Starting Matrix Overlay v2 tests..."

# 1. Verify Config Reload
echo "[1/3] Testing Config Reload Hook..."
# Simulate a config change and trigger reload via log check
cargo run -- --test-reload 2>&1 | grep "Config reloaded and broadcast"
if [ $? -eq 0 ]; then
    echo "PASS: Reload hook active."
else
    echo "FAIL: Reload hook not found in logs."
fi

# 2. Verify Multi-Repo Rotation
echo "[2/3] Testing Git Multi-Repo Rotation..."
# Add temp repos and check log for rotation
cargo run -- --test-git-rotation 2>&1 | grep "GitCollector: Polled"
if [ $? -eq 0 ]; then
    echo "PASS: Batching/Rotation active."
else
    echo "FAIL: Batching/Rotation not detected."
fi

# 3. Verify Rain Physics
echo "[3/3] Testing Matrix Rain Update Loop..."
cargo run -- --test-rain 2>&1 | grep "Rain: Updated"
if [ $? -eq 0 ]; then
    echo "PASS: RainManager update loop active."
else
    echo "FAIL: RainManager update loop not detected."
fi

echo "[DEBUG REPRO] Done."
