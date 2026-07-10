# Unity Native Facade Boundary Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Establish a compilable Unity assembly boundary where Runtime consumes public Native facades and never touches P/Invoke implementation types.

**Architecture:** Move the existing IPC client, control writer, input writer, output reader, frame reader, and status reader into the Native assembly as the public facade. Keep raw handles, results, error mapping, P/Invoke declarations, process implementation, and IME bridge internal. Runtime retains browser lifecycle, Linux IME orchestration, output pumping, ownership, input routing, and rendering.

**Tech Stack:** Unity asmdef, C#, PowerShell contract checks, CSharpier, Rust Cargo, Bun.

---

### Task 1: Add failing assembly boundary checks

- [x] Require the confirmed Unity.Collections GUID in Runtime and Native.
- [x] Require Protocol `noEngineReferences` and reject Unity imports in Protocol.
- [x] Require Runtime to disable unsafe code and reject raw Native implementation types.
- [x] Require IPC facade files to live under `Scripts/Native`.
- [x] Require no `InternalsVisibleTo` usage.
- [x] Scan all existing Unity C# source files, including untracked files.
- [x] Run the contract check and confirm the current boundary fails.

### Task 2: Move the IPC facade into Native

- [x] Move `BrowserIpcClient`, control/input writers, output/frame/status readers and metadata into `Scripts/Native`.
- [x] Move input payload encoding and input event kind into Native internal implementation.
- [x] Preserve all existing script `.meta` GUIDs.
- [x] Keep public facade class names stable for Runtime callers.

### Task 3: Tighten Protocol and asmdef configuration

- [x] Restore the confirmed Unity.Collections GUID.
- [x] Disable unsafe code in Runtime.
- [x] Set Protocol `noEngineReferences` to true.
- [x] Remove Native to Protocol and unrelated business assembly references.
- [x] Keep browser output DTOs in Protocol for Runtime consumption.

### Task 4: Isolate process and IME facades

- [x] Keep process P/Invoke behind `NativeProcessSpawner`.
- [x] Add `NativeImeClient` to convert raw ABI state and event buffers into stable public values.
- [x] Move `LinuxNativeImeModule` back to Runtime because it depends on Runtime input interfaces and Unity orchestration.
- [x] Keep `NativeImeBridge` and raw event types internal.

### Task 5: Update documentation and verification

- [x] Document the public facade and internal implementation boundary.
- [x] Document the actual Runtime, Native, Protocol and Model dependency direction.
- [x] Document complete CSharpier source collection.
- [x] Run CSharpier on all changed C# files.
- [x] Run the plugin contract check.
- [x] Run the full repository quality check.
- [x] Run Rust no-default-features all-target tests.
- [x] Run `git diff --check` and inspect status.
