# Lichora Unity Host

Lichora Unity Host is a Unity package for rendering and interacting with web pages through the Lichora runtime.

## Runtime Model

- Unity starts `lichora.exe` as an external process.
- Unity communicates with the browser process through Rust IPC v2 via `lichora_ipc_native`.
- Mouse, keyboard, IME, JavaScript requests, frame data, output events and status diagnostics use typed IPC channels.
- The old MemoryStacks runtime is not part of this package release line.

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

## Linux Status

Linux helper plugins are present for process hosting and native IME. A Linux IPC v2 plugin file is also staged at `Plugins/Linux/liblichora_ipc_native.so`, but its importer is disabled for this version, so Linux IPC runtime support is not part of the `0.1.0` release surface. Enable and validate it separately before claiming Linux runtime support.

## Samples

The `ECharts Script Sender` sample contains a small helper script that periodically calls `PageRenderer.ExecuteScript("addRandomData();")` for stress-testing a page that defines that function.
