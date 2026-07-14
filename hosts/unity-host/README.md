# Lichora Host

Lichora Host is a Unity package for rendering and interacting with web pages through the Lichora runtime.

## Runtime Model

- Unity starts `lichora.exe` as an external process.
- Unity communicates with the browser process through Rust IPC v2 via `lichora_ipc_native`.
- Mouse, keyboard, IME, JavaScript requests, frame data and output events use typed IPC channels. The current session and status boundary is summarized below.
- The old MemoryStacks runtime is not part of this package release line.

## Current Implementation Boundary

- The handler currently processes only `Shutdown`, `AddBrowser`, `RemoveBrowser` and `ResizeBrowser` control commands.
- `session` and `status` have protocol definitions and mappings, and the host opens both. Only `status` currently exposes a host read API; browser-runtime writes and Unity business consumption do not yet form a complete loop.
- Unity currently consumes caret, surrounding text and ownership output. `ScriptResult` and `PageEvent` are decoded by Protocol and then discarded by Runtime.
- The web SDK can attempt `window.lichora.postMessage(...)`, but the runtime does not currently inject `window.lichora`; the active bridge is the console fallback.
- Controlled overlay pages apply their ownership map as a Chromium SVG luminance mask before publishing it. Chromium frame alpha is the visibility source of truth; Unity consumes that alpha directly, while the ownership map is used only for input routing.

## Windows x64 Release

This package contains the Windows native IPC plugin:

```text
Plugins/Windows/lichora_ipc_native.dll
Plugins/Windows/process_host.dll
```

The matching browser runtime is published to:

```text
dist/win-x64/lichora.exe
```

Set `BrowserConfig.json` in the application directory to point at that executable.
Browser output payload decoding is isolated under `Scripts/Protocol` in the Unity-free, dependency-free `KimoTech.LichoraHost.Protocol` assembly. Runtime references Protocol explicitly. Native owns input payload encoding and does not reference Protocol.

The ownership enum model is isolated under `Scripts/Model` in the Unity-free `KimoTech.LichoraHost.Model` assembly. It contains only `InputOwner` and `InputRegionShape`; Unity-backed `InputRegion` and `InputOwnershipMap` remain in Runtime because they depend on Unity serialization and coordinate types.

The C# native boundary is isolated under `Scripts/Native` in the `KimoTech.LichoraHost.Native` assembly. Runtime consumes public IPC, process and IME facades; raw handles, result mapping, P/Invoke declarations and native event buffers remain internal to Native. Runtime disables unsafe code, and `Scripts/Native` is the only supported C# P/Invoke location.

Plugin support and ABI metadata are recorded in `Plugins/plugin-manifest.json`. The manifest describes the target platform, architecture, P/Invoke name, source crate, ABI version and whether the plugin is part of the current release surface. Run the repository plugin contract check after changing native plugin files or importer settings:

```powershell
.\scripts\check-plugin-contract.ps1
```

The manifest does not contain binary hashes. Release automation is responsible for generating and publishing checksums for the exact native binaries included in a release.

## Linux Status

Linux helper plugins are present for process hosting and native IME. A Linux IPC v2 plugin file is also staged at `Plugins/Linux/liblichora_ipc_native.so`, but its importer is disabled for this version, so Linux IPC runtime support is not part of the `0.1.0` release surface. Enable and validate it separately before claiming Linux runtime support.

## Samples

The `ECharts Script Sender` sample contains a small helper script that periodically calls `PageRenderer.ExecuteScript("addRandomData();")` for stress-testing a page that defines that function.
