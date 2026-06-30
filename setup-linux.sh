#!/bin/bash
# CEF Environment Setup Script for Linux
#
# This script verifies a local CEF runtime directory and prints environment setup.

set -e

CEF_VERSION="145.0.27"
CEF_INSTALL_DIR="${1:-${CEF_PATH:-}}"

echo "=== CEF Environment Setup ==="
echo "CEF Version: $CEF_VERSION"
echo ""

if [ -z "$CEF_INSTALL_DIR" ]; then
    echo "Usage: ./setup-linux.sh /path/to/cef/Release"
    echo ""
    echo "Download CEF $CEF_VERSION Linux x64 Standard Distribution manually,"
    echo "extract it, then pass the extracted Release directory to this script."
    exit 1
fi

if [ ! -f "$CEF_INSTALL_DIR/libcef.so" ]; then
    echo "libcef.so not found in: $CEF_INSTALL_DIR"
    echo "Pass the CEF Release directory that contains libcef.so."
    exit 1
fi

# Set environment variables
echo ""
echo "=== Environment Variables ==="
echo "Add the following to your ~/.bashrc or ~/.zshrc:"
echo ""
echo "export CEF_PATH=\"$CEF_INSTALL_DIR\""
echo "export LD_LIBRARY_PATH=\"\$LD_LIBRARY_PATH:\$CEF_PATH\""
echo ""

echo "CEF runtime verified:"
echo "  Location: $CEF_INSTALL_DIR"
