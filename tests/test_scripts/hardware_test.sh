#!/bin/bash
# Hardware verification script for Dell G15 5515
# Runs specific cargo tests targeting hardware sensors and X11 integration.

set -e

echo "=== Starting Hardware Tests on $(hostname) ==="
echo "Target Hardware: Ryzen 5800H + RTX 3050 Ti"

# Ensure we are in the project root
cd "$(dirname "$0")/.."

# Check for X11
if [ -z "$DISPLAY" ]; then
    echo "Error: DISPLAY environment variable not set. X11 tests require an active session."
    exit 1
fi

# Check for NVIDIA driver
if ! command -v nvidia-smi &> /dev/null; then
    echo "Warning: nvidia-smi not found. NVIDIA tests may be skipped or fail."
fi

# Run hardware tests
echo "Running Cargo Tests (Hardware Suite)..."
RUST_LOG=info cargo test --test hardware_tests -- --nocapture

echo "=== Hardware Tests Complete ==="