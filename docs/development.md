# 开发指南

## 环境要求

Windows：

- Rust toolchain
- Visual Studio Build Tools
- CEF 145.0.27，默认由 `build.ps1` 下载到 `%LOCALAPPDATA%\Lichora\cef\145.0.27-windows64` 并通过 `CEF_PATH` 指向。该目录不是单纯 runtime 目录，还必须包含 `CMakeLists.txt`、`cmake/`、`include/`、`libcef_dll/` 和 `archive.json`，供 `cef-dll-sys` 构建 `libcef_dll_wrapper`。

Linux：

- Rust toolchain
- CEF 145.0.27 Release 目录，并通过 `CEF_PATH` 指向
- GCC/Clang
- pkg-config

## 构建与发布

Windows 发布构建：

```powershell
.\build.ps1 -Release
```

如需使用其他 CEF 安装目录，先设置 `CEF_PATH`：

```powershell
$env:CEF_PATH = "E:\Repositories\.cache\cef\145.0.27-windows64"
.\build.ps1 -Release
```

`build.ps1` 成功后把 Rust 可执行文件、native IPC 插件、process host 插件和 CEF runtime 复制到：

```text
dist/win-x64/
```

该目录是宿主适配层应引用的 Windows runtime 目录。`debug.log` 是 CEF 运行期日志，已通过 `.gitignore` 排除。

`CEF_PATH` 指向开发机上的 CEF 根目录，`dist/win-x64` 才是运行时发布目录。不要把只含 `libcef.dll`、pak/dat/bin 和 `locales/` 的旧运行时目录设置为 `CEF_PATH`，否则 CEF 绑定 crate 无法编译。

Linux 发布构建：

```bash
export CEF_PATH=/path/to/cef/Release
./build.sh --release
```

`build.sh` 会构建 `lichora`、`lichora-ipc-native`、`process-host` 和 Linux native IME 插件，并组装：

```text
dist/linux-x64/
```

跨平台发布由 GitHub Actions 在 tag `v*` 时触发，当前目标为 Windows x64、Linux x64、Linux arm64 和 macOS x64。发布包包含 runtime 可执行文件、原生 ABI 库和对应平台的 CEF runtime。

## Handler 运行模型

宿主集成按 IPC v2 session 模型运行：

- 宿主启动一个 handler 进程，首个非选项参数为 session 或 handler GUID。
- handler GUID 用作 IPC v2 session namespace；`session` 和 `status` 已有协议与映射，宿主会打开两者，当前仅 `status` 暴露宿主读取 API。
- 管理命令走 `control` queue；handler runtime 当前只处理 `Shutdown`、`AddBrowser`、`RemoveBrowser` 和 `ResizeBrowser`。
- 鼠标移动走 latest-only，点击、滚轮、键盘、IME 和脚本请求走 typed input queue。
- Capture 走 `FrameRing`。output payload 覆盖 caret、surrounding text、ownership、script result 和 page event。

示例：

```powershell
.\dist\win-x64\lichora.exe 12345678-1234-1234-1234-123456789abc --graphics-mode=auto
```

## 模块边界

- `src/main.rs`：进程入口、CEF 子进程分流、日志初始化和运行模式组合；旧 heartbeat 启动参数不再支持。
- `src/cli.rs`：CLI 参数模型、graphics mode 解析、handler GUID/URL 模式识别和 usage 输出。
- `src/handler.rs`：handler loop、control queue、浏览器实例编排、控制命令分发和 handler 运行日志。
- `src/browser/`：CEF app/client、浏览器实例生命周期、OSR render handler。
- `src/modules/`：浏览器输入、输出和 capture 的业务适配层。
- `src/ipc/`：IPC v2 runtime 适配层。
- `crates/lichora-ipc/`：共享 IPC core，包括 mmap、header、queue、latest slot、frame ring、typed payload 和 status/output payload。
- `crates/lichora-ipc-native/`：宿主原生插件 C ABI；typed browser handle 是输入、帧和输出热路径的唯一接口，旧 `*_for_browser` session-handle 导出不再保留。
- `crates/process-host/`：宿主进程管理 C ABI，供无法稳定使用托管进程 API 的宿主环境调用。
- `crates/native-ime/`：native IME C ABI。

文档和发布治理改动不应顺手修改 `src`。协议行为变更必须同时更新 `docs/protocol.md`、`crates/lichora-ipc` tests 和相关设计文档。

## IPC v2 开发约束

- 不在业务模块手写 offset；offset 必须封装在 typed protocol 或 IPC runtime 中。
- 所有跨进程共享结构使用 little-endian。
- 每个通道必须有 version、channel kind、sequence/ack 或 seqlock 提交语义。

Capture 走 `FrameRing`，鼠标移动走 latest-only，事件走 SPSC queue。

## 验证边界

Rust 代码变更需要执行：

```powershell
cargo fmt --all
cargo clippy --all-targets --all-features -- -D warnings
```

仓库级质量检查入口为：

```powershell
.\scripts\check-quality.ps1
```

默认检查顺序为 Rust `cargo fmt --all -- --check`、Rust `cargo clippy --all-targets --all-features -- -D warnings`、Rust `cargo test --no-default-features --all-targets`。脚本支持 `-SkipRust`，用于只验证脚本入口本身。

当前架构依据以根目录 `README.md`、本文档、`protocol.md` 和 `design/` 下的设计文档为准。`archive/` 下的材料只用于历史追溯。
