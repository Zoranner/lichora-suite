#!/bin/bash
# Build Script for Unix-like platforms.
#
# Builds Lichora runtime artifacts for Linux and macOS.

set -euo pipefail

RELEASE=0
NO_CEF=0
DIST_NAME=""
CEF_VERSION="${CEF_VERSION:-145.0.27}"
CEF_BASE_URL="https://cef-builds.spotifycdn.com"

print_help() {
    echo "Lichora Build Script for Linux and macOS"
    echo ""
    echo "Usage: ./build.sh [OPTIONS]"
    echo ""
    echo "Options:"
    echo "  --release           Build optimized release artifacts"
    echo "  --no-cef            Build without CEF dependency"
    echo "  --dist-name NAME    Set dist subdirectory name"
    echo "  --help              Show this help message"
    echo ""
    echo "Environment:"
    echo "  CEF_PATH            Existing full CEF build layout"
    echo "  CEF_RUNTIME_PATH    Runtime files to package; defaults to CEF_PATH"
    echo "  CEF_PLATFORM        Override CEF platform archive name"
    echo "  CEF_VERSION         CEF version prefix; default: $CEF_VERSION"
}

if [ "${1:-}" = "--help" ] || [ "${1:-}" = "-h" ]; then
    print_help
    exit 0
fi

detect_platform() {
    local os_name arch
    os_name="$(uname -s)"
    arch="$(uname -m)"

    case "$os_name:$arch" in
        Linux:x86_64)
            HOST_KIND="linux"
            CEF_PLATFORM="${CEF_PLATFORM:-linux64}"
            DEFAULT_DIST_NAME="linux-x64"
            DYLIB_EXT="so"
            BUILD_NATIVE_IME=1
            ;;
        Linux:aarch64|Linux:arm64)
            HOST_KIND="linux"
            CEF_PLATFORM="${CEF_PLATFORM:-linuxarm64}"
            DEFAULT_DIST_NAME="linux-arm64"
            DYLIB_EXT="so"
            BUILD_NATIVE_IME=1
            ;;
        Darwin:x86_64)
            HOST_KIND="macos"
            CEF_PLATFORM="${CEF_PLATFORM:-macosx64}"
            DEFAULT_DIST_NAME="macos-x64"
            DYLIB_EXT="dylib"
            BUILD_NATIVE_IME=0
            ;;
        Darwin:arm64)
            HOST_KIND="macos"
            CEF_PLATFORM="${CEF_PLATFORM:-macosarm64}"
            DEFAULT_DIST_NAME="macos-arm64"
            DYLIB_EXT="dylib"
            BUILD_NATIVE_IME=0
            ;;
        *)
            echo "Unsupported host platform: $os_name $arch" >&2
            exit 1
            ;;
    esac
}

detect_platform

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
                echo "Error: --dist-name requires a value" >&2
                exit 1
            fi
            DIST_NAME="$1"
            ;;
        --help|-h)
            print_help
            exit 0
            ;;
        *)
            echo "Unknown option: $1" >&2
            exit 1
            ;;
    esac
    shift
done

if [ -z "$DIST_NAME" ]; then
    DIST_NAME="$DEFAULT_DIST_NAME"
fi

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
DIST_DIR="$SCRIPT_DIR/dist/$DIST_NAME"

if [ "$HOST_KIND" = "macos" ]; then
    CEF_CACHE_ROOT="${CEF_CACHE_ROOT:-$HOME/Library/Caches/Lichora/cef}"
else
    CEF_CACHE_ROOT="${CEF_CACHE_ROOT:-${XDG_CACHE_HOME:-$HOME/.cache}/lichora/cef}"
fi
DEFAULT_CEF_PATH="$CEF_CACHE_ROOT/$CEF_VERSION-$CEF_PLATFORM"
DEFAULT_CEF_RUNTIME_PATH="$CEF_CACHE_ROOT/$CEF_VERSION-$CEF_PLATFORM-runtime"

require_command() {
    if ! command -v "$1" >/dev/null 2>&1; then
        echo "Error: '$1' is required." >&2
        exit 1
    fi
}

test_cef_layout() {
    local path="$1"
    [ -n "$path" ] || return 1
    [ -d "$path" ] || return 1

    for entry in CMakeLists.txt cmake include libcef_dll archive.json; do
        [ -e "$path/$entry" ] || return 1
    done

    if [ "$HOST_KIND" = "linux" ]; then
        for entry in libcef.so resources.pak locales; do
            [ -e "$path/$entry" ] || return 1
        done
    fi
}

