#!/bin/bash
# CEF Environment Setup Script for Linux
#
# This script installs CEF binaries and sets up the environment

set -e

CEF_VERSION="145.0.27"
CEF_INSTALL_DIR="${CEF_PATH:-$HOME/.local/share/cef}"

echo "=== CEF Environment Setup ==="
echo "CEF Version: $CEF_VERSION"
echo "Install Directory: $CEF_INSTALL_DIR"
echo ""

# Create installation directory
mkdir -p "$CEF_INSTALL_DIR"

# Download and extract CEF
echo "Downloading CEF..."
cargo run -p export-cef-dir -- --force "$CEF_INSTALL_DIR"

# Set environment variables
echo ""
echo "=== Environment Variables ==="
echo "Add the following to your ~/.bashrc or ~/.zshrc:"
echo ""
echo "export CEF_PATH=\"$CEF_INSTALL_DIR\""
echo "export LD_LIBRARY_PATH=\"\$LD_LIBRARY_PATH:\$CEF_PATH\""
echo ""

# Verify installation
if [ -f "$CEF_INSTALL_DIR/libcef.so" ]; then
    echo "✓ CEF installed successfully!"
    echo "  Location: $CEF_INSTALL_DIR"
else
    echo "✗ CEF installation failed"
    exit 1
fi
