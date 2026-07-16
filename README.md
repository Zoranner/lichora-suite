# Lichora Runtime

Lichora Runtime 是面向宿主应用的可嵌入 Web Surface 运行时。当前实现基于 Rust、CEF OSR 和 typed IPC，把网页渲染成宿主可消费的 BGRA frame，并通过 IPC 接收输入和发布输出事件。

本仓库只维护 Rust runtime、IPC crate、原生 C ABI 和跨平台发布产物。宿主适配和 Web overlay 包已拆分到独立仓库：

- `lichora-host-unity`：Unity host adapter。
- `lichora-overlay`：`@lichora/overlay` Web package。

## 当前状态

- Windows 发布产物为 `dist/win-x64/lichora.exe`、`dist/win-x64/lichora_ipc_native.dll` 和 `dist/win-x64/process_host.dll`。
- Linux 发布产物为 `dist/linux-x64/lichora`、`dist/linux-x64/liblichora_ipc_native.so`、`dist/linux-x64/libprocess_host.so` 和 `dist/linux-x64/libnative_ime.so`。
- macOS 发布产物由 CI 组装为 `dist/macos-x64/lichora`、`dist/macos-x64/liblichora_ipc_native.dylib`、`dist/macos-x64/libprocess_host.dylib` 和 CEF runtime。
- Cargo root package 的 binary target 叫 `lichora`，library target 显式命名为 `lichora_core`，避免 Windows MSVC 下同包 bin/lib 同名时争用 PDB。
- `dist/*` 会放置 CEF runtime 文件，例如 `libcef.*`、pak/dat/bin 文件和 `locales/`。
- IPC v2 架构、wire format 和浏览器能力设计由本仓库文档维护。
- Capture 使用 `FrameRing` 一等通道。
- Mouse move 使用 latest-only 状态；点击、滚轮、键盘、IME 和脚本请求使用 typed queue。
- Status page 是协议设计要求，用于定位输入积压、丢帧、ack 延迟和进程状态。

## 项目结构

```text
lichora-runtime/
├── Cargo.toml
├── Cargo.lock
├── build.ps1
├── build.sh
├── crates/
│   ├── lichora-ipc/
│   ├── lichora-ipc-native/
│   ├── native-ime/
│   └── process-host/
├── src/
│   ├── main.rs
│   ├── browser/
│   ├── ipc/
│   └── modules/
└── docs/
    ├── development.md
    ├── protocol.md
    └── design/
```

## Windows 发布目录

```powershell
.\build.ps1 -Release
```

脚本成功后，发布目录为：

```text
dist/win-x64/
```

如果没有设置 `CEF_PATH`，`build.ps1` 会把 Windows x64 CEF 自动安装到 `%LOCALAPPDATA%\Lichora\cef\145.0.27-windows64`。如果已设置 `CEF_PATH`，该目录必须是完整 CEF build layout，而不是只包含 runtime 文件的目录。

该发布目录必须包含：

- `lichora.exe`
- `lichora_ipc_native.dll`
- `process_host.dll`
- CEF runtime：`libcef.dll`、`chrome_elf.dll`、`icudtl.dat`、`resources.pak`、`chrome_*.pak`、`v8_context_snapshot.bin`、`locales/` 等
- 可选调试文件：`lichora.pdb`、`lichora_core.pdb`、`lichora_ipc_native.pdb`、`process_host.pdb`

`dist/win-x64/debug.log` 是运行期日志，不应提交。

## Linux 发布目录

```bash
export CEF_PATH=/path/to/cef/Release
./build.sh --release
```

脚本成功后，发布目录为：

```text
dist/linux-x64/
```

该目录包含 `lichora`、`liblichora_ipc_native.so`、`libprocess_host.so`、`libnative_ime.so` 和 CEF runtime 文件。

## Handler 模式

宿主侧启动 handler 进程时，第一个非选项参数是 handler GUID：

```powershell
.\dist\win-x64\lichora.exe 12345678-1234-1234-1234-123456789abc --graphics-mode=auto
```

handler GUID 用作 IPC v2 session namespace。当前 handler runtime 的 `control` 只处理 `Shutdown`、`AddBrowser`、`RemoveBrowser` 和 `ResizeBrowser`；`session`、`status` 已有协议与映射，宿主会打开两者，当前仅 `status` 暴露宿主读取 API。原生插件输入、帧和输出热路径统一使用 `ebi_browser_input_open`、`ebi_browser_frame_open` 和 `ebi_browser_output_open` 得到的 typed browser handle；旧 `*_for_browser` session-handle 兼容导出已移除。

旧启动参数 `--heartbeat-timeout-ms` 和 `--heartbeat-stall-grace-ms` 不再支持。当前 IPC v2 不保留旧 heartbeat 退出机制，传入这些参数会按未知选项失败。

单 URL 模式仍可用于本地手工调试：

```powershell
.\target\release\lichora.exe --url https://example.com --width 1280 --height 720
```

单 URL 模式不是宿主集成入口；宿主集成入口是 handler GUID 模式。

## IPC v2 通道

| 通道 | 方向 | 用途 |
| --- | --- | --- |
| `session` | 双向 | 已有协议与映射，宿主会打开；当前不暴露宿主读取 API |
| `control` | Host -> Browser | 当前仅处理 `Shutdown`、`AddBrowser`、`RemoveBrowser` 和 `ResizeBrowser` |
| `status` | Browser -> Host | 已有协议与映射，宿主会打开并暴露读取 API |
| `input` | Host -> Browser | 鼠标、键盘、IME 和脚本请求 |
| `frame` | Browser -> Host | BGRA frame ring 和 dirty rect |
| `output` | Browser -> Host | caret、surrounding text、ownership、script result 和 page event payload |

`docs/protocol.md` 记录当前协议入口和验证边界。

## 开发文档

- [Development](docs/development.md)
- [Protocol](docs/protocol.md)
- [Engineering Reviews](docs/reviews/)
- [Historical Archive](docs/archive/aggregate-workspace/README.md)（仅供历史追溯，不作为当前架构依据）

## 仓库质量检查

提交前可从仓库根目录运行 runtime 质量入口：

```powershell
.\scripts\check-quality.ps1
```

该入口只覆盖本仓 Rust 代码：`cargo fmt --all -- --check`、`cargo clippy --all-targets --all-features -- -D warnings` 和 `cargo test --no-default-features --all-targets`。overlay 与 Unity host 在各自仓库维护质量门禁。
