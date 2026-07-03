# 开发指南

## 环境要求

Windows：

- Rust toolchain
- Visual Studio Build Tools
- CEF 145.0.27，默认由 `setup-windows.ps1` 下载到 `%LOCALAPPDATA%\Lichora\cef\145.0.27-windows64` 并通过 `CEF_PATH` 指向。该目录不是单纯 runtime 目录，还必须包含 `CMakeLists.txt`、`cmake/`、`include/`、`libcef_dll/` 和 `archive.json`，供 `cef-dll-sys` 构建 `libcef_dll_wrapper`。

Linux：

- Rust toolchain
- CEF 145.0.27 runtime
- GCC/Clang
- pkg-config

## 构建与发布

Windows 发布构建：

```powershell
.\setup-windows.ps1
.\build.ps1 -Release
```

如需放到其他目录，可显式传入安装路径：

```powershell
.\setup-windows.ps1 -InstallPath "E:\Repositories\.cache\cef\145.0.27-windows64"
```

`build.ps1` 成功后把 Rust 可执行文件、native IPC 插件和 CEF runtime 复制到：

```text
dist/win-x64/
```

该目录是宿主适配层应引用的 Windows runtime 目录。`debug.log` 是 CEF 运行期日志，已通过 `.gitignore` 排除。

`CEF_PATH` 指向开发机上的 CEF 根目录，`dist/win-x64` 才是运行时发布目录。不要把只含 `libcef.dll`、pak/dat/bin 和 `locales/` 的旧运行时目录设置为 `CEF_PATH`，否则 CEF 绑定 crate 无法编译。

Linux 发布构建：

```bash
./setup-linux.sh
./build.sh --release
```

`build.sh` 会构建 `lichora`、`lichora-ipc-native` 和 `process-host`，并组装：

```text
dist/linux-x64/
```

同时会把 `liblichora_ipc_native.so` 和 `libprocess_host.so` 复制到 Unity host 插件目录。

## Unity handler 运行模型

宿主集成按 IPC v2 session 模型运行。当前首个宿主适配是 Unity：

- Unity 启动一个 handler 进程，首个非选项参数为 session 或 handler GUID。
- Browser 进程创建项目自有 IPC session。
- 管理命令走 `control` queue。
- 鼠标移动走 latest-only，点击、滚轮、键盘、IME 和脚本请求走 typed input queue。
- Capture 走 `FrameRing`，状态诊断走 `status` page。

示例：

```powershell
.\dist\win-x64\lichora.exe 12345678-1234-1234-1234-123456789abc --graphics-mode=auto
```

## 模块边界

- `src/main.rs`：CLI、handler loop 和 Unity 管理命令分发；旧 heartbeat 启动参数不再支持。
- `src/browser/`：CEF app/client、浏览器实例生命周期、OSR render handler。
- `src/modules/`：浏览器输入、输出和 capture 的业务适配层。
- `src/ipc/`：IPC v2 runtime 适配层。
- `crates/lichora-ipc/`：共享 IPC core，包括 mmap、header、queue、latest slot、frame ring、typed payload 和 status/output payload。
- `crates/lichora-ipc-native/`：宿主原生插件 C ABI；typed browser handle 是输入、帧和输出热路径的唯一接口，旧 `*_for_browser` session-handle 导出不再保留。
- `crates/process-host/`：宿主进程管理 C ABI，供 Unity IL2CPP 等无法稳定使用托管 `Process` 的环境调用。

文档和发布治理改动不应顺手修改 `src`。协议行为变更必须同时更新 `docs/protocol.md`、`crates/lichora-ipc` tests 和相关 `docs/design` 文档。

## IPC v2 开发约束

- 不在业务模块手写 offset；offset 必须封装在 typed protocol 或 IPC runtime 中。
- 所有跨进程共享结构使用 little-endian。
- 每个通道必须有 version、channel kind、sequence/ack 或 seqlock 提交语义。

Capture 走 `FrameRing`，鼠标移动走 latest-only，事件走 SPSC queue。Web/Host 融合设计见 `design/web-host-overlay.md`。

## 验证边界

按仓库约束，不能通过 Unity Editor、Unity batchmode、Unity Test Framework batchmode、BuildPipeline、`dotnet build` 或 Unity 生成的 `.sln/.csproj` 验证。

Rust 代码变更通常需要 `cargo fmt --all` 和 `cargo clippy --all-targets --all-features -- -D warnings`。仅文档和发布治理改动可用文本扫描、脚本审查和 diff 检查收口。
