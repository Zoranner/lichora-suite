# Third Party Notices

This package contains native binaries and runtime bridges used by Lichora.

## lichora_ipc_native.dll

- Source: `crates/lichora-ipc-native`
- License: MIT
- Purpose: Unity P/Invoke bridge for Lichora IPC v2.

## liblichora_ipc_native.so

- Source: `crates/lichora-ipc-native`
- License: MIT
- Purpose: Linux build of the Unity P/Invoke bridge for Lichora IPC v2.
- Release status: staged in the package with platform import disabled; not part of the supported `0.1.0` runtime surface.

## process_host.dll and libprocess_host.so

- Source: `crates/process-host`
- Purpose: Native process spawn, wait and terminate helper for Unity.

## libnative_ime.so

- Source: `third_party/native-ime`
- License: MIT OR Apache-2.0
- Purpose: Linux native IME bridge for IBus, Fcitx 4 and Fcitx5.

## Runtime Dependencies

The package depends on Unity packages listed in `package.json`. The external browser runtime depends on Chromium Embedded Framework runtime files that are distributed with the Lichora runtime artifact, not as source files in this Unity package.
