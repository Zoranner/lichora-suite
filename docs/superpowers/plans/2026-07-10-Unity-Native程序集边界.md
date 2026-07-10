# Unity Native Assembly Boundary Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** 将 Unity host 的原生 P/Invoke bridge 从通用 Runtime 程序集中隔离出来，形成可检查的平台与职责边界，同时保持现有 namespace、C ABI、插件支持面和运行时行为不变。

**Architecture:** 新增 `KimoTech.LichoraHost.Native` 程序集承载 IPC native、process host 和 native IME bridge。现有 `KimoTech.LichoraHost` Runtime 程序集显式引用 Native 程序集，继续承载页面、输入、渲染、ownership 和生命周期。Native 程序集保留 `KimoTech.LichoraHost` namespace，避免公开 API 和序列化数据迁移；平台支持由 asmdef、插件 `.meta` 和静态契约脚本共同约束。

**Tech Stack:** Unity asmdef JSON、C#、PowerShell、CSharpier、Git 静态引用检查。

---

### Task 1: 建立 Native 程序集边界

**Files:**
- Create: `hosts/unity-host/Scripts/Native/KimoTech.LichoraHost.Native.asmdef`
- Modify: `hosts/unity-host/Scripts/KimoTech.LichoraHost.asmdef`
- Move: `hosts/unity-host/Scripts/Ipc/BrowserIpcNative.cs` -> `hosts/unity-host/Scripts/Native/BrowserIpcNative.cs`
- Move: `hosts/unity-host/Scripts/NativeProcessSpawner.cs` -> `hosts/unity-host/Scripts/Native/NativeProcessSpawner.cs`
- Move: `hosts/unity-host/Scripts/NativeImeBridge.cs` -> `hosts/unity-host/Scripts/Native/NativeImeBridge.cs`
- Move: `hosts/unity-host/Scripts/LinuxNativeImeModule.cs` -> `hosts/unity-host/Scripts/Native/LinuxNativeImeModule.cs`
- Move: corresponding `.meta` files with files

- [x] **Step 1: Create Native asmdef**

Use a runtime assembly with the existing package dependencies required by the native bridge. Keep `rootNamespace` as `KimoTech.LichoraHost`, set `allowUnsafeCode` to `true` only if the moved files require it, and do not include Editor-only platforms.

- [x] **Step 2: Add explicit Runtime reference**

Add the Native asmdef GUID to `KimoTech.LichoraHost.asmdef`. Do not add a reverse reference from Native to Runtime. Keep all existing third-party references unchanged.

- [x] **Step 3: Move bridge files without API changes**

Move only the four native bridge files. Preserve namespace, class names, method signatures, P/Invoke strings, serialized field names and platform branches. Do not move `BrowserIpcClient`, payload models, input controllers or ownership types in this task.

- [x] **Step 4: Preserve Unity metadata**

Move each `.meta` file with its C# file and retain the original GUID. Create only the new directory `.meta` if Unity package layout requires it; do not regenerate script GUIDs.

### Task 2: Add static Native boundary checks

**Files:**
- Modify: `scripts/check-plugin-contract.ps1`
- Modify: `scripts/check-quality.ps1`
- Modify: `hosts/unity-host/README.md`
- Modify: `docs/development.md`

- [x] **Step 1: Validate Native asmdef JSON**

Check that the Native asmdef exists, is runtime-only, has the expected root namespace, and is referenced by the Runtime asmdef.

- [x] **Step 2: Validate bridge placement**

Check that all four native bridge files are under `Scripts/Native`, and that no `DllImport` remains under the generic Runtime paths except the Native directory.

- [x] **Step 3: Validate one-way dependency**

Check that Runtime references Native and Native does not reference Runtime. Report assembly names and file paths on failure.

- [x] **Step 4: Document the boundary**

Update package and development documentation to state that `Scripts/Native` is the only C# P/Invoke boundary and that namespace compatibility is intentional.

### Task 3: Format and static verification

**Files:**
- No additional source files.

- [x] **Step 1: Run CSharpier**

```powershell
csharpier check hosts/unity-host/Scripts/Native/*.cs
```

- [x] **Step 2: Run static checks**

```powershell
.\scripts\check-plugin-contract.ps1
```

- [x] **Step 3: Run repository quality entry**

```powershell
.\scripts\check-quality.ps1
```

- [x] **Step 4: Verify diff boundaries**

```powershell
git diff --check
git status --short
git diff --stat
```

Confirm no Unity Editor, Unity batchmode, BuildPipeline, `.sln`, `.csproj`, `Library` or generated project files were modified.
