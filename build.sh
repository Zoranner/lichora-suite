#!/bin/bash
# Build Script for Linux
#
# Builds the headless browser for Linux

set -e

echo "=== Building Headless Browser ==="

# Check CEF environment
if [ -z "$CEF_PATH" ]; then
    echo "Error: CEF_PATH environment variable not set"
    echo "Please run setup-linux.sh first"
    exit 1
fi

# Build
echo "Building..."
cargo build --release

echo ""
echo "=== Build Complete ==="
echo "Binary: target/release/headless_browser"
echo ""

# Optional: Create distribution package
if [ "$1" == "--package" ]; then
    echo "Creating distribution package..."
    cargo run --bin bundle-cef-app -- headless_browser -o dist/linux --release
    echo "Package created in: dist/linux/"
fi
