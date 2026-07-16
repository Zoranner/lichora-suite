#!/bin/bash
# Build Script for Linux
#
# Builds Lichora runtime artifacts for Linux.

set -e

RELEASE=0
NO_CEF=0
DIST_NAME="linux-x64"
SKIP_UNITY_COPY=0

while [ "$#" -gt 0 ]; do
    case "$1" in
        --release)
            RELEASE=1
            ;;
        --no-cef)
            NO_CEF=1
            ;;
        --dist-name)
            shift
            if [ "$#" -eq 0 ]; then
                echo "Error: --dist-name requires a value"
                exit 1
            fi
            DIST_NAME="$1"
            ;;
        --skip-unity-copy)
            SKIP_UNITY_COPY=1
            ;;
        --help|-h)
            echo "Lichora Build Script for Linux"
            echo ""
            echo "Usage: ./build.sh [OPTIONS]"
            echo ""
            echo "Options:"
            echo "  --release           Build optimized release artifacts"
            echo "  --no-cef            Build without CEF dependency"
            echo "  --dist-name NAME    Set dist subdirectory name (default: linux-x64)"
            echo "  --skip-unity-copy   Do not copy native libraries into hosts/unity-host"
            echo "  --help              Show this help message"
            exit 0
            ;;
        *)
            echo "Unknown option: $1"
            exit 1
            ;;
    esac
    shift
done

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
DIST_DIR="$SCRIPT_DIR/dist/$DIST_NAME"
UNITY_PLUGIN_DIR="$SCRIPT_DIR/hosts/unity-host/Plugins/Linux"

cd "$SCRIPT_DIR"

echo "=== Building Lichora ==="

# Check CEF environment
if [ "$NO_CEF" -eq 0 ] && [ -z "$CEF_PATH" ]; then
    echo "Error: CEF_PATH environment variable not set"
    echo "Set CEF_PATH to the extracted CEF Release directory, or use --no-cef."
    exit 1
fi

BUILD_ARGS=(build -p lichora -p lichora-ipc-native -p process-host)
NATIVE_IME_ARGS=(build --manifest-path crates/native-ime/Cargo.toml -p ime-ffi)
if [ "$RELEASE" -eq 1 ]; then
    BUILD_ARGS+=(--release)
    NATIVE_IME_ARGS+=(--release)
    TARGET_PROFILE="release"
else
    TARGET_PROFILE="debug"
fi

if [ "$NO_CEF" -eq 1 ]; then
    BUILD_ARGS+=(--no-default-features)
fi

echo "Running: cargo ${BUILD_ARGS[*]}"
cargo "${BUILD_ARGS[@]}"
echo "Running: cargo ${NATIVE_IME_ARGS[*]}"
cargo "${NATIVE_IME_ARGS[@]}"

TARGET_DIR="$SCRIPT_DIR/target/$TARGET_PROFILE"
NATIVE_IME_TARGET_DIR="$SCRIPT_DIR/crates/native-ime/target/$TARGET_PROFILE"
mkdir -p "$DIST_DIR"

cp "$TARGET_DIR/lichora" "$DIST_DIR/lichora"
cp "$TARGET_DIR/liblichora_ipc_native.so" "$DIST_DIR/liblichora_ipc_native.so"
cp "$TARGET_DIR/libprocess_host.so" "$DIST_DIR/libprocess_host.so"
cp "$NATIVE_IME_TARGET_DIR/libnative_ime.so" "$DIST_DIR/libnative_ime.so"

if [ "$SKIP_UNITY_COPY" -eq 0 ] && [ -d "$UNITY_PLUGIN_DIR" ]; then
    cp "$TARGET_DIR/liblichora_ipc_native.so" "$UNITY_PLUGIN_DIR/liblichora_ipc_native.so"
    cp "$TARGET_DIR/libprocess_host.so" "$UNITY_PLUGIN_DIR/libprocess_host.so"
    cp "$NATIVE_IME_TARGET_DIR/libnative_ime.so" "$UNITY_PLUGIN_DIR/libnative_ime.so"
    echo "Copied IPC native plugin to Unity plugin dir: $UNITY_PLUGIN_DIR/liblichora_ipc_native.so"
    echo "Copied process host plugin to Unity plugin dir: $UNITY_PLUGIN_DIR/libprocess_host.so"
    echo "Copied native IME plugin to Unity plugin dir: $UNITY_PLUGIN_DIR/libnative_ime.so"
fi

if [ "$NO_CEF" -eq 0 ]; then
    MISSING_CEF_FILES=()
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
        else
            MISSING_CEF_FILES+=("$file")
        fi
    done

    if [ -d "$CEF_PATH/locales" ]; then
        rm -rf "$DIST_DIR/locales"
        cp -R "$CEF_PATH/locales" "$DIST_DIR/locales"
    else
        MISSING_CEF_FILES+=("locales")
    fi

    if [ "${#MISSING_CEF_FILES[@]}" -gt 0 ]; then
        echo "Error: CEF runtime is incomplete. Missing: ${MISSING_CEF_FILES[*]}"
        exit 1
    fi
fi

echo ""
echo "=== Build Complete ==="
echo "Executable: $DIST_DIR/lichora"
echo "IPC native plugin: $DIST_DIR/liblichora_ipc_native.so"
echo "Process host plugin: $DIST_DIR/libprocess_host.so"
echo "Native IME plugin: $DIST_DIR/libnative_ime.so"
echo ""
