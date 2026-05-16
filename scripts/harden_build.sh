#!/bin/bash
# scripts/harden_build.sh
# Production-grade hardening script for Matrix Overlay v2.
# Performs symbol stripping and UPX packing to simulate a secure, closed-source binary.

set -e

APP_NAME="matrix-overlay"
RELEASE_DIR="target/release"
OUTPUT_DIR="dist"
FINAL_BINARY="$OUTPUT_DIR/$APP_NAME"

echo "--- [1/4] Building Hardened Release Binary ---"
cargo build --release

echo "--- [2/4] Initializing Distribution Folder ---"
mkdir -p "$OUTPUT_DIR"
cp "$RELEASE_DIR/$APP_NAME" "$FINAL_BINARY"

echo "--- [3/4] Stripping Symbols (The Silence) ---"
# strip --strip-all ensures no human-readable function names remain.
strip --strip-all "$FINAL_BINARY"

echo "--- [4/4] Applying Binary Packing (The Shell) ---"
# Using UPX to compress and obfuscate the binary entry point.
if command -v upx >/dev/null 2>&1; then
    upx --best --ultra-brute "$FINAL_BINARY"
else
    echo "Warning: UPX not found. Skipping compression step."
    echo "Run 'sudo apt install upx-ucl' to enable this feature."
fi

echo ""
echo "--- Build Complete: $FINAL_BINARY ---"
ls -lh "$FINAL_BINARY"
file "$FINAL_BINARY"
echo "--- Verification: Run 'nm -g $FINAL_BINARY' to confirm no symbols remain. ---"
