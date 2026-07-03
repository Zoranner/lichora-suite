#!/bin/bash
# Build Script for Linux
#
# Builds Lichora and Unity native plugins for Linux.

set -e

RELEASE=0
NO_CEF=0

for arg in "$@"; do
    case "$arg" in
        --release)
            RELEASE=1
            ;;
        --no-cef)
            NO_CEF=1
            ;;
        --help|-h)
            echo "Lichora Build Script for Linux"
            echo ""
            echo "Usage: ./build.sh [OPTIONS]"
            echo ""
            echo "Options:"
            echo "  --release   Build optimized release artifacts"
            echo "  --no-cef    Build without CEF dependency"
            echo "  --help      Show this help message"
            exit 0
            ;;
        *)
            echo "Unknown option: $arg"
            exit 1
            ;;
    esac
done

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
DIST_DIR="$SCRIPT_DIR/dist/linux-x64"
UNITY_PLUGIN_DIR="$SCRIPT_DIR/hosts/unity-host/Plugins/Linux"

cd "$SCRIPT_DIR"

echo "=== Building Lichora ==="

# Check CEF environment
if [ "$NO_CEF" -eq 0 ] && [ -z "$CEF_PATH" ]; then
    echo "Error: CEF_PATH environment variable not set"
    echo "Please run setup-linux.sh first or use --no-cef"
    exit 1
fi

BUILD_ARGS=(build -p lichora -p lichora-ipc-native -p process-host)
if [ "$RELEASE" -eq 1 ]; then
    BUILD_ARGS+=(--release)
    TARGET_PROFILE="release"
else
    TARGET_PROFILE="debug"
fi

if [ "$NO_CEF" -eq 1 ]; then
    BUILD_ARGS+=(--no-default-features)
fi

echo "Running: cargo ${BUILD_ARGS[*]}"
cargo "${BUILD_ARGS[@]}"

TARGET_DIR="$SCRIPT_DIR/target/$TARGET_PROFILE"
mkdir -p "$DIST_DIR"

cp "$TARGET_DIR/lichora" "$DIST_DIR/lichora"
cp "$TARGET_DIR/liblichora_ipc_native.so" "$DIST_DIR/liblichora_ipc_native.so"
cp "$TARGET_DIR/libprocess_host.so" "$DIST_DIR/libprocess_host.so"

if [ -d "$UNITY_PLUGIN_DIR" ]; then
    cp "$TARGET_DIR/liblichora_ipc_native.so" "$UNITY_PLUGIN_DIR/liblichora_ipc_native.so"
    cp "$TARGET_DIR/libprocess_host.so" "$UNITY_PLUGIN_DIR/libprocess_host.so"
    echo "Copied IPC native plugin to Unity plugin dir: $UNITY_PLUGIN_DIR/liblichora_ipc_native.so"
    echo "Copied process host plugin to Unity plugin dir: $UNITY_PLUGIN_DIR/libprocess_host.so"
fi

if [ "$NO_CEF" -eq 0 ]; then
    for file in \
        libcef.so \
        icudtl.dat \
        resources.pak \
        chrome_100_percent.pak \
        chrome_200_percent.pak \
        v8_context_snapshot.bin
    do
        if [ -f "$CEF_PATH/$file" ]; then
            cp "$CEF_PATH/$file" "$DIST_DIR/$file"
        fi
    done

    if [ -d "$CEF_PATH/locales" ]; then
        rm -rf "$DIST_DIR/locales"
        cp -R "$CEF_PATH/locales" "$DIST_DIR/locales"
    fi
fi

echo ""
echo "=== Build Complete ==="
echo "Executable: $DIST_DIR/lichora"
echo "IPC native plugin: $DIST_DIR/liblichora_ipc_native.so"
echo "Process host plugin: $DIST_DIR/libprocess_host.so"
echo ""