resolve_cef_archive() {
    require_command python3
    python3 - "$CEF_BASE_URL/index.json" "$CEF_PLATFORM" "$CEF_VERSION" <<'PY'
import json
import sys
import urllib.request

index_url, platform, version_prefix = sys.argv[1:4]
with urllib.request.urlopen(index_url) as response:
    index = json.load(response)

platform_index = index.get(platform)
if not platform_index:
    raise SystemExit(f"CEF index does not contain platform {platform}")

version = next(
    (item for item in platform_index.get("versions", [])
     if item.get("cef_version", "").startswith(version_prefix + "+")),
    None,
)
if not version:
    raise SystemExit(f"CEF {version_prefix} was not found for {platform}")

archive = next(
    (item for item in version.get("files", []) if item.get("type") == "minimal"),
    None,
)
if not archive:
    raise SystemExit(f"CEF {version.get('cef_version')} does not have a minimal archive for {platform}")

print(archive["name"])
print(archive.get("sha1", ""))
PY
}

sha1_file() {
    require_command python3
    python3 - "$1" <<'PY'
import hashlib
import sys

digest = hashlib.sha1()
with open(sys.argv[1], "rb") as file:
    for chunk in iter(lambda: file.read(1024 * 1024), b""):
        digest.update(chunk)
print(digest.hexdigest())
PY
}

download_cef_archive() {
    local archive_name="$1"
    local archive_sha1="$2"
    local temp_root="$3"
    local archive_path="$temp_root/$archive_name"

    mkdir -p "$temp_root"
    if [ -f "$archive_path" ] && [ -n "$archive_sha1" ]; then
        if [ "$(sha1_file "$archive_path")" = "$archive_sha1" ]; then
            echo "Using verified cached CEF archive: $archive_path" >&2
            printf '%s\n' "$archive_path"
            return
        fi
        rm -f "$archive_path"
    fi

    require_command curl
    local encoded_name="${archive_name//+/%2B}"
    local archive_url="$CEF_BASE_URL/$encoded_name"
    echo "Downloading CEF from: $archive_url" >&2
    curl --location --fail --continue-at - --retry 10 --retry-delay 5 --output "$archive_path" "$archive_url"

    if [ -n "$archive_sha1" ]; then
        local actual_sha1
        actual_sha1="$(sha1_file "$archive_path")"
        if [ "$actual_sha1" != "$archive_sha1" ]; then
            echo "CEF SHA1 mismatch. Expected $archive_sha1, got $actual_sha1" >&2
            exit 1
        fi
    fi

    printf '%s\n' "$archive_path"
}

install_cef() {
    require_command tar
    require_command python3

    echo "Resolving CEF $CEF_VERSION archive for $CEF_PLATFORM..."
    local archive_info archive_name archive_sha1 temp_root archive_path extract_path root
    archive_info="$(resolve_cef_archive)"
    archive_name="$(printf '%s\n' "$archive_info" | sed -n '1p')"
    archive_sha1="$(printf '%s\n' "$archive_info" | sed -n '2p')"
    temp_root="${TMPDIR:-/tmp}/lichora-cef-$CEF_PLATFORM"
    archive_path="$(download_cef_archive "$archive_name" "$archive_sha1" "$temp_root")"
    extract_path="$temp_root/extract"

    rm -rf "$extract_path"
    mkdir -p "$extract_path"
    tar -xjf "$archive_path" -C "$extract_path"
    root="$(find "$extract_path" -mindepth 1 -maxdepth 1 -type d | head -n 1)"
    if [ -z "$root" ]; then
        echo "Extracted CEF folder was not found." >&2
        exit 1
    fi

    for entry in Release CMakeLists.txt cmake include libcef_dll; do
        if [ ! -e "$root/$entry" ]; then
            echo "Required CEF path was not found: $root/$entry" >&2
            exit 1
        fi
    done
    if [ "$HOST_KIND" = "linux" ] && [ ! -d "$root/Resources" ]; then
        echo "Required CEF path was not found: $root/Resources" >&2
        exit 1
    fi

    echo "Installing CEF build layout to $DEFAULT_CEF_PATH..."
    rm -rf "$DEFAULT_CEF_PATH" "$DEFAULT_CEF_RUNTIME_PATH"
    mkdir -p "$DEFAULT_CEF_PATH" "$DEFAULT_CEF_RUNTIME_PATH"
    cp -R "$root/Release/." "$DEFAULT_CEF_PATH/"
    cp -R "$root/Release/." "$DEFAULT_CEF_RUNTIME_PATH/"
    if [ -d "$root/Resources" ]; then
        cp -R "$root/Resources/." "$DEFAULT_CEF_PATH/"
        cp -R "$root/Resources/." "$DEFAULT_CEF_RUNTIME_PATH/"
    fi
    cp "$root/CMakeLists.txt" "$DEFAULT_CEF_PATH/"
    cp -R "$root/cmake" "$DEFAULT_CEF_PATH/"
    cp -R "$root/include" "$DEFAULT_CEF_PATH/"
    cp -R "$root/libcef_dll" "$DEFAULT_CEF_PATH/"
    python3 - "$DEFAULT_CEF_PATH/archive.json" "$archive_name" "$archive_sha1" <<'PY'
import json
import sys

path, name, sha1 = sys.argv[1:4]
with open(path, "w", encoding="utf-8") as file:
    json.dump({"type": "minimal", "name": name, "sha1": sha1}, file, indent=2)
PY
}

