# Lichora

Lichora 是面向宿主应用的可嵌入 Web Surface 运行时。当前实现基于 Rust、CEF OSR 和 typed IPC，可以把网页渲染成宿主可消费的 BGRA frame，并通过 IPC 接收输入和发布输出事件。

当前首个宿主适配是 Unity，位于 `hosts/unity-host`。核心 runtime、IPC crate 和发布产物统一使用 Lichora 命名。

## 当前状态

- Windows 目标产物为 `dist/win-x64/lichora.exe`、`dist/win-x64/lichora_ipc_native.dll` 和 `dist/win-x64/process_host.dll`。
- Linux 构建目标产物为 `dist/linux-x64/lichora`、`dist/linux-x64/liblichora_ipc_native.so` 和 `dist/linux-x64/libprocess_host.so`；其中 Unity Linux IPC importer 当前禁用，插件 manifest 标记为 `release: false`，不属于当前发布支持面。
- Cargo root package 的 binary target 叫 `lichora`，library target 显式命名为 `lichora_core`，避免 Windows MSVC 下同包 bin/lib 同名时争用 PDB。
- `dist/win-x64` 同时放置 CEF runtime 文件，例如 `libcef.dll`、pak/dat/bin 文件和 `locales/`。
- IPC v2 架构、wire format 和浏览器能力设计由本仓库文档维护。
- Capture 使用 `FrameRing` 一等通道。
- Mouse move 使用 latest-only 状态；点击、滚轮、键盘、IME 和脚本请求使用 typed queue。
- Status page 是协议设计要求，用于定位输入积压、丢帧、ack 延迟和进程状态；完整 runtime/Unity 闭环仍待实现。
- Web/Host 融合和输入归属设计见 `docs/design/web-host-overlay.md`。

## 项目结构

```text
lichora/
├── Cargo.toml
├── Cargo.lock
├── build.ps1
├── build.sh
├── dist/
│   └── win-x64/
│       ├── lichora.exe
│       ├── lichora_ipc_native.dll
│       ├── process_host.dll
│       ├── libcef.dll
│       ├── *.pak / *.dat / *.bin
│       └── locales/
├── crates/
│   ├── lichora-ipc/
│   ├── lichora-ipc-native/
│   ├── native-ime/
│   └── process-host/
├── hosts/
│   └── unity-host/
├── packages/
│   └── overlay/
├── src/
│   ├── main.rs
│   ├── browser/
│   ├── ipc/
│   └── modules/
└── docs/
    ├── development.md
    ├── protocol.md
    └── design/
        └── web-host-overlay.md
```

## Windows 发布目录

```powershell
.\setup-windows.ps1
.\build.ps1 -Release
```

脚本成功后，发布目录为：

```text
dist/win-x64/
```

该目录必须包含：

- `lichora.exe`
- `lichora_ipc_native.dll`，供 Unity `BrowserIpcNative` 通过 `DllImport("lichora_ipc_native")` 加载
- `process_host.dll`，供 Unity `NativeProcessSpawner` 通过 `DllImport("process_host")` 加载
- CEF runtime：`libcef.dll`、`chrome_elf.dll`、`icudtl.dat`、`resources.pak`、`chrome_*.pak`、`v8_context_snapshot.bin`、`locales/` 等
- 可选调试文件：`lichora.pdb`、`lichora_core.pdb`、`lichora_ipc_native.pdb`、`process_host.pdb`

native 插件还会复制到 `hosts/unity-host/Plugins/Windows/`，这是 Unity Windows 插件加载位置。不要把 `target/` 或 `dist/` 产物提交到 Git；需要版本化 Unity 插件二进制时，应连同对应 `.meta` 一起纳入 `hosts/unity-host`。

`dist/win-x64/debug.log` 是运行期日志，不应提交。

Linux 发布构建：

```bash
export CEF_PATH=/path/to/cef/Release
./build.sh --release
```

脚本成功后，发布目录为：

```text
dist/linux-x64/
```

该目录包含 `lichora`、`liblichora_ipc_native.so`、`libprocess_host.so` 和 `libnative_ime.so`。native 插件会复制到 `hosts/unity-host/Plugins/Linux/`；当前 `liblichora_ipc_native.so` 的 Unity importer 已禁用，且插件 manifest 标记为 `release: false`，因此不能据此声明 Unity Linux IPC runtime 已受支持。CEF runtime 文件会在 `CEF_PATH` 可用时复制到发布目录。

## Unity handler 模式

Unity 侧启动 handler 进程时，第一个非选项参数是 handler GUID：

```powershell
.\dist\win-x64\lichora.exe 12345678-1234-1234-1234-123456789abc --graphics-mode=auto
```

handler GUID 用作 IPC v2 session namespace。当前 handler runtime 的 `control` 只处理 `Shutdown`、`AddBrowser`、`RemoveBrowser` 和 `ResizeBrowser`；`session`、`status` 已有协议与映射，宿主会打开两者，当前仅 `status` 暴露宿主读取 API，browser runtime 写入和 Unity 业务消费仍未形成完整闭环。原生插件输入、帧和输出热路径统一使用 `ebi_browser_input_open`、`ebi_browser_frame_open` 和 `ebi_browser_output_open` 得到的 typed browser handle；旧 `*_for_browser` session-handle 兼容导出已移除。

旧 Unity 启动参数 `--heartbeat-timeout-ms` 和 `--heartbeat-stall-grace-ms` 不再支持。当前 IPC v2 不保留旧 heartbeat 退出机制，传入这些参数会按未知选项失败。

单 URL 模式仍可用于本地手工调试：

```powershell
.\target\release\lichora.exe --url https://example.com --width 1280 --height 720
```

不要把单 URL 模式写成 Unity 集成入口；Unity 集成入口是 handler GUID 模式。

## IPC v2 通道

| 通道 | 方向 | 用途 |
| --- | --- | --- |
| `session` | 双向 | 已有协议与映射，宿主会打开；当前不暴露宿主读取 API |
| `control` | Unity -> Browser | 当前仅处理 `Shutdown`、`AddBrowser`、`RemoveBrowser` 和 `ResizeBrowser` |
| `status` | Browser -> Unity | 已有协议与映射，宿主会打开并暴露读取 API；browser runtime 写入和 Unity 业务消费仍未形成完整闭环 |
| `input` | Unity -> Browser | 鼠标、键盘、IME 和脚本请求 |
| `frame` | Browser -> Unity | BGRA frame ring 和 dirty rect |
| `output` | Browser -> Unity | 当前实际消费 caret、surrounding text 和 ownership；`ScriptResult`、`PageEvent` 仅完成协议解码，Unity Runtime 会丢弃 |

`docs/protocol.md` 记录当前协议入口和验证边界。

## 开发文档

- [Development](docs/development.md)
- [Protocol](docs/protocol.md)
- [Web/Host Overlay Design](docs/design/web-host-overlay.md)
- [Engineering Reviews](docs/reviews/)
- [Historical Archive](docs/archive/aggregate-workspace/README.md)（仅供历史追溯，不作为当前架构依据）

## 仓库质量检查

提交前可从仓库根目录运行统一质量入口：

```powershell
.\scripts\check-quality.ps1
```

该入口覆盖 Rust、overlay、Unity C# 格式和 Unity 插件静态契约；详细命令清单见 [Development](docs/development.md)。

可使用 `-SkipRust`、`-SkipOverlay`、`-SkipUnity` 或 `-SkipPackaging` 跳过对应阶段。Unity 阶段只读取包内源码，不启动 Unity Editor、batchmode、BuildPipeline，也不执行 Unity 生成的 `.sln`/`.csproj` 编译。