initialize_cef_environment() {
    if [ "$NO_CEF" -eq 1 ]; then
        return
    fi

    if [ -n "${CEF_PATH:-}" ]; then
        if ! test_cef_layout "$CEF_PATH"; then
            echo "Error: CEF_PATH does not contain the full CEF build layout: $CEF_PATH" >&2
            exit 1
        fi
        CEF_RUNTIME_PATH="${CEF_RUNTIME_PATH:-$CEF_PATH}"
        return
    fi

    if test_cef_layout "$DEFAULT_CEF_PATH"; then
        CEF_PATH="$DEFAULT_CEF_PATH"
        CEF_RUNTIME_PATH="${CEF_RUNTIME_PATH:-$DEFAULT_CEF_RUNTIME_PATH}"
        if [ ! -d "$CEF_RUNTIME_PATH" ]; then
            CEF_RUNTIME_PATH="$CEF_PATH"
        fi
        export CEF_PATH CEF_RUNTIME_PATH
        return
    fi

    install_cef
    if ! test_cef_layout "$DEFAULT_CEF_PATH"; then
        echo "Error: CEF install finished but the expected layout is still missing: $DEFAULT_CEF_PATH" >&2
        exit 1
    fi
    CEF_PATH="$DEFAULT_CEF_PATH"
    CEF_RUNTIME_PATH="$DEFAULT_CEF_RUNTIME_PATH"
    export CEF_PATH CEF_RUNTIME_PATH
}

copy_cef_runtime() {
    if [ "$HOST_KIND" = "macos" ]; then
        if [ ! -d "$CEF_RUNTIME_PATH" ]; then
            echo "Error: CEF_RUNTIME_PATH is not a directory: $CEF_RUNTIME_PATH" >&2
            exit 1
        fi
        cp -R "$CEF_RUNTIME_PATH/." "$DIST_DIR/"
        return
    fi

    local missing_files=()
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
            missing_files+=("$file")
        fi
    done

    if [ -d "$CEF_PATH/locales" ]; then
        rm -rf "$DIST_DIR/locales"
        cp -R "$CEF_PATH/locales" "$DIST_DIR/locales"
    else
        missing_files+=("locales")
    fi

    if [ "${#missing_files[@]}" -gt 0 ]; then
        echo "Error: CEF runtime is incomplete. Missing: ${missing_files[*]}" >&2
        exit 1
    fi
}

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
DIST_DIR="$SCRIPT_DIR/dist/$DIST_NAME"

cd "$SCRIPT_DIR"

echo "=== Building Lichora ==="
echo "Host: $HOST_KIND"
echo "CEF platform: $CEF_PLATFORM"
echo "Dist: $DIST_NAME"

initialize_cef_environment

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
if [ "$BUILD_NATIVE_IME" -eq 1 ]; then
    echo "Running: cargo ${NATIVE_IME_ARGS[*]}"
    cargo "${NATIVE_IME_ARGS[@]}"
fi

TARGET_DIR="$SCRIPT_DIR/target/$TARGET_PROFILE"
NATIVE_IME_TARGET_DIR="$SCRIPT_DIR/crates/native-ime/target/$TARGET_PROFILE"
mkdir -p "$DIST_DIR"

cp "$TARGET_DIR/lichora" "$DIST_DIR/lichora"
cp "$TARGET_DIR/liblichora_ipc_native.$DYLIB_EXT" "$DIST_DIR/liblichora_ipc_native.$DYLIB_EXT"
cp "$TARGET_DIR/libprocess_host.$DYLIB_EXT" "$DIST_DIR/libprocess_host.$DYLIB_EXT"
if [ "$BUILD_NATIVE_IME" -eq 1 ]; then
    cp "$NATIVE_IME_TARGET_DIR/libnative_ime.so" "$DIST_DIR/libnative_ime.so"
fi

if [ "$NO_CEF" -eq 0 ]; then
    copy_cef_runtime
fi

echo ""
echo "=== Build Complete ==="
echo "Executable: $DIST_DIR/lichora"
echo "IPC native plugin: $DIST_DIR/liblichora_ipc_native.$DYLIB_EXT"
echo "Process host plugin: $DIST_DIR/libprocess_host.$DYLIB_EXT"
if [ "$BUILD_NATIVE_IME" -eq 1 ]; then
    echo "Native IME plugin: $DIST_DIR/libnative_ime.so"
fi
echo ""
